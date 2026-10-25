pub mod openai_compat;
pub mod requests;

use requests::ChatRequest;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::error::Error;
use tauri::{AppHandle, Emitter};

use crate::database;

pub const AI_PROTOCOL: &str = "openai-compatible";
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
pub const DEEPSEEK_MODEL: &str = "deepseek-flash";

#[derive(Deserialize)]
struct DeepSeekModel {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeepSeekModelCatalog {
    default_model: String,
    models: Vec<DeepSeekModel>,
}

static DEEPSEEK_MODELS: once_cell::sync::Lazy<DeepSeekModelCatalog> =
    once_cell::sync::Lazy::new(|| {
        let catalog: DeepSeekModelCatalog =
            serde_json::from_str(include_str!("../../../src/config/deepseekModels.json"))
                .expect("bundled DeepSeek model catalog must be valid");
        assert_eq!(catalog.default_model, DEEPSEEK_MODEL);
        catalog
    });

pub fn is_deepseek_model(model: &str) -> bool {
    DEEPSEEK_MODELS.models.iter().any(|item| item.id == model)
}

fn is_profile(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|value| {
        uuid::Uuid::parse_str(value)
            .is_ok_and(|uuid| uuid.hyphenated().to_string().eq_ignore_ascii_case(value))
    })
}

pub fn is_deepseek_connection(id: &str) -> bool {
    id == "deepseek" || is_profile(id, "deepseek-profile-")
}

pub fn is_managed_connection(id: &str) -> bool {
    is_deepseek_connection(id) || id == "custom" || is_profile(id, "custom-profile-")
}

/// Provider configuration from database
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub provider_type: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f64,
    pub max_output_tokens: i64,
    pub timeout_ms: i64,
}

/// Chat message for AI requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct AiChatResult {
    pub reply: String,
    pub action: String,
}

/// Events emitted to frontend during streaming
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event")]
pub enum AiChatEvent {
    #[serde(rename = "start")]
    Start {
        request_id: String,
        session_id: String,
        user_message: database::ChatMessage,
    },
    #[serde(rename = "delta")]
    Delta {
        request_id: String,
        session_id: String,
        content: String,
    },
    #[serde(rename = "done")]
    Done {
        request_id: String,
        session_id: String,
        full_response: String,
        action: String,
    },
    #[serde(rename = "error")]
    Error {
        request_id: String,
        session_id: String,
        message: String,
    },
    #[serde(rename = "aborted")]
    Aborted {
        request_id: String,
        session_id: String,
    },
}

const ALLOWED_CHAT_ACTIONS: &[&str] = &["calm", "happy", "sad", "angry", "comfort", "cheer_up"];

fn canonical_chat_action(action: &str) -> Option<&'static str> {
    let normalized = action.trim().to_ascii_lowercase().replace([' ', '-'], "_");

    match normalized.as_str() {
        "calm" | "clam" | "neutral" | "idle" | "normal" => Some("calm"),
        "happy" | "joy" | "joyful" | "pleased" | "excited" => Some("happy"),
        "sad" | "upset" | "down" | "hurt" | "lonely" => Some("sad"),
        "angry" | "mad" | "annoyed" | "irritated" => Some("angry"),
        "comfort" | "comforting" | "support" | "supportive" => Some("comfort"),
        "cheer_up" | "cheer" | "encourage" | "encouraging" | "motivate" | "motivating" => {
            Some("cheer_up")
        }
        _ => None,
    }
}

fn normalize_chat_action(action: Option<&str>) -> String {
    action
        .and_then(canonical_chat_action)
        .filter(|action| ALLOWED_CHAT_ACTIONS.contains(action))
        .unwrap_or("calm")
        .to_string()
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| text.contains(keyword))
}

fn fallback_chat_action(user_text: Option<&str>, reply: &str) -> String {
    let text = format!("{} {}", user_text.unwrap_or_default(), reply).to_lowercase();

    if contains_any(
        &text,
        &[
            "骂你",
            "滚开",
            "闭嘴",
            "讨厌你",
            "废物",
            "垃圾",
            "蠢",
            "挑衅",
            "辱骂",
            "insult",
            "shut up",
        ],
    ) {
        return "angry".to_string();
    }

    if contains_any(
        &text,
        &[
            "没动力",
            "不自信",
            "摆烂",
            "想放弃",
            "坚持不下",
            "做不到",
            "加油",
            "鼓励",
            "振作",
            "拖延",
            "motivate",
            "encourage",
        ],
    ) {
        return "cheer_up".to_string();
    }

    if contains_any(
        &text,
        &[
            "压力", "累", "疲惫", "焦虑", "难过", "低落", "崩溃", "失眠", "害怕", "陪我", "安慰",
            "痛苦", "委屈", "哭", "stress", "tired", "anxious", "sad",
        ],
    ) {
        return "comfort".to_string();
    }

    if contains_any(
        &text,
        &[
            "不要你",
            "不理你",
            "冷落",
            "伤害你",
            "受伤",
            "委屈",
            "失落",
            "rejected",
            "hurt",
        ],
    ) {
        return "sad".to_string();
    }

    if contains_any(
        &text,
        &[
            "喜欢你",
            "爱你",
            "开心",
            "哈哈",
            "可爱",
            "真棒",
            "谢谢",
            "夸",
            "抱抱",
            "亲亲",
            "happy",
            "love",
            "cute",
        ],
    ) {
        return "happy".to_string();
    }

    "calm".to_string()
}

pub fn parse_pet_action_response(raw_response: &str, user_text: Option<&str>) -> AiChatResult {
    const MARKER_START: &str = "[[PET_ACTION:";
    const MARKER_START_LOWER: &str = "[[pet_action:";
    const MARKER_END: &str = "]]";

    let cleaned_response = strip_tool_call_blocks(raw_response);
    let raw_response = cleaned_response.as_str();
    let marker_search_text = raw_response.to_ascii_lowercase();
    let Some(start) = marker_search_text.rfind(MARKER_START_LOWER) else {
        return AiChatResult {
            reply: raw_response.trim().to_string(),
            action: fallback_chat_action(user_text, raw_response),
        };
    };

    let action_start = start + MARKER_START.len();
    let Some(relative_end) = raw_response[action_start..].find(MARKER_END) else {
        return AiChatResult {
            reply: raw_response.trim().to_string(),
            action: fallback_chat_action(user_text, raw_response),
        };
    };

    let end = action_start + relative_end;
    let marker_action = normalize_chat_action(Some(&raw_response[action_start..end]));
    let after_marker = end + MARKER_END.len();
    let reply = format!(
        "{}{}",
        &raw_response[..start],
        &raw_response[after_marker..]
    )
    .trim()
    .to_string();
    let action = if marker_action == "calm"
        && !raw_response[action_start..end]
            .trim()
            .eq_ignore_ascii_case("calm")
    {
        fallback_chat_action(user_text, &reply)
    } else {
        marker_action
    };

    AiChatResult { reply, action }
}

fn strip_tool_call_blocks(input: &str) -> String {
    let mut output = input.to_string();

    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find("<tool_call") else {
            break;
        };

        let Some(relative_end) = lower[start..].find("</tool_call>") else {
            output.truncate(start);
            break;
        };

        let end = start + relative_end + "</tool_call>".len();
        output.replace_range(start..end, "");
    }

    output
}

/// Accept either a Base URL or a complete Chat Completions endpoint.
pub fn build_endpoint(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        normalized.to_string()
    } else {
        format!("{normalized}/chat/completions")
    }
}
/// Keep provider diagnostics useful without dumping huge vendor responses.
pub fn truncate_debug_body(text: &str) -> String {
    const MAX_CHARS: usize = 1200;
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    let snippet: String = chars.by_ref().take(MAX_CHARS).collect();

    if chars.next().is_some() {
        format!("{}... [truncated]", snippet)
    } else {
        snippet
    }
}

/// Debug context that is safe to return to the UI. Never include API keys here.
pub fn provider_debug_context(config: &ProviderConfig, endpoint: &str) -> String {
    format!(
        "provider={}, type={}, model={}, endpoint={}",
        config.provider_id, config.provider_type, config.model, endpoint
    )
}

pub fn build_http_client(_config: &ProviderConfig) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .build()
        .map_err(|error| format!("HTTP client build failed: {}", error))
}

pub fn drain_complete_lines(buffer: &mut Vec<u8>, chunk: &[u8]) -> Result<Vec<String>, String> {
    buffer.extend_from_slice(chunk);
    let mut lines = Vec::new();
    while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
        let bytes: Vec<u8> = buffer.drain(..=index).collect();
        let line = std::str::from_utf8(&bytes)
            .map_err(|_| "服务端返回了无效的 UTF-8 文本。".to_string())?;
        lines.push(line.trim_end_matches(['\r', '\n']).to_string());
    }
    Ok(lines)
}

pub fn drain_remaining_line(buffer: &mut Vec<u8>) -> Result<Option<String>, String> {
    let bytes = std::mem::take(buffer);
    let line =
        String::from_utf8(bytes).map_err(|_| "服务端返回了不完整的 UTF-8 文本。".to_string())?;
    let line = line.trim_matches(['\r', '\n']);
    Ok((!line.trim().is_empty()).then(|| line.to_string()))
}

pub fn interrupted_reply(text: &str) -> String {
    let clean = strip_tool_call_blocks(text);
    let marker = clean.to_ascii_lowercase().find("[[pet_action:");
    clean[..marker.unwrap_or(clean.len())].trim().to_string()
}
pub fn diagnose_request_error(error: &reqwest::Error) -> String {
    let mut hints = Vec::new();

    if error.is_timeout() {
        hints.push("请求超时：检查网络、代理或把超时时间调大");
    }

    if error.is_connect() {
        hints.push("连接失败：通常是域名不可达、代理未生效、TLS 证书或防火墙问题");
    }

    if error.is_builder() {
        hints.push("请求构造失败：检查 Base URL 是否是完整的 http/https 地址");
    }

    if hints.is_empty() {
        hints.push("请求未拿到 HTTP 响应：优先检查网络、系统代理、证书、VPN/防火墙");
    }

    let mut details = Vec::new();
    let mut source = error.source();
    while let Some(err) = source {
        details.push(err.to_string());
        source = err.source();
    }

    if details.is_empty() {
        hints.join("；")
    } else {
        format!("{}；source={}", hints.join("；"), details.join(" <- "))
    }
}

pub fn diagnose_http_error(status: reqwest::StatusCode, body: &str) -> String {
    let body_lower = body.to_ascii_lowercase();
    let status_hint = match status.as_u16() {
        400 => "请求参数错误：检查模型名、消息格式、max_tokens/temperature 等参数",
        401 => "认证失败：API Key 无效、填错平台、复制时多了空格，或不是该厂商的专用 Key",
        403 => "权限不足：Key 没有该模型权限、账户未开通、地域/套餐限制或余额问题",
        404 => "接口或模型不存在：检查 Base URL 是否会被拼成正确 endpoint，以及模型名是否准确",
        408 => "服务端等待超时：稍后重试或调大超时时间",
        409 => "请求冲突：检查厂商账户/并发任务状态",
        422 => "请求语义错误：通常是模型不支持当前参数或消息内容格式",
        429 => "请求频率或额度超限：稍后重试，或检查余额/套餐/限流",
        500..=599 => "厂商服务异常：稍后重试，或切换模型/节点",
        _ => "厂商返回了非成功状态码：按 response 里的错误码继续排查",
    };

    let body_hint = if body_lower.contains("invalid_api_key")
        || body_lower.contains("invalid api key")
        || body_lower.contains("incorrect api key")
        || body_lower.contains("token expired")
        || body_lower.contains("token is invalid")
    {
        Some("响应里包含 Key 无效信号：重新生成该平台 API Key 后再试")
    } else if body_lower.contains("model")
        && (body_lower.contains("not found")
            || body_lower.contains("does not exist")
            || body_lower.contains("not exist"))
    {
        Some("响应里包含模型不存在信号：把模型名改成厂商文档里的精确 ID")
    } else if body_lower.contains("quota")
        || body_lower.contains("insufficient")
        || body_lower.contains("balance")
        || body_lower.contains("credit")
    {
        Some("响应里包含额度/余额信号：检查套餐、余额或模型权限")
    } else {
        None
    };

    match body_hint {
        Some(hint) => format!("{}；{}", status_hint, hint),
        None => status_hint.to_string(),
    }
}

fn ready_config(full: database::AiProviderFullConfig) -> Result<ProviderConfig, String> {
    if full.config.provider_type != AI_PROTOCOL {
        return Err("请使用 OpenAI 兼容接口。".to_string());
    }
    if is_deepseek_connection(&full.config.provider_id) && full.api_key.is_empty() {
        return Err("请先在设置中填写 DeepSeek API Key。".to_string());
    }
    Ok(ProviderConfig {
        provider_id: full.config.provider_id,
        provider_type: full.config.provider_type,
        base_url: full.config.base_url,
        model: full.config.model,
        api_key: full.api_key,
        temperature: full.config.temperature,
        max_output_tokens: full.config.max_output_tokens,
        timeout_ms: full.config.timeout_ms,
    })
}

pub fn active_provider_config(app: &AppHandle) -> Result<ProviderConfig, String> {
    ready_config(
        database::get_active_ai_provider_internal(app)?.ok_or("请先在设置中配置聊天连接。")?,
    )
}

pub async fn stream_ai_chat(
    app: AppHandle,
    request: Arc<ChatRequest>,
    config: ProviderConfig,
    messages: Vec<AiChatMessage>,
) -> Result<AiChatResult, String> {
    let full_response = openai_compat::stream_chat(&config, &messages, |content| {
        request.append(content);
        if !request.is_cancelled() {
            let _ = app.emit(
                "ai-chat-event",
                AiChatEvent::Delta {
                    request_id: request.id.clone(),
                    session_id: request.session_id.clone(),
                    content: content.to_string(),
                },
            );
        }
    })
    .await?;
    let user_text = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.as_str());
    Ok(parse_pet_action_response(&full_response, user_text))
}

pub async fn test_connection(
    app: &AppHandle,
    draft: database::SaveAiProviderRequest,
) -> Result<String, String> {
    let mut config = ready_config(database::preview_ai_provider(app, draft)?)?;
    config.max_output_tokens = config.max_output_tokens.min(64);
    openai_compat::test_request(
        &config,
        &[AiChatMessage {
            role: "user".to_string(),
            content: "Reply with OK.".to_string(),
        }],
    )
    .await
}
#[cfg(test)]
mod tests {
    use super::{build_endpoint, parse_pet_action_response};

    #[test]
    fn builds_base_and_full_endpoints_without_duplicating_paths() {
        assert_eq!(
            build_endpoint("https://api.deepseek.com"),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            build_endpoint("http://localhost:11434/v1/"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint("https://example.com/v1/chat/completions/"),
            "https://example.com/v1/chat/completions"
        );
    }
    #[test]
    fn pet_action_response_removes_marker_and_uses_action() {
        let parsed = parse_pet_action_response(
            "I am here with you.\n\n[[PET_ACTION:comfort]]",
            Some("I feel exhausted today"),
        );

        assert_eq!(parsed.reply, "I am here with you.");
        assert_eq!(parsed.action, "comfort");
    }

    #[test]
    fn pet_action_response_falls_back_to_calm() {
        let invalid_action = parse_pet_action_response("OK\n[[PET_ACTION:unknown]]", None);
        assert_eq!(invalid_action.reply, "OK");
        assert_eq!(invalid_action.action, "calm");

        let plain_text = parse_pet_action_response("plain text reply", None);
        assert_eq!(plain_text.reply, "plain text reply");
        assert_eq!(plain_text.action, "calm");
    }

    #[test]
    fn pet_action_response_uses_local_fallback_when_marker_is_missing() {
        let parsed =
            parse_pet_action_response("I am here with you.", Some("今天压力很大，感觉快崩溃了"));

        assert_eq!(parsed.reply, "I am here with you.");
        assert_eq!(parsed.action, "comfort");
    }

    #[test]
    fn pet_action_response_uses_local_fallback_when_marker_copies_choices() {
        let parsed = parse_pet_action_response(
            "先别急，我们一步一步来。\n[[PET_ACTION:calm|happy|sad|angry|comfort|cheer_up]]",
            Some("我没动力了，想摆烂"),
        );

        assert_eq!(parsed.reply, "先别急，我们一步一步来。");
        assert_eq!(parsed.action, "cheer_up");
    }

    #[test]
    fn pet_action_response_removes_tool_call_blocks() {
        let parsed = parse_pet_action_response(
            "我会陪着你。\n<tool_call>{\"function\":\"add_character_expression\",\"arguments\":{}}</tool_call>\n[[PET_ACTION:comfort]]",
            Some("鼓励一下我"),
        );

        assert_eq!(parsed.reply, "我会陪着你。");
        assert_eq!(parsed.action, "comfort");
    }

    #[test]
    fn utf8_is_preserved_at_every_possible_chunk_boundary() {
        let text = "data: 你好🐱\r\ndata: 下一行\n";
        for split in 0..=text.len() {
            let mut buffer = Vec::new();
            let mut lines =
                super::drain_complete_lines(&mut buffer, &text.as_bytes()[..split]).unwrap();
            lines.extend(
                super::drain_complete_lines(&mut buffer, &text.as_bytes()[split..]).unwrap(),
            );
            assert_eq!(lines, vec!["data: 你好🐱", "data: 下一行"]);
            assert!(super::drain_remaining_line(&mut buffer).unwrap().is_none());
        }
        let mut buffer = Vec::new();
        assert!(super::drain_complete_lines(&mut buffer, &[0xff, b'\n']).is_err());
    }

    #[test]
    fn interrupted_reply_hides_incomplete_action_and_tool_markup() {
        assert_eq!(super::interrupted_reply("你好 [[PET_ACTION:hap"), "你好");
        assert_eq!(super::interrupted_reply("你好 <tool_call>{"), "你好");
    }
}
