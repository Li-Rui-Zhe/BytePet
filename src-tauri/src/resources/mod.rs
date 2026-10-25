use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::database::{FrameAssetOption, SkinValidationIssue, SkinValidationReport};

pub const DATA_DIR_NAME: &str = "bytepet-data";
const OLD_APP_DIR_NAME: &str = "com.rick.ai-pet";
const OLD_DATA_DIR_NAME: &str = "rick-ai-pet-data";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPaths {
    pub root: String,
    pub database: String,
    pub settings: String,
    pub skins: String,
    pub uploads: String,
    pub exports: String,
    pub backups: String,
    pub logs: String,
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn data_root(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let root = base.join(DATA_DIR_NAME);

    if !root.exists() {
        if let Some(parent) = base.parent() {
            let old_root = parent.join(OLD_APP_DIR_NAME).join(OLD_DATA_DIR_NAME);
            if old_root.is_dir() {
                if let Some(root_parent) = root.parent() {
                    fs::create_dir_all(root_parent).map_err(|error| error.to_string())?;
                }
                copy_dir_recursive(&old_root, &root)?;
            }
        }
    }

    Ok(root)
}

pub fn data_paths(app: &AppHandle) -> Result<DataPaths, String> {
    let root = data_root(app)?;
    let database = safe_join(&root, "bytepet.db")?;
    let legacy_database = safe_join(&root, "rick_pet.db")?;
    if !database.exists() && legacy_database.exists() {
        fs::copy(&legacy_database, &database).map_err(|error| error.to_string())?;
    }
    let settings = safe_join(&root, "settings")?;
    let skins = safe_join(&root, "skins")?;
    let uploads = safe_join(&root, "uploads")?;
    let exports = safe_join(&root, "exports")?;
    let backups = safe_join(&root, "backups")?;
    let logs = safe_join(&root, "logs")?;

    Ok(DataPaths {
        root: root.to_string_lossy().to_string(),
        database: database.to_string_lossy().to_string(),
        settings: settings.to_string_lossy().to_string(),
        skins: skins.to_string_lossy().to_string(),
        uploads: uploads.to_string_lossy().to_string(),
        exports: exports.to_string_lossy().to_string(),
        backups: backups.to_string_lossy().to_string(),
        logs: logs.to_string_lossy().to_string(),
    })
}

pub fn ensure_data_dirs(app: &AppHandle) -> Result<DataPaths, String> {
    let paths = data_paths(app)?;
    for path in [
        &paths.root,
        &paths.settings,
        &paths.skins,
        &paths.uploads,
        &paths.exports,
        &paths.backups,
        &paths.logs,
    ] {
        fs::create_dir_all(path).map_err(|error| error.to_string())?;
    }
    Ok(paths)
}

pub fn safe_join(base: &Path, segment: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(segment);
    if candidate.is_absolute() {
        return Err("不允许使用绝对路径片段。".to_string());
    }

    if candidate
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("路径片段包含不安全的跳转。".to_string());
    }

    Ok(base.join(candidate))
}

pub fn open_data_dir(app: &AppHandle) -> Result<String, String> {
    let root = data_root(app)?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    #[cfg(target_os = "windows")]
    Command::new("explorer")
        .arg(&root)
        .spawn()
        .map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    Command::new("/usr/bin/open")
        .arg(&root)
        .spawn()
        .map_err(|error| error.to_string())?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open")
        .arg(&root)
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(root.to_string_lossy().to_string())
}

pub fn list_frame_assets(app: &AppHandle) -> Result<Vec<FrameAssetOption>, String> {
    let skins_root = user_skins_root(app)?;
    fs::create_dir_all(&skins_root).map_err(|error| error.to_string())?;
    migrate_public_imported_assets(app, &skins_root)?;

    let built_in_short_actions = built_in_default_skin_path()
        .and_then(|path| discover_short_action_keys(&path).ok())
        .filter(|keys| !keys.is_empty())
        .unwrap_or_else(default_short_action_keys);

    let mut items = vec![FrameAssetOption {
        id: "rick_default".to_string(),
        name: "系统默认 Rick".to_string(),
        path: None,
        built_in: true,
        imported_at: None,
        short_action_keys: built_in_short_actions,
    }];

    let mut imported_items = Vec::new();
    for entry in fs::read_dir(skins_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let path = entry.path();
        let folder_id = entry.file_name().to_string_lossy().to_string();
        if folder_id == "rick_default" || folder_id.starts_with('.') {
            continue;
        }
        let report = validate_skin_path(app, &path)?;
        if !report.valid {
            continue;
        }
        let metadata = read_frame_asset_metadata(&path);
        let short_action_keys = discover_short_action_keys(&path)?;
        imported_items.push(FrameAssetOption {
            id: folder_id.clone(),
            name: metadata
                .as_ref()
                .and_then(|value| value.get("name"))
                .and_then(|value| value.as_str())
                .unwrap_or(&folder_id)
                .to_string(),
            path: Some(path.to_string_lossy().to_string()),
            built_in: false,
            imported_at: metadata
                .as_ref()
                .and_then(|value| value.get("importedAt"))
                .and_then(|value| value.as_str())
                .map(str::to_string),
            short_action_keys,
        });
    }
    imported_items.sort_by(|a, b| {
        let a_time = a.imported_at.as_deref().unwrap_or("");
        let b_time = b.imported_at.as_deref().unwrap_or("");
        b_time.cmp(a_time).then_with(|| a.name.cmp(&b.name))
    });

    let mut seen_names = std::collections::HashSet::from(["rick_default".to_string()]);
    for item in imported_items {
        let key = item.id.trim().to_lowercase();
        if seen_names.insert(key) {
            items.push(item);
        }
    }
    Ok(items)
}

pub fn choose_and_import_frame_asset(app: &AppHandle) -> Result<Option<FrameAssetOption>, String> {
    let main_window = app.get_webview_window("main");
    if let Some(window) = main_window.as_ref() {
        let _ = window.set_always_on_top(false);
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.hide();
    }

    let selected = rfd::FileDialog::new()
        .set_title("选择 PNG 序列帧文件夹")
        .pick_folder();

    if let Some(window) = main_window.as_ref() {
        let _ = window.show();
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.set_always_on_top(true);
        let _ = window.set_focus();
        let _ = app.emit("cursor-state-reset", ());
    }

    let Some(source) = selected else {
        return Ok(None);
    };

    import_frame_asset(app, &source).map(Some)
}

pub fn import_frame_asset(app: &AppHandle, source: &Path) -> Result<FrameAssetOption, String> {
    let source = resolve_frame_asset_source(source)?;
    let report = validate_skin_path(app, &source)?;
    if !report.valid {
        let first = report
            .issues
            .first()
            .map(|issue| format!("{}: {}", issue.code, issue.message))
            .unwrap_or_else(|| "PNG 序列帧资源不可用。".to_string());
        return Err(first);
    }

    let skins_root = user_skins_root(app)?;
    fs::create_dir_all(&skins_root).map_err(|error| error.to_string())?;
    let id = source
        .file_name()
        .map(|value| sanitize_asset_id(&value.to_string_lossy()))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("frames-{}", uuid::Uuid::new_v4()));
    if id == "rick_default" {
        return Err("不能覆盖系统默认 Rick 资源，请把导入文件夹改成新的名字。".to_string());
    }
    let target = skins_root.join(&id);
    let action_keys = discover_short_action_keys(&source)?;
    if paths_equal(&source, &target) {
        let metadata = read_frame_asset_metadata(&target);
        let name = metadata
            .as_ref()
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str())
            .unwrap_or(&id)
            .to_string();
        return Ok(FrameAssetOption {
            id,
            name,
            path: Some(target.to_string_lossy().to_string()),
            built_in: false,
            imported_at: metadata
                .as_ref()
                .and_then(|value| value.get("importedAt"))
                .and_then(|value| value.as_str())
                .map(str::to_string),
            short_action_keys: action_keys,
        });
    }

    let name = source
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "已导入资源".to_string());
    let imported_at = chrono::Local::now().to_rfc3339();
    let metadata = serde_json::json!({
        "id": id,
        "name": name,
        "importedAt": imported_at,
        "source": source.to_string_lossy().to_string(),
        "shortActionKeys": action_keys
    });
    replace_frame_asset_dir(&source, &target, &action_keys, &metadata)?;

    Ok(FrameAssetOption {
        id,
        name,
        path: Some(target.to_string_lossy().to_string()),
        built_in: false,
        imported_at: Some(imported_at),
        short_action_keys: discover_short_action_keys(&target)?,
    })
}

pub fn delete_frame_asset(
    app: &AppHandle,
    asset_id: &str,
) -> Result<Vec<FrameAssetOption>, String> {
    let id = asset_id.trim();
    if id.is_empty() {
        return Err("请选择要删除的素材包。".to_string());
    }
    if id == "rick_default" {
        return Err("系统默认 Rick 素材包不能删除。".to_string());
    }
    if sanitize_asset_id(id) != id {
        return Err("素材包名称不合法，无法删除。".to_string());
    }

    let current_assets = list_frame_assets(app)?;
    if current_assets.len() <= 1 {
        return Err("至少需要保留一个素材包。".to_string());
    }
    let asset = current_assets
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "素材包不存在。".to_string())?;
    if asset.built_in {
        return Err("系统素材包不能删除。".to_string());
    }

    let target = asset.path.as_ref().map(PathBuf::from).unwrap_or_else(|| {
        user_skins_root(app)
            .map(|root| root.join(id))
            .unwrap_or_default()
    });
    if !target.is_dir() {
        return Err("素材包目录不存在。".to_string());
    }
    fs::remove_dir_all(&target).map_err(|error| error.to_string())?;
    list_frame_assets(app)
}

fn read_frame_asset_metadata(path: &Path) -> Option<serde_json::Value> {
    let text = fs::read_to_string(path.join("asset.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_png_header(path: &Path) -> Result<(u32, u32, bool), String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = [0u8; 33];
    file.read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    let signature = [137, 80, 78, 71, 13, 10, 26, 10];

    if bytes[0..8] != signature {
        return Err("不是有效 PNG 文件。".to_string());
    }

    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let color_type = bytes[25];
    let has_alpha = color_type == 4 || color_type == 6;
    Ok((width, height, has_alpha))
}

fn sanitize_asset_id(name: &str) -> String {
    let mut value = name
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    while value.contains("__") {
        value = value.replace("__", "_");
    }
    value.trim_matches('_').to_string()
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn looks_like_frame_asset_root(path: &Path) -> bool {
    path.join("calm").is_dir()
        && path.join("thinking").is_dir()
        && path.join("run_left").is_dir()
        && path.join("run_right").is_dir()
}

fn resolve_frame_asset_source(source: &Path) -> Result<PathBuf, String> {
    if looks_like_frame_asset_root(source) {
        return Ok(source.to_path_buf());
    }

    let preferred = source.join("rick_default");
    if looks_like_frame_asset_root(&preferred) {
        return Ok(preferred);
    }

    let mut candidates = Vec::new();
    if source.is_dir() {
        for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            if entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_dir()
                && looks_like_frame_asset_root(&entry.path())
            {
                candidates.push(entry.path());
            }
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Ok(source.to_path_buf()),
        _ => Err("选择的文件夹里包含多个角色资源，请直接选择其中一个角色文件夹。".to_string()),
    }
}

fn copy_frame_sequence(
    source_root: &Path,
    target_root: &Path,
    state_key: &str,
    frames: usize,
) -> Result<(), String> {
    let source_dir = source_root.join(state_key);
    let target_dir = target_root.join(state_key);
    fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    for index in 1..=frames {
        let file_name = format!("{state_key}_{index:04}.png");
        fs::copy(source_dir.join(&file_name), target_dir.join(&file_name))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

// Build a complete sibling directory before touching an existing import.
fn replace_directory_atomically(
    target: &Path,
    prepare: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let parent = target.parent().ok_or("素材目录没有父目录。")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let token = uuid::Uuid::new_v4();
    let staging = parent.join(format!(".import-{token}"));
    let backup = parent.join(format!(".backup-{token}"));
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    if let Err(error) = prepare(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let had_target = target.exists();
    if had_target {
        if let Err(error) = fs::rename(target, &backup) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error.to_string());
        }
    }
    if let Err(error) = fs::rename(&staging, target) {
        let restored = !had_target || fs::rename(&backup, target).is_ok();
        let _ = fs::remove_dir_all(&staging);
        return Err(if restored {
            error.to_string()
        } else {
            format!("导入失败，旧素材保留在 {}：{error}", backup.display())
        });
    }
    if had_target {
        let _ = fs::remove_dir_all(&backup);
    }
    Ok(())
}

fn replace_frame_asset_dir(
    source: &Path,
    target: &Path,
    action_keys: &[String],
    metadata: &serde_json::Value,
) -> Result<(), String> {
    if let (Ok(source), Ok(target)) = (source.canonicalize(), target.canonicalize()) {
        if source.starts_with(&target) {
            return Err("不能从待覆盖素材目录内部导入，请先复制到其他文件夹。".to_string());
        }
    }
    replace_directory_atomically(target, |staging| {
        for (state_key, frames) in expected_frame_states() {
            copy_frame_sequence(source, staging, state_key, *frames)?;
        }
        for action_key in action_keys {
            copy_frame_sequence(source, staging, action_key, 6)?;
        }
        copy_frame_sequence(source, staging, "box", 1)?;
        fs::write(
            staging.join("asset.json"),
            serde_json::to_vec_pretty(metadata).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn default_short_action_keys() -> Vec<String> {
    vec!["action1".to_string(), "action2".to_string()]
}

pub fn user_skins_root(app: &AppHandle) -> Result<PathBuf, String> {
    let paths = data_paths(app)?;
    Ok(PathBuf::from(paths.skins))
}

fn migrate_public_imported_assets(app: &AppHandle, target_root: &Path) -> Result<(), String> {
    let Ok(public_root) = public_skins_root() else {
        return Ok(());
    };
    if paths_equal(&public_root, target_root) || !public_root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(public_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let folder_id = entry.file_name().to_string_lossy().to_string();
        if folder_id == "rick_default" {
            continue;
        }
        let source = entry.path();
        let target = target_root.join(&folder_id);
        if target.exists() || !validate_skin_path(app, &source)?.valid {
            continue;
        }
        copy_dir_recursive(&source, &target)?;
    }

    Ok(())
}

pub fn public_skins_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .ok_or_else(|| "无法定位项目根目录。".to_string())?;

    let mut candidates = vec![project_root.join("public/assets/skins")];
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("public/assets/skins"));
        if let Some(parent) = current_dir.parent() {
            candidates.push(parent.join("public/assets/skins"));
        }
    }

    if let Some(existing) = candidates.into_iter().find(|path| path.is_dir()) {
        return Ok(existing);
    }

    Ok(project_root.join("public/assets/skins"))
}

fn built_in_default_skin_path() -> Option<PathBuf> {
    public_skins_root()
        .ok()
        .map(|path| path.join("rick_default"))
}

fn action_key_number(name: &str) -> Option<u32> {
    let suffix = name.strip_prefix("action")?;
    if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<u32>().ok()
}

fn discover_short_action_keys(path: &Path) -> Result<Vec<String>, String> {
    let mut keys = Vec::new();
    if !path.is_dir() {
        return Ok(keys);
    }

    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if action_key_number(&name).is_some() {
            keys.push(name);
        }
    }

    keys.sort_by_key(|key| action_key_number(key).unwrap_or(u32::MAX));
    Ok(keys)
}

fn parse_frame_index(file_name: &str, state_key: &str) -> Option<usize> {
    let prefix = format!("{state_key}_");
    let suffix = file_name.strip_prefix(&prefix)?.strip_suffix(".png")?;
    if suffix.len() != 4 || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<usize>().ok()
}

fn expected_frame_states() -> &'static [(&'static str, usize)] {
    &[
        ("calm", 6),
        ("sleeping", 6),
        ("wake_up", 6),
        ("yawn", 6),
        ("sit", 6),
        ("sit_down", 4),
        ("happy", 6),
        ("cheer_up", 6),
        ("sad", 6),
        ("angry", 6),
        ("comfort", 6),
        ("thinking", 6),
        ("run_left", 4),
        ("run_right", 4),
        ("fly_up", 6),
        ("fall_down", 4),
        ("dizzy", 2),
        ("error", 6),
    ]
}

fn validate_frame_sequence(
    path: &Path,
    state_key: &str,
    expected_frames: usize,
    issues: &mut Vec<SkinValidationIssue>,
    expected_size: &mut Option<(u32, u32)>,
) {
    let dir = path.join(state_key);
    if !dir.is_dir() {
        issues.push(SkinValidationIssue::new(
            "P0",
            "ASSET_DIR_MISSING",
            &format!("缺少必要状态目录：{state_key}。"),
            Some(state_key.to_string()),
        ));
        return;
    }

    let mut found = vec![false; expected_frames + 1];
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(SkinValidationIssue::new(
                "P0",
                "ASSET_DIR_UNREADABLE",
                &error.to_string(),
                Some(state_key.to_string()),
            ));
            return;
        }
    };

    for entry in entries.flatten() {
        if !entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some(index) = parse_frame_index(&file_name, state_key) else {
            continue;
        };
        if index == 0 || index > expected_frames {
            issues.push(SkinValidationIssue::new(
                "P0",
                "ASSET_FRAME_LIMIT",
                &format!("{state_key} 最多只能放 {expected_frames} 张配置帧，请删除多余帧。"),
                Some(state_key.to_string()),
            ));
            continue;
        }
        found[index] = true;

        match read_png_header(&entry.path()) {
            Ok((width, height, has_alpha)) => {
                if !has_alpha {
                    issues.push(SkinValidationIssue::new(
                        "P0",
                        "ASSET_ALPHA_REQUIRED",
                        "PNG 必须包含透明 alpha 通道。",
                        Some(state_key.to_string()),
                    ));
                }
                if let Some((expected_width, expected_height)) = *expected_size {
                    if expected_width != width || expected_height != height {
                        issues.push(SkinValidationIssue::new(
                            "P0",
                            "ASSET_SIZE_MISMATCH",
                            "所有动作帧尺寸必须一致。",
                            Some(state_key.to_string()),
                        ));
                    }
                } else {
                    *expected_size = Some((width, height));
                }
            }
            Err(message) => issues.push(SkinValidationIssue::new(
                "P0",
                "ASSET_PNG_INVALID",
                &message,
                Some(state_key.to_string()),
            )),
        }
    }

    for (index, exists) in found.into_iter().enumerate().skip(1) {
        if !exists {
            issues.push(SkinValidationIssue::new(
                "P0",
                "ASSET_FRAME_SEQUENCE",
                &format!("缺少帧：{state_key}_{index:04}.png。"),
                Some(state_key.to_string()),
            ));
        }
    }
}

pub fn validate_skin_path(_app: &AppHandle, path: &Path) -> Result<SkinValidationReport, String> {
    let mut issues = Vec::new();

    if !path.exists() || !path.is_dir() {
        return Ok(SkinValidationReport {
            valid: false,
            issues: vec![SkinValidationIssue::new(
                "P0",
                "ASSET_DIR_MISSING",
                "序列帧目录不存在，或不是文件夹。",
                None,
            )],
            frame_width: None,
            frame_height: None,
        });
    }

    let mut expected_size: Option<(u32, u32)> = None;

    for (state_key, frames) in expected_frame_states() {
        validate_frame_sequence(path, state_key, *frames, &mut issues, &mut expected_size);
    }

    let action_keys = discover_short_action_keys(path)?;
    if action_keys.is_empty() {
        issues.push(SkinValidationIssue::new(
            "P0",
            "ASSET_ACTION_MISSING",
            "至少需要一个短动作目录，例如 action1、action2 或 action3。",
            None,
        ));
    }
    for action_key in action_keys {
        validate_frame_sequence(path, &action_key, 6, &mut issues, &mut expected_size);
    }
    validate_frame_sequence(path, "box", 1, &mut issues, &mut expected_size);

    Ok(SkinValidationReport {
        valid: !issues.iter().any(|issue| issue.severity == "P0"),
        issues,
        frame_width: expected_size.map(|size| size.0),
        frame_height: expected_size.map(|size| size.1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_import_keeps_previous_files_and_metadata() {
        let root = std::env::temp_dir().join(format!("bytepet-test-{}", uuid::Uuid::new_v4()));
        let target = root.join("pet");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("old.png"), b"old-image").unwrap();
        fs::write(target.join("asset.json"), b"old-metadata").unwrap();
        let result = replace_directory_atomically(&target, |staging| {
            fs::write(staging.join("partial.png"), b"partial").unwrap();
            Err("simulated copy failure".into())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(target.join("old.png")).unwrap(), b"old-image");
        assert_eq!(
            fs::read(target.join("asset.json")).unwrap(),
            b"old-metadata"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_import_publishes_complete_directory() {
        let root = std::env::temp_dir().join(format!("bytepet-test-{}", uuid::Uuid::new_v4()));
        let target = root.join("pet");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("old.png"), b"old").unwrap();
        replace_directory_atomically(&target, |staging| {
            fs::write(staging.join("new.png"), b"new").map_err(|e| e.to_string())?;
            fs::write(staging.join("asset.json"), b"metadata").map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
        assert!(!target.join("old.png").exists());
        assert_eq!(fs::read(target.join("new.png")).unwrap(), b"new");
        assert!(target.join("asset.json").exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
