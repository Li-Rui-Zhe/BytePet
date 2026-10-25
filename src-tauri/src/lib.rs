mod ai;
mod commands;
mod credentials;
mod database;
mod desktop_geometry;
mod help;
mod resources;

use desktop_geometry::{overlay_layout, PhysicalRect, ScreenInfo};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
#[cfg(not(target_os = "macos"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{include_image, Emitter, Manager};

fn restore_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.unminimize();
        let _ = window.show();
        #[cfg(not(target_os = "macos"))]
        let _ = window.set_skip_taskbar(true);
        let _ = window.set_focus();
        let _ = app.emit("cursor-state-reset", ());
    }
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
    primary_overlay(&app).map(|(_, info)| info)
}

fn primary_overlay(app: &tauri::AppHandle) -> Result<(PhysicalRect, ScreenInfo), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No primary monitor found".to_string())?;

    let scale_factor = monitor.scale_factor();
    let size = monitor.size();
    let work_area = monitor.work_area();

    let position = monitor.position();
    overlay_layout(
        PhysicalRect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        },
        PhysicalRect {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width,
            height: work_area.size.height,
        },
        scale_factor,
        cfg!(target_os = "macos"),
    )
}

#[tauri::command]
fn set_window_to_screen_size(app: tauri::AppHandle) -> Result<(), String> {
    sync_overlay(&app)
}

fn sync_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let (area, _) = primary_overlay(app)?;
    if let Some(window) = app.get_webview_window("main") {
        let size = tauri::PhysicalSize::new(area.width, area.height);
        let position = tauri::PhysicalPosition::new(area.x, area.y);
        if window.inner_size().map_err(|e| e.to_string())? != size {
            window.set_size(size).map_err(|e| e.to_string())?;
        }
        if window.inner_position().map_err(|e| e.to_string())? != position {
            window.set_position(position).map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            database::bootstrap(app.handle())
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;

            sync_overlay(app.handle()).map_err(Box::<dyn std::error::Error>::from)?;

            // System tray
            let show_item = MenuItemBuilder::with_id("show-toolbox", "显示工具箱").build(app)?;
            let lull_sleep_item = MenuItemBuilder::with_id("lull-to-sleep", "哄睡").build(app)?;
            let hide_label = if cfg!(target_os = "macos") {
                "隐藏桌宠（菜单栏可恢复）"
            } else {
                "最小化到任务栏"
            };
            let minimize_item =
                MenuItemBuilder::with_id("minimize-to-taskbar", hide_label).build(app)?;
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
                .icon_as_template(cfg!(target_os = "macos"))
                .show_menu_on_left_click(cfg!(target_os = "macos"))
                .menu(&menu)
                .tooltip("BytePet")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show-toolbox" => {
                        restore_main_window(app);
                        let _ = app.emit("tray-show-toolbox", ());
                    }
                    "lull-to-sleep" => {
                        let _ = app.emit("tray-lull-to-sleep", ());
                    }
                    "minimize-to-taskbar" => {
                        if let Some(window) = app.get_webview_window("main") {
                            #[cfg(target_os = "macos")]
                            let _ = window.hide();
                            #[cfg(not(target_os = "macos"))]
                            {
                                let _ = window.set_skip_taskbar(false);
                                let _ = window.minimize();
                            }
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    #[cfg(target_os = "macos")]
                    let _ = (tray, event);
                    #[cfg(not(target_os = "macos"))]
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        restore_main_window(tray.app_handle());
                        let _ = tray.app_handle().emit("tray-show-toolbox", ());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            help::open_deepseek_help_link,
            commands::bootstrap_app,
            commands::list_frame_assets,
            commands::choose_and_import_frame_asset,
            commands::import_frame_asset_from_path,
            commands::delete_frame_asset,
            commands::create_character,
            commands::update_character,
            commands::update_character_scale,
            commands::switch_character,
            commands::delete_character,
            commands::chat_history_days,
            commands::chat_history_messages_for_day,
            commands::delete_chat_history_day,
            commands::open_data_dir,
            commands::get_ai_config_public,
            commands::save_ai_provider_config,
            commands::delete_ai_connection,
            commands::set_active_ai_provider,
            commands::test_ai_connection,
            commands::stream_ai_message,
            commands::abort_ai_request,
            set_ignore_cursor_events,
            get_mouse_pos,
            get_screen_info,
            set_window_to_screen_size
        ])
        .build(tauri::generate_context!())
        .expect("error while building BytePet")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                restore_main_window(app);
                let _ = app.emit("tray-show-toolbox", ());
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
