use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::ai;
use crate::database::{
    self, AiProviderConfig, AppSettings, BootstrapPayload, CreateCharacterRequest, FeedResult,
    FoodItem, FoodReplaceRequest, FrameAssetOption, SaveAiProviderRequest, SkinValidationReport,
    UpdateCharacterRequest,
};
use crate::resources;

static ACTIVE_REQUEST: once_cell::sync::Lazy<Arc<Mutex<Option<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

#[tauri::command]
pub fn bootstrap_app(app: AppHandle) -> Result<BootstrapPayload, String> {
    database::bootstrap(&app)
}

#[tauri::command]
pub fn feed_food(app: AppHandle, food_id: String) -> Result<FeedResult, String> {
    database::feed_food(&app, food_id)
}

#[tauri::command]
pub fn replace_food(app: AppHandle, request: FoodReplaceRequest) -> Result<FoodItem, String> {
    database::replace_food(&app, request)
}

#[tauri::command]
pub fn reorder_foods(app: AppHandle, food_ids: Vec<String>) -> Result<Vec<FoodItem>, String> {
    database::reorder_foods(&app, food_ids)
}

#[tauri::command]
pub fn record_pet_interaction(app: AppHandle) -> Result<BootstrapPayload, String> {
    database::record_pet_interaction(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    database::save_settings(&app, settings)
}

#[tauri::command]
pub fn reset_stats(app: AppHandle) -> Result<BootstrapPayload, String> {
    database::reset_stats(&app)
}

#[tauri::command]
pub fn validate_skin_path(app: AppHandle, path: String) -> Result<SkinValidationReport, String> {
    resources::validate_skin_path(&app, &PathBuf::from(path))
}

#[tauri::command]
pub fn list_frame_assets(app: AppHandle) -> Result<Vec<FrameAssetOption>, String> {
    resources::list_frame_assets(&app)
}

#[tauri::command]
pub fn choose_and_import_frame_asset(app: AppHandle) -> Result<Option<FrameAssetOption>, String> {
    resources::choose_and_import_frame_asset(&app)
}

#[tauri::command]
pub fn import_frame_asset_from_path(
    app: AppHandle,
    path: String,
) -> Result<FrameAssetOption, String> {
    resources::import_frame_asset(&app, &PathBuf::from(path))
}

#[tauri::command]
pub fn delete_frame_asset(
    app: AppHandle,
    asset_id: String,
) -> Result<Vec<FrameAssetOption>, String> {
    resources::delete_frame_asset(&app, &asset_id)
}

#[tauri::command]
pub fn create_character(
    app: AppHandle,
    request: CreateCharacterRequest,
) -> Result<BootstrapPayload, String> {
    database::create_character(&app, request)
}

#[tauri::command]
pub fn update_character(
    app: AppHandle,
    request: UpdateCharacterRequest,
) -> Result<BootstrapPayload, String> {
    database::update_character(&app, request)
}

#[tauri::command]
pub fn update_character_scale(
    app: AppHandle,
    character_id: String,
    display_scale: f64,
) -> Result<BootstrapPayload, String> {
    database::update_character_scale(&app, &character_id, display_scale)
}

#[tauri::command]
pub fn switch_character(app: AppHandle, character_id: String) -> Result<BootstrapPayload, String> {
    database::switch_character(&app, &character_id)
}

#[tauri::command]
pub fn delete_character(app: AppHandle, character_id: String) -> Result<BootstrapPayload, String> {
    database::delete_character(&app, &character_id)
}

#[tauri::command]
pub fn delete_chat_session(app: AppHandle, session_id: String) -> Result<BootstrapPayload, String> {
    database::delete_chat_session(&app, &session_id)
}

#[tauri::command]
pub fn chat_history_days(app: AppHandle) -> Result<Vec<database::ChatHistoryDay>, String> {
    database::chat_history_days(&app)
}

#[tauri::command]
pub fn chat_history_messages_for_day(
    app: AppHandle,
    date_key: String,
) -> Result<Vec<database::ChatMessage>, String> {
    database::chat_history_messages_for_day(&app, &date_key)
}

#[tauri::command]
pub fn delete_chat_history_day(
    app: AppHandle,
    date_key: String,
) -> Result<BootstrapPayload, String> {
    database::delete_chat_history_day(&app, &date_key)
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<String, String> {
    resources::open_data_dir(&app)
}

#[tauri::command]
pub fn get_ai_config_public(app: AppHandle) -> Result<Vec<AiProviderConfig>, String> {
    database::get_ai_config_public(&app)
}

#[tauri::command]
pub fn save_ai_provider_config(
    app: AppHandle,
    config: SaveAiProviderRequest,
) -> Result<AiProviderConfig, String> {
    database::save_ai_provider_config(&app, config)
}

#[tauri::command]
pub fn delete_ai_provider_api_key(app: AppHandle, provider_id: String) -> Result<(), String> {
    database::delete_ai_provider_api_key(&app, provider_id)
}

#[tauri::command]
pub fn delete_ai_provider_config(app: AppHandle, provider_id: String) -> Result<(), String> {
    database::delete_ai_provider_config(&app, provider_id)
}

#[tauri::command]
pub fn set_active_ai_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    database::set_active_ai_provider(&app, provider_id)
}

#[tauri::command]
pub async fn test_ai_connection(app: AppHandle, provider_id: String) -> Result<String, String> {
    ai::test_connection(&app, &provider_id).await
}

#[tauri::command]
pub async fn stream_ai_message(
    app: AppHandle,
    request_id: String,
    session_id: String,
    content: String,
) -> Result<(), String> {
    {
        let mut active = ACTIVE_REQUEST.lock().await;
        *active = Some(request_id.clone());
    }

    let bootstrap = database::bootstrap(&app)?;
    let character = bootstrap.character;
    let session_summary = bootstrap
        .sessions
        .iter()
        .find(|session| session.id == session_id)
        .map(|session| session.summary.clone())
        .unwrap_or_default();

    database::save_user_message(&app, &session_id, &character.id, &content)?;

    let history = database::get_chat_history(&app, &session_id, 20)?;
    let mut messages: Vec<ai::AiChatMessage> = history
        .iter()
        .map(|m| ai::AiChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    messages.insert(
        0,
        ai::AiChatMessage {
            role: "system".to_string(),
            content: format!(
                "你是桌宠应用里的当前人物。请始终保持当前人物的人格，不要混入其他人物的记忆。\n\n人物名字：{}\n\n人物 prompt：\n{}\n\n人物记忆：\n{}\n\n当前会话摘要：\n{}\n\n回复要求：简短、自然、友好，优先延续当前会话。",
                character.name,
                if character.prompt.trim().is_empty() {
                    &character.description
                } else {
                    &character.prompt
                },
                if character.memory_summary.trim().is_empty() {
                    "暂无"
                } else {
                    &character.memory_summary
                },
                if session_summary.trim().is_empty() {
                    "暂无"
                } else {
                    &session_summary
                }
            ),
        },
    );
    messages.insert(
        1,
        ai::AiChatMessage {
            role: "system".to_string(),
            content: "回复用户时，请正常输出自然语言正文。\n在回复最后单独追加一行动作标签。\n动作标签格式必须是：[[PET_ACTION:动作名]]\n动作名必须只填一个，且只能是：calm、happy、sad、angry、comfort、cheer_up。\n正确示例：[[PET_ACTION:comfort]]\n错误示例：[[PET_ACTION:calm|happy|sad|angry|comfort|cheer_up]]\n\n动作含义：\n\ncalm\n普通、中性、默认待机动作。\n用于普通聊天、普通问答、没有明显情绪、动作判断不明确时。\ncalm 是兜底动作。\n\nhappy\n开心、亲近、被夸奖、轻松玩笑、正向互动。\n用于用户表达喜欢、夸奖桌宠、开心、调侃、撒娇、亲密互动时。\n\ncomfort\n安慰、陪伴、理解、接住用户情绪。\n用于用户表达压力、疲惫、焦虑、难过、低落、想放弃、需要陪伴时。\n如果用户是负面情绪，但更需要被安慰，优先使用 comfort，而不是 sad。\n\nsad\n桌宠自己的失落、委屈、受伤。\n用于用户拒绝桌宠、冷落桌宠、说伤害桌宠的话，或者场景需要桌宠表现出难过时。\n\nangry\n桌宠生气、被冒犯、被攻击、炸毛。\n用于用户攻击桌宠、辱骂桌宠、故意挑衅桌宠，或需要桌宠表现出生气反应时。\n如果用户是在对外部事情生气，不要默认 angry；根据语境选择 comfort 或 angry。\n\ncheer_up\n鼓励、打气、激励、帮用户振作。\n用于用户没动力、不自信、想摆烂、想放弃、需要被鼓励继续行动时。\n如果 AI 回复的重点是“鼓励用户迈出下一步”，优先使用 cheer_up。\n\n不要解释动作标签。不要把动作标签包进代码块。动作标签必须是回复的最后一行。\n不要输出 <tool_call>、</tool_call>、function、arguments、add_character_expression 或任何工具调用格式；这些内容不会被用户看到，也不能作为回复正文。".to_string(),
        },
    );

    let result = ai::stream_ai_chat(
        app.clone(),
        request_id.clone(),
        session_id.clone(),
        messages,
    )
    .await;

    let still_active = {
        let mut active = ACTIVE_REQUEST.lock().await;
        let is_active = active.as_ref() == Some(&request_id);
        if is_active {
            *active = None;
        }
        is_active
    };

    if !still_active {
        return Ok(());
    }

    match result {
        Ok(chat_result) => {
            let persisted = (|| {
                database::save_assistant_message(
                    &app,
                    &session_id,
                    &character.id,
                    &chat_result.reply,
                )?;
                database::record_completed_chat_round(&app, &character.id).map(|_| ())
            })();

            match persisted {
                Ok(()) => {
                    let _ = app.emit(
                        "ai-chat-event",
                        ai::AiChatEvent::Done {
                            request_id,
                            session_id,
                            full_response: chat_result.reply,
                            action: chat_result.action,
                        },
                    );
                    Ok(())
                }
                Err(err) => {
                    let _ = app.emit(
                        "ai-chat-event",
                        ai::AiChatEvent::Error {
                            request_id,
                            session_id,
                            message: err.clone(),
                        },
                    );
                    Err(err)
                }
            }
        }
        Err(err) => {
            let _ = app.emit(
                "ai-chat-event",
                ai::AiChatEvent::Error {
                    request_id,
                    session_id,
                    message: err.clone(),
                },
            );
            Err(err)
        }
    }
}

#[tauri::command]
pub async fn abort_ai_request(app: AppHandle, request_id: String) -> Result<(), String> {
    let mut active = ACTIVE_REQUEST.lock().await;
    if active.as_ref() == Some(&request_id) {
        *active = None;
        let _ = app.emit(
            "ai-chat-event",
            ai::AiChatEvent::Aborted {
                request_id,
                session_id: String::new(),
            },
        );
        Ok(())
    } else {
        Err("Request not found or already completed".to_string())
    }
}

#[tauri::command]
pub async fn save_interrupted_ai_message(
    app: AppHandle,
    session_id: String,
    content: String,
) -> Result<BootstrapPayload, String> {
    database::save_interrupted_ai_message(&app, &session_id, &content)
}
