use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// 窗口模式：普通窗口 / 桌面组件 / 悬浮覆盖层。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WindowMode {
    #[default]
    Normal,
    DesktopWidget,
    Overlay,
}

impl WindowMode {
    /// 将字符串模式解析为枚举，非法值统一回退到 `Normal`。
    pub fn from_raw(value: &str) -> Self {
        match value {
            "desktopWidget" => Self::DesktopWidget,
            "overlay" => Self::Overlay,
            _ => Self::Normal,
        }
    }

}

/// 窗口行为快照：用于命令层在多状态间保持一致行为。
#[derive(Debug, Clone, Copy)]
pub struct WindowBehaviorSnapshot {
    pub mode: WindowMode,
    pub click_through: bool,
}

/// 窗口行为运行时状态：共享当前模式和点击穿透配置。
#[derive(Clone)]
pub struct WindowBehaviorState {
    inner: Arc<Mutex<WindowBehaviorSnapshot>>,
}

impl WindowBehaviorState {
    /// 创建窗口状态容器，初始值由持久化设置注入。
    pub fn new(mode: WindowMode, click_through: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(WindowBehaviorSnapshot {
                mode,
                click_through,
            })),
        }
    }

    /// 读取当前窗口行为快照。
    pub fn get(&self) -> WindowBehaviorSnapshot {
        self.inner
            .lock()
            .map(|guard| *guard)
            .unwrap_or(WindowBehaviorSnapshot {
                mode: WindowMode::Normal,
                click_through: false,
            })
    }

    /// 更新当前窗口模式。
    pub fn set_mode(&self, mode: WindowMode) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.mode = mode;
        }
    }

    /// 更新点击穿透配置（是否请求穿透，而非是否最终生效）。
    pub fn set_click_through(&self, enabled: bool) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.click_through = enabled;
        }
    }
}

/// 前端显示器下拉框所需的数据结构。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
    pub is_current: bool,
}

/// 获取主窗口句柄，统一错误文案。
pub fn main_window(app: &tauri::AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())
}

/// 应用窗口模式策略：不同模式切换窗口层级、装饰和任务栏行为。
pub fn apply_window_mode(window: &WebviewWindow, mode: WindowMode) -> Result<(), String> {
    match mode {
        WindowMode::Normal => {
            window
                .set_decorations(true)
                .map_err(|err| format!("failed to enable decorations: {err}"))?;
            window
                .set_resizable(true)
                .map_err(|err| format!("failed to set resizable: {err}"))?;
            window
                .set_skip_taskbar(false)
                .map_err(|err| format!("failed to show taskbar item: {err}"))?;
            window
                .set_always_on_bottom(false)
                .map_err(|err| format!("failed to disable always-on-bottom: {err}"))?;
            window
                .set_always_on_top(false)
                .map_err(|err| format!("failed to disable always-on-top: {err}"))?;
            window
                .set_focusable(true)
                .map_err(|err| format!("failed to set focusable: {err}"))?;
        }
        WindowMode::DesktopWidget => {
            window
                .set_decorations(false)
                .map_err(|err| format!("failed to disable decorations: {err}"))?;
            window
                .set_resizable(false)
                .map_err(|err| format!("failed to set resizable: {err}"))?;
            window
                .set_skip_taskbar(true)
                .map_err(|err| format!("failed to hide taskbar item: {err}"))?;
            window
                .set_always_on_top(false)
                .map_err(|err| format!("failed to disable always-on-top: {err}"))?;
            window
                .set_always_on_bottom(true)
                .map_err(|err| format!("failed to enable always-on-bottom: {err}"))?;
            window
                .set_focusable(true)
                .map_err(|err| format!("failed to set focusable: {err}"))?;
        }
        WindowMode::Overlay => {
            window
                .set_decorations(false)
                .map_err(|err| format!("failed to disable decorations: {err}"))?;
            window
                .set_resizable(false)
                .map_err(|err| format!("failed to set resizable: {err}"))?;
            window
                .set_skip_taskbar(true)
                .map_err(|err| format!("failed to hide taskbar item: {err}"))?;
            window
                .set_always_on_bottom(false)
                .map_err(|err| format!("failed to disable always-on-bottom: {err}"))?;
            window
                .set_always_on_top(true)
                .map_err(|err| format!("failed to enable always-on-top: {err}"))?;
            window
                .set_focusable(true)
                .map_err(|err| format!("failed to set focusable: {err}"))?;
        }
    }

    Ok(())
}

/// 枚举可用显示器并标记主屏/当前屏，供前端选择目标显示器。
pub fn list_monitors(window: &WebviewWindow) -> Result<Vec<MonitorInfo>, String> {
    let monitors = window
        .available_monitors()
        .map_err(|err| format!("failed to get monitors: {err}"))?;
    let primary_name = window
        .primary_monitor()
        .map_err(|err| format!("failed to get primary monitor: {err}"))?
        .and_then(|monitor| monitor.name().cloned());
    let current_name = window
        .current_monitor()
        .map_err(|err| format!("failed to get current monitor: {err}"))?
        .and_then(|monitor| monitor.name().cloned());

    let items = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("显示器 {}", index + 1));
            let size = monitor.size();
            let label = format!("{name} ({}x{})", size.width, size.height);
            let id = monitor_identity(index, monitor);

            MonitorInfo {
                id,
                label,
                width: size.width,
                height: size.height,
                scale_factor: monitor.scale_factor(),
                is_primary: primary_name
                    .as_ref()
                    .is_some_and(|primary| monitor.name().is_some_and(|name| name == primary)),
                is_current: current_name
                    .as_ref()
                    .is_some_and(|current| monitor.name().is_some_and(|name| name == current)),
            }
        })
        .collect();

    Ok(items)
}

/// 将窗口移动到目标显示器工作区，尺寸自动裁剪到工作区内。
pub fn move_window_to_monitor(window: &WebviewWindow, monitor_id: &str) -> Result<(), String> {
    let monitors = window
        .available_monitors()
        .map_err(|err| format!("failed to get monitors: {err}"))?;
    let maybe_target = monitors
        .iter()
        .enumerate()
        .find(|(index, monitor)| monitor_identity(*index, monitor) == monitor_id);

    let Some((_, target_monitor)) = maybe_target else {
        return Err(format!("monitor not found: {monitor_id}"));
    };

    let work_area = target_monitor.work_area();
    let current_size = window
        .outer_size()
        .map_err(|err| format!("failed to read window size: {err}"))?;
    let width = current_size.width.min(work_area.size.width);
    let height = current_size.height.min(work_area.size.height);

    // 关键行：先移动到目标屏工作区左上角，再按工作区限制调整窗口尺寸。
    window
        .set_position(PhysicalPosition::new(work_area.position.x, work_area.position.y))
        .map_err(|err| format!("failed to move window: {err}"))?;
    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|err| format!("failed to resize window: {err}"))?;
    Ok(())
}

/// 生成稳定显示器标识，避免只依赖名称导致重名冲突。
fn monitor_identity(index: usize, monitor: &tauri::Monitor) -> String {
    let position = monitor.position();
    let size = monitor.size();
    format!(
        "{}:{}:{}:{}:{}",
        index, position.x, position.y, size.width, size.height
    )
}
