use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use serde::Serialize;
use std::sync::mpsc::Sender;
use std::time::{SystemTime, UNIX_EPOCH};

/// 采集线程推送给分析线程的数据块，统一使用单声道浮点样本。
#[derive(Debug, Clone)]
pub struct CaptureChunk {
    pub timestamp_ms: u64,
    pub samples: Vec<f32>,
}

/// 当前采集会话句柄，`stream` 生命周期必须被持有，否则系统采集会停止。
pub struct CaptureRuntime {
    pub stream: Stream,
    pub device_id: String,
    pub sample_rate: u32,
    pub channels: u16,
}

/// 前端设备选择面板可用的数据结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: String,
}

/// 统一毫秒时间戳，便于计算采样到渲染链路时延。
fn now_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

/// 列出输入/输出设备，供前端后续做设备切换。
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            let name = device
                .name()
                .unwrap_or_else(|_| "Unknown Input Device".to_string());
            devices.push(AudioDeviceInfo {
                id: format!("input:{name}"),
                name,
                direction: "input".to_string(),
            });
        }
    }

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            let name = device
                .name()
                .unwrap_or_else(|_| "Unknown Output Device".to_string());
            devices.push(AudioDeviceInfo {
                id: format!("output:{name}"),
                name,
                direction: "output".to_string(),
            });
        }
    }

    if devices.is_empty() {
        return Err("no audio devices found".to_string());
    }

    Ok(devices)
}

/// 启动采集流：优先尝试默认输出设备（WASAPI loopback 候选），失败后降级为默认输入设备。
pub fn start_loopback_capture(sender: Sender<CaptureChunk>) -> Result<CaptureRuntime, String> {
    let host = cpal::default_host();
    let mut output_attempt_error = String::new();

    if let Some(output_device) = host.default_output_device() {
        let output_name = output_device
            .name()
            .unwrap_or_else(|_| "Default Output".to_string());

        match output_device.default_output_config() {
            Ok(config) => {
                match build_input_stream_for_config(&output_device, config.clone(), sender.clone())
                {
                    Ok(stream) => {
                        stream.play().map_err(|err| {
                            format!("failed to play output loopback stream: {err}")
                        })?;
                        return Ok(CaptureRuntime {
                            stream,
                            device_id: format!("output:{output_name}"),
                            sample_rate: config.sample_rate().0,
                            channels: config.channels(),
                        });
                    }
                    Err(err) => {
                        output_attempt_error = format!("output loopback failed: {err}");
                    }
                }
            }
            Err(err) => {
                output_attempt_error = format!("failed to read output config: {err}");
            }
        }
    }

    let input_device = host
        .default_input_device()
        .ok_or_else(|| format!("no default input device available; {output_attempt_error}"))?;
    let input_name = input_device
        .name()
        .unwrap_or_else(|_| "Default Input".to_string());
    let input_config = input_device
        .default_input_config()
        .map_err(|err| format!("failed to read input config: {err}"))?;
    let stream = build_input_stream_for_config(&input_device, input_config.clone(), sender)?;
    stream
        .play()
        .map_err(|err| format!("failed to play input capture stream: {err}"))?;

    Ok(CaptureRuntime {
        stream,
        device_id: format!("input:{input_name}"),
        sample_rate: input_config.sample_rate().0,
        channels: input_config.channels(),
    })
}

/// 基于设备采样格式创建输入流，并把多声道样本折叠为单声道发送到分析线程。
fn build_input_stream_for_config(
    device: &cpal::Device,
    supported_config: SupportedStreamConfig,
    sender: Sender<CaptureChunk>,
) -> Result<Stream, String> {
    let stream_config: StreamConfig = supported_config.clone().into();
    let channels = stream_config.channels as usize;
    let error_callback = |error| eprintln!("audio stream error: {error}");

    match supported_config.sample_format() {
        SampleFormat::F32 => {
            let sender_f32 = sender.clone();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| push_mono_f32(data, channels, &sender_f32),
                    error_callback,
                    None,
                )
                .map_err(|err| format!("failed to build f32 input stream: {err}"))
        }
        SampleFormat::I16 => {
            let sender_i16 = sender.clone();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| push_mono_i16(data, channels, &sender_i16),
                    error_callback,
                    None,
                )
                .map_err(|err| format!("failed to build i16 input stream: {err}"))
        }
        SampleFormat::U16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[u16], _| push_mono_u16(data, channels, &sender),
                error_callback,
                None,
            )
            .map_err(|err| format!("failed to build u16 input stream: {err}")),
        _ => Err(format!(
            "unsupported sample format: {:?}",
            supported_config.sample_format()
        )),
    }
}

/// 处理 `f32` 样本并折叠为单声道，减少后续分析计算量。
fn push_mono_f32(samples: &[f32], channels: usize, sender: &Sender<CaptureChunk>) {
    if channels == 0 || samples.is_empty() {
        return;
    }

    let mut mono = Vec::with_capacity(samples.len() / channels + 1);
    for frame in samples.chunks(channels) {
        let sum = frame.iter().copied().sum::<f32>();
        mono.push(sum / frame.len() as f32);
    }

    let _ = sender.send(CaptureChunk {
        timestamp_ms: now_timestamp_ms(),
        samples: mono,
    });
}

/// 处理 `i16` 样本并标准化到 `[-1, 1]` 区间。
fn push_mono_i16(samples: &[i16], channels: usize, sender: &Sender<CaptureChunk>) {
    if channels == 0 || samples.is_empty() {
        return;
    }

    let mut mono = Vec::with_capacity(samples.len() / channels + 1);
    for frame in samples.chunks(channels) {
        let sum = frame
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .sum::<f32>();
        mono.push(sum / frame.len() as f32);
    }

    let _ = sender.send(CaptureChunk {
        timestamp_ms: now_timestamp_ms(),
        samples: mono,
    });
}

/// 处理 `u16` 样本并映射到 `[-1, 1]` 区间，保持不同格式处理一致性。
fn push_mono_u16(samples: &[u16], channels: usize, sender: &Sender<CaptureChunk>) {
    if channels == 0 || samples.is_empty() {
        return;
    }

    let mut mono = Vec::with_capacity(samples.len() / channels + 1);
    for frame in samples.chunks(channels) {
        let sum = frame
            .iter()
            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
            .sum::<f32>();
        mono.push(sum / frame.len() as f32);
    }

    let _ = sender.send(CaptureChunk {
        timestamp_ms: now_timestamp_ms(),
        samples: mono,
    });
}
