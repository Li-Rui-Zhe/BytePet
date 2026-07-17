use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{
    build_http_client, diagnose_http_error, diagnose_request_error, drain_complete_lines,
    drain_remaining_line, mask_sensitive, provider_debug_context, truncate_debug_body, AiChatEvent,
    AiChatMessage, ProviderConfig,
};

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f64,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i64,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

fn build_url(config: &ProviderConfig) -> String {
    let base = config.base_url.trim_end_matches('/');
    format!(
        "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
        base, config.model
    )
}

fn build_safe_stream_url(config: &ProviderConfig) -> String {
    build_url(config)
}

fn build_generate_url(config: &ProviderConfig) -> String {
    let base = config.base_url.trim_end_matches('/');
    format!("{}/v1beta/models/{}:generateContent", base, config.model)
}

fn build_safe_generate_url(config: &ProviderConfig) -> String {
    build_generate_url(config)
}

fn api_key_header(config: &ProviderConfig) -> Result<reqwest::header::HeaderValue, String> {
    reqwest::header::HeaderValue::from_str(&config.api_key)
        .map_err(|_| "API key contains invalid header characters".to_string())
}

fn build_request_body(config: &ProviderConfig, messages: &[AiChatMessage]) -> GenerateRequest {
    let system_prompt = messages
        .iter()
        .filter(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut injected_system = false;

    GenerateRequest {
        contents: messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| Content {
                role: if m.role == "assistant" {
                    "model".to_string()
                } else {
                    "user".to_string()
                },
                parts: vec![Part {
                    text: if !injected_system && m.role == "user" && !system_prompt.is_empty() {
                        injected_system = true;
                        format!(
                            "System instructions:\n{}\n\nUser message:\n{}",
                            system_prompt, m.content
                        )
                    } else {
                        m.content.clone()
                    },
                }],
            })
            .collect(),
        generation_config: GenerationConfig {
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
        },
    }
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

    if let Ok(resp) = serde_json::from_str::<GenerateResponse>(data) {
        if let Some(candidates) = resp.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(content) = &candidate.content {
                    if let Some(parts) = &content.parts {
                        for part in parts {
                            if let Some(text) = &part.text {
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
    let url = build_url(config);
    let safe_url = build_safe_stream_url(config);
    log::info!(
        "Gemini native stream: model={}, key={}",
        config.model,
        mask_sensitive(&config.api_key)
    );

    let client = build_http_client(config)?;
    let body = build_request_body(config, messages);

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("x-goog-api-key", api_key_header(config)?)
        .json(&body)
        .timeout(std::time::Duration::from_millis(config.timeout_ms as u64))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &safe_url),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "API error {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &safe_url),
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
                provider_debug_context(config, &safe_url),
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
            provider_debug_context(config, &safe_url)
        ));
    }

    Ok(full_response)
}

pub async fn test_request(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
) -> Result<String, String> {
    let url = build_generate_url(config);
    let safe_url = build_safe_generate_url(config);

    let client = build_http_client(config)?;
    let body = build_request_body(config, messages);

    let response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("x-goog-api-key", api_key_header(config)?)
        .json(&body)
        .timeout(std::time::Duration::from_millis(10000))
        .send()
        .await
        .map_err(|e| {
            format!(
                "Request failed: {}\n{}\ndiagnosis={}",
                e,
                provider_debug_context(config, &safe_url),
                diagnose_request_error(&e)
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Test failed {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &safe_url),
            diagnose_http_error(status, &text),
            truncate_debug_body(&text)
        ));
    }

    Ok("Connection successful".to_string())
}
