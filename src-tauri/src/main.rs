#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod desktop;
mod settings;
mod telemetry;

use desktop::window_mode::{WindowBehaviorState, WindowMode};
use tauri::{Emitter, Manager};

#[cfg(desktop)]
const TRAY_SHOW_ID: &str = "tray_show";
#[cfg(desktop)]
const TRAY_HIDE_ID: &str = "tray_hide";
#[cfg(desktop)]
const TRAY_PAUSE_ID: &str = "tray_pause";
#[cfg(desktop)]
const TRAY_RESUME_ID: &str = "tray_resume";
#[cfg(desktop)]
const TRAY_SETTINGS_ID: &str = "tray_settings";
#[cfg(desktop)]
const TRAY_DISABLE_CLICK_THROUGH_ID: &str = "tray_disable_click_through";
#[cfg(desktop)]
const TRAY_EXIT_ID: &str = "tray_exit";

#[cfg(desktop)]
fn show_main_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = desktop::window_mode::main_window(app)?;
    window
        .show()
        .map_err(|err| format!("failed to show main window: {err}"))?;
    window
        .set_focus()
        .map_err(|err| format!("failed to focus main window: {err}"))?;
    Ok(())
}

#[cfg(desktop)]
fn hide_main_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = desktop::window_mode::main_window(app)?;
    window
        .hide()
        .map_err(|err| format!("failed to hide main window: {err}"))
}

#[cfg(desktop)]
fn open_settings_from_tray(app: &tauri::AppHandle) -> Result<(), String> {
    show_main_window(app)?;
    app.emit("app:open_settings", ())
        .map_err(|err| format!("failed to emit open settings event: {err}"))
}

#[cfg(desktop)]
fn set_visual_paused_from_tray(app: &tauri::AppHandle, paused: bool) -> Result<(), String> {
    let visual_state = app.state::<telemetry::RuntimeVisualState>();
    visual_state.set_paused(paused);
    app.emit("app:visual_paused", paused)
        .map_err(|err| format!("failed to emit pause event: {err}"))
}

#[cfg(desktop)]
fn disable_click_through_from_tray(app: &tauri::AppHandle) -> Result<(), String> {
    let window = desktop::window_mode::main_window(app)?;
    let behavior_state = app.state::<WindowBehaviorState>();
    let mode = behavior_state.get().mode;

    desktop::click_through::apply_click_through(&window, mode, false)?;
    behavior_state.set_click_through(false);

    // 关键行：托盘关闭点击穿透后同步落盘，避免重启后又恢复到穿透状态。
    if let Ok(mut persisted_settings) = settings::load_settings_from_disk() {
        persisted_settings.click_through = false;
        let _ = settings::save_settings_to_disk(&persisted_settings);
    }

    app.emit("app:click_through_changed", false)
        .map_err(|err| format!("failed to emit click-through event: {err}"))
}

#[cfg(desktop)]
fn handle_tray_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    let result = match menu_id {
        TRAY_SHOW_ID => show_main_window(app),
        TRAY_HIDE_ID => hide_main_window(app),
        TRAY_PAUSE_ID => set_visual_paused_from_tray(app, true),
        TRAY_RESUME_ID => set_visual_paused_from_tray(app, false),
        TRAY_SETTINGS_ID => open_settings_from_tray(app),
        TRAY_DISABLE_CLICK_THROUGH_ID => disable_click_through_from_tray(app),
        TRAY_EXIT_ID => {
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        eprintln!("tray action failed ({menu_id}): {error}");
    }
}

#[cfg(desktop)]
fn setup_tray(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let item_show = MenuItem::with_id(app, TRAY_SHOW_ID, "显示主窗口", true, None::<&str>)
        .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_hide = MenuItem::with_id(app, TRAY_HIDE_ID, "隐藏主窗口", true, None::<&str>)
        .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_pause = MenuItem::with_id(app, TRAY_PAUSE_ID, "暂停可视化", true, None::<&str>)
        .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_resume = MenuItem::with_id(app, TRAY_RESUME_ID, "恢复可视化", true, None::<&str>)
        .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_settings =
        MenuItem::with_id(app, TRAY_SETTINGS_ID, "打开设置", true, None::<&str>)
            .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_disable_click_through = MenuItem::with_id(
        app,
        TRAY_DISABLE_CLICK_THROUGH_ID,
        "关闭点击穿透",
        true,
        None::<&str>,
    )
    .map_err(|err| format!("failed to create tray item: {err}"))?;
    let item_exit = MenuItem::with_id(app, TRAY_EXIT_ID, "退出", true, None::<&str>)
        .map_err(|err| format!("failed to create tray item: {err}"))?;

    let separator_1 =
        PredefinedMenuItem::separator(app).map_err(|err| format!("failed to create separator: {err}"))?;
    let separator_2 =
        PredefinedMenuItem::separator(app).map_err(|err| format!("failed to create separator: {err}"))?;
    let separator_3 =
        PredefinedMenuItem::separator(app).map_err(|err| format!("failed to create separator: {err}"))?;

    let menu = Menu::with_items(
        app,
        &[
            &item_show,
            &item_hide,
            &separator_1,
            &item_pause,
            &item_resume,
            &separator_2,
            &item_settings,
            &item_disable_click_through,
            &separator_3,
            &item_exit,
        ],
    )
    .map_err(|err| format!("failed to build tray menu: {err}"))?;

    let mut tray_builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("tt-audio-lab")
        .menu(&menu)
        .show_menu_on_left_click(true);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder
        .build(app)
        .map_err(|err| format!("failed to create tray icon: {err}"))?;
    Ok(())
}

fn main() {
    let initial_settings = settings::load_settings_from_disk().unwrap_or_default();
    let initial_window_mode = WindowMode::from_raw(&initial_settings.window_mode);

    let runtime_dsp =
        telemetry::RuntimeDspState::new(telemetry::runtime_config_from_settings(&initial_settings));
    let runtime_dsp_for_setup = runtime_dsp.clone();

    let runtime_visual = telemetry::RuntimeVisualState::default();
    let runtime_visual_for_setup = runtime_visual.clone();

    let window_behavior_state =
        WindowBehaviorState::new(initial_window_mode, initial_settings.click_through);
    let window_behavior_for_setup = window_behavior_state.clone();
    let settings_for_setup = initial_settings.clone();

    // 启动实时分析事件流，并在 setup 阶段应用窗口相关初始设置。
    let builder = tauri::Builder::default()
        .manage(runtime_dsp)
        .manage(runtime_visual)
        .manage(window_behavior_state)
        .setup(move |app| {
            commands::apply_runtime_window_behavior(
                app.handle(),
                &settings_for_setup,
                &window_behavior_for_setup,
            )?;

            telemetry::start_analysis_emitter(
                app.handle().clone(),
                runtime_dsp_for_setup.clone(),
                runtime_visual_for_setup.clone(),
            );

            #[cfg(desktop)]
            {
                setup_tray(app.handle())?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::list_audio_devices,
            commands::list_monitors,
            commands::load_settings,
            commands::save_settings,
            commands::set_window_mode,
            commands::set_target_monitor,
            commands::set_click_through,
            commands::set_visual_paused,
        ]);

    #[cfg(desktop)]
    let builder = builder
        .on_menu_event(|app, event| {
            handle_tray_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|app, event| {
            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                let _ = show_main_window(app);
            }
        });

    builder
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
