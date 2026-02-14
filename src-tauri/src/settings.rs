use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub quality: String,
    pub smoothing: f32,
    pub gain: f32,
    pub click_through: bool,
    pub launch_at_startup: bool,
    pub window_mode: String,
    pub target_monitor_id: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            quality: "ultra".to_string(),
            smoothing: 0.58,
            gain: 1.8,
            click_through: false,
            launch_at_startup: false,
            window_mode: "normal".to_string(),
            target_monitor_id: String::new(),
        }
    }
}

/// 解析设置目录并自动创建，统一使用 `%APPDATA%/tt-audio-lab`。
fn settings_dir() -> Result<PathBuf, String> {
    let app_data =
        std::env::var("APPDATA").map_err(|err| format!("APPDATA is not available: {err}"))?;
    let dir = PathBuf::from(app_data).join("tt-audio-lab");
    fs::create_dir_all(&dir)
        .map_err(|err| format!("failed to create settings directory: {err}"))?;
    Ok(dir)
}

/// 设置文件路径：`%APPDATA%/tt-audio-lab/settings.json`。
fn settings_path() -> Result<PathBuf, String> {
    Ok(settings_dir()?.join(SETTINGS_FILE_NAME))
}

/// 加载设置，文件不存在时返回默认设置，保证首次运行可用。
pub fn load_settings_from_disk() -> Result<AppSettings, String> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let raw = fs::read_to_string(&path).map_err(|err| format!("failed to read settings: {err}"))?;
    serde_json::from_str::<AppSettings>(&raw)
        .map_err(|err| format!("failed to parse settings json: {err}"))
}

/// 保存设置为格式化 JSON，便于本地排障和手工调整参数。
pub fn save_settings_to_disk(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;
    fs::write(path, content).map_err(|err| format!("failed to write settings file: {err}"))?;
    Ok(())
}
