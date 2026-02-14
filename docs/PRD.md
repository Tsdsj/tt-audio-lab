# tt-audio-lab PRD (v0.2)

## 1. Product Overview
- Product name: `tt-audio-lab`
- Target platform: `Windows 10 22H2+ / Windows 11`
- Product type: desktop audio visualization widget
- UI language: Simplified Chinese (`zh-CN`) for MVP.
- Core objective: provide a low-latency, low-overhead, highly customizable desktop visualizer for system playback audio.

## 2. Goals and Non-Goals
### 2.1 Goals
- Deliver an MVP based on `Tauri 2 + TypeScript frontend + Rust audio core`.
- Support smooth rendering on high-refresh displays, with a target compatibility ceiling of `240Hz`.
- Ensure all user-facing UI text is Simplified Chinese (`zh-CN`) in MVP.
- Provide practical desktop-widget behaviors:
- transparent frameless window
- optional click-through mode
- tray control and auto-start
- controllable style/theme and layout

### 2.2 Non-Goals (MVP)
- No online theme marketplace.
- No cross-platform support (macOS/Linux) in MVP.
- No advanced DAW routing/multi-bus mixing.

## 3. Target Users
- Users who want music-reactive desktop visuals while coding/gaming/streaming.
- Frontend developers/designers who want customizable visual themes.
- Streamers who want lightweight always-on visual overlay/widget behavior.

## 4. Core Scenarios
- Scenario A: user starts app, it auto-captures default playback device, shows visualizer within 2 seconds.
- Scenario B: user changes style/color/sensitivity in settings, effect applies in real time.
- Scenario C: user enables click-through and pins widget to desktop layer while continuing normal desktop usage.
- Scenario D: user with 144Hz/240Hz display gets fluid animation with graceful fallback when load rises.

## 5. Functional Requirements
- `FR-001` Audio capture
- Capture system playback audio via WASAPI loopback.
- Allow selecting audio output device and auto-reconnect on device changes.
- `FR-002` Signal processing
- Provide FFT-based spectrum extraction and configurable smoothing/gain/noise gate.
- `FR-003` Visualization styles
- MVP includes at least 3 styles: bar spectrum, waveform line, radial spectrum.
- `FR-004` Desktop widget behavior
- Frameless transparent window.
- Optional always-on-bottom desktop behavior.
- Optional click-through (ignore mouse events).
- `FR-005` Controls and settings
- System tray menu: show/hide, pause, resume, settings, exit.
- Persist settings in local JSON file.
- `FR-006` Startup and recovery
- Optional start-on-boot.
- Auto-recover after explorer.exe restart.
- `FR-007` Multi-monitor baseline
- User can choose target monitor.
- `FR-008` Diagnostics
- Built-in performance panel: FPS, render time, CPU estimate, audio latency estimate.
- `FR-009` Language and copy
- All visible UI text (settings panel, tray menu, status text, error prompts) must be Simplified Chinese (`zh-CN`).

## 6. Non-Functional Requirements
- `NFR-001` High refresh compatibility
- Render loop should follow monitor refresh (`requestAnimationFrame`) up to `240Hz` when hardware and WebView allow.
- `NFR-002` Frame budget targets
- 60Hz: <= 16.7ms/frame
- 120Hz: <= 8.3ms/frame
- 240Hz: <= 4.2ms/frame
- `NFR-003` Latency target
- End-to-end (audio sample -> visual frame): <= 100ms (target), <= 140ms (max).
- `NFR-004` Resource target (1080p single widget baseline)
- CPU average <= 6%
- Memory <= 250MB
- `NFR-005` Stability
- 24h continuous run without crash/hang.
- `NFR-006` Degradation behavior
- If sustained frame misses occur, system auto-downgrades visual complexity before dropping UX-critical functions.

## 7. 240Hz Adaptation Policy
- Rendering must be decoupled from IPC update frequency.
- Rust side emits analysis frames at configurable rates (for example 60/90/120Hz tiers).
- Frontend interpolates between analysis frames and still renders at display refresh.
- Provide quality levels:
- `Ultra` (target 240Hz displays, WebGL path)
- `High` (target 120Hz)
- `Balanced` (target 60Hz / lower-power devices)
- Auto-switch down one tier if frame-time p95 exceeds budget for longer than 5 seconds.

## 8. Acceptance Criteria (MVP)
- AC-01 App launches and produces non-empty visual response to system audio within 2s.
- AC-02 User can switch between at least 3 visual styles without restart.
- AC-03 Click-through mode and tray controls work reliably.
- AC-04 On a compatible 240Hz monitor, FPS can exceed 180 and approach monitor refresh under default style/preset.
- AC-05 When under stress, app degrades gracefully without freezing UI.
- AC-06 Settings persist and reload correctly after restart.
- AC-07 All visible UI copy is Simplified Chinese (`zh-CN`) and no placeholder English remains in release build.

## 9. Milestones
- `M1` Foundation (Week 1)
- Tauri app shell, frontend scaffold, Rust command/event bridge, settings storage.
- `M2` Audio + DSP (Week 2)
- WASAPI loopback capture, FFT pipeline, normalization/smoothing, telemetry metrics.
- `M3` Visual + Widget behaviors (Week 3)
- Three visual styles, transparent window, click-through, tray menu, monitor select.
- `M4` Performance and 240Hz tuning (Week 4)
- WebGL path optimization, IPC throttling/interpolation, presets, stress test, bug fixes.

## 10. Risks and Mitigations
- Risk: IPC overhead too high for per-frame data at high Hz.
- Mitigation: lower analysis push rate + frontend interpolation + compact payload format.
- Risk: Windows desktop-layer behavior differences across versions.
- Mitigation: keep a fallback window mode if always-on-bottom behavior is unstable.
- Risk: 240Hz varies by GPU/WebView2 environment.
- Mitigation: expose quality presets and performance diagnostics; define "compatibility target" not guaranteed fixed FPS.
