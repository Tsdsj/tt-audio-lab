use std::f32::consts::PI;

/// 频谱分析结果，会被量化后发送给前端渲染层。
#[derive(Debug, Clone)]
pub struct SpectrumFrame {
    pub bins: Vec<u16>,
    pub rms: f32,
    pub peak: f32,
}

/// 分析参数：平滑和增益直接影响视觉响应速度和幅度。
#[derive(Debug, Clone, Copy)]
pub struct DspParams {
    pub smoothing: f32,
    pub gain: f32,
}

impl Default for DspParams {
    fn default() -> Self {
        Self {
            smoothing: 0.58,
            gain: 1.8,
        }
    }
}

/// 频谱分析器：窗口化 + DFT + 频段均衡 + 平滑后处理。
pub struct SpectrumAnalyzer {
    bin_count: usize,
    window_size: usize,
    params: DspParams,
    previous_bins: Vec<f32>,
    band_baseline: Vec<f32>,
}

impl SpectrumAnalyzer {
    /// 创建分析器并初始化平滑缓存与频段基线。
    pub fn new(bin_count: usize, window_size: usize, params: DspParams) -> Self {
        Self {
            bin_count,
            window_size,
            params,
            previous_bins: vec![0.0; bin_count],
            band_baseline: vec![0.02; bin_count],
        }
    }

    /// 返回最小样本窗口，调用方据此控制缓冲区长度。
    pub fn required_samples(&self) -> usize {
        self.window_size
    }

    /// 更新分析参数，供运行时滑块调整立即生效。
    pub fn set_params(&mut self, params: DspParams) {
        self.params = params;
    }

    /// 对采样窗口做分析并输出量化频谱、RMS、峰值。
    pub fn analyze(&mut self, samples: &[f32]) -> SpectrumFrame {
        let window = prepare_window(samples, self.window_size);
        let rms = calculate_rms(&window);
        let peak = calculate_peak(&window);

        let max_k = (window.len() / 2).saturating_sub(1).max(1);
        let mut raw_bins = Vec::with_capacity(self.bin_count);

        for index in 0..self.bin_count {
            let mapped_k = mixed_mapped_frequency_bin(index, self.bin_count, max_k);
            let magnitude = calculate_dft_magnitude(&window, mapped_k);
            let energy = magnitude * self.params.gain * 180.0;

            // 关键行：先 log 压缩，再按频段历史基线做自适应均衡，避免只动某几个频段。
            let compressed = ((1.0 + energy).ln() / (1.0 + 180.0f32).ln()).clamp(0.0, 1.0);
            let baseline = self.band_baseline[index];
            self.band_baseline[index] = baseline * 0.992 + compressed * 0.008;
            let whitened = (compressed / (self.band_baseline[index] * 1.6 + 0.015)).clamp(0.0, 1.0);

            raw_bins.push(whitened);
        }

        // 关键行：注入全局能量，让低活跃频段也保持可见动态，但不覆盖频率结构差异。
        let global_motion = (rms * 0.8 + peak * 0.6).clamp(0.0, 1.0);
        for value in &mut raw_bins {
            *value = (*value * 0.84 + global_motion * 0.16).clamp(0.0, 1.0);
        }

        let spread_bins = diffuse_neighbors(&raw_bins);
        let mut bins = Vec::with_capacity(self.bin_count);

        for (index, value) in spread_bins.into_iter().enumerate() {
            let smoothed = self.previous_bins[index] * self.params.smoothing
                + value * (1.0 - self.params.smoothing);
            self.previous_bins[index] = smoothed;
            bins.push((smoothed * 1023.0).round() as u16);
        }

        SpectrumFrame { bins, rms, peak }
    }
}

/// 对每个频段做邻域扩散，减少“只动局部几根柱子”的割裂感。
fn diffuse_neighbors(values: &[f32]) -> Vec<f32> {
    if values.is_empty() {
        return Vec::new();
    }

    let mut output = vec![0.0; values.len()];
    for (index, value) in values.iter().copied().enumerate() {
        let left = if index > 0 { values[index - 1] } else { value };
        let right = if index + 1 < values.len() {
            values[index + 1]
        } else {
            value
        };

        output[index] = (value * 0.64 + left * 0.18 + right * 0.18).clamp(0.0, 1.0);
    }
    output
}

/// 生成固定窗口样本并应用 Hann 窗，降低频谱泄漏。
fn prepare_window(samples: &[f32], window_size: usize) -> Vec<f32> {
    let mut output = Vec::with_capacity(window_size);
    if samples.is_empty() {
        output.resize(window_size, 0.0);
        return output;
    }

    let start = samples.len().saturating_sub(window_size);
    let slice = &samples[start..];

    if slice.len() < window_size {
        output.resize(window_size - slice.len(), 0.0);
    }
    output.extend_from_slice(slice);

    let n = output.len().max(2) as f32;
    for (i, value) in output.iter_mut().enumerate() {
        let phase = i as f32 / (n - 1.0);
        let hann = 0.5 - 0.5 * (2.0 * PI * phase).cos();
        *value *= hann;
    }

    output
}

/// 计算短时均方根，用于前端展示整体能量。
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let square_sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (square_sum / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

/// 计算峰值包络，帮助前端做冲击感响应。
fn calculate_peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0f32, f32::max)
        .clamp(0.0, 1.0)
}

/// 对目标频点计算 DFT 幅值，窗口较小时可接受且依赖更少。
fn calculate_dft_magnitude(samples: &[f32], k: usize) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let n = samples.len() as f32;
    let mut real = 0.0;
    let mut imag = 0.0;

    for (index, sample) in samples.iter().copied().enumerate() {
        let angle = 2.0 * PI * k as f32 * index as f32 / n;
        real += sample * angle.cos();
        imag -= sample * angle.sin();
    }

    (real * real + imag * imag).sqrt() / n
}

/// 混合“对数映射 + 线性映射”，兼顾低频细节和高频活跃度。
fn mixed_mapped_frequency_bin(bin_index: usize, bin_count: usize, max_k: usize) -> usize {
    if bin_count <= 1 {
        return 1;
    }

    let ratio = bin_index as f32 / (bin_count - 1) as f32;
    let log_ratio = (1.0 + ratio * 9.0).ln() / 10.0f32.ln();
    let mixed_ratio = log_ratio * 0.7 + ratio * 0.3;
    (1.0 + mixed_ratio * max_k as f32).round() as usize
}
