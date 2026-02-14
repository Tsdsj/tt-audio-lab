import type { AppSettings, QualityTier, WindowMode } from "@/types";

const SETTINGS_KEY = "tt-audio-lab.settings";
const QUALITY_SET: QualityTier[] = ["ultra", "high", "balanced"];
const WINDOW_MODE_SET: WindowMode[] = ["normal", "desktopWidget", "overlay"];

export const defaultSettings: AppSettings = {
  quality: "ultra",
  smoothing: 0.58,
  gain: 1.8,
  clickThrough: false,
  launchAtStartup: false,
  windowMode: "normal",
  targetMonitorId: ""
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * 校验画质档位，只允许预定义枚举值。
 */
function normalizeQuality(value: unknown): QualityTier {
  return QUALITY_SET.includes(value as QualityTier) ? (value as QualityTier) : defaultSettings.quality;
}

/**
 * 校验窗口模式，非法值统一回退普通窗口。
 */
function normalizeWindowMode(value: unknown): WindowMode {
  return WINDOW_MODE_SET.includes(value as WindowMode) ? (value as WindowMode) : defaultSettings.windowMode;
}

/**
 * 统一收敛设置范围，防止异常值影响可视化稳定性。
 */
export function normalizeSettings(input: Partial<AppSettings> | null | undefined): AppSettings {
  return {
    quality: normalizeQuality(input?.quality),
    smoothing: clamp(Number(input?.smoothing ?? defaultSettings.smoothing), 0, 0.95),
    gain: clamp(Number(input?.gain ?? defaultSettings.gain), 0.2, 6),
    clickThrough: Boolean(input?.clickThrough ?? defaultSettings.clickThrough),
    launchAtStartup: Boolean(input?.launchAtStartup ?? defaultSettings.launchAtStartup),
    windowMode: normalizeWindowMode(input?.windowMode),
    targetMonitorId: typeof input?.targetMonitorId === "string" ? input.targetMonitorId : ""
  };
}

/**
 * 从浏览器本地存储加载设置，解析失败时返回 null。
 */
export function loadLocalSettings(): AppSettings | null {
  const raw = localStorage.getItem(SETTINGS_KEY);
  if (!raw) {
    return null;
  }

  try {
    return normalizeSettings(JSON.parse(raw) as Partial<AppSettings>);
  } catch {
    return null;
  }
}

/**
 * 保存设置到本地存储。
 */
export function saveLocalSettings(settings: AppSettings): void {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}
