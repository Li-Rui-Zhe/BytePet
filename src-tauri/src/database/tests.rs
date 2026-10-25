use super::*;

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    connection
}

fn request(id: &str, url: &str) -> SaveAiProviderRequest {
    SaveAiProviderRequest {
        provider_id: id.into(),
        provider_type: crate::ai::AI_PROTOCOL.into(),
        display_name: "test".into(),
        base_url: url.into(),
        model: if crate::ai::is_deepseek_connection(id) {
            crate::ai::DEEPSEEK_MODEL.into()
        } else {
            "test-model".into()
        },
        api_key: Some("test-only-key".into()),
        stream: None,
        temperature: None,
        max_output_tokens: None,
        timeout_ms: None,
    }
}

fn legacy(connection: &Connection, id: &str, protocol: &str, url: &str) {
    connection.execute(
        "INSERT INTO ai_provider_configs VALUES (?1,?2,'legacy',?3,'legacy-model',1,1,0.8,1200,60000,0)",
        params![id,protocol,url],
    ).unwrap();
    connection
        .execute(
            "INSERT INTO ai_provider_secrets VALUES (?1,'test-only-legacy-key',0)",
            params![id],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO settings VALUES ('activeAiProvider',?1,'test')",
            params![json!(id).to_string()],
        )
        .unwrap();
}

fn active(connection: &Connection) -> String {
    connection
        .query_row(
            "SELECT value_json FROM settings WHERE key='activeAiProvider'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn fresh_database_uses_deepseek_without_creating_care_tables() {
    let connection = database();
    migrate_ai_connections(&connection).unwrap();
    assert_eq!(active(&connection), "\"deepseek\"");
    let removed: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('pet_stats','foods','food_inventory','daily_activity')", [], |row| row.get(0)).unwrap();
    assert_eq!(removed, 0);
    let model: String = connection
        .query_row(
            "SELECT model FROM ai_provider_configs WHERE provider_id='deepseek'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(model, crate::ai::DEEPSEEK_MODEL);
}

#[test]
fn migration_preserves_legacy_care_data_without_settling_it() {
    let connection = database();
    connection.execute_batch("CREATE TABLE pet_stats(energy REAL,affection REAL); INSERT INTO pet_stats VALUES (17,23);").unwrap();
    migrate(&connection).unwrap();
    migrate_ai_connections(&connection).unwrap();
    let values: (f64, f64) = connection
        .query_row("SELECT energy,affection FROM pet_stats", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(values, (17.0, 23.0));
}

#[test]
fn active_compatible_config_and_key_migrate_together_and_only_once() {
    let connection = database();
    legacy(
        &connection,
        "old-provider",
        crate::ai::AI_PROTOCOL,
        "https://example.com/v1",
    );
    migrate_ai_connections(&connection).unwrap();
    assert_eq!(active(&connection), "\"custom\"");
    let migrated: (String,String) = connection.query_row("SELECT c.base_url,s.api_key FROM ai_provider_configs c JOIN ai_provider_secrets s ON s.provider_id=c.provider_id WHERE c.provider_id='custom'", [], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
    assert_eq!(
        migrated,
        (
            "https://example.com/v1".into(),
            "test-only-legacy-key".into()
        )
    );
    connection
        .execute(
            "UPDATE settings SET value_json='\"deepseek\"' WHERE key='activeAiProvider'",
            [],
        )
        .unwrap();
    migrate_ai_connections(&connection).unwrap();
    assert_eq!(active(&connection), "\"deepseek\"");
}

#[test]
fn an_existing_custom_slot_never_gets_a_different_providers_key() {
    let connection = database();
    let mut custom = request("custom", "https://custom.example.com/v1");
    custom.api_key = None;
    save_connection(&connection, custom, false).unwrap();
    legacy(
        &connection,
        "old-provider",
        crate::ai::AI_PROTOCOL,
        "https://old.example.com/v1",
    );
    migrate_ai_connections(&connection).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id='custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn native_configs_are_retained_but_default_connection_is_deepseek() {
    let connection = database();
    legacy(
        &connection,
        "gemini",
        "gemini-native",
        "https://example.com",
    );
    migrate_ai_connections(&connection).unwrap();
    assert_eq!(active(&connection), "\"deepseek\"");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id='gemini'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn a_deepseek_proxy_key_is_archived_instead_of_sent_to_official_service() {
    let connection = database();
    legacy(
        &connection,
        "deepseek",
        crate::ai::AI_PROTOCOL,
        "https://proxy.example.com/v1",
    );
    migrate_ai_connections(&connection).unwrap();
    assert_eq!(active(&connection), "\"custom\"");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id='deepseek'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    let archived: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id LIKE 'legacy-deepseek-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(archived, 1);
}

#[test]
fn saving_a_model_change_keeps_key_but_host_change_requires_reentry() {
    let connection = database();
    save_connection(
        &connection,
        request("custom", "https://example.com/v1"),
        false,
    )
    .unwrap();
    let mut update = request("custom", "https://example.com/v1");
    update.api_key = None;
    update.model = "new-model".into();
    assert!(
        save_connection(&connection, update.clone(), false)
            .unwrap()
            .has_api_key
    );
    update.base_url = "https://other.example.com/v1".into();
    assert!(save_connection(&connection, update, false)
        .unwrap_err()
        .contains("重新填写"));
    let unchanged: String = connection
        .query_row(
            "SELECT base_url FROM ai_provider_configs WHERE provider_id='custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unchanged, "https://example.com/v1");
}

#[test]
fn connection_validation_blocks_native_protocols_and_accepts_local_without_key() {
    let connection = database();
    let mut native = request("custom", "https://example.com");
    native.provider_type = "gemini-native".into();
    assert!(save_connection(&connection, native, false).is_err());
    let mut local = request("custom", "http://localhost:11434/v1");
    local.api_key = None;
    assert!(
        !save_connection(&connection, local, false)
            .unwrap()
            .has_api_key
    );
    let mut deepseek = request("deepseek", "https://wrong.example.com");
    deepseek.api_key = None;
    assert!(save_connection(&connection, deepseek, false)
        .unwrap_err()
        .contains("API Key"));
}

#[test]
fn draft_preview_changes_nothing_and_blocks_old_key_on_different_origin() {
    let connection = database();
    let saved = save_connection(
        &connection,
        request("custom", "https://example.com/v1"),
        false,
    )
    .unwrap();
    let full = AiProviderFullConfig {
        config: saved,
        api_key: "test-only-key".into(),
    };
    let mut draft = request("custom", "https://example.com/v1");
    draft.api_key = None;
    draft.model = "unsaved-model".into();
    let preview = resolve_preview(draft.clone(), Some(full.clone())).unwrap();
    assert_eq!(preview.config.model, "unsaved-model");
    let model: String = connection
        .query_row(
            "SELECT model FROM ai_provider_configs WHERE provider_id='custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(model, "test-model");
    draft.base_url = "https://other.example.com/v1".into();
    assert!(resolve_preview(draft, Some(full)).is_err());
}

#[test]
fn sessions_reject_a_different_character() {
    let connection = database();
    connection.execute("INSERT INTO characters(id,name,skin_id,prompt,description,personality_tags,opening_line,created_at,updated_at) VALUES ('a','A','rick_default','','','[]','','now','now')", []).unwrap();
    get_or_create_session(&connection, "a", Some("session-a".into())).unwrap();
    assert!(ensure_session_owner(&connection, "session-a", "a").is_ok());
    assert!(ensure_session_owner(&connection, "session-a", "b").is_err());
    assert!(ensure_session_owner(&connection, "missing", "a").is_err());
}

#[test]
fn numeric_request_limits_are_validated() {
    let mut draft = request("custom", "http://localhost:11434/v1");
    draft.max_output_tokens = Some(-1);
    assert!(normalize_connection_request(draft.clone()).is_err());
    draft.max_output_tokens = Some(100);
    draft.temperature = Some(f64::NAN);
    assert!(normalize_connection_request(draft.clone()).is_err());
    draft.temperature = Some(0.8);
    draft.timeout_ms = Some(-1);
    assert!(normalize_connection_request(draft).is_err());
}

#[test]
fn deepseek_model_selection_is_saved_and_reuses_the_same_key() {
    let connection = database();
    migrate_ai_connections(&connection).unwrap();
    save_connection(
        &connection,
        request("deepseek", crate::ai::DEEPSEEK_BASE_URL),
        true,
    )
    .unwrap();
    let mut update = request("deepseek", "https://wrong.example.com");
    update.model = "deepseek-v4-pro".into();
    update.api_key = None;
    let saved = save_connection(&connection, update.clone(), true).unwrap();
    assert_eq!(saved.model, "deepseek-v4-pro");
    assert_eq!(saved.base_url, crate::ai::DEEPSEEK_BASE_URL);
    assert!(saved.has_api_key);
    let key: String = connection
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id='deepseek'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(key, "test-only-key");
    migrate_ai_connections(&connection).unwrap();
    let model: String = connection
        .query_row(
            "SELECT model FROM ai_provider_configs WHERE provider_id='deepseek'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(model, "deepseek-v4-pro");
    let full = AiProviderFullConfig {
        config: saved,
        api_key: key,
    };
    update.model = crate::ai::DEEPSEEK_MODEL.into();
    assert_eq!(
        resolve_preview(update, Some(full)).unwrap().config.model,
        crate::ai::DEEPSEEK_MODEL
    );
}

#[test]
fn unsupported_models_and_invalid_profile_names_are_rejected() {
    let mut invalid = request("deepseek", crate::ai::DEEPSEEK_BASE_URL);
    invalid.model = "arbitrary-model".into();
    assert!(normalize_connection_request(invalid)
        .unwrap_err()
        .contains("DeepSeek 模型"));
    let mut custom = request("custom", "https://example.com/v1");
    assert_eq!(
        normalize_connection_request(custom.clone()).unwrap().model,
        "test-model"
    );
    custom.display_name = " ".into();
    assert!(normalize_connection_request(custom.clone()).is_err());
    custom.display_name = "bad\nname".into();
    assert!(normalize_connection_request(custom).is_err());
    assert!(!crate::ai::is_managed_connection(
        "deepseek-profile-invalid"
    ));
    assert!(!crate::ai::is_managed_connection("legacy-deepseek-test"));
}

#[test]
fn multiple_deepseek_profiles_keep_separate_keys_and_only_one_active() {
    let connection = database();
    migrate_ai_connections(&connection).unwrap();
    let first = format!("deepseek-profile-{}", Uuid::new_v4());
    let second = format!("deepseek-profile-{}", Uuid::new_v4());
    for (id, key) in [
        (&first, "test-only-personal-key-1234"),
        (&second, "test-only-backup-key-5678"),
    ] {
        let mut config = request(id, "https://wrong.example.com");
        config.display_name = if id == &first {
            "日常密钥".into()
        } else {
            "备用密钥".into()
        };
        config.api_key = Some(crate::credentials::store(key).unwrap());
        save_connection(&connection, config, false).unwrap();
    }
    activate_connection(&connection, &first).unwrap();
    assert_eq!(
        active_connection_id(&connection).unwrap().as_deref(),
        Some(first.as_str())
    );
    activate_connection(&connection, &second).unwrap();
    let public = public_ai_configs(&connection, Some(&second)).unwrap();
    assert_eq!(public.iter().filter(|item| item.is_active).count(), 1);
    assert_eq!(
        public
            .iter()
            .find(|item| item.provider_id == first)
            .unwrap()
            .api_key_hint
            .as_deref(),
        Some("•••• 1234")
    );
    let serialized = serde_json::to_string(&public).unwrap();
    assert!(!serialized.contains("test-only-personal-key"));
    assert!(!serialized.contains("@keyring:v1:"));
    assert!(delete_connection(&connection, &second)
        .unwrap_err()
        .contains("正在启用"));
    let reference = delete_connection(&connection, &first).unwrap().unwrap();
    cleanup_unreferenced_credential(&connection, &reference);
    assert!(crate::credentials::load(&reference).is_err());
    assert_eq!(
        active_connection_id(&connection).unwrap().as_deref(),
        Some(second.as_str())
    );
    let remaining: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_provider_secrets WHERE provider_id=?1",
            [&second],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 1);
    let reference: String = connection
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id=?1",
            [&second],
            |row| row.get(0),
        )
        .unwrap();
    crate::credentials::delete(&reference).unwrap();
}

#[test]
fn new_profile_never_inherits_another_profiles_key() {
    let connection = database();
    save_connection(
        &connection,
        request("deepseek", crate::ai::DEEPSEEK_BASE_URL),
        true,
    )
    .unwrap();
    let id = format!("deepseek-profile-{}", Uuid::new_v4());
    let mut empty = request(&id, crate::ai::DEEPSEEK_BASE_URL);
    empty.api_key = None;
    assert!(save_connection(&connection, empty.clone(), false)
        .unwrap_err()
        .contains("API Key"));
    assert!(resolve_preview(empty, None)
        .unwrap_err()
        .contains("API Key"));
    assert!(activate_connection(&connection, &id).is_err());
    assert_eq!(credential_hint("short"), Some("••••（已保存）".into()));
    assert_eq!(credential_hint(""), None);
}

#[test]
fn legacy_plaintext_migrates_once_to_recoverable_system_reference() {
    let connection = database();
    legacy(
        &connection,
        "custom",
        crate::ai::AI_PROTOCOL,
        "https://example.com",
    );
    migrate_api_credentials(&connection).unwrap();
    let reference: String = connection
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id='custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(crate::credentials::is_reference(&reference));
    assert_eq!(
        crate::credentials::load(&reference).unwrap(),
        "test-only-legacy-key"
    );
    migrate_api_credentials(&connection).unwrap();
    let unchanged: String = connection
        .query_row(
            "SELECT api_key FROM ai_provider_secrets WHERE provider_id='custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reference, unchanged);
    crate::credentials::delete(&reference).unwrap();
}
