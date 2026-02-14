<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { zhCN } from "@/locales/zh-CN";
import { defaultSettings, loadLocalSettings, normalizeSettings, saveLocalSettings } from "@/stores/settings";
import type { AnalysisFrame, AppSettings, AudioDeviceInfo, MonitorInfo, WindowMode } from "@/types";

type VisualStyle = "bars" | "wave" | "radial";

const t = zhCN;
const VISUAL_STYLE_KEY = "tt-audio-lab.visualStyle";

const settings = ref<AppSettings>(loadLocalSettings() ?? { ...defaultSettings });
const editingSettings = ref<AppSettings>({ ...settings.value });
const statusText = ref<string>(t.status.idle);
const bins = ref<number[]>(new Array(64).fill(0));
const deviceLabel = ref("未知设备");
const discoveredDevices = ref<AudioDeviceInfo[]>([]);
const discoveredMonitors = ref<MonitorInfo[]>([]);
const visualStyle = ref<VisualStyle>(loadVisualStyle());
const settingsOpen = ref(false);
const settingsSaving = ref(false);
const settingsError = ref("");

let unlistenFrame: UnlistenFn | null = null;
let unlistenOpenSettings: UnlistenFn | null = null;
let unlistenVisualPause: UnlistenFn | null = null;
let unlistenClickThroughChanged: UnlistenFn | null = null;

const styleOptions: Array<{ label: string; value: VisualStyle }> = [
  { label: t.visualizer.styles.bars, value: "bars" },
  { label: t.visualizer.styles.wave, value: "wave" },
  { label: t.visualizer.styles.radial, value: "radial" }
];

const windowModeOptions: Array<{ label: string; value: WindowMode }> = [
  { label: t.windowMode.normal, value: "normal" },
  { label: t.windowMode.desktopWidget, value: "desktopWidget" },
  { label: t.windowMode.overlay, value: "overlay" }
];

const frameBars = computed(() =>
  bins.value.map((value, index) => ({
    id: index,
    height: `${Math.max(2, Math.round(value * 100))}%`
  }))
);

const waveformPoints = computed(() => {
  const total = bins.value.length;
  if (total <= 1) {
    return "0,50 100,50";
  }

  return bins.value
    .map((value, index) => {
      const x = (index / (total - 1)) * 100;
      const y = 100 - value * 90;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
});

const radialRays = computed(() => {
  const total = bins.value.length;
  return bins.value.map((value, index) => {
    const angle = (index / total) * Math.PI * 2 - Math.PI / 2;
    const innerRadius = 20;
    const outerRadius = innerRadius + value * 26;
    const x1 = 50 + Math.cos(angle) * innerRadius;
    const y1 = 50 + Math.sin(angle) * innerRadius;
    const x2 = 50 + Math.cos(angle) * outerRadius;
    const y2 = 50 + Math.sin(angle) * outerRadius;
    return { id: index, x1, y1, x2, y2, strength: value };
  });
});

/**
 * 从本地存储加载可视化样式，非法值自动回退到柱状图。
 */
function loadVisualStyle(): VisualStyle {
  const raw = localStorage.getItem(VISUAL_STYLE_KEY);
  if (raw === "bars" || raw === "wave" || raw === "radial") {
    return raw;
  }
  return "bars";
}

/**
 * 保存可视化样式到本地，保证下次启动保持一致。
 */
function saveVisualStyle(style: VisualStyle): void {
  localStorage.setItem(VISUAL_STYLE_KEY, style);
}

/**
 * 刷新显示器列表，供设置弹窗中的目标显示器下拉框使用。
 */
async function refreshMonitorList(): Promise<void> {
  try {
    discoveredMonitors.value = await invoke<MonitorInfo[]>("list_monitors");
  } catch (error) {
    console.warn("Failed to load monitors:", error);
    discoveredMonitors.value = [];
  }
}

/**
 * 打开设置弹窗并加载当前配置快照。
 */
function openSettings(): void {
  editingSettings.value = { ...settings.value };
  settingsError.value = "";
  settingsOpen.value = true;
}

/**
 * 关闭设置弹窗，不提交修改。
 */
function closeSettings(): void {
  settingsOpen.value = false;
  settingsError.value = "";
}

/**
 * 恢复默认设置并同步到编辑态。
 */
function resetEditingSettings(): void {
  editingSettings.value = { ...defaultSettings };
}

/**
 * 提交设置并同步到后端运行时：窗口模式、显示器、点击穿透和 DSP 参数即时生效。
 */
async function applySettings(): Promise<void> {
  settingsSaving.value = true;
  settingsError.value = "";

  try {
    const normalized = normalizeSettings(editingSettings.value);
    normalized.quality = "ultra";

    await invoke("save_settings", { settings: normalized });

    // 关键行：保存后重新读取后端设置，确保 UI 显示与后端最终生效状态一致。
    const latest = normalizeSettings(await invoke<AppSettings>("load_settings"));
    latest.quality = "ultra";

    settings.value = latest;
    editingSettings.value = { ...latest };
    saveLocalSettings(latest);
    settingsOpen.value = false;
  } catch (error) {
    settingsError.value = `保存失败：${String(error)}`;
  } finally {
    settingsSaving.value = false;
  }
}

watch(visualStyle, (value) => {
  saveVisualStyle(value);
});

onMounted(async () => {
  statusText.value = t.status.connecting;

  try {
    await invoke<string>("health_check");
    discoveredDevices.value = await invoke<AudioDeviceInfo[]>("list_audio_devices");
    await refreshMonitorList();

    const remoteSettings = await invoke<AppSettings>("load_settings");
    const normalized = normalizeSettings(remoteSettings);
    normalized.quality = "ultra";

    settings.value = normalized;
    editingSettings.value = { ...normalized };
    saveLocalSettings(normalized);

    statusText.value = t.status.running;
  } catch (error) {
    console.error("Failed to initialize backend bridge:", error);
    settings.value = normalizeSettings(loadLocalSettings() ?? defaultSettings);
    editingSettings.value = { ...settings.value };
    statusText.value = t.status.fallback;
  }

  unlistenFrame = await listen<AnalysisFrame>("audio:analysis_frame", (event) => {
    const payload = event.payload;
    deviceLabel.value = payload.deviceId;

    // 关键行：做噪声底抑制 + 动态增益 + 非线性压缩，提升弱信号可见性。
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

    bins.value = nextBins;
  });

  unlistenOpenSettings = await listen("app:open_settings", () => {
    openSettings();
  });

  unlistenVisualPause = await listen<boolean>("app:visual_paused", (event) => {
    statusText.value = event.payload ? t.status.paused : t.status.running;
  });

  unlistenClickThroughChanged = await listen<boolean>("app:click_through_changed", (event) => {
    settings.value = { ...settings.value, clickThrough: event.payload };
    editingSettings.value = { ...editingSettings.value, clickThrough: event.payload };
    saveLocalSettings(settings.value);
  });
});

onBeforeUnmount(() => {
  unlistenFrame?.();
  unlistenOpenSettings?.();
  unlistenVisualPause?.();
  unlistenClickThroughChanged?.();
});
</script>

<template>
  <main class="app">
    <header class="hero panel">
      <div class="hero-main">
        <h1>{{ t.title }}</h1>
        <p>{{ t.subtitle }}</p>
      </div>

      <button type="button" class="settings-btn" @click="openSettings">{{ t.settings.open }}</button>

      <div class="hero-pills">
        <span class="pill">{{ t.statusLabel }}: {{ statusText }}</span>
        <span class="pill">{{ t.audio.discovered }}: {{ discoveredDevices.length }}</span>
        <span class="pill">{{ t.metrics.device }}: {{ deviceLabel }}</span>
      </div>
    </header>

    <section class="panel visual-panel">
      <div class="visual-head">
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
        <div v-if="visualStyle === 'bars'" class="bars">
          <div v-for="bar in frameBars" :key="bar.id" class="bar" :style="{ height: bar.height }"></div>
        </div>

        <svg v-else-if="visualStyle === 'wave'" class="wave-svg" viewBox="0 0 100 100" preserveAspectRatio="none">
          <g class="wave-grid">
            <line x1="0" y1="25" x2="100" y2="25"></line>
            <line x1="0" y1="50" x2="100" y2="50"></line>
            <line x1="0" y1="75" x2="100" y2="75"></line>
          </g>
          <polyline class="wave-line" :points="waveformPoints"></polyline>
        </svg>

        <svg v-else class="radial-svg" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet">
          <circle class="radial-core" cx="50" cy="50" r="18"></circle>
          <line
            v-for="ray in radialRays"
            :key="ray.id"
            class="radial-ray"
            :x1="ray.x1"
            :y1="ray.y1"
            :x2="ray.x2"
            :y2="ray.y2"
            :style="{ opacity: (0.3 + ray.strength * 0.7).toFixed(2) }"
          ></line>
        </svg>
      </div>
    </section>

    <div v-if="settingsOpen" class="modal-mask">
      <section class="modal">
        <header class="modal-head">
          <h3>{{ t.settings.title }}</h3>
          <button type="button" class="modal-close" @click="closeSettings">×</button>
        </header>

        <label class="field">
          <span>{{ t.settings.smoothing }}: {{ editingSettings.smoothing.toFixed(2) }}</span>
          <small>{{ t.settings.smoothingHelp }}</small>
          <input
            v-model.number="editingSettings.smoothing"
            class="slider"
            type="range"
            min="0"
            max="0.95"
            step="0.01"
          />
        </label>

        <label class="field">
          <span>{{ t.settings.gain }}: {{ editingSettings.gain.toFixed(2) }}</span>
          <small>{{ t.settings.gainHelp }}</small>
          <input v-model.number="editingSettings.gain" class="slider" type="range" min="0.2" max="6" step="0.05" />
        </label>

        <label class="field">
          <span>{{ t.settings.windowMode }}</span>
          <small>{{ t.settings.windowModeHelp }}</small>
          <div class="select-wrap">
            <select v-model="editingSettings.windowMode" class="select-input">
              <option v-for="item in windowModeOptions" :key="item.value" :value="item.value">
                {{ item.label }}
              </option>
            </select>
          </div>
        </label>

        <label class="field">
          <span>{{ t.settings.monitor }}</span>
          <small>{{ t.settings.monitorHelp }}</small>
          <div class="select-wrap">
            <select v-model="editingSettings.targetMonitorId" class="select-input">
              <option value="">{{ t.settings.monitorAuto }}</option>
              <option v-for="monitor in discoveredMonitors" :key="monitor.id" :value="monitor.id">
                {{ monitor.label }}
              </option>
            </select>
          </div>
        </label>

        <label class="switch-field">
          <input v-model="editingSettings.clickThrough" class="switch-input" type="checkbox" />
          <span class="switch-ui"></span>
          <span>{{ t.settings.clickThrough }}</span>
          <small>{{ t.settings.clickThroughHelp }}</small>
        </label>

        <label class="switch-field">
          <input v-model="editingSettings.launchAtStartup" class="switch-input" type="checkbox" />
          <span class="switch-ui"></span>
          <span>{{ t.settings.launchAtStartup }}</span>
          <small>{{ t.settings.launchAtStartupHelp }}</small>
        </label>

        <p v-if="settingsError" class="error">{{ settingsError }}</p>

        <div class="modal-actions">
          <button type="button" class="action-btn ghost" @click="closeSettings">{{ t.actions.cancel }}</button>
          <button type="button" class="action-btn ghost" @click="resetEditingSettings">{{ t.actions.reset }}</button>
          <button type="button" class="action-btn" :disabled="settingsSaving" @click="applySettings">
            {{ settingsSaving ? "保存中..." : t.actions.apply }}
          </button>
        </div>
      </section>
    </div>
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

.settings-btn {
  align-self: start;
  height: 32px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid rgba(164, 211, 200, 0.4);
  background: rgba(31, 61, 82, 0.8);
  color: #ebfaf5;
  cursor: pointer;
}

.hero-pills {
  grid-column: 1 / -1;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
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
  display: inline-grid;
  grid-auto-flow: column;
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
}

.style-btn.active {
  color: #ffffff;
  background: linear-gradient(180deg, #2fb48b 0%, #1f8f6e 100%);
}

.visual-stage {
  min-height: 0;
  height: 100%;
}

.bars {
  height: 100%;
  padding: 8px;
  border-radius: 12px;
  border: 1px solid rgba(185, 220, 212, 0.25);
  background: linear-gradient(180deg, rgba(7, 16, 25, 0.45), rgba(7, 16, 25, 0.88));
  display: grid;
  grid-template-columns: repeat(64, minmax(0, 1fr));
  gap: 2px;
  align-items: end;
  box-sizing: border-box;
}

.bar {
  border-radius: 999px 999px 0 0;
  background: linear-gradient(180deg, #ffd37b 0%, #2dc596 50%, #58dbff 100%);
  transition: height 0.08s linear;
}

.wave-svg,
.radial-svg {
  width: 100%;
  height: 100%;
  border-radius: 12px;
  border: 1px solid rgba(185, 220, 212, 0.25);
  background: linear-gradient(180deg, rgba(7, 16, 25, 0.45), rgba(7, 16, 25, 0.88));
  box-sizing: border-box;
}

.wave-grid line {
  stroke: rgba(182, 216, 208, 0.2);
  stroke-width: 0.6;
}

.wave-line {
  fill: none;
  stroke: #65e0ff;
  stroke-width: 1.7;
  stroke-linecap: round;
  stroke-linejoin: round;
  filter: drop-shadow(0 0 4px rgba(82, 203, 255, 0.4));
}

.radial-core {
  fill: rgba(71, 206, 168, 0.15);
  stroke: rgba(95, 221, 184, 0.45);
  stroke-width: 0.8;
}

.radial-ray {
  stroke: #6be7c0;
  stroke-width: 0.9;
  stroke-linecap: round;
}

.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(3, 9, 14, 0.56);
  display: grid;
  place-items: center;
  padding: 16px;
}

.modal {
  width: min(560px, 100%);
  border-radius: 14px;
  border: 1px solid rgba(188, 225, 216, 0.26);
  background: rgba(10, 22, 33, 0.97);
  padding: 14px;
  display: grid;
  gap: 10px;
}

.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.modal h3 {
  margin: 0;
  font-size: 1rem;
}

.modal-close {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid rgba(186, 223, 213, 0.34);
  background: rgba(255, 255, 255, 0.08);
  color: #effbf7;
  font-size: 1rem;
  cursor: pointer;
}

.field {
  display: grid;
  gap: 4px;
}

.field > span {
  font-size: 0.82rem;
}

.field > small,
.switch-field > small {
  color: #acc3bc;
  font-size: 0.74rem;
}

.slider {
  width: 100%;
  height: 6px;
  border-radius: 999px;
  appearance: none;
  background: linear-gradient(90deg, rgba(94, 176, 149, 0.9), rgba(82, 186, 220, 0.95));
  outline: none;
}

.slider::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid #d9f8ef;
  background: #1b9f7a;
  box-shadow: 0 0 0 3px rgba(54, 175, 142, 0.28);
}

.slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid #d9f8ef;
  background: #1b9f7a;
  box-shadow: 0 0 0 3px rgba(54, 175, 142, 0.28);
}

.select-wrap {
  position: relative;
}

.select-wrap::after {
  content: "▾";
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: #dcf9f1;
  pointer-events: none;
}

.select-input {
  width: 100%;
  height: 34px;
  border-radius: 10px;
  border: 1px solid rgba(188, 227, 216, 0.36);
  background: linear-gradient(180deg, rgba(16, 42, 58, 0.95), rgba(13, 31, 45, 0.95));
  color: #ebfff8;
  padding: 0 34px 0 10px;
  font-size: 0.8rem;
  appearance: none;
  outline: none;
}

.select-input option {
  color: #102433;
  background: #d8ece5;
}

.switch-field {
  position: relative;
  display: grid;
  grid-template-columns: auto auto;
  align-items: center;
  gap: 8px;
}

.switch-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.switch-ui {
  position: relative;
  width: 36px;
  height: 20px;
  border-radius: 999px;
  background: rgba(151, 171, 182, 0.45);
  transition: background-color 0.2s ease;
}

.switch-ui::before {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #ffffff;
  transition: transform 0.2s ease;
}

.switch-input:checked + .switch-ui {
  background: rgba(34, 178, 133, 0.95);
}

.switch-input:checked + .switch-ui::before {
  transform: translateX(16px);
}

.error {
  margin: 0;
  color: #ff9f9f;
  font-size: 0.8rem;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.action-btn {
  height: 32px;
  padding: 0 12px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: linear-gradient(180deg, #28ae83 0%, #188c66 100%);
  color: #fff;
  font-size: 0.8rem;
  cursor: pointer;
}

.action-btn:disabled {
  opacity: 0.65;
  cursor: not-allowed;
}

.action-btn.ghost {
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(214, 234, 227, 0.3);
}
</style>
