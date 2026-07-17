pub mod anthropic_compat;
pub mod gemini_native;
pub mod ollama;
pub mod openai_compat;

use serde::{Deserialize, Serialize};
use std::error::Error;
use tauri::{AppHandle, Emitter};

use crate::database;

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

/// Normalize base URL before applying the provider-owned endpoint rule.
fn normalize_base_url(base_url: &str) -> String {
    let url = base_url.trim_end_matches('/');
    url.to_string()
}

/// Build endpoint URL based on provider type
pub fn build_endpoint(base_url: &str, provider_type: &str, model: &str) -> String {
    let normalized = normalize_base_url(base_url);
    match provider_type {
        "openai-compatible" => format!("{}/chat/completions", normalized),
        "anthropic-compatible" => {
            if normalized.ends_with("/v1") {
                format!("{}/messages", normalized)
            } else {
                format!("{}/v1/messages", normalized)
            }
        }
        "gemini-native" => {
            format!(
                "{}/v1beta/models/{}:streamGenerateContent",
                normalized, model
            )
        }
        "ollama" => {
            format!("{}/api/chat", normalized)
        }
        _ => format!("{}/chat/completions", normalized),
    }
}

/// Mask sensitive strings in logs
pub fn mask_sensitive(text: &str) -> String {
    if text.len() <= 8 {
        return "***".to_string();
    }
    format!("{}...{}", &text[..4], &text[text.len() - 4..])
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

pub fn drain_complete_lines(buffer: &mut Vec<u8>, chunk: &[u8]) -> Vec<String> {
    buffer.extend_from_slice(chunk);

    let mut lines = Vec::new();
    while let Some(newline_index) = buffer.iter().position(|byte| *byte == b'\n') {
        let mut line = buffer.drain(..=newline_index).collect::<Vec<_>>();
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        lines.push(String::from_utf8_lossy(&line).into_owned());
    }

    lines
}

pub fn drain_remaining_line(buffer: &mut Vec<u8>) -> Option<String> {
    let line = String::from_utf8_lossy(buffer)
        .trim_matches(['\r', '\n'])
        .to_string();
    buffer.clear();

    if line.trim().is_empty() {
        None
    } else {
        Some(line)
    }
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

/// Send a streaming AI request and emit events to frontend
pub async fn stream_ai_chat(
    app: AppHandle,
    request_id: String,
    session_id: String,
    messages: Vec<AiChatMessage>,
) -> Result<AiChatResult, String> {
    // Get active provider config (internal, includes API key)
    let full_config = database::get_active_ai_provider_internal(&app)?
        .ok_or("No active AI provider configured")?;

    if full_config.config.provider_type != "ollama" && full_config.api_key.is_empty() {
        return Err("API key is not set".to_string());
    }

    let config = ProviderConfig {
        provider_id: full_config.config.provider_id.clone(),
        provider_type: full_config.config.provider_type.clone(),
        base_url: full_config.config.base_url.clone(),
        model: full_config.config.model.clone(),
        api_key: full_config.api_key,
        temperature: full_config.config.temperature,
        max_output_tokens: full_config.config.max_output_tokens,
        timeout_ms: full_config.config.timeout_ms,
    };

    // Emit start event
    let _ = app.emit(
        "ai-chat-event",
        AiChatEvent::Start {
            request_id: request_id.clone(),
            session_id: session_id.clone(),
        },
    );

    // Route to appropriate adapter
    let result = match config.provider_type.as_str() {
        "openai-compatible" => {
            openai_compat::stream_chat(&app, &config, &messages, &request_id, &session_id, true)
                .await
        }
        "anthropic-compatible" => {
            anthropic_compat::stream_chat(&app, &config, &messages, &request_id, &session_id, true)
                .await
        }
        "gemini-native" => {
            gemini_native::stream_chat(&app, &config, &messages, &request_id, &session_id, true)
                .await
        }
        "ollama" => {
            ollama::stream_chat(&app, &config, &messages, &request_id, &session_id, true).await
        }
        _ => Err(format!("Unknown provider type: {}", config.provider_type)),
    };

    match result {
        Ok(full_response) => {
            let latest_user_text = messages
                .iter()
                .rev()
                .find(|message| message.role == "user")
                .map(|message| message.content.as_str());
            let parsed = parse_pet_action_response(&full_response, latest_user_text);
            // Emit done event
            let _ = app.emit(
                "ai-chat-event",
                AiChatEvent::Done {
                    request_id: request_id.clone(),
                    session_id: session_id.clone(),
                    full_response: parsed.reply.clone(),
                    action: parsed.action.clone(),
                },
            );
            Ok(parsed)
        }
        Err(err_msg) => {
            // Emit error event
            let _ = app.emit(
                "ai-chat-event",
                AiChatEvent::Error {
                    request_id: request_id.clone(),
                    session_id: session_id.clone(),
                    message: err_msg.clone(),
                },
            );
            Err(err_msg)
        }
    }
}

/// Test connection for a provider
pub async fn test_connection(app: &AppHandle, provider_id: &str) -> Result<String, String> {
    let full_config =
        database::get_ai_provider_full(app, provider_id)?.ok_or("Provider not found")?;

    if full_config.config.provider_type != "ollama" && full_config.api_key.is_empty() {
        return Err("API key is not set for this provider".to_string());
    }

    let config = ProviderConfig {
        provider_id: full_config.config.provider_id,
        provider_type: full_config.config.provider_type.clone(),
        base_url: full_config.config.base_url,
        model: full_config.config.model,
        api_key: full_config.api_key,
        temperature: full_config.config.temperature,
        max_output_tokens: full_config.config.max_output_tokens,
        timeout_ms: full_config.config.timeout_ms,
    };

    let test_messages = vec![AiChatMessage {
        role: "user".to_string(),
        content: "Hi".to_string(),
    }];

    let result = match config.provider_type.as_str() {
        "openai-compatible" => openai_compat::test_request(&config, &test_messages).await,
        "anthropic-compatible" => anthropic_compat::test_request(&config, &test_messages).await,
        "gemini-native" => gemini_native::test_request(&config, &test_messages).await,
        "ollama" => ollama::test_request(&config, &test_messages).await,
        _ => Err("Unknown provider type".to_string()),
    };

    match result {
        Ok(message) => Ok(format!(
            "{}\nprovider={}, type={}, model={}, baseUrl={}",
            message, config.provider_id, config.provider_type, config.model, config.base_url
        )),
        Err(message) => Err(format!(
            "{}\nprovider={}, type={}, model={}, baseUrl={}",
            message, config.provider_id, config.provider_type, config.model, config.base_url
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_endpoint, drain_complete_lines, drain_remaining_line, parse_pet_action_response,
    };

    #[test]
    fn streaming_line_buffer_preserves_utf8_split_across_chunks() {
        let bytes = "你好\n下一行".as_bytes();
        let mut buffer = Vec::new();

        assert!(drain_complete_lines(&mut buffer, &bytes[..1]).is_empty());
        assert_eq!(
            drain_complete_lines(&mut buffer, &bytes[1..7]),
            vec!["你好".to_string()]
        );
        assert!(drain_complete_lines(&mut buffer, &bytes[7..]).is_empty());
        assert_eq!(
            drain_remaining_line(&mut buffer),
            Some("下一行".to_string())
        );
    }

    #[test]
    fn openai_compatible_appends_chat_completions_without_forcing_v1() {
        assert_eq!(
            build_endpoint(
                "https://api.deepseek.com",
                "openai-compatible",
                "deepseek-v4-flash"
            ),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            build_endpoint(
                "https://dashscope.aliyuncs.com/compatible-mode/v1/",
                "openai-compatible",
                "qwen-plus"
            ),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint(
                "https://open.bigmodel.cn/api/paas/v4",
                "openai-compatible",
                "glm-4.7-flash"
            ),
            "https://open.bigmodel.cn/api/paas/v4/chat/completions"
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
}
