mod ai;
mod commands;
mod database;
mod resources;

use serde::Serialize;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{include_image, Emitter, Manager};

#[derive(Serialize, Clone)]
struct ScreenInfo {
    screen_width: f64,
    screen_height: f64,
    taskbar_height: f64,
    scale_factor: f64,
}

#[tauri::command]
fn set_ignore_cursor_events(app: tauri::AppHandle, ignore: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_ignore_cursor_events(ignore)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[tauri::command]
fn get_mouse_pos(app: tauri::AppHandle, window: tauri::Window) -> Result<(f64, f64), String> {
    match app.cursor_position() {
        Ok(pos) => {
            let scale_factor = window.scale_factor().unwrap_or(1.0);
            let screen_logical = pos.to_logical::<f64>(scale_factor);
            let window_physical = window
                .inner_position()
                .unwrap_or(tauri::PhysicalPosition::new(0, 0));
            let window_logical = window_physical.to_logical::<f64>(scale_factor);
            let client_x = screen_logical.x - window_logical.x;
            let client_y = screen_logical.y - window_logical.y;
            Ok((client_x, client_y))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn get_screen_info(app: tauri::AppHandle) -> Result<ScreenInfo, String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor found".to_string())?;

    let scale_factor = monitor.scale_factor();
    let size = monitor.size();
    let work_area = monitor.work_area();

    let screen_width = size.width as f64 / scale_factor;
    let screen_height = size.height as f64 / scale_factor;
    let work_height = work_area.size.height as f64 / scale_factor;
    let taskbar_height = screen_height - work_height;

    Ok(ScreenInfo {
        screen_width,
        screen_height,
        taskbar_height,
        scale_factor,
    })
}

#[tauri::command]
fn set_window_to_screen_size(app: tauri::AppHandle) -> Result<(), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor found".to_string())?;

    let size = monitor.size();
    let position = monitor.position();

    if let Some(window) = app.get_webview_window("main") {
        window
            .set_size(tauri::PhysicalSize::new(size.width, size.height))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(position.x, position.y))
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            database::bootstrap(app.handle()).map_err(Box::<dyn std::error::Error>::from)?;

            // Set window to cover the primary monitor
            let handle = app.handle().clone();
            if let Ok(Some(monitor)) = handle.primary_monitor() {
                let size = monitor.size();
                let position = monitor.position();
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.set_size(tauri::PhysicalSize::new(size.width, size.height));
                    let _ =
                        window.set_position(tauri::PhysicalPosition::new(position.x, position.y));
                }
            }

            // System tray
            let show_item = MenuItemBuilder::with_id("show-toolbox", "显示工具箱").build(app)?;
            let lull_sleep_item = MenuItemBuilder::with_id("lull-to-sleep", "哄睡").build(app)?;
            let minimize_item =
                MenuItemBuilder::with_id("minimize-to-taskbar", "最小化到任务栏").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&lull_sleep_item)
                .separator()
                .item(&minimize_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(include_image!("icons/32x32.png"))
                .menu(&menu)
                .tooltip("BytePet")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show-toolbox" => {
                        let _ = app.emit("tray-show-toolbox", ());
                    }
                    "lull-to-sleep" => {
                        let _ = app.emit("tray-lull-to-sleep", ());
                    }
                    "minimize-to-taskbar" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_skip_taskbar(false);
                            let _ = window.minimize();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let _ = tray.app_handle().emit("tray-show-toolbox", ());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_app,
            commands::send_chat_message,
            commands::feed_food,
            commands::replace_food,
            commands::reorder_foods,
            commands::record_pet_interaction,
            commands::save_settings,
            commands::reset_stats,
            commands::validate_skin_path,
            commands::list_frame_assets,
            commands::choose_and_import_frame_asset,
            commands::import_frame_asset_from_path,
            commands::delete_frame_asset,
            commands::create_character,
            commands::update_character,
            commands::update_character_scale,
            commands::switch_character,
            commands::delete_character,
            commands::delete_chat_session,
            commands::chat_history_days,
            commands::chat_history_messages_for_day,
            commands::delete_chat_history_day,
            commands::open_data_dir,
            commands::get_ai_config_public,
            commands::save_ai_provider_config,
            commands::delete_ai_provider_api_key,
            commands::delete_ai_provider_config,
            commands::set_active_ai_provider,
            commands::test_ai_connection,
            commands::stream_ai_message,
            commands::abort_ai_request,
            commands::save_interrupted_ai_message,
            set_ignore_cursor_events,
            get_mouse_pos,
            get_screen_info,
            set_window_to_screen_size
        ])
        .run(tauri::generate_context!())
        .expect("error while running BytePet");
}
