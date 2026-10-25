use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use uuid::Uuid;

use crate::resources::{self, DataPaths};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterProfile {
    pub id: String,
    pub name: String,
    pub skin_id: String,
    pub prompt: String,
    pub frame_assets_path: Option<String>,
    pub memory_summary: String,
    pub description: String,
    pub personality_tags: Vec<String>,
    pub opening_line: String,
    pub display_scale: f64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub character_id: String,
    pub title: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub character_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub token_count: i64,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryDay {
    pub date_key: String,
    pub message_count: i64,
    pub session_count: i64,
    pub preview: String,
    pub last_message_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub data_paths: DataPaths,
    pub character: CharacterProfile,
    pub characters: Vec<CharacterProfile>,
    pub sessions: Vec<ChatSession>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCharacterRequest {
    pub name: String,
    pub prompt: String,
    pub frame_source_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCharacterRequest {
    pub character_id: String,
    pub name: String,
    pub prompt: String,
    pub frame_source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinValidationIssue {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub state_key: Option<String>,
}

impl SkinValidationIssue {
    pub fn new(severity: &str, code: &str, message: &str, state_key: Option<String>) -> Self {
        Self {
            severity: severity.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            state_key,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinValidationReport {
    pub valid: bool,
    pub issues: Vec<SkinValidationIssue>,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameAssetOption {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub built_in: bool,
    pub imported_at: Option<String>,
    pub short_action_keys: Vec<String>,
}

fn now_string() -> String {
    Local::now().to_rfc3339()
}

fn db_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(resources::data_root(app)?.join("bytepet.db"))
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
    resources::ensure_data_dirs(app)?;
    let connection = Connection::open(db_path(app)?).map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "secure_delete", "ON")
        .and_then(|_| connection.pragma_update(None, "foreign_keys", "ON"))
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

pub fn bootstrap(app: &AppHandle) -> Result<BootstrapPayload, String> {
    resources::ensure_data_dirs(app)?;
    let connection = open_connection(app)?;
    migrate(&connection)?;
    seed_defaults(app, &connection)?;
    repair_character_integrity(app, &connection)?;
    migrate_ai_connections(&connection)?;
    migrate_api_credentials(&connection)?;
    fetch_bootstrap(app, &connection)
}

fn migrate(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS characters (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              skin_id TEXT NOT NULL,
              prompt TEXT NOT NULL DEFAULT '',
              frame_assets_path TEXT,
              memory_summary TEXT NOT NULL DEFAULT '',
              description TEXT NOT NULL,
              personality_tags TEXT NOT NULL,
              opening_line TEXT NOT NULL,
              display_scale REAL NOT NULL DEFAULT 1.0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              is_active INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS chat_sessions (
              id TEXT PRIMARY KEY,
              character_id TEXT NOT NULL,
              title TEXT NOT NULL,
              summary TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS chat_messages (
              id TEXT PRIMARY KEY,
              session_id TEXT NOT NULL,
              character_id TEXT NOT NULL,
              role TEXT NOT NULL,
              content TEXT NOT NULL,
              created_at TEXT NOT NULL,
              token_count INTEGER NOT NULL DEFAULT 0,
              metadata_json TEXT,
              FOREIGN KEY(session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS skins (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              character_id TEXT NOT NULL,
              root_path TEXT NOT NULL,
              manifest_path TEXT NOT NULL,
              frame_width INTEGER NOT NULL,
              frame_height INTEGER NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ai_provider_configs (
              provider_id TEXT PRIMARY KEY,
              provider_type TEXT NOT NULL,
              display_name TEXT NOT NULL,
              base_url TEXT NOT NULL,
              model TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              stream INTEGER NOT NULL DEFAULT 1,
              temperature REAL DEFAULT 0.8,
              max_output_tokens INTEGER DEFAULT 1200,
              timeout_ms INTEGER DEFAULT 60000,
              updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ai_provider_secrets (
              provider_id TEXT PRIMARY KEY,
              api_key TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_chat_sessions_character ON chat_sessions(character_id);
            CREATE INDEX IF NOT EXISTS idx_chat_messages_character ON chat_messages(character_id);
            CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (1, datetime('now'));
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (2, datetime('now'));
            "#,
        )
        .map_err(|error| error.to_string())?;
    ensure_legacy_columns(connection)?;
    remove_legacy_settings_api_key(connection)?;
    Ok(())
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(|error| error.to_string())?;
    Ok(columns)
}

fn remove_legacy_settings_api_key(connection: &Connection) -> Result<(), String> {
    connection
        .execute("DELETE FROM settings WHERE key='apiKey'", [])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    ddl: &str,
) -> Result<(), String> {
    let columns = table_columns(connection, table)?;
    if !columns.iter().any(|name| name == column) {
        connection
            .execute_batch(ddl)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn ensure_legacy_columns(connection: &Connection) -> Result<(), String> {
    ensure_column(
        connection,
        "characters",
        "prompt",
        "ALTER TABLE characters ADD COLUMN prompt TEXT NOT NULL DEFAULT '';",
    )?;
    ensure_column(
        connection,
        "characters",
        "frame_assets_path",
        "ALTER TABLE characters ADD COLUMN frame_assets_path TEXT;",
    )?;
    ensure_column(
        connection,
        "characters",
        "display_scale",
        "ALTER TABLE characters ADD COLUMN display_scale REAL NOT NULL DEFAULT 1.0;",
    )?;
    ensure_column(
        connection,
        "characters",
        "memory_summary",
        "ALTER TABLE characters ADD COLUMN memory_summary TEXT NOT NULL DEFAULT '';",
    )?;
    connection
        .execute_batch("UPDATE characters SET prompt=description WHERE prompt='';")
        .map_err(|error| error.to_string())?;
    ensure_column(
        connection,
        "chat_sessions",
        "summary",
        "ALTER TABLE chat_sessions ADD COLUMN summary TEXT NOT NULL DEFAULT '';",
    )?;

    // Removed care-system tables in older databases are retained without being read or updated.
    Ok(())
}

fn seed_defaults(app: &AppHandle, connection: &Connection) -> Result<(), String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM characters", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;

    if count > 0 {
        return Ok(());
    }

    let now = now_string();
    let character_id = "rick-default-character";
    let skin_id = "rick_default";
    let tags = json!(["gentle", "curious", "reliable"]).to_string();

    connection
        .execute(
            "INSERT INTO characters(id,name,skin_id,prompt,frame_assets_path,memory_summary,description,personality_tags,opening_line,display_scale,created_at,updated_at,is_active)
             VALUES (?1,'Rick',?2,?3,NULL,'',?3,?4,?5,1.0,?6,?6,1)",
            params![
                character_id,
                skin_id,
                "A quiet blue-white desktop pet that keeps the user company.",
                tags,
                "Hi, I am Rick. Let us take today one step at a time.",
                now
            ],
        )
        .map_err(|error| error.to_string())?;

    let paths = resources::data_paths(app)?;
    connection
        .execute(
            "INSERT INTO skins(id,name,character_id,root_path,manifest_path,frame_width,frame_height,enabled,created_at,updated_at)
             VALUES (?1,'Default Rick',?2,?3,?4,500,500,1,?5,?5)",
            params![
                skin_id,
                character_id,
                format!("{}/rick_default", paths.skins),
                format!("{}/rick_default/manifest.json", paths.skins),
                now
            ],
        )
        .map_err(|error| error.to_string())?;

    seed_default_session(connection, character_id, &now)?;
    Ok(())
}

fn seed_default_session(
    connection: &Connection,
    character_id: &str,
    now: &str,
) -> Result<(), String> {
    let opening_line = connection
        .query_row(
            "SELECT opening_line FROM characters WHERE id=?1",
            params![character_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .filter(|line| !line.trim().is_empty())
        .unwrap_or_else(|| "你好，我在这里。".to_string());
    let session_id = if character_id == "rick-default-character" {
        "default-session".to_string()
    } else {
        format!("default-session-{character_id}")
    };
    let opening_message_id = if character_id == "rick-default-character" {
        "default-opening-message".to_string()
    } else {
        format!("default-opening-message-{character_id}")
    };
    connection
        .execute(
            "INSERT OR IGNORE INTO chat_sessions(id,character_id,title,summary,created_at,updated_at) VALUES (?1,?2,'New chat','',?3,?3)",
            params![&session_id, character_id, now],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT OR IGNORE INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
             VALUES (?1,?2,?3,'assistant',?4,?5,0,NULL)",
            params![
                &opening_message_id,
                &session_id,
                character_id,
                opening_line,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn repair_character_integrity(app: &AppHandle, connection: &Connection) -> Result<(), String> {
    let now = now_string();
    let profiles = characters(connection)?;
    if profiles.is_empty() {
        return Ok(());
    }

    let active_count = profiles.iter().filter(|profile| profile.is_active).count();
    if active_count == 0 {
        if let Some(profile) = profiles.first() {
            connection
                .execute(
                    "UPDATE characters SET is_active=1 WHERE id=?1",
                    params![profile.id],
                )
                .map_err(|error| error.to_string())?;
        }
    } else if active_count > 1 {
        let keep_id = profiles
            .iter()
            .find(|profile| profile.is_active)
            .map(|profile| profile.id.clone())
            .unwrap_or_else(|| profiles[0].id.clone());
        connection
            .execute(
                "UPDATE characters SET is_active=CASE WHEN id=?1 THEN 1 ELSE 0 END",
                params![keep_id],
            )
            .map_err(|error| error.to_string())?;
    }

    for profile in characters(connection)? {
        let missing_frame_asset_path = profile
            .frame_assets_path
            .as_ref()
            .map(|path| !std::path::Path::new(path).is_dir())
            .unwrap_or(true);
        if profile.skin_id != "rick_default" && missing_frame_asset_path {
            if let Some(asset) = resources::list_frame_assets(app)?
                .into_iter()
                .find(|asset| asset.id == profile.skin_id && asset.path.is_some())
            {
                connection
                    .execute(
                        "UPDATE characters SET frame_assets_path=?1,updated_at=?2 WHERE id=?3",
                        params![asset.path, now, profile.id],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
    }

    Ok(())
}

fn active_character(connection: &Connection) -> Result<CharacterProfile, String> {
    if let Some(character) = connection
        .query_row(
            "SELECT id,name,skin_id,prompt,frame_assets_path,memory_summary,description,personality_tags,opening_line,display_scale,created_at,updated_at,is_active
             FROM characters WHERE is_active=1 LIMIT 1",
            [],
            read_character,
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(character);
    }

    let character = connection
        .query_row(
            "SELECT id,name,skin_id,prompt,frame_assets_path,memory_summary,description,personality_tags,opening_line,display_scale,created_at,updated_at,is_active
             FROM characters ORDER BY updated_at DESC LIMIT 1",
            [],
            read_character,
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE characters SET is_active=CASE WHEN id=?1 THEN 1 ELSE 0 END",
            params![character.id],
        )
        .map_err(|error| error.to_string())?;
    Ok(CharacterProfile {
        is_active: true,
        ..character
    })
}

fn read_character(row: &rusqlite::Row<'_>) -> rusqlite::Result<CharacterProfile> {
    let id: String = row.get(0)?;
    let tags_json: String = row.get(7)?;
    let personality_tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
    Ok(CharacterProfile {
        id,
        name: row.get(1)?,
        skin_id: row.get(2)?,
        prompt: row.get(3)?,
        frame_assets_path: row.get(4)?,
        memory_summary: row.get(5)?,
        description: row.get(6)?,
        personality_tags,
        opening_line: row.get(8)?,
        display_scale: row.get::<_, f64>(9)?.clamp(0.75, 3.0),
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        is_active: row.get::<_, i64>(12)? == 1,
    })
}

fn characters(connection: &Connection) -> Result<Vec<CharacterProfile>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,name,skin_id,prompt,frame_assets_path,memory_summary,description,personality_tags,opening_line,display_scale,created_at,updated_at,is_active
             FROM characters ORDER BY is_active DESC, updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let profiles = statement
        .query_map([], read_character)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<CharacterProfile>>>()
        .map_err(|error| error.to_string())?;

    Ok(profiles)
}

fn sessions(connection: &Connection, character_id: &str) -> Result<Vec<ChatSession>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,character_id,title,summary,created_at,updated_at FROM chat_sessions
             WHERE character_id=?1 ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let items = statement
        .query_map(params![character_id], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                character_id: row.get(1)?,
                title: row.get(2)?,
                summary: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<ChatSession>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

fn messages(connection: &Connection, character_id: &str) -> Result<Vec<ChatMessage>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,session_id,character_id,role,content,created_at,token_count,metadata_json
             FROM chat_messages WHERE character_id=?1 ORDER BY created_at DESC LIMIT 300",
        )
        .map_err(|error| error.to_string())?;
    let mut items = statement
        .query_map(params![character_id], read_message)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<ChatMessage>>>()
        .map_err(|error| error.to_string())?;
    items.reverse();
    Ok(items)
}

fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatMessage> {
    Ok(ChatMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        character_id: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
        token_count: row.get(6)?,
        metadata_json: row.get(7)?,
    })
}

pub fn chat_history_days(app: &AppHandle) -> Result<Vec<ChatHistoryDay>, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let mut statement = connection
        .prepare(
            "SELECT substr(created_at,1,10) AS date_key,
                    COUNT(*) AS message_count,
                    COUNT(DISTINCT session_id) AS session_count,
                    MAX(created_at) AS last_message_at,
                    COALESCE(
                      (
                        SELECT m2.content
                        FROM chat_messages m2
                        WHERE m2.character_id=?1 AND substr(m2.created_at,1,10)=substr(chat_messages.created_at,1,10)
                        ORDER BY m2.created_at DESC, m2.id DESC
                        LIMIT 1
                      ),
                      ''
                    ) AS preview
             FROM chat_messages
             WHERE character_id=?1
             GROUP BY date_key
             ORDER BY date_key DESC",
        )
        .map_err(|error| error.to_string())?;
    let items = statement
        .query_map(params![character.id], |row| {
            Ok(ChatHistoryDay {
                date_key: row.get(0)?,
                message_count: row.get(1)?,
                session_count: row.get(2)?,
                last_message_at: row.get(3)?,
                preview: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<ChatHistoryDay>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

pub fn chat_history_messages_for_day(
    app: &AppHandle,
    date_key: &str,
) -> Result<Vec<ChatMessage>, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id,session_id,character_id,role,content,created_at,token_count,metadata_json
             FROM chat_messages
             WHERE character_id=?1 AND substr(created_at,1,10)=?2
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|error| error.to_string())?;
    let items = statement
        .query_map(params![character.id, date_key], read_message)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<ChatMessage>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

pub fn delete_chat_history_day(
    app: &AppHandle,
    date_key: &str,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    connection
        .execute(
            "DELETE FROM chat_messages WHERE character_id=?1 AND substr(created_at,1,10)=?2",
            params![character.id, date_key],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM chat_sessions
             WHERE character_id=?1
               AND id NOT IN (SELECT DISTINCT session_id FROM chat_messages WHERE character_id=?1)",
            params![character.id],
        )
        .map_err(|error| error.to_string())?;
    fetch_bootstrap(app, &connection)
}

fn fetch_bootstrap(app: &AppHandle, connection: &Connection) -> Result<BootstrapPayload, String> {
    let character = active_character(connection)?;
    Ok(BootstrapPayload {
        data_paths: resources::data_paths(app)?,
        sessions: sessions(connection, &character.id)?,
        messages: messages(connection, &character.id)?,
        characters: characters(connection)?,
        character,
    })
}

fn validate_character_fields(name: &str, prompt: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("请输入人物名字。".to_string());
    }
    if name.trim().chars().count() > 30 {
        return Err("人物名字最多 30 个字符。".to_string());
    }
    if prompt.trim().is_empty() {
        return Err("请输入人物 prompt。".to_string());
    }
    if prompt.trim().chars().count() > 1000 {
        return Err("人物 prompt 最多 1000 个字符。".to_string());
    }
    Ok(())
}

fn resolve_frame_asset_selection(
    app: &AppHandle,
    frame_source_path: Option<&str>,
) -> Result<(String, Option<String>), String> {
    let Some(path_text) = frame_source_path
        .map(str::trim)
        .filter(|text| !text.is_empty())
    else {
        return Ok(("rick_default".to_string(), None));
    };

    if let Some(asset) = resources::list_frame_assets(app)?
        .into_iter()
        .find(|asset| asset.id == path_text)
    {
        return Ok((asset.id, asset.path));
    }

    let source = std::path::PathBuf::from(path_text);
    if source.exists() {
        let asset = resources::import_frame_asset(app, &source)?;
        return Ok((asset.id, asset.path));
    }

    Err("序列帧资源不存在，请先重新导入。".to_string())
}

pub fn create_character(
    app: &AppHandle,
    request: CreateCharacterRequest,
) -> Result<BootstrapPayload, String> {
    let mut connection = open_connection(app)?;
    let name = request.name.trim();
    let prompt = request.prompt.trim();
    validate_character_fields(name, prompt)?;

    let now = now_string();
    let character_id = Uuid::new_v4().to_string();
    let (skin_id, frame_assets_path) =
        resolve_frame_asset_selection(app, request.frame_source_path.as_deref())?;
    let opening_line = format!("你好，我是{name}。");

    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute("UPDATE characters SET is_active=0", [])
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO characters(id,name,skin_id,prompt,frame_assets_path,memory_summary,description,personality_tags,opening_line,display_scale,created_at,updated_at,is_active)
             VALUES (?1,?2,?3,?4,?5,'',?4,'[]',?6,1.0,?7,?7,1)",
            params![character_id, name, skin_id, prompt, frame_assets_path, opening_line, now],
        )
        .map_err(|error| error.to_string())?;
    seed_default_session(&transaction, &character_id, &now)?;
    transaction.commit().map_err(|error| error.to_string())?;
    fetch_bootstrap(app, &connection)
}

pub fn update_character(
    app: &AppHandle,
    request: UpdateCharacterRequest,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let name = request.name.trim();
    let prompt = request.prompt.trim();
    validate_character_fields(name, prompt)?;

    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM characters WHERE id=?1",
            params![request.character_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if exists.is_none() {
        return Err("人物不存在。".to_string());
    }

    let now = now_string();
    if request
        .frame_source_path
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .is_some()
    {
        let (skin_id, frame_assets_path) =
            resolve_frame_asset_selection(app, request.frame_source_path.as_deref())?;
        connection
            .execute(
                "UPDATE characters
                 SET name=?1,prompt=?2,description=?2,frame_assets_path=?3,skin_id=?4,updated_at=?5
                 WHERE id=?6",
                params![
                    name,
                    prompt,
                    frame_assets_path,
                    skin_id,
                    now,
                    request.character_id
                ],
            )
            .map_err(|error| error.to_string())?;
    } else {
        connection
            .execute(
                "UPDATE characters SET name=?1,prompt=?2,description=?2,updated_at=?3 WHERE id=?4",
                params![name, prompt, now, request.character_id],
            )
            .map_err(|error| error.to_string())?;
    }
    fetch_bootstrap(app, &connection)
}

pub fn update_character_scale(
    app: &AppHandle,
    character_id: &str,
    display_scale: f64,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let scale = display_scale.clamp(0.75, 3.0);
    let changed = connection
        .execute(
            "UPDATE characters SET display_scale=?1,updated_at=?2 WHERE id=?3",
            params![scale, now_string(), character_id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("人物不存在。".to_string());
    }
    fetch_bootstrap(app, &connection)
}

pub fn switch_character(app: &AppHandle, character_id: &str) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM characters WHERE id=?1",
            params![character_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if exists.is_none() {
        return Err("人物不存在。".to_string());
    }
    let now = now_string();
    connection
        .execute("UPDATE characters SET is_active=0", [])
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE characters SET is_active=1,updated_at=?1 WHERE id=?2",
            params![now, character_id],
        )
        .map_err(|error| error.to_string())?;
    fetch_bootstrap(app, &connection)
}

pub fn delete_character(app: &AppHandle, character_id: &str) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM characters", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if count <= 1 {
        return Err("至少需要保留一个人物。".to_string());
    }
    let was_active: i64 = connection
        .query_row(
            "SELECT is_active FROM characters WHERE id=?1",
            params![character_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "人物不存在。".to_string())?;
    connection
        .execute("DELETE FROM characters WHERE id=?1", params![character_id])
        .map_err(|error| error.to_string())?;
    if was_active == 1 {
        let next_id: String = connection
            .query_row(
                "SELECT id FROM characters ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE characters SET is_active=1 WHERE id=?1",
                params![next_id],
            )
            .map_err(|error| error.to_string())?;
    }
    fetch_bootstrap(app, &connection)
}

fn ensure_session_owner(
    connection: &Connection,
    session_id: &str,
    character_id: &str,
) -> Result<(), String> {
    let matches: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM chat_sessions WHERE id=?1 AND character_id=?2)",
            params![session_id, character_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if matches {
        Ok(())
    } else {
        Err("聊天会话不存在或不属于该人物。".to_string())
    }
}
fn get_or_create_session(
    connection: &Connection,
    character_id: &str,
    session_id: Option<String>,
) -> Result<ChatSession, String> {
    if let Some(id) = session_id.as_deref() {
        if let Some(session) = connection
            .query_row(
                "SELECT id,character_id,title,summary,created_at,updated_at FROM chat_sessions WHERE id=?1 AND character_id=?2",
                params![id, character_id],
                |row| {
                    Ok(ChatSession {
                        id: row.get(0)?,
                        character_id: row.get(1)?,
                        title: row.get(2)?,
                        summary: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())?
        {
            return Ok(session);
        }
    }

    let now = now_string();
    let id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    connection
        .execute(
            "INSERT INTO chat_sessions(id,character_id,title,summary,created_at,updated_at) VALUES (?1,?2,'New chat','',?3,?3)",
            params![id, character_id, now],
        )
        .map_err(|error| error.to_string())?;
    sessions(connection, character_id)?
        .into_iter()
        .find(|session| session.id == id)
        .ok_or_else(|| "Failed to create chat session.".to_string())
}

// ==================== AI Provider CRUD ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConfig {
    pub provider_id: String,
    pub provider_type: String,
    pub display_name: String,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub stream: bool,
    pub temperature: f64,
    pub max_output_tokens: i64,
    pub timeout_ms: i64,
    pub has_api_key: bool,
    pub api_key_hint: Option<String>,
    pub is_active: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderRequest {
    pub provider_id: String,
    pub provider_type: String,
    pub display_name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub stream: Option<bool>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<i64>,
    pub timeout_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AiProviderFullConfig {
    pub config: AiProviderConfig,
    pub api_key: String,
}

fn now_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// One-time migration: keep old rows for rollback, and copy only the active
// compatible connection (including its secret) into the single custom slot.
fn migrate_ai_connections(connection: &Connection) -> Result<(), String> {
    let migrated: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=3)",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if migrated {
        return Ok(());
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let active_json: Option<String> = transaction
        .query_row(
            "SELECT value_json FROM settings WHERE key='activeAiProvider'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let active_id = active_json.and_then(|value| serde_json::from_str::<String>(&value).ok());
    let previous: Option<(String, String)> = transaction
        .query_row(
            "SELECT provider_type,base_url FROM ai_provider_configs WHERE provider_id=?1",
            params![active_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let use_custom = previous.as_ref().is_some_and(|(protocol, url)| {
        protocol == crate::ai::AI_PROTOCOL
            && (active_id.as_deref() != Some("deepseek")
                || url.trim_end_matches('/') != crate::ai::DEEPSEEK_BASE_URL)
    });
    if use_custom && active_id.as_deref() != Some("custom") {
        let copied = transaction.execute(
            "INSERT OR IGNORE INTO ai_provider_configs
             SELECT 'custom','openai-compatible','自定义接口',base_url,model,enabled,stream,temperature,max_output_tokens,timeout_ms,updated_at
             FROM ai_provider_configs WHERE provider_id=?1", params![active_id],
        ).map_err(|error| error.to_string())?;
        if copied > 0 {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO ai_provider_secrets(provider_id,api_key,updated_at)
             SELECT 'custom',api_key,updated_at FROM ai_provider_secrets WHERE provider_id=?1",
                    params![active_id],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    // A formerly customized DeepSeek URL must never reuse its key at the
    // official endpoint. Archive both fields together before resetting it.
    let archive_id = format!("legacy-deepseek-{}", Uuid::new_v4());
    let archived = transaction.execute(
        "INSERT INTO ai_provider_configs SELECT ?1,provider_type,display_name,base_url,model,enabled,stream,temperature,max_output_tokens,timeout_ms,updated_at
         FROM ai_provider_configs WHERE provider_id='deepseek' AND rtrim(base_url,'/')<>?2",
        params![archive_id,crate::ai::DEEPSEEK_BASE_URL],
    ).map_err(|error| error.to_string())?;
    if archived > 0 {
        transaction.execute(
            "INSERT INTO ai_provider_secrets SELECT ?1,api_key,updated_at FROM ai_provider_secrets WHERE provider_id='deepseek'",
            params![archive_id],
        ).map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM ai_provider_secrets WHERE provider_id='deepseek'",
                [],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.execute(
        "INSERT INTO ai_provider_configs(provider_id,provider_type,display_name,base_url,model,enabled,stream,temperature,max_output_tokens,timeout_ms,updated_at)
         VALUES ('deepseek',?1,'DeepSeek',?2,?3,1,1,0.8,1200,60000,?4)
         ON CONFLICT(provider_id) DO UPDATE SET provider_type=excluded.provider_type,display_name=excluded.display_name,
           base_url=excluded.base_url,model=excluded.model,enabled=1,stream=1",
        params![crate::ai::AI_PROTOCOL,crate::ai::DEEPSEEK_BASE_URL,crate::ai::DEEPSEEK_MODEL,now_epoch()],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO settings(key,value_json,updated_at) VALUES ('activeAiProvider',?1,?2)
         ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at",
        params![json!(if use_custom { "custom" } else { "deepseek" }).to_string(), now_string()],
    ).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES (3,?1)",
            params![now_string()],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(super) fn normalize_connection_request(
    mut request: SaveAiProviderRequest,
) -> Result<SaveAiProviderRequest, String> {
    if !crate::ai::is_managed_connection(&request.provider_id)
        || request.provider_type != crate::ai::AI_PROTOCOL
    {
        return Err("仅支持 DeepSeek 或高级设置中的 OpenAI 兼容接口。".to_string());
    }
    request.display_name = request.display_name.trim().to_string();
    if request.display_name.is_empty()
        || request.display_name.chars().count() > 40
        || request.display_name.chars().any(char::is_control)
    {
        return Err("请填写 1 至 40 个字符的配置名称。".to_string());
    }
    if crate::ai::is_deepseek_connection(&request.provider_id) {
        request.base_url = crate::ai::DEEPSEEK_BASE_URL.to_string();
        request.model = request.model.trim().to_string();
        if !crate::ai::is_deepseek_model(&request.model) {
            return Err("请选择列表中的 DeepSeek 模型。".to_string());
        }
    } else {
        request.base_url = request.base_url.trim().trim_end_matches('/').to_string();
        request.model = request.model.trim().to_string();
    }
    let url = reqwest::Url::parse(&request.base_url)
        .map_err(|_| "请填写完整的 HTTP / HTTPS 接口地址。".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("接口地址不能包含账号、密码或查询参数。".to_string());
    }
    if !request.temperature.unwrap_or(0.8).is_finite()
        || !(0.0..=2.0).contains(&request.temperature.unwrap_or(0.8))
        || !(1..=32768).contains(&request.max_output_tokens.unwrap_or(1200))
        || !(1000..=180000).contains(&request.timeout_ms.unwrap_or(60000))
    {
        return Err("模型参数超出允许范围。".to_string());
    }
    if request.model.is_empty() {
        return Err("请填写模型名称。".to_string());
    }
    if let Some(key) = &mut request.api_key {
        *key = key.trim().to_string();
        if reqwest::header::HeaderValue::from_str(&format!("Bearer {key}")).is_err() {
            return Err("API Key 含有无效字符，请重新粘贴。".to_string());
        }
    }
    Ok(request)
}

pub fn get_ai_config_public(app: &AppHandle) -> Result<Vec<AiProviderConfig>, String> {
    let connection = open_connection(app)?;
    let active_provider_id = get_active_ai_provider_id(app)?;
    public_ai_configs(&connection, active_provider_id.as_deref())
}

fn credential_hint(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    match crate::credentials::load(value) {
        Ok(key) if key.chars().count() >= 12 => {
            let suffix: String = key
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            Some(format!("•••• {suffix}"))
        }
        Ok(_) => Some("••••（已保存）".to_string()),
        Err(_) => Some("已保存（需重新填写）".to_string()),
    }
}

fn public_ai_configs(
    connection: &Connection,
    active_provider_id: Option<&str>,
) -> Result<Vec<AiProviderConfig>, String> {
    let mut statement = connection
        .prepare(
            "SELECT c.provider_id, c.provider_type, c.display_name, c.base_url, c.model,
                    c.enabled, c.stream, c.temperature, c.max_output_tokens, c.timeout_ms, c.updated_at,
                    CASE WHEN COALESCE(s.api_key, '')<>'' THEN 1 ELSE 0 END as has_key,
                    COALESCE(s.api_key, '')
             FROM ai_provider_configs c
             LEFT JOIN ai_provider_secrets s ON c.provider_id = s.provider_id
             ORDER BY c.display_name",
        )
        .map_err(|error| error.to_string())?;
    let items = statement
        .query_map([], |row| {
            Ok(AiProviderConfig {
                provider_id: row.get(0)?,
                provider_type: row.get(1)?,
                display_name: row.get(2)?,
                base_url: row.get(3)?,
                model: row.get(4)?,
                enabled: row.get::<_, i64>(5)? == 1,
                stream: row.get::<_, i64>(6)? == 1,
                temperature: row.get(7)?,
                max_output_tokens: row.get(8)?,
                timeout_ms: row.get(9)?,
                updated_at: row.get(10)?,
                has_api_key: row.get::<_, i64>(11)? == 1,
                api_key_hint: if crate::ai::is_managed_connection(&row.get::<_, String>(0)?) {
                    credential_hint(&row.get::<_, String>(12)?)
                } else {
                    None
                },
                is_active: active_provider_id == Some(row.get::<_, String>(0)?.as_str()),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<AiProviderConfig>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

fn resolve_preview(
    req: SaveAiProviderRequest,
    saved: Option<AiProviderFullConfig>,
) -> Result<AiProviderFullConfig, String> {
    let req = normalize_connection_request(req)?;
    let new_key = req.api_key.as_deref().unwrap_or_default();
    let api_key = if !new_key.is_empty() {
        new_key.to_string()
    } else if let Some(saved) = saved {
        let same_origin = reqwest::Url::parse(&saved.config.base_url)
            .ok()
            .map(|url| url.origin())
            == reqwest::Url::parse(&req.base_url)
                .ok()
                .map(|url| url.origin());
        if !saved.api_key.is_empty() && !same_origin {
            return Err("接口地址已更换，请重新填写 API Key。".to_string());
        }
        saved.api_key
    } else {
        String::new()
    };
    if crate::ai::is_deepseek_connection(&req.provider_id) && api_key.is_empty() {
        return Err("请填写 DeepSeek API Key。".to_string());
    }
    Ok(AiProviderFullConfig {
        config: AiProviderConfig {
            provider_id: req.provider_id,
            provider_type: req.provider_type,
            display_name: req.display_name,
            base_url: req.base_url,
            model: req.model,
            enabled: true,
            stream: true,
            temperature: req.temperature.unwrap_or(0.8),
            max_output_tokens: req.max_output_tokens.unwrap_or(1200),
            timeout_ms: req.timeout_ms.unwrap_or(60000),
            has_api_key: !api_key.is_empty(),
            api_key_hint: None,
            is_active: false,
            updated_at: 0,
        },
        api_key,
    })
}

pub fn preview_ai_provider(
    app: &AppHandle,
    req: SaveAiProviderRequest,
) -> Result<AiProviderFullConfig, String> {
    if req
        .api_key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty())
    {
        return resolve_preview(req, None);
    }
    let saved = get_ai_provider_full(app, &req.provider_id)?;
    resolve_preview(req, saved)
}
fn migrate_api_credentials(connection: &Connection) -> Result<(), String> {
    let _guard = CREDENTIAL_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let rows: Vec<(String, String)> = {
        let mut query = connection
            .prepare("SELECT provider_id,api_key FROM ai_provider_secrets WHERE api_key<>''")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut references: Vec<String> = Vec::new();
    for (id, value) in rows {
        if crate::credentials::is_reference(&value) {
            continue;
        }
        let reference = match crate::credentials::store(&value) {
            Ok(reference) => reference,
            Err(error) => {
                for item in &references {
                    let _ = crate::credentials::delete(item);
                }
                return Err(error);
            }
        };
        transaction
            .execute(
                "UPDATE ai_provider_secrets SET api_key=?1 WHERE provider_id=?2",
                params![reference, id],
            )
            .map_err(|e| e.to_string())?;
        references.push(reference);
    }
    transaction.commit().map_err(|e| e.to_string())
}

static CREDENTIAL_WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn cleanup_unreferenced_credential(connection: &Connection, value: &str) {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE api_key=?1",
            [value],
            |row| row.get(0),
        )
        .unwrap_or(1);
    if count == 0 {
        let _ = crate::credentials::delete(value);
    }
}

pub fn save_ai_provider_config(
    app: &AppHandle,
    req: SaveAiProviderRequest,
) -> Result<AiProviderConfig, String> {
    let _guard = CREDENTIAL_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let connection = open_connection(app)?;
    let is_active = get_active_ai_provider_id(app)?.as_deref() == Some(req.provider_id.as_str());
    let mut req = normalize_connection_request(req)?;
    let previous: Option<String> = connection
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id=?1",
            [&req.provider_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let reference = req
        .api_key
        .as_deref()
        .filter(|key| !key.is_empty())
        .map(crate::credentials::store)
        .transpose()?;
    if let Some(reference) = &reference {
        req.api_key = Some(reference.clone());
    }
    match save_connection(&connection, req, is_active) {
        Ok(config) => {
            if let Some(previous) = previous {
                cleanup_unreferenced_credential(&connection, &previous);
            }
            Ok(config)
        }
        Err(error) => {
            if let Some(reference) = reference {
                let _ = crate::credentials::delete(&reference);
            }
            Err(error)
        }
    }
}

fn save_connection(
    connection: &Connection,
    req: SaveAiProviderRequest,
    is_active: bool,
) -> Result<AiProviderConfig, String> {
    let req = normalize_connection_request(req)?;
    let connection = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let previous: Option<(String, String)> = connection.query_row(
        "SELECT c.base_url,COALESCE(s.api_key,'') FROM ai_provider_configs c LEFT JOIN ai_provider_secrets s ON s.provider_id=c.provider_id WHERE c.provider_id=?1",
        params![req.provider_id], |row| Ok((row.get(0)?,row.get(1)?)),
    ).optional().map_err(|error| error.to_string())?;
    let has_new_key = req.api_key.as_ref().is_some_and(|key| !key.is_empty());
    if !has_new_key {
        if let Some((url, key)) = &previous {
            let same_origin = reqwest::Url::parse(url).ok().map(|url| url.origin())
                == reqwest::Url::parse(&req.base_url)
                    .ok()
                    .map(|url| url.origin());
            if !key.is_empty() && !same_origin {
                return Err(
                    "接口地址已更换，请重新填写 API Key，避免将旧密钥发送到其他服务。".to_string(),
                );
            }
        }
        if crate::ai::is_deepseek_connection(&req.provider_id)
            && !previous.as_ref().is_some_and(|(_, key)| !key.is_empty())
        {
            return Err("请填写 DeepSeek API Key。".to_string());
        }
    }
    let now = now_epoch();

    connection
        .execute(
            "INSERT INTO ai_provider_configs(provider_id, provider_type, display_name, base_url, model, enabled, stream, temperature, max_output_tokens, timeout_ms, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(provider_id) DO UPDATE SET
               provider_type=excluded.provider_type,
               enabled=1,
               display_name=excluded.display_name,
               base_url=excluded.base_url,
               model=excluded.model,
               stream=excluded.stream,
               temperature=excluded.temperature,
               max_output_tokens=excluded.max_output_tokens,
               timeout_ms=excluded.timeout_ms,
               updated_at=excluded.updated_at",
            params![
                req.provider_id,
                req.provider_type,
                req.display_name,
                req.base_url,
                req.model,
                if req.stream.unwrap_or(true) { 1 } else { 0 },
                req.temperature.unwrap_or(0.8),
                req.max_output_tokens.unwrap_or(1200),
                req.timeout_ms.unwrap_or(60000),
                now,
            ],
        )
        .map_err(|error| error.to_string())?;

    // Only update API key if provided
    if let Some(ref key) = req.api_key {
        if !key.is_empty() {
            connection
                .execute(
                    "INSERT INTO ai_provider_secrets(provider_id, api_key, updated_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(provider_id) DO UPDATE SET api_key=excluded.api_key, updated_at=excluded.updated_at",
                    params![req.provider_id, key, now],
                )
                .map_err(|error| error.to_string())?;
        }
    }

    // Return the saved config
    let has_key = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id=?1",
            params![req.provider_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    connection.commit().map_err(|error| error.to_string())?;

    Ok(AiProviderConfig {
        provider_id: req.provider_id,
        provider_type: req.provider_type,
        display_name: req.display_name,
        base_url: req.base_url,
        model: req.model,
        enabled: true,
        stream: req.stream.unwrap_or(true),
        temperature: req.temperature.unwrap_or(0.8),
        max_output_tokens: req.max_output_tokens.unwrap_or(1200),
        timeout_ms: req.timeout_ms.unwrap_or(60000),
        has_api_key: has_key,
        api_key_hint: None,
        is_active,
        updated_at: now,
    })
}

pub fn delete_ai_connection(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let _guard = CREDENTIAL_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let connection = open_connection(app)?;
    let previous = delete_connection(&connection, &provider_id)?;
    if let Some(previous) = previous {
        cleanup_unreferenced_credential(&connection, &previous);
    }
    Ok(())
}

pub fn set_active_ai_provider(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_connection(app)?;
    activate_connection(&connection, &provider_id)
}

fn delete_connection(connection: &Connection, provider_id: &str) -> Result<Option<String>, String> {
    if !crate::ai::is_managed_connection(provider_id) {
        return Err("请在设置中选择 DeepSeek 或自定义接口。".to_string());
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    if active_connection_id(&transaction)?.as_deref() == Some(provider_id) {
        return Err("当前配置正在启用，请先启用另一个配置再删除。".to_string());
    }
    let previous = transaction
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id=?1",
            [provider_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    transaction
        .execute(
            "DELETE FROM ai_provider_secrets WHERE provider_id=?1",
            [provider_id],
        )
        .map_err(|e| e.to_string())?;
    transaction
        .execute(
            "DELETE FROM ai_provider_configs WHERE provider_id=?1",
            [provider_id],
        )
        .map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(previous)
}

fn activate_connection(connection: &Connection, provider_id: &str) -> Result<(), String> {
    if !crate::ai::is_managed_connection(provider_id) {
        return Err("请在设置中选择 DeepSeek 或自定义接口。".to_string());
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM ai_provider_configs c WHERE c.provider_id=?1 AND c.provider_type=?2 AND c.enabled=1 AND (?3=0 OR EXISTS(SELECT 1 FROM ai_provider_secrets s WHERE s.provider_id=c.provider_id AND s.api_key<>'')))",
        params![provider_id,crate::ai::AI_PROTOCOL,crate::ai::is_deepseek_connection(provider_id)], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if !exists {
        return Err("请先保存有效的聊天配置和 API Key。".to_string());
    }
    let now = now_string();
    transaction
        .execute(
            "INSERT INTO settings(key, value_json, updated_at) VALUES ('activeAiProvider', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![json!(provider_id).to_string(), now],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}

pub fn get_active_ai_provider_id(app: &AppHandle) -> Result<Option<String>, String> {
    let connection = open_connection(app)?;
    active_connection_id(&connection)
}

fn active_connection_id(connection: &Connection) -> Result<Option<String>, String> {
    let result = connection
        .query_row(
            "SELECT value_json FROM settings WHERE key='activeAiProvider'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match result {
        Some(json_str) => {
            let value: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);
            Ok(value.as_str().map(|s| s.to_string()))
        }
        None => Ok(None),
    }
}

/// Internal function: get full config including API key. Only for Rust backend use.
pub fn get_active_ai_provider_internal(
    app: &AppHandle,
) -> Result<Option<AiProviderFullConfig>, String> {
    let provider_id = match get_active_ai_provider_id(app)? {
        Some(id) => id,
        None => return Ok(None),
    };

    let connection = open_connection(app)?;
    let config = connection
        .query_row(
            "SELECT c.provider_id, c.provider_type, c.display_name, c.base_url, c.model,
                    c.enabled, c.stream, c.temperature, c.max_output_tokens, c.timeout_ms, c.updated_at,
                    COALESCE(s.api_key, '') as api_key
             FROM ai_provider_configs c
             LEFT JOIN ai_provider_secrets s ON c.provider_id = s.provider_id
             WHERE c.provider_id=?1",
            params![provider_id],
            |row| {
                Ok(AiProviderFullConfig {
                    config: AiProviderConfig {
                        provider_id: row.get(0)?,
                        provider_type: row.get(1)?,
                        display_name: row.get(2)?,
                        base_url: row.get(3)?,
                        model: row.get(4)?,
                        enabled: row.get::<_, i64>(5)? == 1,
                        stream: row.get::<_, i64>(6)? == 1,
                        temperature: row.get(7)?,
                        max_output_tokens: row.get(8)?,
                        timeout_ms: row.get(9)?,
                        updated_at: row.get(10)?,
                        has_api_key: !row.get::<_, String>(11)?.is_empty(),
                        api_key_hint: None,
                        is_active: true,
                    },
                    api_key: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    config
        .map(|mut config| {
            config.api_key = crate::credentials::load(&config.api_key)?;
            Ok(config)
        })
        .transpose()
}

pub fn save_assistant_message(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    content: &str,
) -> Result<ChatMessage, String> {
    let connection = open_connection(app)?;
    ensure_session_owner(&connection, session_id, character_id)?;
    let now = now_string();
    let message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        character_id: character_id.to_string(),
        role: "assistant".to_string(),
        content: content.to_string(),
        created_at: now.clone(),
        token_count: content.chars().count() as i64,
        metadata_json: None,
    };
    connection
        .execute(
            "INSERT INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)",
            params![
                message.id,
                message.session_id,
                message.character_id,
                message.role,
                message.content,
                message.created_at,
                message.token_count
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE chat_sessions SET updated_at=?1 WHERE id=?2",
            params![now, session_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(message)
}

pub fn save_user_message(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    content: &str,
) -> Result<ChatMessage, String> {
    let connection = open_connection(app)?;
    let session = get_or_create_session(&connection, character_id, Some(session_id.to_string()))?;
    let now = now_string();

    let message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        character_id: character_id.to_string(),
        role: "user".to_string(),
        content: content.to_string(),
        created_at: now.clone(),
        token_count: content.chars().count() as i64,
        metadata_json: None,
    };

    connection
        .execute(
            "INSERT INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)",
            params![
                message.id,
                message.session_id,
                message.character_id,
                message.role,
                message.content,
                message.created_at,
                message.token_count
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE chat_sessions SET updated_at=?1 WHERE id=?2",
            params![now, session.id],
        )
        .map_err(|error| error.to_string())?;

    Ok(message)
}

pub fn save_interrupted_ai_message(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    content: &str,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    ensure_session_owner(&connection, session_id, character_id)?;
    let now = now_string();
    if !content.trim().is_empty() {
        let message = ChatMessage {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            character_id: character_id.to_string(),
            role: "assistant".to_string(),
            content: content.to_string(),
            created_at: now.clone(),
            token_count: content.chars().count() as i64,
            metadata_json: Some(json!({ "status": "interrupted" }).to_string()),
        };
        connection
            .execute(
                "INSERT INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    message.id,
                    message.session_id,
                    message.character_id,
                    message.role,
                    message.content,
                    message.created_at,
                    message.token_count,
                    message.metadata_json
                ],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE chat_sessions SET updated_at=?1 WHERE id=?2",
                params![now, session_id],
            )
            .map_err(|error| error.to_string())?;
    }
    fetch_bootstrap(app, &connection)
}

/// Get chat history for a session (for building AI context)
pub fn get_chat_history(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    limit: usize,
) -> Result<Vec<ChatMessage>, String> {
    let connection = open_connection(app)?;
    ensure_session_owner(&connection, session_id, character_id)?;
    let mut statement = connection
        .prepare(
            "SELECT id,session_id,character_id,role,content,created_at,token_count,metadata_json
             FROM chat_messages WHERE session_id=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let mut items = statement
        .query_map(params![session_id, limit as i64], read_message)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<ChatMessage>>>()
        .map_err(|error| error.to_string())?;
    items.reverse(); // chronological order
    Ok(items)
}

/// Get full provider config by ID (internal use only, includes API key)
pub fn get_ai_provider_full(
    app: &AppHandle,
    provider_id: &str,
) -> Result<Option<AiProviderFullConfig>, String> {
    let connection = open_connection(app)?;
    let active_provider_id = get_active_ai_provider_id(app)?;
    let result = connection
        .query_row(
            "SELECT c.provider_id, c.provider_type, c.display_name, c.base_url, c.model,
                    c.enabled, c.stream, c.temperature, c.max_output_tokens, c.timeout_ms, c.updated_at,
                    COALESCE(s.api_key, '') as api_key
             FROM ai_provider_configs c
             LEFT JOIN ai_provider_secrets s ON c.provider_id = s.provider_id
             WHERE c.provider_id=?1",
            params![provider_id],
            |row| {
                Ok(AiProviderFullConfig {
                    config: AiProviderConfig {
                        provider_id: row.get(0)?,
                        provider_type: row.get(1)?,
                        display_name: row.get(2)?,
                        base_url: row.get(3)?,
                        model: row.get(4)?,
                        enabled: row.get::<_, i64>(5)? == 1,
                        stream: row.get::<_, i64>(6)? == 1,
                        temperature: row.get(7)?,
                        max_output_tokens: row.get(8)?,
                        timeout_ms: row.get(9)?,
                        updated_at: row.get(10)?,
                        has_api_key: !row.get::<_, String>(11)?.is_empty(),
                        api_key_hint: None,
                        is_active: active_provider_id.as_deref()
                            == Some(row.get::<_, String>(0)?.as_str()),
                    },
                    api_key: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    result
        .map(|mut config| {
            config.api_key = crate::credentials::load(&config.api_key)?;
            Ok(config)
        })
        .transpose()
}
