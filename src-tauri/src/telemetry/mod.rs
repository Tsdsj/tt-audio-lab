use crate::audio::capture::{self, CaptureChunk};
use crate::audio::dsp::{DspParams, SpectrumAnalyzer};
use crate::settings;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeDspConfig {
    pub smoothing: f32,
    pub gain: f32,
    pub emit_interval_ms: u64,
}

#[derive(Clone)]
pub struct RuntimeDspState {
    inner: Arc<Mutex<RuntimeDspConfig>>,
}

/// 可视化运行时状态：用于暂停/恢复前端分析帧推送。
#[derive(Clone, Default)]
pub struct RuntimeVisualState {
    paused: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisFrame {
    timestamp_ms: u64,
    device_id: String,
    bins: Vec<u16>,
    rms: f32,
    peak: f32,
    latency_estimate_ms: f32,
}

impl RuntimeDspState {
    /// 创建运行时 DSP 配置状态，供命令层和分析线程共享。
    pub fn new(config: RuntimeDspConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(config)),
        }
    }

    /// 读取当前运行时 DSP 参数快照。
    pub fn get(&self) -> RuntimeDspConfig {
        self.inner
            .lock()
            .map(|guard| *guard)
            .unwrap_or(RuntimeDspConfig {
                smoothing: 0.58,
                gain: 1.8,
                emit_interval_ms: quality_emit_interval_ms("ultra"),
            })
    }

    /// 更新运行时 DSP 参数，使滑块调节可以立刻生效。
    pub fn set(&self, config: RuntimeDspConfig) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = config;
        }
    }
}

impl RuntimeVisualState {
    /// 设置可视化暂停状态：暂停后仍采集音频，但停止向前端发帧。
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// 查询当前是否处于暂停状态。
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

/// 从持久化设置构建 DSP 初始参数。
pub fn runtime_config_from_settings(settings: &settings::AppSettings) -> RuntimeDspConfig {
    RuntimeDspConfig {
        smoothing: settings.smoothing.clamp(0.0, 0.95),
        gain: settings.gain.clamp(0.2, 6.0),
        emit_interval_ms: quality_emit_interval_ms(&settings.quality),
    }
}

/// 将画质档位映射到 IPC 发帧节流间隔（毫秒）。
fn quality_emit_interval_ms(raw_quality: &str) -> u64 {
    let normalized = raw_quality.trim().to_ascii_lowercase();
    match normalized.as_str() {
        // Ultra：目标约 120Hz（8.3ms），取 8ms。
        "ultra" => 8,
        // High：目标约 90Hz（11.1ms），取 11ms。
        "high" => 11,
        // Balanced：目标约 60Hz（16.7ms），取 16ms。
        "balanced" => 16,
        _ => 11,
    }
}

/// 启动分析事件流：优先真实采集，失败时自动回退模拟数据。
pub fn start_analysis_emitter(
    app: AppHandle,
    runtime_dsp: RuntimeDspState,
    runtime_visual: RuntimeVisualState,
) {
    thread::spawn(move || {
        if let Err(error) =
            run_realtime_analysis_loop(app.clone(), runtime_dsp.clone(), runtime_visual.clone())
        {
            eprintln!("realtime audio loop failed, fallback to mock emitter: {error}");
            run_mock_analysis_loop(app, runtime_dsp, runtime_visual);
        }
    });
}

/// 实时链路：采集线程 -> 样本缓存 -> 频谱分析 -> 向前端推送事件。
fn run_realtime_analysis_loop(
    app: AppHandle,
    runtime_dsp: RuntimeDspState,
    runtime_visual: RuntimeVisualState,
) -> Result<(), String> {
    let (chunk_tx, chunk_rx) = mpsc::channel::<CaptureChunk>();
    let runtime = capture::start_loopback_capture(chunk_tx)?;

    let initial = runtime_dsp.get();
    let mut last_config = initial;
    let mut analyzer = SpectrumAnalyzer::new(
        64,
        1024,
        DspParams {
            smoothing: initial.smoothing,
            gain: initial.gain,
        },
    );

    let mut sample_buffer = Vec::<f32>::with_capacity(8192);
    let mut latest_capture_ts = now_timestamp_ms();
    let mut last_emit_ts = 0u64;

    // 持有流句柄，避免采集对象被释放后回调停止。
    let _stream_guard = runtime.stream;

    loop {
        match chunk_rx.recv_timeout(Duration::from_millis(20)) {
            Ok(chunk) => {
                latest_capture_ts = chunk.timestamp_ms;
                sample_buffer.extend_from_slice(&chunk.samples);

                let max_buffer = analyzer.required_samples() * 8;
                if sample_buffer.len() > max_buffer {
                    let drain_count = sample_buffer.len() - analyzer.required_samples() * 4;
                    sample_buffer.drain(0..drain_count);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("audio capture channel disconnected".to_string());
            }
        }

        let now_ts = now_timestamp_ms();
        let current_config = runtime_dsp.get();
        if now_ts.saturating_sub(last_emit_ts) < current_config.emit_interval_ms {
            continue;
        }

        if sample_buffer.len() < analyzer.required_samples() {
            continue;
        }

        // 关键行：每次推送前读取运行时参数，保证平滑、增益、发帧频率都“实时生效”。
        if (current_config.smoothing - last_config.smoothing).abs() > f32::EPSILON
            || (current_config.gain - last_config.gain).abs() > f32::EPSILON
        {
            analyzer.set_params(DspParams {
                smoothing: current_config.smoothing,
                gain: current_config.gain,
            });
            last_config = current_config;
        }

        let frame_window_start = sample_buffer.len() - analyzer.required_samples();
        let analysis = analyzer.analyze(&sample_buffer[frame_window_start..]);

        // 延迟估算：采样到当前推送的时间差 + 当前发送节流间隔。
        let latency_ms =
            now_ts.saturating_sub(latest_capture_ts) as f32 + current_config.emit_interval_ms as f32;

        if runtime_visual.is_paused() {
            continue;
        }

        let frame = AnalysisFrame {
            timestamp_ms: now_ts,
            device_id: runtime.device_id.clone(),
            bins: analysis.bins,
            rms: analysis.rms,
            peak: analysis.peak,
            latency_estimate_ms: latency_ms,
        };

        let _ = app.emit("audio:analysis_frame", frame);
        last_emit_ts = now_ts;
    }
}

/// 模拟链路：真实采集不可用时提供可预测波形，便于前端验证渲染逻辑。
fn run_mock_analysis_loop(
    app: AppHandle,
    runtime_dsp: RuntimeDspState,
    runtime_visual: RuntimeVisualState,
) {
    let mut phase: f32 = 0.0;

    loop {
        let emit_interval_ms = runtime_dsp.get().emit_interval_ms;

        if runtime_visual.is_paused() {
            thread::sleep(Duration::from_millis(emit_interval_ms));
            continue;
        }

        phase += 0.09;
        let bins = (0..64)
            .map(|index| {
                let energy = ((phase + index as f32 * 0.2).sin() * 0.5 + 0.5) * 1023.0;
                energy.round() as u16
            })
            .collect::<Vec<_>>();

        let now_ts = now_timestamp_ms();
        let frame = AnalysisFrame {
            timestamp_ms: now_ts,
            device_id: "mock-device".to_string(),
            bins,
            rms: ((phase * 1.2).sin() * 0.5 + 0.5).clamp(0.0, 1.0),
            peak: ((phase * 0.7).cos() * 0.5 + 0.5).clamp(0.0, 1.0),
            latency_estimate_ms: emit_interval_ms as f32 + 4.0,
        };

        let _ = app.emit("audio:analysis_frame", frame);
        thread::sleep(Duration::from_millis(emit_interval_ms));
    }
}

/// 统一毫秒时间戳函数，避免多处实现不一致。
fn now_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
