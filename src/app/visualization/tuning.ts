import { DEFAULT_VISUAL_TUNING, type VisualStyle, type VisualTuning } from "@/visualization/types";

// 统一管理可视化风格与特效参数的本地持久化键。
export const VISUAL_STYLE_KEY = "tt-audio-lab.visualStyle";
export const VISUAL_TUNING_KEY = "tt-audio-lab.visualTuning";

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }
  return Math.min(max, Math.max(min, value));
}

export function normalizeVisualTuning(raw: Partial<VisualTuning> | null | undefined): VisualTuning {
  const source = raw ?? {};
  return {
    particlesSpeed: clampNumber(source.particlesSpeed, 0.4, 2.4, DEFAULT_VISUAL_TUNING.particlesSpeed),
    particlesDensity: clampNumber(source.particlesDensity, 0.4, 2.0, DEFAULT_VISUAL_TUNING.particlesDensity),
    particlesGlow: clampNumber(source.particlesGlow, 0, 1.8, DEFAULT_VISUAL_TUNING.particlesGlow),
    waterfallSpeed: clampNumber(source.waterfallSpeed, 0.4, 2.4, DEFAULT_VISUAL_TUNING.waterfallSpeed),
    waterfallDensity: clampNumber(source.waterfallDensity, 0.5, 2.2, DEFAULT_VISUAL_TUNING.waterfallDensity),
    waterfallTrail: clampNumber(source.waterfallTrail, 0.2, 1.8, DEFAULT_VISUAL_TUNING.waterfallTrail),
    radarSpeed: clampNumber(source.radarSpeed, 0.4, 2.4, DEFAULT_VISUAL_TUNING.radarSpeed),
    radarDensity: clampNumber(source.radarDensity, 0.4, 2.2, DEFAULT_VISUAL_TUNING.radarDensity),
    radarGlow: clampNumber(source.radarGlow, 0, 1.8, DEFAULT_VISUAL_TUNING.radarGlow)
  };
}

export function loadVisualStyle(): VisualStyle {
  const raw = localStorage.getItem(VISUAL_STYLE_KEY);
  if (
    raw === "bars" ||
    raw === "wave" ||
    raw === "radial" ||
    raw === "mirror" ||
    raw === "spiral" ||
    raw === "matrix" ||
    raw === "particles" ||
    raw === "waterfall" ||
    raw === "radar"
  ) {
    return raw;
  }
  return "bars";
}

export function saveVisualStyle(style: VisualStyle): void {
  localStorage.setItem(VISUAL_STYLE_KEY, style);
}

export function loadVisualTuning(): VisualTuning {
  const raw = localStorage.getItem(VISUAL_TUNING_KEY);
  if (!raw) {
    return { ...DEFAULT_VISUAL_TUNING };
  }
  try {
    const parsed = JSON.parse(raw) as Partial<VisualTuning>;
    return normalizeVisualTuning(parsed);
  } catch {
    return { ...DEFAULT_VISUAL_TUNING };
  }
}

export function saveVisualTuning(tuning: VisualTuning): void {
  localStorage.setItem(VISUAL_TUNING_KEY, JSON.stringify(normalizeVisualTuning(tuning)));
}
