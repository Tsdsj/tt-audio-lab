<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import SettingsModal from "@/components/SettingsModal.vue";
import SpectrumCanvas from "@/components/SpectrumCanvas.vue";
import { zhCN } from "@/locales/zh-CN";
import { defaultSettings, loadLocalSettings, normalizeSettings, saveLocalSettings } from "@/stores/settings";
import { spectrumFrameBins } from "@/visualization/frame-store";
import { loadVisualStyle, loadVisualTuning, normalizeVisualTuning, saveVisualStyle, saveVisualTuning } from "@/visualization/tuning";
import { DEFAULT_VISUAL_TUNING, type NonBarsVisualStyle, type VisualStyle, type VisualTuning } from "@/visualization/types";
import type { AnalysisFrame, AppSettings, AudioDeviceInfo, MonitorInfo, QualityTier, WindowMode } from "@/types";

interface QualityProfile {
  binCount: number;
  frameBudgetMs: number;
  interpolationMs: number;
  staleDecay: number;
}

interface FrameSample {
  ts: number;
  duration: number;
}

type BarsRenderMode = "webgl2" | "canvas2d" | "dom";

interface BarsWebglRenderer {
  gl: WebGL2RenderingContext;
  program: WebGLProgram;
  vertexBuffer: WebGLBuffer;
  positionLocation: number;
  binsLocation: WebGLUniformLocation;
  binCountLocation: WebGLUniformLocation;
  resolutionLocation: WebGLUniformLocation;
  timeLocation: WebGLUniformLocation;
}

const t = zhCN;
// 寮€鍙戞ā寮忔樉绀鸿瘖鏂俊鎭紝鐢熶骇鏋勫缓榛樿闅愯棌銆?
const isDevMode = import.meta.env.DEV;
const PERF_WINDOW_MS = 6000;
const PERF_EVAL_INTERVAL_MS = 1000;
const FPS_WINDOW_MS = 1000;
const AUTO_DOWNGRADE_HOLD_MS = 5000;
const QUALITY_ORDER: QualityTier[] = ["ultra", "high", "balanced"];
const QUALITY_PROFILE: Record<QualityTier, QualityProfile> = {
  ultra: { binCount: 64, frameBudgetMs: 4.2, interpolationMs: 12, staleDecay: 0.995 },
  high: { binCount: 48, frameBudgetMs: 8.3, interpolationMs: 18, staleDecay: 0.992 },
  balanced: { binCount: 32, frameBudgetMs: 16.7, interpolationMs: 24, staleDecay: 0.988 }
};
const initialSettings = loadLocalSettings() ?? { ...defaultSettings };

const settings = ref<AppSettings>(initialSettings);
const editingSettings = ref<AppSettings>({ ...settings.value });
const visualTuning = ref<VisualTuning>(loadVisualTuning());
const editingVisualTuning = ref<VisualTuning>({ ...visualTuning.value });
const editingVisualStyle = ref<VisualStyle>(loadVisualStyle());
const statusText = ref<string>(t.status.idle);
const bins = ref<number[]>(new Array(QUALITY_PROFILE[settings.value.quality].binCount).fill(0));
spectrumFrameBins.value = bins.value;
const deviceLabel = ref("鏈煡璁惧");
const discoveredDevices = ref<AudioDeviceInfo[]>([]);
const discoveredMonitors = ref<MonitorInfo[]>([]);
const visualStyle = ref<VisualStyle>(editingVisualStyle.value);
const settingsOpen = ref(false);
const settingsSaving = ref(false);
const settingsError = ref("");
const immersiveMode = ref(false);
const perfFps = ref(0);
const perfFrameTimeMs = ref(0);
const perfFrameP95Ms = ref(0);
const autoQualityMessage = ref("");
const barsCanvasRef = ref<HTMLCanvasElement | null>(null);
const barsRenderMode = ref<BarsRenderMode>("dom");

const renderBinsBuffer = new Array<number>(64).fill(0);
const targetBinsBuffer = new Array<number>(64).fill(0);
const frameTimeSamples: FrameSample[] = [];

let unlistenFrame: UnlistenFn | null = null;
let unlistenOpenSettings: UnlistenFn | null = null;
let unlistenVisualPause: UnlistenFn | null = null;
let unlistenClickThroughChanged: UnlistenFn | null = null;
let animationFrameHandle: number | null = null;
let lastRenderTs = 0;
let lastAnalysisTs = 0;
let lastPerfEvalTs = 0;
let downgradeBreachStartTs = 0;
let autoDowngradePending = false;
let qualityMessageTimer: number | null = null;
let bars2dContext: CanvasRenderingContext2D | null = null;
let barsWebglRenderer: BarsWebglRenderer | null = null;

const barsUniformBuffer = new Float32Array(64);

const styleOptions: Array<{ label: string; value: VisualStyle }> = [
  { label: t.visualizer.styles.bars, value: "bars" },
  { label: t.visualizer.styles.wave, value: "wave" },
  { label: t.visualizer.styles.radial, value: "radial" },
  { label: t.visualizer.styles.mirror, value: "mirror" },
  { label: t.visualizer.styles.spiral, value: "spiral" },
  { label: t.visualizer.styles.matrix, value: "matrix" },
  { label: t.visualizer.styles.particles, value: "particles" },
  { label: t.visualizer.styles.waterfall, value: "waterfall" },
  { label: t.visualizer.styles.radar, value: "radar" }
];

const qualityOptions: Array<{ label: string; value: QualityTier }> = [
  { label: t.settings.qualityUltra, value: "ultra" },
  { label: t.settings.qualityHigh, value: "high" },
  { label: t.settings.qualityBalanced, value: "balanced" }
];

const windowModeOptions: Array<{ label: string; value: WindowMode }> = [
  { label: t.windowMode.normal, value: "normal" },
  { label: t.windowMode.desktopWidget, value: "desktopWidget" },
  { label: t.windowMode.overlay, value: "overlay" }
];

const activeVisualStyleLabel = computed(() => {
  const currentStyle = settingsOpen.value ? editingVisualStyle.value : visualStyle.value;
  const current = styleOptions.find((option) => option.value === currentStyle);
  return current?.label ?? t.visualizer.styles.bars;
});

const showVisualTuningSection = computed(() => {
  const currentStyle = settingsOpen.value ? editingVisualStyle.value : visualStyle.value;
  return currentStyle === "particles" || currentStyle === "waterfall" || currentStyle === "radar";
});

const nonBarsVisualStyle = computed<NonBarsVisualStyle>(() =>
  visualStyle.value === "bars" ? "wave" : visualStyle.value
);

const activeQualityLabel = computed(() => qualityLabel(settings.value.quality));
const visualRendererLabel = computed(() => {
  if (visualStyle.value !== "bars") {
    return "Canvas2D";
  }
  if (barsRenderMode.value === "webgl2") {
    return "WebGL2";
  }
  if (barsRenderMode.value === "canvas2d") {
    return "Canvas2D";
  }
  return "DOM";
});
const frameTimeDisplay = computed(() => perfFrameTimeMs.value.toFixed(2));
const frameP95Display = computed(() => perfFrameP95Ms.value.toFixed(2));
const budgetDisplay = computed(() => QUALITY_PROFILE[settings.value.quality].frameBudgetMs.toFixed(1));

/**
 * 鍒涘缓 WebGL shader锛涘け璐ユ椂杩斿洖 null 骞惰褰曠紪璇戞棩蹇椼€?
 */
function createWebglShader(
  gl: WebGL2RenderingContext,
  type: number,
  source: string
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) {
    return null;
  }

  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.warn("WebGL shader compile failed:", gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

/**
 * 鍒涘缓 WebGL program锛涗换涓€ shader 澶辫触鏃惰嚜鍔ㄥ洖鏀惰祫婧愩€?
 */
function createWebglProgram(
  gl: WebGL2RenderingContext,
  vertexSource: string,
  fragmentSource: string
): WebGLProgram | null {
  const vertexShader = createWebglShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = createWebglShader(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vertexShader || !fragmentShader) {
    if (vertexShader) {
      gl.deleteShader(vertexShader);
    }
    if (fragmentShader) {
      gl.deleteShader(fragmentShader);
    }
    return null;
  }

  const program = gl.createProgram();
  if (!program) {
    gl.deleteShader(vertexShader);
    gl.deleteShader(fragmentShader);
    return null;
  }

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.warn("WebGL program link failed:", gl.getProgramInfoLog(program));
    gl.deleteProgram(program);
    return null;
  }

  return program;
}

/**
 * 渚濇嵁瀹瑰櫒灏哄涓?DPR 鍚屾 canvas 鍍忕礌澶у皬锛岄伩鍏嶉珮鍒嗗睆妯＄硦銆?
 */
function ensureBarsCanvasSize(canvas: HTMLCanvasElement): { width: number; height: number; dpr: number } {
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const width = Math.max(1, Math.floor(canvas.clientWidth * dpr));
  const height = Math.max(1, Math.floor(canvas.clientHeight * dpr));

  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }

  return { width, height, dpr };
}

/**
 * 閲婃斁鏌辩姸鍥炬覆鏌撳櫒璧勬簮锛岄伩鍏嶆ā寮忓垏鎹㈠悗娈嬬暀 GPU/Canvas 鍙ユ焺銆?
 */
function disposeBarsRenderer(): void {
  if (barsWebglRenderer) {
    const { gl, program, vertexBuffer } = barsWebglRenderer;
    gl.deleteBuffer(vertexBuffer);
    gl.deleteProgram(program);
    barsWebglRenderer = null;
  }

  bars2dContext = null;
  barsRenderMode.value = "dom";
}

/**
 * 鍒濆鍖栨煴鐘跺浘娓叉煋鍣細浼樺厛 WebGL2锛屼笉鍙敤鏃跺洖閫€鍒?Canvas2D銆?
 */
function initBarsRenderer(canvas: HTMLCanvasElement | null): void {
  disposeBarsRenderer();
  if (!canvas) {
    return;
  }

  const webgl = canvas.getContext("webgl2", {
    antialias: false,
    alpha: false,
    depth: false,
    stencil: false,
    preserveDrawingBuffer: false,
    powerPreference: "high-performance"
  });

  if (webgl) {
    const vertexSource = `#version 300 es
      in vec2 a_position;
      out vec2 v_uv;
      void main() {
        v_uv = a_position * 0.5 + 0.5;
        gl_Position = vec4(a_position, 0.0, 1.0);
      }`;

    const fragmentSource = `#version 300 es
      precision highp float;
      in vec2 v_uv;
      out vec4 out_color;
      uniform float u_bins[64];
      uniform int u_bin_count;
      uniform vec2 u_resolution;
      uniform float u_time;

      void main() {
        float count = max(float(u_bin_count), 1.0);
        float x = clamp(v_uv.x, 0.0, 0.999999);
        float y = 1.0 - v_uv.y;
        int bin_index = int(floor(x * count));
        bin_index = clamp(bin_index, 0, max(u_bin_count - 1, 0));

        float amplitude = clamp(u_bins[bin_index], 0.0, 1.0);
        float beat = 0.5 + 0.5 * sin(u_time * 3.2 + float(bin_index) * 0.22);
        float dynamic_amp = clamp(amplitude * (0.84 + beat * 0.18), 0.0, 1.0);
        float local_x = fract(x * count);
        float bar_width_px = u_resolution.x / count;
        float edge = clamp(1.2 / max(bar_width_px, 1.0), 0.02, 0.12);
        float left = smoothstep(edge, edge + 0.03, local_x);
        float right = 1.0 - smoothstep(1.0 - edge - 0.03, 1.0 - edge, local_x);
        float inside_x = left * right;
        float inside_y = step(y, dynamic_amp);
        float bar_mask = inside_x * inside_y;

        vec3 bg_top = vec3(0.03, 0.07, 0.11);
        vec3 bg_bottom = vec3(0.02, 0.05, 0.09);
        vec3 bg = mix(bg_bottom, bg_top, v_uv.y);
        bg += 0.025 * vec3(0.13, 0.35, 0.45) * sin(u_time * 0.7 + v_uv.y * 9.0 + v_uv.x * 4.2);

        vec3 c0 = vec3(1.0, 0.83, 0.48);
        vec3 c1 = vec3(0.18, 0.77, 0.59);
        vec3 c2 = vec3(0.35, 0.86, 1.0);
        vec3 bar = mix(c2, c1, y);
        bar = mix(bar, c0, y * y);
        bar *= 0.86 + 0.14 * sin(u_time * 4.8 + float(bin_index) * 0.17);

        float top_dist = abs(y - dynamic_amp);
        float top_glow = exp(-top_dist * 36.0) * inside_x * (0.35 + dynamic_amp * 0.65);
        vec3 glow_color = mix(vec3(0.38, 0.93, 1.0), vec3(1.0, 0.9, 0.62), dynamic_amp);
        float scan = sin((v_uv.y + u_time * 0.12) * 180.0) * 0.013;
        float vignette = smoothstep(1.08, 0.3, length(v_uv - vec2(0.5)));

        vec3 color = mix(bg, bar, bar_mask);
        color += glow_color * top_glow * 0.8;
        color += scan;
        color *= vignette;
        out_color = vec4(clamp(color, 0.0, 1.0), 1.0);
      }`;

    const program = createWebglProgram(webgl, vertexSource, fragmentSource);
    const vertexBuffer = webgl.createBuffer();
    if (program && vertexBuffer) {
      webgl.bindBuffer(webgl.ARRAY_BUFFER, vertexBuffer);
      webgl.bufferData(
        webgl.ARRAY_BUFFER,
        new Float32Array([
          -1, -1, 1, -1, -1, 1,
          -1, 1, 1, -1, 1, 1
        ]),
        webgl.STATIC_DRAW
      );

      const positionLocation = webgl.getAttribLocation(program, "a_position");
      const binsLocation = webgl.getUniformLocation(program, "u_bins");
      const binCountLocation = webgl.getUniformLocation(program, "u_bin_count");
      const resolutionLocation = webgl.getUniformLocation(program, "u_resolution");
      const timeLocation = webgl.getUniformLocation(program, "u_time");

      if (
        positionLocation >= 0 &&
        binsLocation &&
        binCountLocation &&
        resolutionLocation &&
        timeLocation
      ) {
        barsWebglRenderer = {
          gl: webgl,
          program,
          vertexBuffer,
          positionLocation,
          binsLocation,
          binCountLocation,
          resolutionLocation,
          timeLocation
        };
        barsRenderMode.value = "webgl2";
        return;
      }

      webgl.deleteBuffer(vertexBuffer);
      webgl.deleteProgram(program);
    }
  }

  const context2d = canvas.getContext("2d", { alpha: false, desynchronized: true });
  if (context2d) {
    bars2dContext = context2d;
    barsRenderMode.value = "canvas2d";
    return;
  }

  barsRenderMode.value = "dom";
}

/**
 * 浣跨敤 WebGL2 缁樺埗鏌辩姸棰戣氨锛岄檷浣庨珮鍒锋柊鍦烘櫙涓?DOM 璐熸媴銆?
 */
function drawBarsWithWebgl(nextBins: number[]): void {
  if (!barsWebglRenderer || !barsCanvasRef.value) {
    return;
  }

  const { width, height } = ensureBarsCanvasSize(barsCanvasRef.value);
  const renderer = barsWebglRenderer;
  const { gl } = renderer;

  for (let index = 0; index < 64; index += 1) {
    barsUniformBuffer[index] = nextBins[index] ?? 0;
  }

  gl.viewport(0, 0, width, height);
  gl.useProgram(renderer.program);
  gl.bindBuffer(gl.ARRAY_BUFFER, renderer.vertexBuffer);
  gl.enableVertexAttribArray(renderer.positionLocation);
  gl.vertexAttribPointer(renderer.positionLocation, 2, gl.FLOAT, false, 0, 0);
  gl.uniform1fv(renderer.binsLocation, barsUniformBuffer);
  gl.uniform1i(renderer.binCountLocation, Math.max(1, Math.min(64, nextBins.length)));
  gl.uniform2f(renderer.resolutionLocation, width, height);
  gl.uniform1f(renderer.timeLocation, performance.now() * 0.001);
  gl.drawArrays(gl.TRIANGLES, 0, 6);
}

/**
 * Canvas2D 鍥為€€璺緞锛歐ebGL2 涓嶅彲鐢ㄦ椂浠嶄繚鎸佸钩婊戞煴鐘舵覆鏌撱€?
 */
function drawBarsWithCanvas2d(nextBins: number[]): void {
  if (!bars2dContext || !barsCanvasRef.value) {
    return;
  }

  const { width, height, dpr } = ensureBarsCanvasSize(barsCanvasRef.value);
  const ctx = bars2dContext;
  const count = Math.max(1, nextBins.length);
  const nowSec = performance.now() * 0.001;

  ctx.clearRect(0, 0, width, height);
  const bgGradient = ctx.createLinearGradient(0, 0, 0, height);
  bgGradient.addColorStop(0, "rgba(7, 16, 25, 0.45)");
  bgGradient.addColorStop(1, "rgba(7, 16, 25, 0.88)");
  ctx.fillStyle = bgGradient;
  ctx.fillRect(0, 0, width, height);

  ctx.save();
  ctx.globalAlpha = 0.24;
  for (let row = 1; row <= 5; row += 1) {
    const y = (height / 6) * row;
    ctx.strokeStyle = "rgba(140, 188, 205, 0.16)";
    ctx.lineWidth = Math.max(1, dpr * 0.8);
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();
  }
  ctx.restore();

  const sweepX = ((nowSec * 120 * dpr) % (width + 120 * dpr)) - 120 * dpr;
  const sweepGradient = ctx.createLinearGradient(sweepX, 0, sweepX + 120 * dpr, 0);
  sweepGradient.addColorStop(0, "rgba(90, 210, 255, 0)");
  sweepGradient.addColorStop(0.45, "rgba(90, 210, 255, 0.1)");
  sweepGradient.addColorStop(1, "rgba(90, 210, 255, 0)");
  ctx.fillStyle = sweepGradient;
  ctx.fillRect(0, 0, width, height);

  const gapPx = settings.value.quality === "ultra" ? 2 * dpr : settings.value.quality === "high" ? 3 * dpr : 4 * dpr;
  const totalGap = gapPx * (count - 1);
  const barWidth = Math.max(1, (width - totalGap) / count);
  const colorGradient = ctx.createLinearGradient(0, 0, 0, height);
  colorGradient.addColorStop(0, "#ffd37b");
  colorGradient.addColorStop(0.5, "#2dc596");
  colorGradient.addColorStop(1, "#58dbff");
  ctx.fillStyle = colorGradient;
  ctx.shadowColor = "rgba(93, 219, 255, 0.35)";
  ctx.shadowBlur = 10 * dpr;

  for (let index = 0; index < count; index += 1) {
    const value = Math.max(0, Math.min(1, nextBins[index] ?? 0));
    const barHeight = Math.max(2 * dpr, value * height * 0.92);
    const x = index * (barWidth + gapPx);
    const y = height - barHeight;

    ctx.fillRect(x, y, barWidth, barHeight);

    const capHeight = Math.max(1.4 * dpr, 2.2 * dpr);
    ctx.shadowBlur = 0;
    ctx.fillStyle = "rgba(255, 230, 180, 0.85)";
    ctx.fillRect(x, y - capHeight, barWidth, capHeight);
    ctx.fillStyle = colorGradient;
    ctx.shadowBlur = 10 * dpr;
  }

  ctx.shadowBlur = 0;
}

/**
 * 缁樺埗鏌辩姸甯э細浼樺厛璧?WebGL2锛屽け璐ュ悗鑷姩鍥為€€ Canvas2D銆?
 */
function drawBarsFrame(nextBins: number[]): void {
  if (visualStyle.value !== "bars") {
    return;
  }

  const canvas = barsCanvasRef.value;
  if (!canvas) {
    return;
  }

  if (!barsWebglRenderer && !bars2dContext) {
    initBarsRenderer(canvas);
  }

  if (barsWebglRenderer) {
    drawBarsWithWebgl(nextBins);
    return;
  }

  if (bars2dContext) {
    drawBarsWithCanvas2d(nextBins);
  }
}

/**
 * 閼惧嘲褰囪ぐ鎾冲閻㈡槒宸濈€电懓绨查惃鍕閺屾捇顣╃粻妤€鎷伴幓鎺戔偓鐓庡棘閺佽埇鈧? */
function qualityProfile(tier: QualityTier): QualityProfile {
  return QUALITY_PROFILE[tier];
}

/**
 * 閻㈡槒宸濋弸姘鏉烆兛鑵戦弬鍥ㄧ垼缁涙拝绱濈紒鐔剁閻劋绨拋鍓х枂妞ょ懓鎷版潻鎰攽閻樿埖鈧礁鐫嶇粈鎭掆偓? */
function qualityLabel(tier: QualityTier): string {
  switch (tier) {
    case "ultra":
      return t.settings.qualityUltra;
    case "high":
      return t.settings.qualityHigh;
    case "balanced":
      return t.settings.qualityBalanced;
    default:
      return t.settings.qualityBalanced;
  }
}

/**
 * 鐠侊紕鐣婚惂鎯у瀻娴ｅ秴鈧》绱濋悽銊ょ艾 frame-time p95 鐠囧嫪鍙婇妴? */
function percentile(values: number[], ratio: number): number {
  if (values.length === 0) {
    return 0;
  }

  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.max(0, Math.min(sorted.length - 1, Math.ceil(sorted.length * ratio) - 1));
  return sorted[index];
}

/**
 * 閹稿娲伴弽鍥ㄦ殶闁插繗浠涢崥鍫ヮ暥鐠嬭鲸鐓撮敍宀€鈥樻穱婵呯瑝閸氬瞼鏁剧拹銊ょ瑓濞撳弶鐓嬫径宥嗘絽鎼达箑褰查幒褋鈧? */
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

function syncDisplayBinLength(tier: QualityTier): void {
  const expected = qualityProfile(tier).binCount;
  if (bins.value.length === expected) {
    return;
  }

  const next = bins.value.slice(0, expected);
  while (next.length < expected) {
    next.push(0);
  }
  bins.value = next;
  spectrumFrameBins.value = next;
}

/**
 * 鐏忓棛绮烘稉鈧?64-bin 閸愬懘鍎寸紓鎾冲暱閺勭姴鐨犻幋鎰秼閸撳秶鏁剧拹銊ф畱閺勫墽銇氭０鎴ｆ皑閵? */
function buildDisplayBins(source: number[], tier: QualityTier): number[] {
  const expected = qualityProfile(tier).binCount;
  if (expected === source.length) {
    return source.slice(0, expected);
  }
  return downsampleBins(source, expected);
}

/**
 * 鐠佹澘缍?frame-time 閺嶉攱婀伴獮鍫曟閸掑墎鐛ラ崣锝夋毐鎼达讣绱濋悽銊ょ艾 p95 娑?FPS 鐠侊紕鐣婚妴? */
function pushFrameSample(now: number, duration: number): void {
  frameTimeSamples.push({ ts: now, duration });
  while (frameTimeSamples.length > 0 && now - frameTimeSamples[0].ts > PERF_WINDOW_MS) {
    frameTimeSamples.shift();
  }
}

/**
 * 閸╄桨绨張鈧潻?1 缁夋帗鐗遍張顑垮強缁?FPS閿涘矂浼╅崗宥囩叚閺冭埖濮堥崝銊┾偓鐘冲灇 UI 鐠囶垰鍨介妴? */
function estimateFps(now: number): number {
  const cutoff = now - FPS_WINDOW_MS;
  let count = 0;

  for (let index = frameTimeSamples.length - 1; index >= 0; index -= 1) {
    if (frameTimeSamples[index].ts < cutoff) {
      break;
    }
    count += 1;
  }

  return count;
}

/**
 * 瀵懓鍤懛顏勫З闂勫秶楠囬幓鎰仛閿涘苯鎲￠惌銉ф暏閹磋渹绮犻崫顏冪濡楋綁妾烽崚棰佺啊閸濐亙绔村锝冣偓? */
function showAutoQualityMessage(fromTier: QualityTier, toTier: QualityTier, saveFailed = false): void {
  const baseMessage = `${t.metrics.autoDowngrade}: ${qualityLabel(fromTier)} -> ${qualityLabel(toTier)}`;
  autoQualityMessage.value = saveFailed ? `${baseMessage}（${t.metrics.autoDowngradeSaveFailed}）` : baseMessage;

  if (qualityMessageTimer !== null) {
    window.clearTimeout(qualityMessageTimer);
  }

  qualityMessageTimer = window.setTimeout(() => {
    autoQualityMessage.value = "";
    qualityMessageTimer = null;
  }, 5200);
}

/**
 * 缂佺喍绔存穱婵嗙摠鐠佸墽鐤嗛獮璺烘礀鐠囪鎮楃粩顖氭彥閻撗嶇礉娣囨繆鐦夐崜宥呮倵缁旑垳濮搁幀浣风閼锋番鈧? */
async function persistSettings(nextSettings: AppSettings, syncEditing = true): Promise<AppSettings> {
  await invoke("save_settings", { settings: nextSettings });

  // 鍏抽敭琛岋細淇濆瓨鍚庣珛鍗冲洖璇诲悗绔揩鐓э紝閬垮厤绯荤粺绾ц涓轰慨姝ｅ悗鍓嶇鐘舵€佹紓绉汇€?
  const latest = normalizeSettings(await invoke<AppSettings>("load_settings"));
  settings.value = latest;
  if (syncEditing) {
    editingSettings.value = { ...latest };
  }
  saveLocalSettings(latest);
  syncDisplayBinLength(latest.quality);
  return latest;
}

/**
 * 閼奉亜濮╅梽宥囬獓闁槒绶敍姘矌閸氭垳绗呴崚鍥ㄣ€傞敍宀勬Щ濮濄垺鈧嗗厴閹舵牕濮╅弮璺哄冀婢跺秴鍨忛幑顫偓? */
async function autoDowngradeQuality(): Promise<void> {
  if (autoDowngradePending) {
    return;
  }

  const currentTier = settings.value.quality;
  const currentIndex = QUALITY_ORDER.indexOf(currentTier);
  if (currentIndex < 0 || currentIndex >= QUALITY_ORDER.length - 1) {
    return;
  }

  autoDowngradePending = true;
  const nextTier = QUALITY_ORDER[currentIndex + 1];
  const nextSettings = normalizeSettings({ ...settings.value, quality: nextTier });

  settings.value = nextSettings;
  saveLocalSettings(nextSettings);
  if (!settingsOpen.value) {
    editingSettings.value = { ...nextSettings };
  }
  syncDisplayBinLength(nextTier);
  showAutoQualityMessage(currentTier, nextTier);

  try {
    await persistSettings(nextSettings, !settingsOpen.value);
  } catch (error) {
    console.warn("Auto downgrade save failed:", error);
    showAutoQualityMessage(currentTier, nextTier, true);
  } finally {
    autoDowngradePending = false;
    downgradeBreachStartTs = 0;
  }
}

/**
 * 濮ｅ繒顫楃拠鍕強娑撯偓濞嗏剝瑕嗛弻鎾冲竾閸旀冻绱濋幐?p95 閺勵垰鎯佺搾鍛搭暕缁犳鍨界€规碍妲搁崥锕佇曢崣鎴ｅ殰閸斻劑妾风痪褋鈧? */
function evaluatePerformance(now: number): void {
  if (frameTimeSamples.length === 0) {
    return;
  }

  perfFps.value = estimateFps(now);
  perfFrameP95Ms.value = percentile(
    frameTimeSamples.map((sample) => sample.duration),
    0.95
  );

  const budget = qualityProfile(settings.value.quality).frameBudgetMs;
  if (perfFrameP95Ms.value > budget) {
    if (downgradeBreachStartTs === 0) {
      downgradeBreachStartTs = now;
    } else if (now - downgradeBreachStartTs >= AUTO_DOWNGRADE_HOLD_MS) {
      void autoDowngradeQuality();
    }
    return;
  }

  downgradeBreachStartTs = 0;
}

/**
 * 閸氼垰濮╁〒鍙夌厠瀵邦亞骞嗛敍姘瀻閺嬫劕鎶氶崣顏呮纯閺傛壆娲伴弽鍥р偓纭风礉閺勫墽銇氱敮褎瀵滈崚閿嬫煀閻滃洦褰冮崐鑹扮翻閸戞亽鈧? */
function startRenderLoop(): void {
  if (animationFrameHandle !== null) {
    return;
  }

  lastRenderTs = performance.now();
  lastPerfEvalTs = lastRenderTs;

  const renderFrame = (now: number): void => {
    const deltaMs = Math.max(0.5, Math.min(50, now - lastRenderTs || 16.67));
    lastRenderTs = now;
    perfFrameTimeMs.value = deltaMs;
    pushFrameSample(now, deltaMs);

    const profile = qualityProfile(settings.value.quality);
    const blend = Math.min(1, deltaMs / profile.interpolationMs);
    const stale = now - lastAnalysisTs > 140;

    for (let index = 0; index < renderBinsBuffer.length; index += 1) {
      const target = targetBinsBuffer[index] ?? 0;
      const eased = renderBinsBuffer[index] + (target - renderBinsBuffer[index]) * blend;
      renderBinsBuffer[index] = stale ? Math.max(0, eased * profile.staleDecay) : eased;
    }

    const displayBins = buildDisplayBins(renderBinsBuffer, settings.value.quality);
    bins.value = displayBins;
    spectrumFrameBins.value = displayBins;
    drawBarsFrame(displayBins);

    if (now - lastPerfEvalTs >= PERF_EVAL_INTERVAL_MS) {
      evaluatePerformance(now);
      lastPerfEvalTs = now;
    }

    animationFrameHandle = requestAnimationFrame(renderFrame);
  };

  animationFrameHandle = requestAnimationFrame(renderFrame);
}

/**
 * 閸嬫粍顒涘〒鍙夌厠瀵邦亞骞嗛敍宀勪缉閸忓秶绮嶆禒鍫曟敘濮ｄ礁鎮楃紒褏鐢婚崡鐘垫暏 CPU閵? */
function stopRenderLoop(): void {
  if (animationFrameHandle !== null) {
    cancelAnimationFrame(animationFrameHandle);
    animationFrameHandle = null;
  }
}

/**
 * 娴犲孩婀伴崷鏉跨摠閸屻劌濮炴潪钘夊讲鐟欏棗瀵查弽宄扮础閿涘矂娼▔鏇炩偓鑹板殰閸斻劌娲栭柅鈧崚鐗堢叴閻樿泛娴橀妴? */
async function refreshMonitorList(): Promise<void> {
  try {
    discoveredMonitors.value = await invoke<MonitorInfo[]>("list_monitors");
  } catch (error) {
    console.warn("Failed to load monitors:", error);
    discoveredMonitors.value = [];
  }
}

/**
 * 閹垫挸绱戠拋鍓х枂瀵湱鐛ラ獮璺哄鏉炶棄缍嬮崜宥夊帳缂冾喖鎻╅悡褋鈧? */
function openSettings(): void {
  editingSettings.value = { ...settings.value };
  editingVisualTuning.value = { ...visualTuning.value };
  editingVisualStyle.value = visualStyle.value;
  settingsError.value = "";
  settingsOpen.value = true;
}

/**
 * 閸忔娊妫寸拋鍓х枂瀵湱鐛ラ敍灞肩瑝閹绘劒姘︽穱顔芥暭閵? */
function closeSettings(): void {
  settingsOpen.value = false;
  settingsError.value = "";
}

async function enterImmersiveMode(): Promise<void> {
  immersiveMode.value = true;
  const root = document.documentElement;
  if (!document.fullscreenElement && root.requestFullscreen) {
    try {
      await root.requestFullscreen();
    } catch (error) {
      console.warn("Enter fullscreen failed:", error);
    }
  }
}

async function exitImmersiveMode(): Promise<void> {
  if (document.fullscreenElement && document.exitFullscreen) {
    try {
      await document.exitFullscreen();
    } catch (error) {
      console.warn("Exit fullscreen failed:", error);
    }
  }
  immersiveMode.value = false;
}

function handleFullscreenChange(): void {
  immersiveMode.value = Boolean(document.fullscreenElement);
}

function handleWindowKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && immersiveMode.value && !document.fullscreenElement) {
    immersiveMode.value = false;
  }
}

/**
 * 閹垹顦叉妯款吇鐠佸墽鐤嗛獮璺烘倱濮濄儱鍩岀紓鏍帆閹降鈧? */
function resetEditingSettings(): void {
  editingSettings.value = { ...defaultSettings };
  editingVisualTuning.value = { ...DEFAULT_VISUAL_TUNING };
  editingVisualStyle.value = "bars";
}

function updateVisualStyle(next: VisualStyle): void {
  editingVisualStyle.value = next;
}

/**
 * 閹绘劒姘︾拋鍓х枂楠炶泛鎮撳銉ュ煂閸氬海顏潻鎰攽閺冭绱扮粣妤€褰涘Ο鈥崇础閵嗕焦妯夌粈鍝勬珤閵嗕胶鍋ｉ崙鑽も敍闁繐鎷?DSP 閸欏倹鏆熼崡铏閻㈢喐鏅ラ妴? */
async function applySettings(): Promise<void> {
  settingsSaving.value = true;
  settingsError.value = "";

  try {
    const normalized = normalizeSettings(editingSettings.value);
    await persistSettings(normalized);
    const tuned = normalizeVisualTuning(editingVisualTuning.value);
    visualTuning.value = tuned;
    saveVisualTuning(tuned);
    visualStyle.value = editingVisualStyle.value;
    settingsOpen.value = false;
  } catch (error) {
    settingsError.value = `保存失败: ${String(error)}`;
  } finally {
    settingsSaving.value = false;
  }
}

watch(visualStyle, (value) => {
  saveVisualStyle(value);
  if (value !== "bars") {
    disposeBarsRenderer();
    return;
  }

  requestAnimationFrame(() => {
    initBarsRenderer(barsCanvasRef.value);
  });
});

watch(
  () => barsCanvasRef.value,
  (canvas) => {
    if (!canvas || visualStyle.value !== "bars") {
      disposeBarsRenderer();
      return;
    }

    initBarsRenderer(canvas);
  },
  { flush: "post" }
);

watch(
  () => settings.value.quality,
  (tier) => {
    syncDisplayBinLength(tier);
    downgradeBreachStartTs = 0;
  }
);

onMounted(async () => {
  document.addEventListener("fullscreenchange", handleFullscreenChange);
  window.addEventListener("keydown", handleWindowKeydown);
  statusText.value = t.status.connecting;

  try {
    await invoke<string>("health_check");
    discoveredDevices.value = await invoke<AudioDeviceInfo[]>("list_audio_devices");
    await refreshMonitorList();

    const remoteSettings = await invoke<AppSettings>("load_settings");
    const normalized = normalizeSettings(remoteSettings);

    settings.value = normalized;
    editingSettings.value = { ...normalized };
    saveLocalSettings(normalized);
    syncDisplayBinLength(normalized.quality);

    statusText.value = t.status.running;
  } catch (error) {
    console.error("Failed to initialize backend bridge:", error);
    settings.value = normalizeSettings(loadLocalSettings() ?? defaultSettings);
    editingSettings.value = { ...settings.value };
    syncDisplayBinLength(settings.value.quality);
    statusText.value = t.status.fallback;
  }

  unlistenFrame = await listen<AnalysisFrame>("audio:analysis_frame", (event) => {
    const payload = event.payload;
    deviceLabel.value = payload.deviceId;

    // 鍏抽敭琛岋細鍋氬櫔澹板簳鎶戝埗 + 鍔ㄦ€佸鐩?+ 闈炵嚎鎬у帇缂╋紝鎻愬崌寮变俊鍙峰彲瑙佹€с€?
    const adaptiveGain = payload.rms < 0.06 ? 2.6 : payload.rms < 0.16 ? 2.0 : 1.4;
    const globalFloor = Math.min(0.16, payload.rms * 0.55 + payload.peak * 0.2);
    const nextBins = payload.bins.slice(0, 64).map((bin) => {
      const normalized = Math.min(1, Math.max(0, bin / 1023));
      const floorRemoved = Math.max(0, normalized - 0.005) / 0.995;
      const lifted = Math.min(1, floorRemoved * adaptiveGain);
      return Math.min(1, Math.pow(lifted, 0.62) * 0.9 + globalFloor * 0.1);
    });

    while (nextBins.length < 64) {
      nextBins.push(0);
    }

    // 鍏抽敭琛岋細鍒嗘瀽浜嬩欢浠呮洿鏂扮洰鏍囩紦鍐诧紝瀹為檯鏄剧ず鐢?rAF 鎻掑€奸┍鍔紝閬垮厤 IPC 棰戠巼缁戝畾娓叉煋棰戠巼銆?
    for (let index = 0; index < 64; index += 1) {
      targetBinsBuffer[index] = nextBins[index];
    }
    lastAnalysisTs = performance.now();
  });

  unlistenOpenSettings = await listen("app:open_settings", () => {
    openSettings();
  });

  unlistenVisualPause = await listen<boolean>("app:visual_paused", (event) => {
    statusText.value = event.payload ? t.status.paused : t.status.running;
    if (event.payload) {
      targetBinsBuffer.fill(0);
    }
  });

  unlistenClickThroughChanged = await listen<boolean>("app:click_through_changed", (event) => {
    settings.value = { ...settings.value, clickThrough: event.payload };
    if (!settingsOpen.value) {
      editingSettings.value = { ...settings.value };
    }
    saveLocalSettings(settings.value);
  });

  startRenderLoop();
});

onBeforeUnmount(() => {
  stopRenderLoop();
  disposeBarsRenderer();
  document.removeEventListener("fullscreenchange", handleFullscreenChange);
  window.removeEventListener("keydown", handleWindowKeydown);
  if (qualityMessageTimer !== null) {
    window.clearTimeout(qualityMessageTimer);
    qualityMessageTimer = null;
  }

  unlistenFrame?.();
  unlistenOpenSettings?.();
  unlistenVisualPause?.();
  unlistenClickThroughChanged?.();
});
</script>

<template>
  <main class="app" :class="{ 'app-prod': !isDevMode, 'app-immersive': immersiveMode }">
    <header v-if="isDevMode && !immersiveMode" class="hero panel">
      <div class="hero-main">
        <h1>{{ t.title }}</h1>
        <p>{{ t.subtitle }}</p>
      </div>

      <div class="header-actions">
        <button type="button" class="settings-btn" @click="openSettings">{{ t.settings.open }}</button>
        <button type="button" class="immersive-btn" @click="enterImmersiveMode">沉浸全屏</button>
      </div>

      <div class="hero-pills">
        <span class="pill">{{ t.statusLabel }}: {{ statusText }}</span>
        <span class="pill">{{ t.audio.discovered }}: {{ discoveredDevices.length }}</span>
        <span class="pill">{{ t.metrics.device }}: {{ deviceLabel }}</span>
        <span class="pill">{{ t.metrics.quality }}: {{ activeQualityLabel }}</span>
        <span class="pill">{{ t.metrics.fps }}: {{ perfFps }}</span>
        <span class="pill">{{ t.metrics.frameTime }}: {{ frameTimeDisplay }}ms</span>
        <span class="pill">{{ t.metrics.p95 }}: {{ frameP95Display }}ms / {{ budgetDisplay }}ms</span>
        <span class="pill">{{ t.metrics.renderer }}: {{ visualRendererLabel }}</span>
      </div>
      <p v-if="autoQualityMessage" class="quality-alert">{{ autoQualityMessage }}</p>
    </header>

    <div v-else-if="!immersiveMode" class="top-actions">
      <button type="button" class="settings-btn" @click="openSettings">{{ t.settings.open }}</button>
      <button type="button" class="immersive-btn" @click="enterImmersiveMode">沉浸全屏</button>
    </div>

    <div v-if="immersiveMode" class="immersive-actions">
      <button type="button" class="immersive-btn immersive-exit" @click="exitImmersiveMode">退出沉浸</button>
    </div>

    <section
      class="panel visual-panel"
      :class="{ 'visual-panel-prod': !isDevMode || immersiveMode, 'visual-panel-immersive': immersiveMode }"
    >
      <div v-if="isDevMode && !immersiveMode" class="visual-head">
        <h2>{{ t.visualizer.section }}</h2>
        <div class="style-switch">
          <button
            v-for="item in styleOptions"
            :key="item.value"
            type="button"
            class="style-btn"
            :class="{ active: visualStyle === item.value }"
            @click="visualStyle = item.value"
          >
            {{ item.label }}
          </button>
        </div>
      </div>

      <div class="visual-stage">
        <Transition name="visual-swap" mode="out-in">
          <div v-if="visualStyle === 'bars'" key="bars" class="bars-canvas-wrap" :class="`quality-${settings.quality}`">
            <canvas ref="barsCanvasRef" class="bars-canvas"></canvas>
          </div>
          <SpectrumCanvas
            v-else
            :key="visualStyle"
            :visual-style="nonBarsVisualStyle"
            :tuning="visualTuning"
          />
        </Transition>
      </div>
    </section>

    <SettingsModal
      :open="settingsOpen"
      :settings-saving="settingsSaving"
      :settings-error="settingsError"
      :editing-settings="editingSettings"
      :editing-visual-tuning="editingVisualTuning"
      :style-options="styleOptions"
      :quality-options="qualityOptions"
      :window-mode-options="windowModeOptions"
      :discovered-monitors="discoveredMonitors"
      :visual-style="editingVisualStyle"
      :show-visual-tuning-section="showVisualTuningSection"
      :active-visual-style-label="activeVisualStyleLabel"
      :t="t"
      @close="closeSettings"
      @reset="resetEditingSettings"
      @apply="applySettings"
      @change-visual-style="updateVisualStyle"
    />
  </main>
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
}

:root {
  font-family: "Noto Sans SC", "Microsoft YaHei", sans-serif;
}

.app {
  position: relative;
  width: 100%;
  height: 100dvh;
  overflow: hidden;
  padding: clamp(8px, 1.2vw, 14px);
  color: #f3f8f5;
  background:
    radial-gradient(circle at 20% 18%, rgba(31, 94, 82, 0.75) 0%, rgba(31, 94, 82, 0) 46%),
    radial-gradient(circle at 82% 10%, rgba(98, 77, 35, 0.68) 0%, rgba(98, 77, 35, 0) 42%),
    linear-gradient(145deg, #0c1e2f 0%, #0d2334 45%, #112b3c 100%);
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: clamp(8px, 1vw, 12px);
  box-sizing: border-box;
}

.app-prod {
  gap: clamp(6px, 0.8vw, 10px);
}

.app-immersive {
  padding: 0;
  gap: 0;
  grid-template-rows: minmax(0, 1fr);
  background: #08131c;
}

.app-immersive .panel {
  border: 0;
  border-radius: 0;
  padding: 0;
  background: transparent;
  backdrop-filter: none;
}

.app-immersive .bars-canvas-wrap {
  border: 0;
  border-radius: 0;
  padding: 0;
}

.panel {
  border: 1px solid rgba(220, 240, 235, 0.2);
  border-radius: 14px;
  padding: clamp(10px, 1vw, 14px);
  background-color: rgba(9, 18, 27, 0.52);
  backdrop-filter: blur(5px);
  min-height: 0;
  overflow: hidden;
}

.hero {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

.header-actions {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.hero-main h1 {
  margin: 0;
  font-size: clamp(1rem, 1.9vw, 1.4rem);
  letter-spacing: 0.02em;
}

.hero-main p {
  margin: 3px 0 0;
  color: #cadbd7;
  font-size: clamp(0.78rem, 1.3vw, 0.9rem);
}

.top-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.immersive-actions {
  position: fixed;
  top: clamp(8px, 1.1vw, 14px);
  right: clamp(8px, 1.1vw, 14px);
  z-index: 24;
}

.settings-btn {
  align-self: start;
  height: 32px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid rgba(164, 211, 200, 0.4);
  background: rgba(31, 61, 82, 0.8);
  color: #ebfaf5;
  cursor: pointer;
  transition: transform 0.24s ease, box-shadow 0.24s ease, background-color 0.24s ease;
}

.settings-btn:hover {
  transform: translateY(-1px);
  background: rgba(42, 79, 103, 0.9);
  box-shadow: 0 8px 18px rgba(27, 74, 95, 0.28);
}

.immersive-btn {
  height: 32px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid rgba(211, 204, 164, 0.4);
  background: rgba(83, 69, 31, 0.86);
  color: #fff7da;
  cursor: pointer;
  transition: transform 0.24s ease, box-shadow 0.24s ease, background-color 0.24s ease;
}

.immersive-btn:hover {
  transform: translateY(-1px);
  background: rgba(110, 87, 32, 0.94);
  box-shadow: 0 8px 18px rgba(95, 74, 27, 0.32);
}

.immersive-exit {
  border-color: rgba(220, 164, 164, 0.5);
  background: rgba(111, 45, 45, 0.9);
  color: #ffeded;
}

.immersive-exit:hover {
  background: rgba(136, 52, 52, 0.94);
  box-shadow: 0 8px 18px rgba(115, 42, 42, 0.34);
}

.hero-pills {
  grid-column: 1 / -1;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.quality-alert {
  grid-column: 1 / -1;
  margin: 0;
  font-size: 0.76rem;
  color: #ffd88a;
}

.pill {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 0.78rem;
  color: #f5fffa;
  background: rgba(133, 191, 176, 0.2);
  border: 1px solid rgba(175, 224, 211, 0.28);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.visual-panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 8px;
}

.visual-panel-prod {
  grid-template-rows: minmax(0, 1fr);
}

.visual-panel-immersive {
  grid-template-rows: minmax(0, 1fr);
}

.visual-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.visual-panel h2 {
  margin: 0;
  font-size: 0.95rem;
}

.style-switch {
  display: inline-flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 4px;
  padding: 3px;
  border-radius: 999px;
  border: 1px solid rgba(172, 215, 205, 0.35);
  background: rgba(17, 37, 52, 0.8);
}

.style-btn {
  border: 0;
  min-width: 54px;
  height: 28px;
  border-radius: 999px;
  padding: 0 10px;
  color: #cfe9e2;
  background: transparent;
  font-size: 0.76rem;
  cursor: pointer;
  transition: transform 0.2s ease, color 0.2s ease, background-color 0.2s ease;
}

.style-btn:hover {
  transform: translateY(-1px);
  color: #e8faf5;
}

.style-btn.active {
  color: #ffffff;
  background: linear-gradient(180deg, #2fb48b 0%, #1f8f6e 100%);
}

.visual-stage {
  position: relative;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.app-immersive .visual-stage {
  height: 100dvh;
}

.bars-canvas-wrap {
  position: relative;
  height: 100%;
  padding: 8px;
  border-radius: 12px;
  border: 1px solid rgba(185, 220, 212, 0.25);
  background: linear-gradient(180deg, rgba(7, 16, 25, 0.45), rgba(7, 16, 25, 0.88));
  box-sizing: border-box;
  z-index: 1;
}

.bars-canvas-wrap.quality-high {
  padding: 9px;
}

.bars-canvas-wrap.quality-balanced {
  padding: 10px;
}

.bars-canvas {
  width: 100%;
  height: 100%;
  display: block;
  border-radius: 8px;
}


.visual-swap-enter-active,
.visual-swap-leave-active {
  transition: opacity 0.34s ease, transform 0.34s ease, filter 0.34s ease;
}

.visual-swap-enter-from,
.visual-swap-leave-to {
  opacity: 0;
  transform: translateY(8px) scale(0.985);
  filter: blur(2px);
}

</style>




