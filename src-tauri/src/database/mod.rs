use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Weekday};
use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use uuid::Uuid;

use crate::resources::{self, DataPaths};

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
    pub favorite_slot_ids: Vec<i64>,
    pub display_scale: f64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetStats {
    pub character_id: String,
    pub energy: f64,
    pub affection: f64,
    pub today_chat_rounds: i64,
    pub today_interaction_count: i64,
    pub today_had_activity: bool,
    pub reward_50_triggered: bool,
    pub reward_100_triggered: bool,
    pub last_chat_at: Option<String>,
    pub last_daily_settlement_at: Option<String>,
    pub last_weekly_inventory_clear_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodItem {
    pub id: String,
    pub character_id: String,
    pub slot_id: i64,
    pub food_level: i64,
    pub display_order: i64,
    pub name: String,
    pub icon_path: Option<String>,
    pub energy_delta: f64,
    pub affection_delta: f64,
    pub category: String,
    pub rarity: String,
    pub description: String,
    pub enabled: bool,
    pub count: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPrefs {
    pub food_drops: bool,
    pub errors: bool,
    pub reminders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub api_key: String,
    pub provider: String,
    pub model_name: String,
    pub auto_start: bool,
    pub sound_enabled: bool,
    pub tts_enabled: bool,
    pub volume: i64,
    pub notification_prefs: NotificationPrefs,
    pub privacy_lock: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub data_paths: DataPaths,
    pub character: CharacterProfile,
    pub characters: Vec<CharacterProfile>,
    pub stats: PetStats,
    pub foods: Vec<FoodItem>,
    pub settings: AppSettings,
    pub sessions: Vec<ChatSession>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendResult {
    pub session: ChatSession,
    pub user_message: ChatMessage,
    pub assistant_message: ChatMessage,
    pub stats: PetStats,
    pub foods: Vec<FoodItem>,
    pub triggered_state_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedResult {
    pub food: FoodItem,
    pub stats: PetStats,
    pub triggered_state_key: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodReplaceRequest {
    pub food_id: String,
    pub name: String,
    pub icon_data_url: String,
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

fn local_date_key() -> String {
    business_date_for(Local::now())
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

const ENERGY_DEFAULT: f64 = 60.0;
const FOOD_DROP_CHANCE: f64 = 0.15;
const DAILY_SETTLEMENT_HOUR: u32 = 6;

fn food_effect(food_level: i64) -> (f64, f64) {
    match food_level {
        0 => (5.0, -2.0),
        1 => (2.0, 0.0),
        2 => (5.0, 0.0),
        _ => (10.0, 0.0),
    }
}

fn business_date_for(dt: DateTime<Local>) -> String {
    let cutoff = Local
        .with_ymd_and_hms(dt.year(), dt.month(), dt.day(), DAILY_SETTLEMENT_HOUR, 0, 0)
        .single()
        .unwrap_or(dt);
    let date = if dt < cutoff {
        dt.date_naive() - Duration::days(1)
    } else {
        dt.date_naive()
    };
    date.format("%Y-%m-%d").to_string()
}

fn settlement_time_for_date(date: chrono::NaiveDate) -> DateTime<Local> {
    Local
        .from_local_datetime(
            &date.and_time(NaiveTime::from_hms_opt(DAILY_SETTLEMENT_HOUR, 0, 0).unwrap()),
        )
        .single()
        .unwrap_or_else(Local::now)
}

fn next_settlement_after(last: DateTime<Local>) -> DateTime<Local> {
    let today_cutoff = settlement_time_for_date(last.date_naive());
    if last < today_cutoff {
        today_cutoff
    } else {
        settlement_time_for_date(last.date_naive() + Duration::days(1))
    }
}

fn db_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(resources::data_root(app)?.join("bytepet.db"))
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
    resources::ensure_data_dirs(app)?;
    let connection = Connection::open(db_path(app)?).map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| error.to_string())?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

pub fn bootstrap(app: &AppHandle) -> Result<BootstrapPayload, String> {
    resources::ensure_data_dirs(app)?;
    let connection = open_connection(app)?;
    migrate(&connection)?;
    seed_defaults(app, &connection)?;
    repair_character_integrity(app, &connection)?;
    seed_foods_for_all_characters(&connection, &now_string())?;
    run_due_settlements(&connection)?;
    repair_character_integrity(app, &connection)?;
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

            CREATE TABLE IF NOT EXISTS pet_stats (
              character_id TEXT PRIMARY KEY,
              energy REAL NOT NULL CHECK(energy >= 0 AND energy <= 100),
              affection REAL NOT NULL CHECK(affection >= 0 AND affection <= 100),
              today_chat_rounds INTEGER NOT NULL DEFAULT 0,
              today_interaction_count INTEGER NOT NULL DEFAULT 0,
              today_had_activity INTEGER NOT NULL DEFAULT 0,
              reward_50_triggered INTEGER NOT NULL DEFAULT 0,
              reward_100_triggered INTEGER NOT NULL DEFAULT 0,
              last_chat_at TEXT,
              last_daily_settlement_at TEXT,
              last_weekly_inventory_clear_at TEXT,
              updated_at TEXT NOT NULL,
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS foods (
              id TEXT PRIMARY KEY,
              character_id TEXT NOT NULL,
              slot_id INTEGER NOT NULL CHECK(slot_id >= 1 AND slot_id <= 9),
              food_level INTEGER NOT NULL DEFAULT 1 CHECK(food_level >= 0 AND food_level <= 3),
              display_order INTEGER NOT NULL DEFAULT 1,
              name TEXT NOT NULL,
              icon_path TEXT,
              energy_delta REAL NOT NULL,
              affection_delta REAL NOT NULL,
              category TEXT NOT NULL,
              rarity TEXT NOT NULL,
              description TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              UNIQUE(character_id, slot_id),
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS food_inventory (
              character_id TEXT NOT NULL,
              food_id TEXT NOT NULL,
              count INTEGER NOT NULL DEFAULT 0 CHECK(count >= 0),
              updated_at TEXT NOT NULL,
              PRIMARY KEY(character_id, food_id),
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE,
              FOREIGN KEY(food_id) REFERENCES foods(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS favorite_food_slots (
              character_id TEXT NOT NULL,
              slot_id INTEGER NOT NULL CHECK(slot_id >= 1 AND slot_id <= 12),
              created_at TEXT NOT NULL,
              PRIMARY KEY(character_id, slot_id),
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

            CREATE TABLE IF NOT EXISTS daily_activity (
              date TEXT NOT NULL,
              character_id TEXT NOT NULL,
              chat_round_count INTEGER NOT NULL DEFAULT 0,
              interaction_count INTEGER NOT NULL DEFAULT 0,
              food_generated_count INTEGER NOT NULL DEFAULT 0,
              affection_delta REAL NOT NULL DEFAULT 0,
              energy_delta REAL NOT NULL DEFAULT 0,
              settled_at TEXT,
              PRIMARY KEY(date, character_id),
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
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
    migrate_food_tables_to_character_scope(connection)?;
    ensure_food_indexes(connection)?;
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

fn food_id_for_character_slot(character_id: &str, slot_id: i64) -> String {
    format!("food::{character_id}::slot::{slot_id}")
}

fn ensure_food_indexes(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_foods_character ON foods(character_id);
            CREATE INDEX IF NOT EXISTS idx_food_inventory_character ON food_inventory(character_id);
            "#,
        )
        .map_err(|error| error.to_string())
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

    ensure_column(
        connection,
        "pet_stats",
        "affection",
        "ALTER TABLE pet_stats ADD COLUMN affection REAL NOT NULL DEFAULT 40;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "today_chat_rounds",
        "ALTER TABLE pet_stats ADD COLUMN today_chat_rounds INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "today_interaction_count",
        "ALTER TABLE pet_stats ADD COLUMN today_interaction_count INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "today_had_activity",
        "ALTER TABLE pet_stats ADD COLUMN today_had_activity INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "reward_50_triggered",
        "ALTER TABLE pet_stats ADD COLUMN reward_50_triggered INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "reward_100_triggered",
        "ALTER TABLE pet_stats ADD COLUMN reward_100_triggered INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "last_chat_at",
        "ALTER TABLE pet_stats ADD COLUMN last_chat_at TEXT;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "last_daily_settlement_at",
        "ALTER TABLE pet_stats ADD COLUMN last_daily_settlement_at TEXT;",
    )?;
    ensure_column(
        connection,
        "pet_stats",
        "last_weekly_inventory_clear_at",
        "ALTER TABLE pet_stats ADD COLUMN last_weekly_inventory_clear_at TEXT;",
    )?;

    let columns = table_columns(connection, "pet_stats")?;
    if columns.iter().any(|name| name == "favorability") {
        connection
            .execute_batch(
                "UPDATE pet_stats
                 SET affection=favorability
                 WHERE affection=40;",
            )
            .map_err(|error| error.to_string())?;
    }
    if columns.iter().any(|name| name == "today_chat_count") {
        connection
            .execute_batch("UPDATE pet_stats SET today_chat_rounds=today_chat_count WHERE today_chat_rounds=0;")
            .map_err(|error| error.to_string())?;
    }
    if columns.iter().any(|name| name == "last_chat_date") {
        connection
            .execute_batch(
                "UPDATE pet_stats SET last_chat_at=last_chat_date WHERE last_chat_at IS NULL;",
            )
            .map_err(|error| error.to_string())?;
    }
    if columns
        .iter()
        .any(|name| name == "last_daily_settlement_date")
    {
        connection
            .execute_batch("UPDATE pet_stats SET last_daily_settlement_at=last_daily_settlement_date WHERE last_daily_settlement_at IS NULL;")
            .map_err(|error| error.to_string())?;
    }
    if columns.iter().any(|name| name == "temperature") {
        connection
            .execute_batch("ALTER TABLE pet_stats DROP COLUMN temperature;")
            .map_err(|error| error.to_string())?;
    }

    ensure_column(
        connection,
        "foods",
        "food_level",
        "ALTER TABLE foods ADD COLUMN food_level INTEGER NOT NULL DEFAULT 1;",
    )?;
    ensure_column(
        connection,
        "foods",
        "display_order",
        "ALTER TABLE foods ADD COLUMN display_order INTEGER NOT NULL DEFAULT 1;",
    )?;
    ensure_column(
        connection,
        "foods",
        "icon_path",
        "ALTER TABLE foods ADD COLUMN icon_path TEXT;",
    )?;
    ensure_column(
        connection,
        "foods",
        "energy_delta",
        "ALTER TABLE foods ADD COLUMN energy_delta REAL NOT NULL DEFAULT 10;",
    )?;
    ensure_column(
        connection,
        "foods",
        "affection_delta",
        "ALTER TABLE foods ADD COLUMN affection_delta REAL NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "foods",
        "category",
        "ALTER TABLE foods ADD COLUMN category TEXT NOT NULL DEFAULT '鏁版嵁椋熺墿';",
    )?;
    ensure_column(
        connection,
        "foods",
        "rarity",
        "ALTER TABLE foods ADD COLUMN rarity TEXT NOT NULL DEFAULT 'common';",
    )?;
    ensure_column(
        connection,
        "foods",
        "description",
        "ALTER TABLE foods ADD COLUMN description TEXT NOT NULL DEFAULT '';",
    )?;
    ensure_column(
        connection,
        "foods",
        "enabled",
        "ALTER TABLE foods ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;",
    )?;
    ensure_column(
        connection,
        "foods",
        "created_at",
        "ALTER TABLE foods ADD COLUMN created_at TEXT NOT NULL DEFAULT '';",
    )?;
    ensure_column(
        connection,
        "foods",
        "updated_at",
        "ALTER TABLE foods ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';",
    )?;
    let food_columns = table_columns(connection, "foods")?;
    if food_columns.iter().any(|name| name == "favor_delta") {
        connection
            .execute_batch("UPDATE foods SET affection_delta=favor_delta WHERE affection_delta=0;")
            .map_err(|error| error.to_string())?;
    }
    ensure_column(
        connection,
        "daily_activity",
        "chat_round_count",
        "ALTER TABLE daily_activity ADD COLUMN chat_round_count INTEGER NOT NULL DEFAULT 0;",
    )?;
    ensure_column(
        connection,
        "daily_activity",
        "interaction_count",
        "ALTER TABLE daily_activity ADD COLUMN interaction_count INTEGER NOT NULL DEFAULT 0;",
    )?;
    let daily_columns = table_columns(connection, "daily_activity")?;
    if daily_columns.iter().any(|name| name == "chat_count") {
        connection
            .execute_batch(
                "UPDATE daily_activity SET chat_round_count=chat_count WHERE chat_round_count=0;",
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

struct FoodMigrationRow {
    id: String,
    character_id: String,
    slot_id: i64,
    food_level: i64,
    display_order: i64,
    name: String,
    icon_path: Option<String>,
    energy_delta: f64,
    affection_delta: f64,
    category: String,
    rarity: String,
    description: String,
    enabled: i64,
    count: i64,
    created_at: String,
    updated_at: String,
}

fn active_or_first_character_id(connection: &Connection) -> Result<Option<String>, String> {
    let active = connection
        .query_row(
            "SELECT id FROM characters WHERE is_active=1 LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if active.is_some() {
        return Ok(active);
    }

    connection
        .query_row(
            "SELECT id FROM characters ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn create_character_scoped_food_tables(
    connection: &Connection,
    food_table: &str,
    inventory_table: &str,
) -> Result<(), String> {
    connection
        .execute_batch(&format!(
            r#"
            CREATE TABLE {food_table} (
              id TEXT PRIMARY KEY,
              character_id TEXT NOT NULL,
              slot_id INTEGER NOT NULL CHECK(slot_id >= 1 AND slot_id <= 9),
              food_level INTEGER NOT NULL DEFAULT 1 CHECK(food_level >= 0 AND food_level <= 3),
              display_order INTEGER NOT NULL DEFAULT 1,
              name TEXT NOT NULL,
              icon_path TEXT,
              energy_delta REAL NOT NULL,
              affection_delta REAL NOT NULL,
              category TEXT NOT NULL,
              rarity TEXT NOT NULL,
              description TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              UNIQUE(character_id, slot_id),
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE
            );

            CREATE TABLE {inventory_table} (
              character_id TEXT NOT NULL,
              food_id TEXT NOT NULL,
              count INTEGER NOT NULL DEFAULT 0 CHECK(count >= 0),
              updated_at TEXT NOT NULL,
              PRIMARY KEY(character_id, food_id),
              FOREIGN KEY(character_id) REFERENCES characters(id) ON DELETE CASCADE,
              FOREIGN KEY(food_id) REFERENCES {food_table}(id) ON DELETE CASCADE
            );
            "#
        ))
        .map_err(|error| error.to_string())
}

fn migrate_food_tables_to_character_scope(connection: &Connection) -> Result<(), String> {
    let food_columns = table_columns(connection, "foods")?;
    let inventory_columns = table_columns(connection, "food_inventory")?;
    let foods_are_scoped = food_columns.iter().any(|name| name == "character_id");
    let inventory_is_scoped = inventory_columns.iter().any(|name| name == "character_id");
    if foods_are_scoped && inventory_is_scoped {
        return Ok(());
    }

    let Some(default_character_id) = active_or_first_character_id(connection)? else {
        return Ok(());
    };
    let now = now_string();

    let rows = if foods_are_scoped {
        let join = if inventory_is_scoped {
            "LEFT JOIN food_inventory i ON i.food_id=f.id AND i.character_id=f.character_id"
        } else {
            "LEFT JOIN food_inventory i ON i.food_id=f.id"
        };
        let mut statement = connection
            .prepare(&format!(
                "SELECT f.id,f.character_id,f.slot_id,f.food_level,f.display_order,f.name,f.icon_path,
                        f.energy_delta,f.affection_delta,f.category,f.rarity,f.description,f.enabled,
                        COALESCE(i.count,0),f.created_at,f.updated_at
                 FROM foods f {join}"
            ))
            .map_err(|error| error.to_string())?;
        let items = statement
            .query_map([], |row| {
                Ok(FoodMigrationRow {
                    id: row.get(0)?,
                    character_id: row.get(1)?,
                    slot_id: row.get(2)?,
                    food_level: row.get(3)?,
                    display_order: row.get(4)?,
                    name: row.get(5)?,
                    icon_path: row.get(6)?,
                    energy_delta: row.get(7)?,
                    affection_delta: row.get(8)?,
                    category: row.get(9)?,
                    rarity: row.get(10)?,
                    description: row.get(11)?,
                    enabled: row.get(12)?,
                    count: row.get(13)?,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        items
    } else {
        let mut statement = connection
            .prepare(
                "SELECT f.id,f.slot_id,f.food_level,f.display_order,f.name,f.icon_path,
                        f.energy_delta,f.affection_delta,f.category,f.rarity,f.description,f.enabled,
                        COALESCE(i.count,0),f.created_at,f.updated_at
                 FROM foods f LEFT JOIN food_inventory i ON i.food_id=f.id",
            )
            .map_err(|error| error.to_string())?;
        let items = statement
            .query_map([], |row| {
                Ok(FoodMigrationRow {
                    id: row.get(0)?,
                    character_id: default_character_id.clone(),
                    slot_id: row.get(1)?,
                    food_level: row.get(2)?,
                    display_order: row.get(3)?,
                    name: row.get(4)?,
                    icon_path: row.get(5)?,
                    energy_delta: row.get(6)?,
                    affection_delta: row.get(7)?,
                    category: row.get(8)?,
                    rarity: row.get(9)?,
                    description: row.get(10)?,
                    enabled: row.get(11)?,
                    count: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        items
    };

    create_character_scoped_food_tables(
        connection,
        "foods_scoped_new",
        "food_inventory_scoped_new",
    )?;

    for row in rows {
        let food_id = if foods_are_scoped && row.id.starts_with("food::") {
            row.id
        } else {
            food_id_for_character_slot(&row.character_id, row.slot_id)
        };
        connection
            .execute(
                "INSERT OR REPLACE INTO foods_scoped_new(id,character_id,slot_id,food_level,display_order,name,icon_path,energy_delta,affection_delta,category,rarity,description,enabled,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    food_id,
                    row.character_id,
                    row.slot_id,
                    row.food_level,
                    row.display_order,
                    row.name,
                    row.icon_path,
                    row.energy_delta,
                    row.affection_delta,
                    row.category,
                    row.rarity,
                    row.description,
                    row.enabled,
                    if row.created_at.is_empty() { now.clone() } else { row.created_at },
                    if row.updated_at.is_empty() { now.clone() } else { row.updated_at },
                ],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT OR REPLACE INTO food_inventory_scoped_new(character_id,food_id,count,updated_at)
                 VALUES (?1,?2,?3,?4)",
                params![row.character_id, food_id, row.count.max(0), now],
            )
            .map_err(|error| error.to_string())?;
    }

    connection
        .execute_batch(
            r#"
            DROP TABLE food_inventory;
            DROP TABLE foods;
            ALTER TABLE foods_scoped_new RENAME TO foods;
            ALTER TABLE food_inventory_scoped_new RENAME TO food_inventory;
            CREATE INDEX IF NOT EXISTS idx_foods_character ON foods(character_id);
            CREATE INDEX IF NOT EXISTS idx_food_inventory_character ON food_inventory(character_id);
            "#,
        )
        .map_err(|error| error.to_string())?;

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

    connection
        .execute(
            "INSERT INTO pet_stats(character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at)
             VALUES (?1,60,40,0,0,0,0,0,NULL,NULL,NULL,?2)",
            params![character_id, now],
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

    seed_foods_for_character(connection, character_id, &now)?;
    seed_default_session(connection, character_id, &now)?;
    save_settings_values(connection, &default_settings(), &now)?;
    Ok(())
}

fn canonical_foods() -> [(i64, i64, &'static str, &'static str); 9] {
    [
        (
            1,
            0,
            "调试苦瓜",
            "0级食物：恢复 5 点能量，但会减少 2 点好感。",
        ),
        (2, 1, "数据饼干", "1级食物：恢复 2 点能量。"),
        (3, 1, "像素糖", "1级食物：恢复 2 点能量。"),
        (4, 1, "小电池", "1级食物：恢复 2 点能量。"),
        (5, 2, "能量模块", "2级食物：恢复 5 点能量。"),
        (6, 2, "代码曲奇", "2级食物：恢复 5 点能量。"),
        (7, 2, "记忆果冻", "2级食物：恢复 5 点能量。"),
        (8, 2, "云同步包", "2级食物：恢复 5 点能量。"),
        (9, 3, "星核便当", "3级食物：恢复 10 点能量。"),
    ]
}

fn legacy_food_name(slot_id: i64) -> &'static str {
    match slot_id {
        1 => "Debug Bittergourd",
        2 => "Data Cookie",
        3 => "Pixel Candy",
        4 => "Tiny Battery",
        5 => "Energy Module",
        6 => "Code Cookie",
        7 => "Memory Jelly",
        8 => "Cloud Sync Pack",
        9 => "Star Core Bento",
        _ => "",
    }
}

fn seed_foods_for_all_characters(connection: &Connection, now: &str) -> Result<(), String> {
    let mut statement = connection
        .prepare("SELECT id FROM characters")
        .map_err(|error| error.to_string())?;
    let character_ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    for character_id in character_ids {
        seed_foods_for_character(connection, &character_id, now)?;
    }
    Ok(())
}

fn seed_foods_for_character(
    connection: &Connection,
    character_id: &str,
    now: &str,
) -> Result<(), String> {
    for food in canonical_foods() {
        let (energy_delta, affection_delta) = food_effect(food.1);
        let rarity = match food.1 {
            3 => "legendary",
            2 => "rare",
            _ => "common",
        };
        let food_id = food_id_for_character_slot(character_id, food.0);
        connection
            .execute(
                "INSERT INTO foods(id,character_id,slot_id,food_level,display_order,name,icon_path,energy_delta,affection_delta,category,rarity,description,enabled,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?3,?5,NULL,?6,?7,?8,?9,?10,1,?11,?11)
                 ON CONFLICT(id) DO UPDATE SET
                   character_id=excluded.character_id,
                   slot_id=excluded.slot_id,
                   food_level=excluded.food_level,
                   name=CASE WHEN foods.name=?12 THEN excluded.name ELSE foods.name END,
                   energy_delta=excluded.energy_delta,
                   affection_delta=excluded.affection_delta,
                   category=excluded.category,
                   rarity=excluded.rarity,
                   description=excluded.description,
                   enabled=1,
                   updated_at=excluded.updated_at",
                 params![
                    food_id,
                    character_id,
                    food.0,
                    food.1,
                    food.2,
                    energy_delta,
                    affection_delta,
                    format!("{}级食物", food.1),
                    rarity,
                    food.3,
                    now,
                    legacy_food_name(food.0)
                 ],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO food_inventory(character_id,food_id,count,updated_at) VALUES (?1,?2,0,?3)",
                params![character_id, food_id, now],
            )
            .map_err(|error| error.to_string())?;
    }
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
    connection
        .execute(
            "INSERT OR IGNORE INTO pet_stats(character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at)
             SELECT id,60,40,0,0,0,0,0,NULL,NULL,NULL,?1 FROM characters",
            params![now],
        )
        .map_err(|error| error.to_string())?;

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

        connection
            .execute(
                "INSERT OR IGNORE INTO pet_stats(character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at)
                 VALUES (?1,60,40,0,0,0,0,0,NULL,NULL,NULL,?2)",
                params![profile.id, now],
            )
            .map_err(|error| error.to_string())?;

        seed_foods_for_character(connection, &profile.id, &now)?;
    }

    Ok(())
}

fn default_settings() -> AppSettings {
    AppSettings {
        api_key: String::new(),
        provider: "openai_compatible".to_string(),
        model_name: "gpt-4o".to_string(),
        auto_start: false,
        sound_enabled: true,
        tts_enabled: false,
        volume: 50,
        notification_prefs: NotificationPrefs {
            food_drops: true,
            errors: true,
            reminders: false,
        },
        privacy_lock: false,
    }
}

fn save_settings_values(
    connection: &Connection,
    settings: &AppSettings,
    now: &str,
) -> Result<(), String> {
    let pairs = [
        ("provider", json!(settings.provider)),
        ("modelName", json!(settings.model_name)),
        ("autoStart", json!(settings.auto_start)),
        ("soundEnabled", json!(settings.sound_enabled)),
        ("ttsEnabled", json!(settings.tts_enabled)),
        ("volume", json!(settings.volume)),
        ("notificationPrefs", json!(settings.notification_prefs)),
        ("privacyLock", json!(settings.privacy_lock)),
    ];

    for (key, value) in pairs {
        connection
            .execute(
                "INSERT INTO settings(key,value_json,updated_at) VALUES (?1,?2,?3)
                 ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
                params![key, value.to_string(), now],
            )
            .map_err(|error| error.to_string())?;
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
        favorite_slot_ids: Vec::new(),
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

fn stats(connection: &Connection, character_id: &str) -> Result<PetStats, String> {
    let item = connection
        .query_row(
            "SELECT character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,
                    reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at
             FROM pet_stats WHERE character_id=?1",
            params![character_id],
            |row| {
                Ok(PetStats {
                    character_id: row.get(0)?,
                    energy: row.get(1)?,
                    affection: row.get(2)?,
                    today_chat_rounds: row.get(3)?,
                    today_interaction_count: row.get(4)?,
                    today_had_activity: row.get::<_, i64>(5)? == 1,
                    reward_50_triggered: row.get::<_, i64>(6)? == 1,
                    reward_100_triggered: row.get::<_, i64>(7)? == 1,
                    last_chat_at: row.get(8)?,
                    last_daily_settlement_at: row.get(9)?,
                    last_weekly_inventory_clear_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    if let Some(item) = item {
        return Ok(item);
    }

    let now = now_string();
    connection
        .execute(
            "INSERT OR IGNORE INTO pet_stats(character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at)
             VALUES (?1,60,40,0,0,0,0,0,NULL,NULL,NULL,?2)",
            params![character_id, now],
        )
        .map_err(|error| error.to_string())?;

    Ok(PetStats {
        character_id: character_id.to_string(),
        energy: 60.0,
        affection: 40.0,
        today_chat_rounds: 0,
        today_interaction_count: 0,
        today_had_activity: false,
        reward_50_triggered: false,
        reward_100_triggered: false,
        last_chat_at: None,
        last_daily_settlement_at: None,
        last_weekly_inventory_clear_at: None,
        updated_at: now,
    })
}

fn foods(connection: &Connection, character_id: &str) -> Result<Vec<FoodItem>, String> {
    let mut statement = connection
        .prepare(
            "SELECT f.id,f.character_id,f.slot_id,f.food_level,f.display_order,f.name,f.icon_path,f.energy_delta,f.affection_delta,f.category,f.rarity,f.description,
                    f.enabled,COALESCE(i.count,0),f.created_at,f.updated_at
             FROM foods f LEFT JOIN food_inventory i ON i.food_id=f.id AND i.character_id=f.character_id
             WHERE f.character_id=?1
             ORDER BY f.display_order, f.slot_id",
        )
        .map_err(|error| error.to_string())?;
    let items = statement
        .query_map(params![character_id], |row| {
            Ok(FoodItem {
                id: row.get(0)?,
                character_id: row.get(1)?,
                slot_id: row.get(2)?,
                food_level: row.get(3)?,
                display_order: row.get(4)?,
                name: row.get(5)?,
                icon_path: row.get(6)?,
                energy_delta: row.get(7)?,
                affection_delta: row.get(8)?,
                category: row.get(9)?,
                rarity: row.get(10)?,
                description: row.get(11)?,
                enabled: row.get::<_, i64>(12)? == 1,
                count: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<FoodItem>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

fn settings(connection: &Connection) -> Result<AppSettings, String> {
    let mut defaults = default_settings();
    let mut statement = connection
        .prepare("SELECT key,value_json FROM settings")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;

    for row in rows {
        let (key, value_json) = row.map_err(|error| error.to_string())?;
        let value = serde_json::from_str::<Value>(&value_json).unwrap_or(Value::Null);
        match key.as_str() {
            "provider" => {
                defaults.provider = value.as_str().unwrap_or("openai_compatible").to_string()
            }
            "modelName" => defaults.model_name = value.as_str().unwrap_or("gpt-4o").to_string(),
            "autoStart" => defaults.auto_start = value.as_bool().unwrap_or(false),
            "soundEnabled" => defaults.sound_enabled = value.as_bool().unwrap_or(true),
            "ttsEnabled" => defaults.tts_enabled = value.as_bool().unwrap_or(false),
            "volume" => defaults.volume = value.as_i64().unwrap_or(50),
            "privacyLock" => defaults.privacy_lock = value.as_bool().unwrap_or(false),
            "notificationPrefs" => {
                defaults.notification_prefs = serde_json::from_value::<NotificationPrefs>(value)
                    .unwrap_or(defaults.notification_prefs)
            }
            _ => {}
        }
    }
    Ok(defaults)
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
        stats: stats(connection, &character.id)?,
        foods: foods(connection, &character.id)?,
        settings: settings(connection)?,
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
    transaction
        .execute(
            "INSERT INTO pet_stats(character_id,energy,affection,today_chat_rounds,today_interaction_count,today_had_activity,reward_50_triggered,reward_100_triggered,last_chat_at,last_daily_settlement_at,last_weekly_inventory_clear_at,updated_at)
             VALUES (?1,60,40,0,0,0,0,0,NULL,NULL,NULL,?2)",
            params![character_id, now],
        )
        .map_err(|error| error.to_string())?;
    seed_foods_for_character(&transaction, &character_id, &now)?;
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

pub fn delete_chat_session(app: &AppHandle, session_id: &str) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let deleted = connection
        .execute(
            "DELETE FROM chat_sessions WHERE id=?1 AND character_id=?2",
            params![session_id, character.id],
        )
        .map_err(|error| error.to_string())?;
    if deleted == 0 {
        return Err("聊天记录不存在。".to_string());
    }
    fetch_bootstrap(app, &connection)
}

fn ensure_daily_activity(
    connection: &Connection,
    character_id: &str,
    date: &str,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT OR IGNORE INTO daily_activity(date,character_id,chat_round_count,interaction_count,food_generated_count,affection_delta,energy_delta,settled_at)
             VALUES (?1,?2,0,0,0,0,0,NULL)",
            params![date, character_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn settle_one_day(
    connection: &Connection,
    character_id: &str,
    settlement_at: DateTime<Local>,
) -> Result<(), String> {
    let current = stats(connection, character_id)?;
    let mut affection = current.affection;
    let active_delta = if affection <= 50.0 {
        if current.today_had_activity {
            2.0
        } else {
            -5.0
        }
    } else if current.today_had_activity {
        1.0
    } else {
        -10.0
    };
    affection = clamp(affection + active_delta, 0.0, 100.0);
    let energy_delta = if current.energy < 30.0 {
        -2.0
    } else if current.energy > 50.0 {
        2.0
    } else {
        0.0
    };
    affection = clamp(affection + energy_delta, 0.0, 100.0);

    let settlement = settlement_at.to_rfc3339();
    let date = business_date_for(settlement_at - Duration::seconds(1));
    ensure_daily_activity(connection, character_id, &date)?;
    connection
        .execute(
            "UPDATE daily_activity
             SET affection_delta=affection_delta+?1, energy_delta=energy_delta+?2, settled_at=?3
             WHERE date=?4 AND character_id=?5",
            params![
                active_delta + energy_delta,
                ENERGY_DEFAULT - current.energy,
                settlement,
                date,
                character_id
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE pet_stats
             SET affection=?1,
                 energy=?2,
                 today_chat_rounds=0,
                 today_interaction_count=0,
                 today_had_activity=0,
                 reward_50_triggered=0,
                 reward_100_triggered=0,
                 last_daily_settlement_at=?3,
                 updated_at=?3
             WHERE character_id=?4",
            params![affection, ENERGY_DEFAULT, settlement, character_id],
        )
        .map_err(|error| error.to_string())?;

    if settlement_at.weekday() == Weekday::Mon {
        connection
            .execute(
                "UPDATE food_inventory SET count=0, updated_at=?1 WHERE character_id=?2",
                params![settlement, character_id],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE pet_stats SET last_weekly_inventory_clear_at=?1, updated_at=?1 WHERE character_id=?2",
                params![settlement, character_id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn run_due_settlements(connection: &Connection) -> Result<(), String> {
    let now = Local::now();
    let mut statement = connection
        .prepare("SELECT character_id,last_daily_settlement_at,updated_at FROM pet_stats")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    for (character_id, last_settlement_at, updated_at) in rows {
        let anchor_text = last_settlement_at.unwrap_or(updated_at);
        let anchor = DateTime::parse_from_rfc3339(&anchor_text)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|_| now);
        let mut next = next_settlement_after(anchor);
        while next <= now {
            settle_one_day(connection, &character_id, next)?;
            next += Duration::days(1);
        }
    }
    Ok(())
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

pub fn send_chat_message(
    app: &AppHandle,
    content: String,
    session_id: Option<String>,
) -> Result<ChatSendResult, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let session = get_or_create_session(&connection, &character.id, session_id)?;
    let now = now_string();
    let date = local_date_key();
    ensure_daily_activity(&connection, &character.id, &date)?;

    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        character_id: character.id.clone(),
        role: "user".to_string(),
        content: content.clone(),
        created_at: now.clone(),
        token_count: content.chars().count() as i64,
        metadata_json: None,
    };
    connection
        .execute(
            "INSERT INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)",
            params![
                user_message.id,
                user_message.session_id,
                user_message.character_id,
                user_message.role,
                user_message.content,
                user_message.created_at,
                user_message.token_count
            ],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "UPDATE pet_stats SET today_chat_rounds=today_chat_rounds+1,last_chat_at=?2,today_had_activity=1,energy=MAX(0,energy-1),updated_at=?2 WHERE character_id=?3",
            params![date, now, character.id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE daily_activity SET chat_round_count=chat_round_count+1 WHERE date=?1 AND character_id=?2",
            params![date, character.id],
        )
        .map_err(|error| error.to_string())?;
    let chat_rounds = stats(&connection, &character.id)?.today_chat_rounds;
    maybe_drop_food(&connection, &character.id, &date, chat_rounds, 3, &now)?;

    let reply = format!("I heard you: {content}. This is a local mock reply.");
    let assistant_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        character_id: character.id.clone(),
        role: "assistant".to_string(),
        content: reply,
        created_at: now.clone(),
        token_count: 0,
        metadata_json: Some(json!({ "mock": true }).to_string()),
    };
    connection
        .execute(
            "INSERT INTO chat_messages(id,session_id,character_id,role,content,created_at,token_count,metadata_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                assistant_message.id,
                assistant_message.session_id,
                assistant_message.character_id,
                assistant_message.role,
                assistant_message.content,
                assistant_message.created_at,
                assistant_message.token_count,
                assistant_message.metadata_json
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE chat_sessions SET updated_at=?1 WHERE id=?2",
            params![now, session.id],
        )
        .map_err(|error| error.to_string())?;

    Ok(ChatSendResult {
        session,
        user_message,
        assistant_message,
        stats: stats(&connection, &character.id)?,
        foods: foods(&connection, &character.id)?,
        triggered_state_key: "thinking".to_string(),
    })
}

fn maybe_drop_food(
    connection: &Connection,
    character_id: &str,
    date: &str,
    source_count: i64,
    interval: i64,
    now: &str,
) -> Result<(), String> {
    if source_count <= 0 || source_count % interval != 0 {
        return Ok(());
    }
    if !rand::thread_rng().gen_bool(FOOD_DROP_CHANCE) {
        return Ok(());
    }

    let generated: i64 = connection
        .query_row(
            "SELECT food_generated_count FROM daily_activity WHERE date=?1 AND character_id=?2",
            params![date, character_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let roll = rand::thread_rng().gen_range(0..100);
    let level = match roll {
        0..=24 => 0,
        25..=54 => 1,
        55..=89 => 2,
        _ => 3,
    };
    let pool = foods(connection, character_id)?
        .into_iter()
        .filter(|food| food.food_level == level)
        .collect::<Vec<_>>();
    if let Some(food) = pool.choose(&mut rand::thread_rng()) {
        connection
            .execute(
                "UPDATE food_inventory SET count=count+1,updated_at=?1 WHERE character_id=?2 AND food_id=?3",
                params![now, character_id, food.id],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE daily_activity SET food_generated_count=food_generated_count+1 WHERE date=?1 AND character_id=?2",
                params![date, character_id],
            )
            .map_err(|error| error.to_string())?;
    }
    let _ = generated;
    Ok(())
}

pub fn feed_food(app: &AppHandle, food_id: String) -> Result<FeedResult, String> {
    let mut connection = open_connection(app)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let character = active_character(&transaction)?;
    let current_stats = stats(&transaction, &character.id)?;
    let food = foods(&transaction, &character.id)?
        .into_iter()
        .find(|item| item.id == food_id)
        .ok_or_else(|| "Food does not exist.".to_string())?;

    if food.count <= 0 || !food.enabled {
        return Ok(FeedResult {
            food,
            stats: current_stats,
            triggered_state_key: "feed_blocked".to_string(),
            message: "No inventory for this food.".to_string(),
        });
    }

    let (energy_delta, affection_delta) = food_effect(food.food_level);
    let next_energy = clamp(current_stats.energy + energy_delta, 0.0, 100.0);
    let next_affection = clamp(current_stats.affection + affection_delta, 0.0, 100.0);
    let now = now_string();

    let affected_rows = transaction
        .execute(
            "UPDATE food_inventory SET count=count-1,updated_at=?1 WHERE character_id=?2 AND food_id=?3 AND count>0",
            params![now, character.id, food.id],
        )
        .map_err(|error| error.to_string())?;
    if affected_rows != 1 {
        let updated_food = foods(&transaction, &character.id)?
            .into_iter()
            .find(|item| item.id == food.id)
            .ok_or_else(|| "Failed to read food after feeding.".to_string())?;
        return Ok(FeedResult {
            food: updated_food,
            stats: stats(&transaction, &character.id)?,
            triggered_state_key: "feed_blocked".to_string(),
            message: "No inventory for this food.".to_string(),
        });
    }

    transaction
        .execute(
            "UPDATE pet_stats SET energy=?1,affection=?2,updated_at=?3 WHERE character_id=?4",
            params![next_energy, next_affection, now, character.id],
        )
        .map_err(|error| error.to_string())?;

    let updated_food = foods(&transaction, &character.id)?
        .into_iter()
        .find(|item| item.id == food.id)
        .ok_or_else(|| "Failed to read food after feeding.".to_string())?;
    let updated_stats = stats(&transaction, &character.id)?;
    transaction.commit().map_err(|error| error.to_string())?;
    let triggered_state_key = "feed_neutral";
    Ok(FeedResult {
        food: updated_food,
        stats: updated_stats,
        triggered_state_key: triggered_state_key.to_string(),
        message: if food.food_level == 0 {
            "Rick ate it, restored energy, and lost a little affection."
        } else {
            "Rick received the food."
        }
        .to_string(),
    })
}

pub fn save_settings(app: &AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let connection = open_connection(app)?;
    let now = now_string();
    save_settings_values(&connection, &settings, &now)?;
    Ok(settings)
}

pub fn reset_stats(app: &AppHandle) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let now = now_string();
    connection
        .execute(
            "UPDATE pet_stats
             SET energy=60, affection=40, updated_at=?1
             WHERE character_id=?2",
            params![now, character.id],
        )
        .map_err(|error| error.to_string())?;
    fetch_bootstrap(app, &connection)
}

pub fn replace_food(app: &AppHandle, request: FoodReplaceRequest) -> Result<FoodItem, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let trimmed_name = request.name.trim();
    if trimmed_name.is_empty() || trimmed_name.chars().count() > 20 {
        return Err("Food name must be 1-20 characters.".to_string());
    }
    if !(request.icon_data_url.starts_with("data:image/png;")
        || request.icon_data_url.starts_with("data:image/svg+xml;"))
    {
        return Err("Food icon must be PNG or SVG.".to_string());
    }
    let existing = foods(&connection, &character.id)?
        .into_iter()
        .find(|food| food.id == request.food_id)
        .ok_or_else(|| "椋熺墿妲戒笉瀛樺湪".to_string())?;
    let now = now_string();
    connection
        .execute(
            "UPDATE foods SET name=?1, icon_path=?2, updated_at=?3 WHERE id=?4 AND character_id=?5",
            params![
                trimmed_name,
                request.icon_data_url,
                now,
                request.food_id,
                character.id
            ],
        )
        .map_err(|error| error.to_string())?;
    foods(&connection, &character.id)?
        .into_iter()
        .find(|food| food.id == existing.id)
        .ok_or_else(|| "璇诲彇鏇挎崲鍚庣殑椋熺墿澶辫触".to_string())
}

pub fn reorder_foods(app: &AppHandle, food_ids: Vec<String>) -> Result<Vec<FoodItem>, String> {
    if food_ids.len() != 9 {
        return Err("Food order must include all 9 slots.".to_string());
    }
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let current = foods(&connection, &character.id)?;
    let current_ids = current
        .iter()
        .map(|food| food.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let requested_ids = food_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    if current_ids != requested_ids {
        return Err("Food order cannot add or remove slots.".to_string());
    }
    let now = now_string();
    for (index, id) in food_ids.iter().enumerate() {
        connection
            .execute(
                "UPDATE foods SET display_order=?1, updated_at=?2 WHERE id=?3 AND character_id=?4",
                params![(index + 1) as i64, now, id, character.id],
            )
            .map_err(|error| error.to_string())?;
    }
    foods(&connection, &character.id)
}

pub fn record_pet_interaction(app: &AppHandle) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let character = active_character(&connection)?;
    let now = now_string();
    let date = local_date_key();
    ensure_daily_activity(&connection, &character.id, &date)?;
    connection
        .execute(
            "UPDATE pet_stats
             SET today_interaction_count=today_interaction_count+1,
                 today_had_activity=1,
                 updated_at=?1
             WHERE character_id=?2",
            params![now, character.id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE daily_activity SET interaction_count=interaction_count+1 WHERE date=?1 AND character_id=?2",
            params![date, character.id],
        )
        .map_err(|error| error.to_string())?;
    let interactions = stats(&connection, &character.id)?.today_interaction_count;
    maybe_drop_food(&connection, &character.id, &date, interactions, 2, &now)?;
    fetch_bootstrap(app, &connection)
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

pub fn get_ai_config_public(app: &AppHandle) -> Result<Vec<AiProviderConfig>, String> {
    let connection = open_connection(app)?;
    let active_provider_id = get_active_ai_provider_id(app)?;
    let mut statement = connection
        .prepare(
            "SELECT c.provider_id, c.provider_type, c.display_name, c.base_url, c.model,
                    c.enabled, c.stream, c.temperature, c.max_output_tokens, c.timeout_ms, c.updated_at,
                    CASE WHEN s.provider_id IS NOT NULL THEN 1 ELSE 0 END as has_key
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
                is_active: active_provider_id.as_deref() == Some(row.get::<_, String>(0)?.as_str()),
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<AiProviderConfig>>>()
        .map_err(|error| error.to_string())?;
    Ok(items)
}

pub fn save_ai_provider_config(
    app: &AppHandle,
    req: SaveAiProviderRequest,
) -> Result<AiProviderConfig, String> {
    let connection = open_connection(app)?;
    let now = now_epoch();

    connection
        .execute(
            "INSERT INTO ai_provider_configs(provider_id, provider_type, display_name, base_url, model, enabled, stream, temperature, max_output_tokens, timeout_ms, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(provider_id) DO UPDATE SET
               provider_type=excluded.provider_type,
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

    let is_active = get_active_ai_provider_id(app)?.as_deref() == Some(req.provider_id.as_str());

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
        is_active,
        updated_at: now,
    })
}

pub fn delete_ai_provider_api_key(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_connection(app)?;
    connection
        .execute(
            "DELETE FROM ai_provider_secrets WHERE provider_id=?1",
            params![provider_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn delete_ai_provider_config(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_connection(app)?;
    connection
        .execute(
            "DELETE FROM ai_provider_configs WHERE provider_id=?1",
            params![provider_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM ai_provider_secrets WHERE provider_id=?1",
            params![provider_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn set_active_ai_provider(app: &AppHandle, provider_id: String) -> Result<(), String> {
    let connection = open_connection(app)?;
    let now = now_string();
    connection
        .execute(
            "INSERT INTO settings(key, value_json, updated_at) VALUES ('activeAiProvider', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![json!(provider_id).to_string(), now],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn get_active_ai_provider_id(app: &AppHandle) -> Result<Option<String>, String> {
    let connection = open_connection(app)?;
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
                        is_active: true,
                    },
                    api_key: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    Ok(config)
}

pub fn save_assistant_message(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    content: &str,
) -> Result<ChatMessage, String> {
    let connection = open_connection(app)?;
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
    let date = local_date_key();
    ensure_daily_activity(&connection, character_id, &date)?;

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

fn apply_hidden_rewards(
    connection: &Connection,
    character_id: &str,
    now: &str,
) -> Result<(), String> {
    let current = stats(connection, character_id)?;
    if current.today_chat_rounds >= 100 && !current.reward_100_triggered {
        connection
            .execute(
                "UPDATE pet_stats
                 SET energy=100,
                     affection=MIN(100, affection+10),
                     reward_100_triggered=1,
                     updated_at=?1
                 WHERE character_id=?2",
                params![now, character_id],
            )
            .map_err(|error| error.to_string())?;
    } else if current.today_chat_rounds >= 50
        && !current.reward_50_triggered
        && !current.reward_100_triggered
    {
        connection
            .execute(
                "UPDATE pet_stats SET energy=100, reward_50_triggered=1, updated_at=?1 WHERE character_id=?2",
                params![now, character_id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn record_completed_chat_round_with_connection(
    connection: &Connection,
    character_id: &str,
    date: &str,
    now: &str,
) -> Result<(), String> {
    ensure_daily_activity(connection, character_id, date)?;
    connection
        .execute(
            "UPDATE pet_stats
             SET today_chat_rounds=today_chat_rounds+1,
                 today_had_activity=1,
                 energy=MAX(0, energy-1),
                 last_chat_at=?1,
                 updated_at=?1
             WHERE character_id=?2",
            params![now, character_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE daily_activity SET chat_round_count=chat_round_count+1 WHERE date=?1 AND character_id=?2",
            params![date, character_id],
        )
        .map_err(|error| error.to_string())?;
    let rounds = stats(connection, character_id)?.today_chat_rounds;
    maybe_drop_food(connection, character_id, date, rounds, 3, now)?;
    apply_hidden_rewards(connection, character_id, now)?;
    Ok(())
}

pub fn record_completed_chat_round(
    app: &AppHandle,
    character_id: &str,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
    let now = now_string();
    let date = local_date_key();
    record_completed_chat_round_with_connection(&connection, character_id, &date, &now)?;
    fetch_bootstrap(app, &connection)
}

pub fn save_interrupted_ai_message(
    app: &AppHandle,
    session_id: &str,
    character_id: &str,
    content: &str,
) -> Result<BootstrapPayload, String> {
    let connection = open_connection(app)?;
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
    let date = local_date_key();
    record_completed_chat_round_with_connection(&connection, character_id, &date, &now)?;
    fetch_bootstrap(app, &connection)
}

/// Get chat history for a session (for building AI context)
pub fn get_chat_history(
    app: &AppHandle,
    session_id: &str,
    limit: usize,
) -> Result<Vec<ChatMessage>, String> {
    let connection = open_connection(app)?;
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
                        is_active: active_provider_id.as_deref()
                            == Some(row.get::<_, String>(0)?.as_str()),
                    },
                    api_key: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(result)
}
