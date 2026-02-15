export type VisualStyle =
  | "bars"
  | "wave"
  | "radial"
  | "mirror"
  | "spiral"
  | "matrix"
  | "particles"
  | "waterfall"
  | "radar";

export type NonBarsVisualStyle = Exclude<VisualStyle, "bars">;

export interface VisualTuning {
  particlesSpeed: number;
  particlesDensity: number;
  particlesGlow: number;
  waterfallSpeed: number;
  waterfallDensity: number;
  waterfallTrail: number;
  radarSpeed: number;
  radarDensity: number;
  radarGlow: number;
}

export const DEFAULT_VISUAL_TUNING: VisualTuning = {
  particlesSpeed: 1,
  particlesDensity: 1,
  particlesGlow: 1,
  waterfallSpeed: 1,
  waterfallDensity: 1,
  waterfallTrail: 1,
  radarSpeed: 1,
  radarDensity: 1,
  radarGlow: 1
};
