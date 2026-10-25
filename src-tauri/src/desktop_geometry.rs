use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct WorkArea {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScreenInfo {
    pub screen_width: f64,
    pub screen_height: f64,
    pub taskbar_height: f64,
    pub scale_factor: f64,
    pub work_area: WorkArea,
}

// On macOS keep the overlay itself out of the menu bar and Dock. Coordinates sent
// to the webview are relative to that overlay, not Cocoa's global screen origin.
pub fn overlay_layout(
    monitor: PhysicalRect,
    work: PhysicalRect,
    scale: f64,
    use_work_area: bool,
) -> Result<(PhysicalRect, ScreenInfo), String> {
    if !scale.is_finite() || scale <= 0.0 || monitor.width == 0 || monitor.height == 0 {
        return Err("显示器尺寸或缩放比例无效。".into());
    }
    let left = (work.x as i64).max(monitor.x as i64);
    let top = (work.y as i64).max(monitor.y as i64);
    let right = (work.x as i64 + work.width as i64).min(monitor.x as i64 + monitor.width as i64);
    let bottom = (work.y as i64 + work.height as i64).min(monitor.y as i64 + monitor.height as i64);
    let work = if right > left && bottom > top {
        PhysicalRect {
            x: left as i32,
            y: top as i32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        }
    } else {
        monitor
    };
    let overlay = if use_work_area { work } else { monitor };
    let area = WorkArea {
        left: (work.x as f64 - overlay.x as f64) / scale,
        top: (work.y as f64 - overlay.y as f64) / scale,
        right: (work.x as f64 - overlay.x as f64 + work.width as f64) / scale,
        bottom: (work.y as f64 - overlay.y as f64 + work.height as f64) / scale,
    };
    let screen_height = overlay.height as f64 / scale;
    Ok((
        overlay,
        ScreenInfo {
            screen_width: overlay.width as f64 / scale,
            screen_height,
            taskbar_height: (screen_height - area.bottom).max(0.0),
            scale_factor: scale,
            work_area: area,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    const MONITOR: PhysicalRect = PhysicalRect {
        x: 0,
        y: 0,
        width: 2880,
        height: 1800,
    };
    #[test]
    fn retina_menu_bar_and_bottom_dock_use_work_area_without_double_scaling() {
        let work = PhysicalRect {
            x: 0,
            y: 50,
            width: 2880,
            height: 1620,
        };
        let (overlay, info) = overlay_layout(MONITOR, work, 2.0, true).unwrap();
        assert_eq!(overlay, work);
        assert_eq!((info.screen_width, info.screen_height), (1440.0, 810.0));
        assert_eq!(
            info.work_area,
            WorkArea {
                left: 0.0,
                top: 0.0,
                right: 1440.0,
                bottom: 810.0
            }
        );
        assert_eq!(info.taskbar_height, 0.0);
    }
    #[test]
    fn side_dock_and_negative_monitor_origins_remain_window_relative() {
        let monitor = PhysicalRect {
            x: -2880,
            ..MONITOR
        };
        let work = PhysicalRect {
            x: -2750,
            y: 50,
            width: 2750,
            height: 1750,
        };
        let (overlay, info) = overlay_layout(monitor, work, 2.0, true).unwrap();
        assert_eq!(overlay.x, -2750);
        assert_eq!(info.work_area.left, 0.0);
        assert_eq!(info.screen_width, 1375.0);
    }
    #[test]
    fn windows_overlay_still_covers_monitor_and_reports_taskbar_insets() {
        let work = PhysicalRect {
            x: 0,
            y: 0,
            width: 2880,
            height: 1720,
        };
        let (overlay, info) = overlay_layout(MONITOR, work, 2.0, false).unwrap();
        assert_eq!(overlay, MONITOR);
        assert_eq!(info.taskbar_height, 40.0);
        assert_eq!(info.work_area.bottom, 860.0);
    }
    #[test]
    fn empty_work_area_falls_back_and_invalid_scale_is_rejected() {
        assert_eq!(
            overlay_layout(
                MONITOR,
                PhysicalRect {
                    width: 0,
                    ..MONITOR
                },
                1.0,
                true
            )
            .unwrap()
            .0,
            MONITOR
        );
        for scale in [0.0, -1.0, f64::NAN] {
            assert!(overlay_layout(MONITOR, MONITOR, scale, true).is_err());
        }
    }

    fn merge_config(base: &mut serde_json::Value, overlay: serde_json::Value) {
        match (base, overlay) {
            (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) => {
                for (key, value) in overlay {
                    merge_config(base.entry(key).or_insert(serde_json::Value::Null), value);
                }
            }
            (base, overlay) => *base = overlay,
        }
    }

    #[test]
    fn macos_configuration_deserializes_with_the_locked_tauri_version() {
        let mut value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        merge_config(
            &mut value,
            serde_json::from_str(include_str!("../tauri.macos.conf.json")).unwrap(),
        );
        let config: tauri::Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.app.windows.len(), 1);
        assert!(config.app.windows[0].transparent);
        assert_eq!(config.app.windows[0].label, "main");
    }

    #[test]
    fn windows_configuration_deserializes_with_the_locked_tauri_version() {
        let mut value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        merge_config(
            &mut value,
            serde_json::from_str(include_str!("../tauri.windows.conf.json")).unwrap(),
        );
        assert_eq!(
            value["bundle"]["targets"],
            serde_json::json!(["nsis", "msi"])
        );
        assert_eq!(
            value["bundle"]["windows"]["nsis"]["installerIcon"],
            "icons/icon.ico"
        );
        let config: tauri::Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.identifier, "com.bytepet.app");
        assert_eq!(config.app.windows.len(), 1);
        assert!(config.app.windows[0].transparent);
    }
}
