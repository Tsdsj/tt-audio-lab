// 画质档位与后端保持一致，用于 60/120/240Hz 策略切换。
export type QualityTier = "ultra" | "high" | "balanced";

export type WindowMode = "normal" | "desktopWidget" | "overlay";

export interface AppSettings {
  quality: QualityTier;
  smoothing: number;
  gain: number;
  clickThrough: boolean;
  launchAtStartup: boolean;
  windowMode: WindowMode;
  targetMonitorId: string;
}

export interface AnalysisFrame {
  timestampMs: number;
  deviceId: string;
  bins: number[];
  rms: number;
  peak: number;
  latencyEstimateMs: number;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  direction: "input" | "output" | string;
}

export interface MonitorInfo {
  id: string;
  label: string;
  width: number;
  height: number;
  scaleFactor: number;
  isPrimary: boolean;
  isCurrent: boolean;
}
