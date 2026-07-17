use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{
    build_endpoint, build_http_client, diagnose_http_error, diagnose_request_error,
    drain_complete_lines, drain_remaining_line, mask_sensitive, provider_debug_context,
    truncate_debug_body, AiChatEvent, AiChatMessage, ProviderConfig,
};

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<MessageContent>,
    stream: bool,
    max_tokens: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Serialize)]
struct MessageContent {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    delta: Option<DeltaBlock>,
}

#[derive(Deserialize)]
struct DeltaBlock {
    text: Option<String>,
}

fn build_headers(config: &ProviderConfig) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert("x-api-key", config.api_key.parse().unwrap());
    headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
    headers
}

fn split_system_messages(messages: &[AiChatMessage]) -> (Option<String>, Vec<MessageContent>) {
    let mut system_parts = Vec::new();
    let mut chat_messages = Vec::new();

    for msg in messages {
        if msg.role == "system" {
            system_parts.push(msg.content.clone());
        } else {
            chat_messages.push(MessageContent {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };

    (system, chat_messages)
}

fn handle_stream_line(
    app: &AppHandle,
    request_id: &str,
    session_id: &str,
    line: &str,
    emit_deltas: bool,
    full_response: &mut String,
) {
    let line = line.trim();
    if !line.starts_with("data: ") {
        return;
    }
    let data = &line[6..];

    if let Ok(resp) = serde_json::from_str::<MessagesResponse>(data) {
        if let Some(delta) = &resp.delta {
            if let Some(text) = &delta.text {
                full_response.push_str(text);
                if emit_deltas {
                    let _ = app.emit(
                        "ai-chat-event",
                        AiChatEvent::Delta {
                            request_id: request_id.to_string(),
                            session_id: session_id.to_string(),
                            content: text.clone(),
                        },
                    );
                }
            }
        }
    }
}

pub async fn stream_chat(
    app: &AppHandle,
    config: &ProviderConfig,
    messages: &[AiChatMessage],
    request_id: &str,
    session_id: &str,
    emit_deltas: bool,
) -> Result<String, String> {
    let endpoint = build_endpoint(&config.base_url, "anthropic-compatible", &config.model);
    log::info!(
        "Anthropic-compat stream: endpoint={}, model={}, key={}",
        endpoint,
        config.model,
        mask_sensitive(&config.api_key)
    );

    let (system, chat_messages) = split_system_messages(messages);
    let client = build_http_client(config)?;
    let body = MessagesRequest {
        model: config.model.clone(),
        messages: chat_messages,
        stream: true,
        max_tokens: config.max_output_tokens,
        system,
    };

    let response = client
        .post(&endpoint)
        .headers(build_headers(config))
        .json(&body)
        .timeout(std::time::Duration::from_millis(config.timeout_ms as u64))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &endpoint),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "API error {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &endpoint),
            diagnose_http_error(status, &text),
            truncate_debug_body(&text)
        ));
    }

    let mut full_response = String::new();
    let mut stream = response.bytes_stream();
    let mut line_buffer = Vec::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            format!(
                "Stream error: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &endpoint),
                diagnose_request_error(&e)
            )
        })?;

        for line in drain_complete_lines(&mut line_buffer, &chunk) {
            handle_stream_line(
                app,
                request_id,
                session_id,
                &line,
                emit_deltas,
                &mut full_response,
            );
        }
    }

    if let Some(line) = drain_remaining_line(&mut line_buffer) {
        handle_stream_line(
            app,
            request_id,
            session_id,
            &line,
            emit_deltas,
            &mut full_response,
        );
    }

    if full_response.trim().is_empty() {
        return Err(format!(
            "Stream completed without content\n{}",
            provider_debug_context(config, &endpoint)
        ));
    }

    Ok(full_response)
}

pub async fn test_request(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
) -> Result<String, String> {
    let endpoint = build_endpoint(&config.base_url, "anthropic-compatible", &config.model);
    let (system, chat_messages) = split_system_messages(messages);
    let client = build_http_client(config)?;
    let body = MessagesRequest {
        model: config.model.clone(),
        messages: chat_messages,
        stream: false,
        max_tokens: 100,
        system,
    };

    let response = client
        .post(&endpoint)
        .headers(build_headers(config))
        .json(&body)
        .timeout(std::time::Duration::from_millis(10000))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &endpoint),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Test failed {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &endpoint),
            diagnose_http_error(status, &text),
            truncate_debug_body(&text)
        ));
    }

    Ok("Connection successful".to_string())
}
