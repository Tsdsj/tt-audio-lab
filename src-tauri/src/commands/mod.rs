use crate::audio::capture::{self, AudioDeviceInfo};
use crate::desktop::{
    click_through,
    window_mode::{self, MonitorInfo, WindowBehaviorState, WindowMode},
};
use crate::settings::{self, AppSettings};
use crate::telemetry::{runtime_config_from_settings, RuntimeDspState, RuntimeVisualState};
use tauri::{Emitter, State};

/// 基础健康检查命令，用于验证前后端命令桥接是否可用。
#[tauri::command]
pub fn health_check() -> &'static str {
    "ok"
}

/// 读取可用音频设备列表，供前端设备选择器使用。
#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    capture::list_audio_devices()
}

/// 枚举系统显示器信息，供前端设置目标显示器。
#[tauri::command]
pub fn list_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let window = window_mode::main_window(&app)?;
    window_mode::list_monitors(&window)
}

/// 加载持久化设置，如果不存在则返回默认值。
#[tauri::command]
pub fn load_settings() -> Result<AppSettings, String> {
    settings::load_settings_from_disk()
}

/// 保存完整设置对象，并同步运行时 DSP 与窗口行为。
#[tauri::command]
pub fn save_settings(
    mut settings: AppSettings,
    app: tauri::AppHandle,
    runtime_dsp: State<'_, RuntimeDspState>,
    window_state: State<'_, WindowBehaviorState>,
) -> Result<(), String> {
    runtime_dsp.set(runtime_config_from_settings(&settings));

    let requested_click = settings.click_through;
    let effective_click = apply_runtime_window_behavior(&app, &settings, &window_state)?;
    settings.click_through = effective_click;

    if requested_click != effective_click {
        let _ = app.emit("app:click_through_changed", effective_click);
    }

    settings::save_settings_to_disk(&settings)
}

/// 切换窗口模式：普通窗口 / 桌面组件 / 悬浮覆盖层。
#[tauri::command]
pub fn set_window_mode(
    app: tauri::AppHandle,
    mode: String,
    window_state: State<'_, WindowBehaviorState>,
) -> Result<(), String> {
    let window = window_mode::main_window(&app)?;
    let parsed_mode = WindowMode::from_raw(&mode);

    window_mode::apply_window_mode(&window, parsed_mode)?;
    window_state.set_mode(parsed_mode);

    let click_requested = window_state.get().click_through;
    let effective = click_through::apply_click_through(&window, parsed_mode, click_requested)?;

    // 当普通模式强制关闭穿透时，通知前端同步状态，避免 UI 与实际行为不一致。
    if click_requested && !effective {
        window_state.set_click_through(false);
        let _ = app.emit("app:click_through_changed", false);
    }

    Ok(())
}

/// 将窗口移动到指定显示器。
#[tauri::command]
pub fn set_target_monitor(app: tauri::AppHandle, monitor_id: String) -> Result<(), String> {
    if monitor_id.trim().is_empty() {
        return Ok(());
    }

    let window = window_mode::main_window(&app)?;
    window_mode::move_window_to_monitor(&window, &monitor_id)
}

/// 切换点击穿透：仅在桌面组件/悬浮模式生效，普通模式会自动禁用。
#[tauri::command]
pub fn set_click_through(
    app: tauri::AppHandle,
    enabled: bool,
    window_state: State<'_, WindowBehaviorState>,
) -> Result<(), String> {
    let window = window_mode::main_window(&app)?;
    let snapshot = window_state.get();
    let effective = click_through::apply_click_through(&window, snapshot.mode, enabled)?;

    window_state.set_click_through(effective);
    if effective != enabled {
        let _ = app.emit("app:click_through_changed", effective);
    }

    Ok(())
}

/// 切换可视化暂停状态，用于托盘菜单的暂停/恢复。
#[tauri::command]
pub fn set_visual_paused(
    paused: bool,
    runtime_visual: State<'_, RuntimeVisualState>,
) -> Result<(), String> {
    runtime_visual.set_paused(paused);
    Ok(())
}

/// 统一应用窗口相关设置，避免不同命令分叉出不一致行为。
pub fn apply_runtime_window_behavior(
    app: &tauri::AppHandle,
    settings: &AppSettings,
    window_state: &WindowBehaviorState,
) -> Result<bool, String> {
    let window = window_mode::main_window(app)?;
    let mode = WindowMode::from_raw(&settings.window_mode);

    window_mode::apply_window_mode(&window, mode)?;
    window_state.set_mode(mode);

    if !settings.target_monitor_id.trim().is_empty() {
        if let Err(error) = window_mode::move_window_to_monitor(&window, &settings.target_monitor_id)
        {
            eprintln!(
                "failed to move window to monitor {}: {error}",
                settings.target_monitor_id
            );
        }
    }

    let effective_click = click_through::apply_click_through(&window, mode, settings.click_through)?;
    window_state.set_click_through(effective_click);
    Ok(effective_click)
}
