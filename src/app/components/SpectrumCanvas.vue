<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { spectrumFrameBins } from "@/visualization/frame-store";
import type { NonBarsVisualStyle, VisualTuning } from "@/visualization/types";

// Canvas 版本的非柱状图渲染，减少 SVG 大量节点带来的主线程压力。
const props = defineProps<{
  visualStyle: NonBarsVisualStyle;
  tuning: VisualTuning;
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);

let ctx: CanvasRenderingContext2D | null = null;
let rafHandle: number | null = null;
let resizeObserver: ResizeObserver | null = null;
let logicalWidth = 0;
let logicalHeight = 0;

let waterfallPeaks: number[] = [];

function downsampleBins(source: number[], targetCount: number): number[] {
  if (targetCount >= source.length) {
    return source.slice(0, source.length);
  }

  const result = new Array<number>(targetCount).fill(0);
  const bucket = source.length / targetCount;
  for (let index = 0; index < targetCount; index += 1) {
    const start = Math.floor(index * bucket);
    const end = Math.max(start + 1, Math.floor((index + 1) * bucket));
    let peak = 0;
    for (let inner = start; inner < end; inner += 1) {
      peak = Math.max(peak, source[inner] ?? 0);
    }
    result[index] = peak;
  }
  return result;
}

function ensureCanvasSize(): void {
  const canvas = canvasRef.value;
  if (!canvas || !ctx) {
    return;
  }

  const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
  const width = Math.max(1, Math.floor(canvas.clientWidth * dpr));
  const height = Math.max(1, Math.floor(canvas.clientHeight * dpr));
  const viewWidth = Math.max(1, canvas.clientWidth);
  const viewHeight = Math.max(1, canvas.clientHeight);

  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  if (logicalWidth !== viewWidth || logicalHeight !== viewHeight) {
    logicalWidth = viewWidth;
    logicalHeight = viewHeight;
    waterfallPeaks = [];
  }
}

function drawWave(now: number, bins: number[]): void {
  if (!ctx) {
    return;
  }

  const sampleCount = Math.max(18, Math.min(96, Math.floor(logicalWidth / 10)));
  const sampled = downsampleBins(bins, sampleCount);
  const baseY = logicalHeight * 0.5;
  const amp = logicalHeight * 0.45;

  ctx.lineWidth = 1.8;
  ctx.strokeStyle = "#6de4ff";
  ctx.beginPath();
  for (let index = 0; index < sampled.length; index += 1) {
    const x = (index / Math.max(1, sampled.length - 1)) * logicalWidth;
    const wave = Math.sin((index * 0.32) + now * 0.0035) * 0.06;
    const y = baseY - (sampled[index] + wave) * amp;
    if (index === 0) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();
}

function drawRadial(now: number, bins: number[]): void {
  if (!ctx) {
    return;
  }

  const count = Math.max(20, Math.min(72, bins.length));
  const sampled = downsampleBins(bins, count);
  const cx = logicalWidth * 0.5;
  const cy = logicalHeight * 0.5;
  const baseRadius = Math.min(logicalWidth, logicalHeight) * 0.16;

  ctx.strokeStyle = "rgba(120, 238, 200, 0.85)";
  ctx.lineWidth = 1.1;
  for (let index = 0; index < sampled.length; index += 1) {
    const v = sampled[index];
    const angle = (index / sampled.length) * Math.PI * 2 + now * 0.00075;
    const r = baseRadius + v * Math.min(logicalWidth, logicalHeight) * 0.32;
    const x1 = cx + Math.cos(angle) * baseRadius;
    const y1 = cy + Math.sin(angle) * baseRadius;
    const x2 = cx + Math.cos(angle) * r;
    const y2 = cy + Math.sin(angle) * r;
    ctx.globalAlpha = 0.2 + v * 0.8;
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
}

function drawMirror(now: number, bins: number[]): void {
  if (!ctx) {
    return;
  }

  const count = Math.max(16, Math.min(80, Math.floor(logicalWidth / 8)));
  const sampled = downsampleBins(bins, count);
  const mid = logicalHeight * 0.5;

  ctx.strokeStyle = "rgba(119, 230, 255, 0.86)";
  ctx.lineWidth = 1;
  for (let index = 0; index < sampled.length; index += 1) {
    const v = sampled[index];
    const x = (index / Math.max(1, sampled.length - 1)) * logicalWidth;
    const pulse = (Math.sin(now * 0.004 + index * 0.42) + 1) * 0.5;
    const reach = (0.08 + v * 0.42 + pulse * 0.08) * logicalHeight;
    ctx.globalAlpha = 0.2 + v * 0.75;
    ctx.beginPath();
    ctx.moveTo(x, mid - reach);
    ctx.lineTo(x, mid + reach);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
}

function drawSpiral(now: number, bins: number[]): void {
  if (!ctx) {
    return;
  }

  const sampled = downsampleBins(bins, Math.max(22, Math.min(90, bins.length)));
  const cx = logicalWidth * 0.5;
  const cy = logicalHeight * 0.5;
  const maxRadius = Math.min(logicalWidth, logicalHeight) * 0.44;

  ctx.strokeStyle = "#6ce4ff";
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  for (let index = 0; index < sampled.length; index += 1) {
    const progress = index / Math.max(1, sampled.length - 1);
    const v = sampled[index];
    const angle = progress * Math.PI * 7 + now * 0.0024;
    const radius = maxRadius * (0.12 + progress * 0.88) * (0.65 + v * 0.45);
    const x = cx + Math.cos(angle) * radius;
    const y = cy + Math.sin(angle) * radius;
    if (index === 0) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();
}

function drawMatrix(now: number, bins: number[]): void {
  if (!ctx) {
    return;
  }

  const rows = 8;
  const cols = 8;
  const sampled = downsampleBins(bins, rows * cols);
  const gap = 2;
  const size = Math.min((logicalWidth - gap * (cols + 1)) / cols, (logicalHeight - gap * (rows + 1)) / rows);
  const offsetX = (logicalWidth - (size * cols + gap * (cols - 1))) * 0.5;
  const offsetY = (logicalHeight - (size * rows + gap * (rows - 1))) * 0.5;

  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) {
      const index = row * cols + col;
      const v = sampled[index] ?? 0;
      const pulse = (Math.sin(now * 0.005 + index * 0.36) + 1) * 0.5;
      const alpha = Math.min(1, 0.15 + v * 0.7 + pulse * 0.15);
      const x = offsetX + col * (size + gap);
      const y = offsetY + row * (size + gap);
      ctx.fillStyle = `rgba(99, 232, 255, ${alpha.toFixed(3)})`;
      ctx.fillRect(x, y, size, size);
    }
  }
}

function drawParticles(now: number, bins: number[], tuning: VisualTuning): void {
  if (!ctx) {
    return;
  }

  const count = Math.max(12, Math.min(52, Math.round(12 + tuning.particlesDensity * 20)));
  const sampled = downsampleBins(bins, count);
  const cx = logicalWidth * 0.5;
  const cy = logicalHeight * 0.5;
  const speed = tuning.particlesSpeed;
  const glow = tuning.particlesGlow;
  const core = Math.min(logicalWidth, logicalHeight) * (0.06 + 0.05 * glow);

  ctx.fillStyle = `rgba(120, 238, 208, ${(0.16 + glow * 0.09).toFixed(3)})`;
  ctx.beginPath();
  ctx.arc(cx, cy, core, 0, Math.PI * 2);
  ctx.fill();

  for (let index = 0; index < sampled.length; index += 1) {
    const progress = index / Math.max(1, sampled.length - 1);
    const v = sampled[index];
    const angle = progress * Math.PI * 2 + now * 0.0011 * speed;
    const radial = core * (1.1 + v * (2.2 + glow * 0.9)) + ((now * 0.006 * speed + index * 1.1) % 6);
    const x = cx + Math.cos(angle) * radial;
    const y = cy + Math.sin(angle) * radial * 0.78;
    const r = 0.8 + v * (1.4 + glow);
    ctx.globalAlpha = Math.min(1, 0.2 + v * 0.8);
    ctx.fillStyle = "rgba(116, 233, 255, 0.92)";
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}

function drawWaterfall(now: number, bins: number[], tuning: VisualTuning): void {
  if (!ctx) {
    return;
  }

  const count = Math.max(10, Math.min(42, Math.round(10 + tuning.waterfallDensity * 16)));
  const sampled = downsampleBins(bins, count);
  const speed = tuning.waterfallSpeed;
  const trail = tuning.waterfallTrail;
  const gap = 2;
  const width = Math.max(1, (logicalWidth - gap * (count + 1)) / count);

  if (waterfallPeaks.length !== count) {
    waterfallPeaks = new Array<number>(count).fill(0);
  }

  for (let index = 0; index < count; index += 1) {
    const v = sampled[index] ?? 0;
    const pulse = (Math.sin(now * 0.004 * speed + index * 0.41) + 1) * 0.5;
    const intensity = Math.min(1, v * 0.86 + pulse * 0.24);
    const x = gap + index * (width + gap);
    const head = (0.1 + intensity * (0.34 + trail * 0.22)) * logicalHeight;

    waterfallPeaks[index] = Math.max(intensity, (waterfallPeaks[index] ?? 0) - (0.012 + speed * 0.003));
    const peakHeight = (0.18 + waterfallPeaks[index] * (0.4 + trail * 0.26)) * logicalHeight;

    ctx.fillStyle = `rgba(92, 214, 245, ${(0.14 + waterfallPeaks[index] * 0.38).toFixed(3)})`;
    ctx.fillRect(x, logicalHeight - peakHeight, width, peakHeight);

    ctx.fillStyle = `rgba(136, 243, 255, ${(0.2 + intensity * 0.75).toFixed(3)})`;
    ctx.fillRect(x, logicalHeight - head, width, head);
  }
}

function drawRadar(now: number, bins: number[], tuning: VisualTuning): void {
  if (!ctx) {
    return;
  }

  const cx = logicalWidth * 0.5;
  const cy = logicalHeight * 0.5;
  const radius = Math.min(logicalWidth, logicalHeight) * 0.42;
  const speed = tuning.radarSpeed;
  const glow = tuning.radarGlow;
  const density = Math.max(10, Math.min(58, Math.round(10 + tuning.radarDensity * 22)));
  const sampled = downsampleBins(bins, density);

  ctx.strokeStyle = "rgba(136, 220, 206, 0.24)";
  ctx.lineWidth = 1;
  for (let ring = 1; ring <= 4; ring += 1) {
    ctx.beginPath();
    ctx.arc(cx, cy, (radius * ring) / 4, 0, Math.PI * 2);
    ctx.stroke();
  }

  ctx.beginPath();
  ctx.moveTo(cx - radius, cy);
  ctx.lineTo(cx + radius, cy);
  ctx.moveTo(cx, cy - radius);
  ctx.lineTo(cx, cy + radius);
  ctx.stroke();

  const sweepAngle = (now * 0.0016 * speed) % (Math.PI * 2);
  const sweepWidth = 0.18 + glow * 0.12;
  ctx.fillStyle = `rgba(118, 236, 206, ${(0.1 + glow * 0.16).toFixed(3)})`;
  ctx.beginPath();
  ctx.moveTo(cx, cy);
  ctx.arc(cx, cy, radius, sweepAngle - sweepWidth, sweepAngle + sweepWidth);
  ctx.closePath();
  ctx.fill();

  for (let index = 0; index < sampled.length; index += 1) {
    const v = sampled[index];
    const angle = (index / sampled.length) * Math.PI * 2 + now * 0.0005 * speed;
    const r = radius * (0.18 + v * (0.62 + glow * 0.16));
    const x = cx + Math.cos(angle) * r;
    const y = cy + Math.sin(angle) * r;
    const dot = 1 + v * (1.8 + glow * 0.7);
    ctx.globalAlpha = Math.min(1, 0.18 + v * 0.78);
    ctx.fillStyle = "rgba(140, 255, 225, 0.95)";
    ctx.beginPath();
    ctx.arc(x, y, dot, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.globalAlpha = 1;
}

function drawFrame(now: number): void {
  if (!ctx) {
    return;
  }

  ensureCanvasSize();
  ctx.clearRect(0, 0, logicalWidth, logicalHeight);

  const bins = spectrumFrameBins.value.length > 0 ? spectrumFrameBins.value : [0];
  switch (props.visualStyle) {
    case "wave":
      drawWave(now, bins);
      break;
    case "radial":
      drawRadial(now, bins);
      break;
    case "mirror":
      drawMirror(now, bins);
      break;
    case "spiral":
      drawSpiral(now, bins);
      break;
    case "matrix":
      drawMatrix(now, bins);
      break;
    case "particles":
      drawParticles(now, bins, props.tuning);
      break;
    case "waterfall":
      drawWaterfall(now, bins, props.tuning);
      break;
    case "radar":
      drawRadar(now, bins, props.tuning);
      break;
  }
}

function startRenderLoop(): void {
  if (rafHandle !== null) {
    return;
  }

  const render = (now: number): void => {
    drawFrame(now);
    rafHandle = requestAnimationFrame(render);
  };
  rafHandle = requestAnimationFrame(render);
}

function stopRenderLoop(): void {
  if (rafHandle !== null) {
    cancelAnimationFrame(rafHandle);
    rafHandle = null;
  }
}

onMounted(() => {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }

  ctx = canvas.getContext("2d");
  if (!ctx) {
    return;
  }

  ensureCanvasSize();
  startRenderLoop();

  resizeObserver = new ResizeObserver(() => {
    ensureCanvasSize();
    drawFrame(performance.now());
  });
  resizeObserver.observe(canvas);
});

watch(
  () => props.visualStyle,
  () => {
    drawFrame(performance.now());
  }
);

watch(
  () => props.tuning,
  () => {
    drawFrame(performance.now());
  },
  { deep: true }
);

onBeforeUnmount(() => {
  stopRenderLoop();
  resizeObserver?.disconnect();
  resizeObserver = null;
  ctx = null;
});
</script>

<template>
  <canvas ref="canvasRef" class="spectrum-canvas"></canvas>
</template>

<style scoped>
.spectrum-canvas {
  position: relative;
  width: 100%;
  height: 100%;
  display: block;
  border-radius: 12px;
  border: 1px solid rgba(185, 220, 212, 0.25);
  background: linear-gradient(180deg, rgba(7, 16, 25, 0.45), rgba(7, 16, 25, 0.88));
  box-sizing: border-box;
  z-index: 1;
}
</style>
