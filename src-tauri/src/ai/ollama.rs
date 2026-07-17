use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{
    build_http_client, diagnose_http_error, diagnose_request_error, drain_complete_lines,
    drain_remaining_line, provider_debug_context, truncate_debug_body, AiChatEvent, AiChatMessage,
    ProviderConfig,
};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Option<ChatResponseMessage>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

fn build_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{}/api/chat", base)
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
    if line.is_empty() {
        return;
    }

    if let Ok(resp) = serde_json::from_str::<ChatResponse>(line) {
        if let Some(msg) = resp.message {
            if let Some(content) = msg.content {
                full_response.push_str(&content);
                if emit_deltas {
                    let _ = app.emit(
                        "ai-chat-event",
                        AiChatEvent::Delta {
                            request_id: request_id.to_string(),
                            session_id: session_id.to_string(),
                            content,
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
    let url = build_url(&config.base_url);
    log::info!("Ollama stream: url={}, model={}", url, config.model);

    let client = build_http_client(config)?;
    let body = ChatRequest {
        model: config.model.clone(),
        messages: messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        stream: true,
    };

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_millis(config.timeout_ms as u64))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &url),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ollama error {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &url),
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
                provider_debug_context(config, &url),
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
            provider_debug_context(config, &url)
        ));
    }

    Ok(full_response)
}

pub async fn test_request(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
) -> Result<String, String> {
    let url = build_url(&config.base_url);
    let client = build_http_client(config)?;
    let body = ChatRequest {
        model: config.model.clone(),
        messages: messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        stream: false,
    };

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_millis(10000))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &url),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Test failed {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &url),
            diagnose_http_error(status, &text),
            truncate_debug_body(&text)
        ));
    }

    Ok("Connection successful".to_string())
}
