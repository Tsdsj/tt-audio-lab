use crate::desktop::window_mode::WindowMode;
use tauri::WebviewWindow;

/// 应用点击穿透策略：仅在桌面组件/覆盖层模式允许真正穿透，避免普通模式锁死交互。
pub fn apply_click_through(
    window: &WebviewWindow,
    mode: WindowMode,
    requested_enabled: bool,
) -> Result<bool, String> {
    // 关键行：普通窗口强制禁用系统级穿透，确保设置窗口始终可恢复操作。
    let effective_enabled = requested_enabled && !matches!(mode, WindowMode::Normal);
    window
        .set_ignore_cursor_events(effective_enabled)
        .map_err(|err| format!("failed to set click-through: {err}"))?;
    Ok(effective_enabled)
}
