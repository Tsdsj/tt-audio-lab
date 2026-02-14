# tt-audio-lab Architecture (v0.2)

## 1. Stack Decision
- Desktop runtime: `Tauri 2`
- Frontend: `TypeScript + Vite + Vue 3` (React is swappable if team preference changes)
- Rendering: `WebGL2` first (Canvas2D fallback)
- Rust core: WASAPI capture + DSP + native window helpers
- Localization: `zh-CN` as required UI language in MVP, managed in frontend locale resources
- Packaging: `tauri build` to MSI/EXE

Reasoning:
- Tauri keeps memory/cpu lower than Electron for always-running desktop widgets.
- Frontend stack remains familiar to Java/frontend developers.
- Rust is used only where native realtime/audio behavior is required.

## 2. High-Level Components
- `frontend-app` (TS/Vue)
- Settings UI, visualizer scene, perf HUD, preset management.
- `tauri-shell`
- App lifecycle, tray menu, window management, command/event bridge.
- `audio-core` (Rust)
- WASAPI loopback capture, FFT pipeline, smoothing/normalization, metrics.
- `desktop-host` (Rust + Windows API)
- Click-through, window level policy, monitor targeting, explorer recovery hooks.

## 3. Proposed Repository Layout
```text
tt-audio-lab/
  docs/
    PRD.md
    ARCH.md
  src/
    app/
      main.ts
      locales/
        zh-CN.ts
      stores/
      views/
      visualizers/
      workers/
  src-tauri/
    src/
      main.rs
      commands/
      audio/
        capture.rs
        dsp.rs
        ring_buffer.rs
      desktop/
        window_mode.rs
        click_through.rs
      telemetry/
        metrics.rs
    tauri.conf.json
```

## 4. Data Flow
1. Rust `capture` thread reads PCM frames from WASAPI loopback.
2. Rust `dsp` thread/window computes FFT bins + energy + peak envelope.
3. Rust publishes compact analysis frame to frontend at configured rate (60/90/120Hz tiers).
4. Frontend render loop (`requestAnimationFrame`) runs at monitor refresh (up to 240Hz), interpolating between latest analysis frames.
5. Frontend visualizer draws using WebGL shaders; UI controls send setting updates back to Rust.

## 5. 240Hz Performance Strategy
Key principle: do not bind analysis push rate to render refresh rate.

- Render budget targets:
- 60Hz: <= 16.7ms/frame
- 120Hz: <= 8.3ms/frame
- 240Hz: <= 4.2ms/frame
- IPC policy:
- Avoid sending full-resolution float arrays every render frame.
- Send quantized or packed bins at max 120 updates/sec by default.
- Frontend policy:
- Maintain lightweight scene graph.
- Preallocate typed arrays.
- Use interpolation/decay logic in shader or render step.
- Quality tiers:
- `Ultra`: high bin count/effects, target high refresh monitors.
- `High`: medium complexity.
- `Balanced`: reduced bins and effects.
- Auto degradation:
- If render frame-time p95 breaches tier budget for >5s, step down one tier.

## 6. Rust Module Design
- `audio/capture.rs`
- Device enumeration, default-device tracking, loopback stream start/stop.
- `audio/dsp.rs`
- Windowing (Hann/Blackman), FFT, logarithmic bin mapping, smoothing filters.
- `audio/ring_buffer.rs`
- Lock-minimized transfer between capture and DSP stages.
- `commands/*.rs`
- Tauri command handlers (`start_capture`, `set_device`, `set_quality`, `get_metrics`).
- `desktop/*.rs`
- Windows-specific window mode operations and recovery behavior.
- `telemetry/metrics.rs`
- FPS counters, DSP timing, queue depth, underrun/overrun counters.

## 7. Frontend Design
- State:
- `settings store`: visual style, colors, quality tier, selected monitor/device.
- `runtime store`: latest analysis frame + metrics.
- `locale store`: fixed `zh-CN` in MVP (future-ready for additional locales).
- Rendering:
- `visualizers/*`: each style as pluggable renderer sharing common frame input.
- `workers/*`:
- Optional compute worker for heavy pre-processing if needed.
- UI:
- Settings panel + mini performance overlay + preset load/save.
- All user-facing copy comes from centralized locale keys; no hard-coded English in components.

## 8. IPC Contract (Draft)
- Rust -> frontend event `audio:analysis_frame`
- `timestamp_ms`
- `device_id`
- `bins` (packed/quantized array)
- `rms`, `peak`, `latency_estimate_ms`
- Frontend -> Rust commands
- `cmd_set_device(device_id)`
- `cmd_set_quality(tier)`
- `cmd_set_dsp_params(gain, smoothing, gate)`
- `cmd_set_window_mode(mode)`

## 9. Window and Desktop Modes
- Mode A: normal transparent window (default safe fallback).
- Mode B: desktop-widget mode (always-on-bottom + optional click-through).
- Mode C: overlay mode (always-on-top) for streaming use.

Notes:
- Some behaviors differ by Windows build and shell state.
- Keep fallback logic and mode switch accessible in tray menu.
- Tray labels and window-mode labels must use `zh-CN` locale keys.

## 10. Observability and Testing
- Runtime metrics panel:
- Render FPS/current frame-time/p95 frame-time.
- DSP processing time and queue lag.
- Device reconnect count and error codes.
- Test matrix:
- Win10 + Win11
- 60/120/144/240Hz displays
- Intel + NVIDIA + AMD GPU combinations if available
- Long-run soak test (>=24h)

## 11. Open Decisions
- Choose final frontend framework: Vue 3 (default) vs React (if team preference).
- Confirm FFT crate choice (`realfft` + `rustfft` chain) and bin count defaults.
- Decide whether to implement advanced desktop embedding (WorkerW) in MVP or post-MVP.
