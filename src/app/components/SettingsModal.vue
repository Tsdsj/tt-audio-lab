<script setup lang="ts">
import { computed } from "vue";
import type { AppSettings, MonitorInfo, QualityTier, WindowMode } from "@/types";
import type { VisualStyle, VisualTuning } from "@/visualization/types";

const props = defineProps<{
  open: boolean;
  settingsSaving: boolean;
  settingsError: string;
  editingSettings: AppSettings;
  editingVisualTuning: VisualTuning;
  styleOptions: Array<{ label: string; value: VisualStyle }>;
  qualityOptions: Array<{ label: string; value: QualityTier }>;
  windowModeOptions: Array<{ label: string; value: WindowMode }>;
  discoveredMonitors: MonitorInfo[];
  visualStyle: VisualStyle;
  showVisualTuningSection: boolean;
  activeVisualStyleLabel: string;
  t: any;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "reset"): void;
  (event: "apply"): void;
  (event: "changeVisualStyle", value: VisualStyle): void;
}>();

const visualStyleProxy = computed<VisualStyle>({
  get: () => props.visualStyle,
  set: (value) => emit("changeVisualStyle", value)
});
</script>

<template>
  <Transition name="modal-fade">
    <div v-if="props.open" class="modal-mask">
      <section class="modal">
        <header class="modal-head">
          <h3>{{ props.t.settings.title }}</h3>
          <button type="button" class="modal-close" @click="emit('close')">×</button>
        </header>

        <label class="field">
          <span>{{ props.t.settings.smoothing }}: {{ props.editingSettings.smoothing.toFixed(2) }}</span>
          <small>{{ props.t.settings.smoothingHelp }}</small>
          <input
            v-model.number="props.editingSettings.smoothing"
            class="slider"
            type="range"
            min="0"
            max="0.95"
            step="0.01"
          />
        </label>

        <label class="field">
          <span>{{ props.t.settings.gain }}: {{ props.editingSettings.gain.toFixed(2) }}</span>
          <small>{{ props.t.settings.gainHelp }}</small>
          <input v-model.number="props.editingSettings.gain" class="slider" type="range" min="0.2" max="6" step="0.05" />
        </label>

        <label class="field">
          <span>{{ props.t.visualizer.section }}</span>
          <small>切换首页频谱效果风格</small>
          <div class="select-wrap">
            <select v-model="visualStyleProxy" class="select-input">
              <option v-for="item in props.styleOptions" :key="item.value" :value="item.value">
                {{ item.label }}
              </option>
            </select>
          </div>
        </label>

        <section v-if="props.showVisualTuningSection" class="fx-panel">
          <header class="fx-panel-head">
            <span>视觉特效参数 · {{ props.activeVisualStyleLabel }}</span>
            <small>仅对当前频谱风格生效</small>
          </header>

          <template v-if="props.visualStyle === 'particles'">
            <label class="field">
              <span>运动速度: {{ props.editingVisualTuning.particlesSpeed.toFixed(2) }}</span>
              <small>控制粒子旋转和扩散速度</small>
              <input
                v-model.number="props.editingVisualTuning.particlesSpeed"
                class="slider"
                type="range"
                min="0.4"
                max="2.4"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>元素密度: {{ props.editingVisualTuning.particlesDensity.toFixed(2) }}</span>
              <small>控制屏幕中的粒子数量</small>
              <input
                v-model.number="props.editingVisualTuning.particlesDensity"
                class="slider"
                type="range"
                min="0.4"
                max="2.0"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>发光强度: {{ props.editingVisualTuning.particlesGlow.toFixed(2) }}</span>
              <small>控制粒子亮度和辉光范围</small>
              <input
                v-model.number="props.editingVisualTuning.particlesGlow"
                class="slider"
                type="range"
                min="0"
                max="1.8"
                step="0.05"
              />
            </label>
          </template>

          <template v-else-if="props.visualStyle === 'waterfall'">
            <label class="field">
              <span>运动速度: {{ props.editingVisualTuning.waterfallSpeed.toFixed(2) }}</span>
              <small>控制瀑布脉冲的节奏变化速度</small>
              <input
                v-model.number="props.editingVisualTuning.waterfallSpeed"
                class="slider"
                type="range"
                min="0.4"
                max="2.4"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>元素密度: {{ props.editingVisualTuning.waterfallDensity.toFixed(2) }}</span>
              <small>控制瀑布柱条数量</small>
              <input
                v-model.number="props.editingVisualTuning.waterfallDensity"
                class="slider"
                type="range"
                min="0.5"
                max="2.2"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>拖尾强度: {{ props.editingVisualTuning.waterfallTrail.toFixed(2) }}</span>
              <small>控制拖尾长度与泛光强度</small>
              <input
                v-model.number="props.editingVisualTuning.waterfallTrail"
                class="slider"
                type="range"
                min="0.2"
                max="1.8"
                step="0.05"
              />
            </label>
          </template>

          <template v-else-if="props.visualStyle === 'radar'">
            <label class="field">
              <span>运动速度: {{ props.editingVisualTuning.radarSpeed.toFixed(2) }}</span>
              <small>控制扇扫旋转速度</small>
              <input
                v-model.number="props.editingVisualTuning.radarSpeed"
                class="slider"
                type="range"
                min="0.4"
                max="2.4"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>元素密度: {{ props.editingVisualTuning.radarDensity.toFixed(2) }}</span>
              <small>控制雷达回波点数量</small>
              <input
                v-model.number="props.editingVisualTuning.radarDensity"
                class="slider"
                type="range"
                min="0.4"
                max="2.2"
                step="0.05"
              />
            </label>
            <label class="field">
              <span>发光强度: {{ props.editingVisualTuning.radarGlow.toFixed(2) }}</span>
              <small>控制回波亮度和扫描扇区发光</small>
              <input
                v-model.number="props.editingVisualTuning.radarGlow"
                class="slider"
                type="range"
                min="0"
                max="1.8"
                step="0.05"
              />
            </label>
          </template>
        </section>

        <label class="field">
          <span>{{ props.t.settings.quality }}</span>
          <small>{{ props.t.settings.qualityHelp }}</small>
          <div class="select-wrap">
            <select v-model="props.editingSettings.quality" class="select-input">
              <option v-for="item in props.qualityOptions" :key="item.value" :value="item.value">
                {{ item.label }}
              </option>
            </select>
          </div>
        </label>

        <label class="field">
          <span>{{ props.t.settings.windowMode }}</span>
          <small>{{ props.t.settings.windowModeHelp }}</small>
          <div class="select-wrap">
            <select v-model="props.editingSettings.windowMode" class="select-input">
              <option v-for="item in props.windowModeOptions" :key="item.value" :value="item.value">
                {{ item.label }}
              </option>
            </select>
          </div>
        </label>

        <label class="field">
          <span>{{ props.t.settings.monitor }}</span>
          <small>{{ props.t.settings.monitorHelp }}</small>
          <div class="select-wrap">
            <select v-model="props.editingSettings.targetMonitorId" class="select-input">
              <option value="">{{ props.t.settings.monitorAuto }}</option>
              <option v-for="monitor in props.discoveredMonitors" :key="monitor.id" :value="monitor.id">
                {{ monitor.label }}
              </option>
            </select>
          </div>
        </label>

        <label class="switch-field">
          <input v-model="props.editingSettings.clickThrough" class="switch-input" type="checkbox" />
          <span class="switch-ui"></span>
          <span>{{ props.t.settings.clickThrough }}</span>
          <small>{{ props.t.settings.clickThroughHelp }}</small>
        </label>

        <label class="switch-field">
          <input v-model="props.editingSettings.launchAtStartup" class="switch-input" type="checkbox" />
          <span class="switch-ui"></span>
          <span>{{ props.t.settings.launchAtStartup }}</span>
          <small>{{ props.t.settings.launchAtStartupHelp }}</small>
        </label>

        <p v-if="props.settingsError" class="error">{{ props.settingsError }}</p>

        <div class="modal-actions">
          <button type="button" class="action-btn ghost" @click="emit('close')">{{ props.t.actions.cancel }}</button>
          <button type="button" class="action-btn ghost" @click="emit('reset')">{{ props.t.actions.reset }}</button>
          <button type="button" class="action-btn" :disabled="props.settingsSaving" @click="emit('apply')">
            {{ props.settingsSaving ? "保存中..." : props.t.actions.apply }}
          </button>
        </div>
      </section>
    </div>
  </Transition>
</template>

<style scoped>
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
  transform-origin: center;
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
  transition: transform 0.2s ease, background-color 0.2s ease;
}

.modal-close:hover {
  transform: rotate(90deg);
  background: rgba(255, 255, 255, 0.14);
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

.fx-panel {
  display: grid;
  gap: 8px;
  padding: 10px;
  border-radius: 10px;
  border: 1px solid rgba(170, 217, 207, 0.28);
  background: rgba(14, 33, 47, 0.52);
}

.fx-panel-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.fx-panel-head > span {
  font-size: 0.82rem;
}

.fx-panel-head > small {
  color: #acc3bc;
  font-size: 0.72rem;
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
  border: 1px solid rgba(78, 174, 142, 0.4);
  background: linear-gradient(180deg, rgba(37, 152, 116, 0.98), rgba(28, 126, 96, 0.98));
  color: #fff;
  font-size: 0.8rem;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease, opacity 0.2s ease;
}

.action-btn:disabled {
  opacity: 0.65;
  cursor: not-allowed;
}

.action-btn:not(:disabled):hover {
  transform: translateY(-1px);
  box-shadow: 0 8px 16px rgba(21, 111, 82, 0.25);
}

.action-btn.ghost {
  background: rgba(255, 255, 255, 0.07);
  border-color: rgba(214, 234, 227, 0.3);
}

.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.26s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}

.modal-fade-enter-active .modal,
.modal-fade-leave-active .modal {
  transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.28s ease;
}

.modal-fade-enter-from .modal,
.modal-fade-leave-to .modal {
  transform: translateY(14px) scale(0.96);
  opacity: 0;
}
</style>
