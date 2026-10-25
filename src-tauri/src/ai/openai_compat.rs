use futures::StreamExt;
use serde::{Deserialize, Serialize};

use super::{
    build_endpoint, build_http_client, diagnose_http_error, diagnose_request_error,
    drain_complete_lines, drain_remaining_line, provider_debug_context, truncate_debug_body,
    AiChatMessage, ProviderConfig,
};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f64,
    max_tokens: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    delta: Option<ChatDelta>,
    message: Option<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

fn build_headers(config: &ProviderConfig) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    if !config.api_key.is_empty() {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", config.api_key)
                .parse()
                .map_err(|_| "API Key 含有无效字符。".to_string())?,
        );
    }
    Ok(headers)
}

fn build_request_body(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
    stream: bool,
) -> ChatRequest {
    ChatRequest {
        model: config.model.clone(),
        messages: messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect(),
        stream,
        temperature: config.temperature,
        max_tokens: config.max_output_tokens,
        thinking: super::is_deepseek_connection(&config.provider_id)
            .then(|| serde_json::json!({ "type": "disabled" })),
    }
}

enum StreamLine {
    Delta(String),
    Done,
    Ignore,
}

fn parse_stream_line(line: &str) -> Result<StreamLine, String> {
    let Some(data) = line.trim().strip_prefix("data:") else {
        return Ok(StreamLine::Ignore);
    };
    let data = data.trim();
    if data == "[DONE]" {
        return Ok(StreamLine::Done);
    }
    if data.is_empty() {
        return Ok(StreamLine::Ignore);
    }
    let response: ChatResponse = serde_json::from_str(data)
        .map_err(|_| "服务端流式响应不符合 Chat Completions 格式。".to_string())?;
    Ok(response
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.delta)
        .and_then(|delta| delta.content)
        .map(StreamLine::Delta)
        .unwrap_or(StreamLine::Ignore))
}

fn safe_body(config: &ProviderConfig, body: &str) -> String {
    let redacted = if config.api_key.is_empty() {
        body.to_string()
    } else {
        body.replace(&config.api_key, "[REDACTED]")
    };
    truncate_debug_body(&redacted)
}

pub async fn stream_chat(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
    mut on_delta: impl FnMut(&str),
) -> Result<String, String> {
    let endpoint = build_endpoint(&config.base_url);
    log::info!(
        "Chat Completions stream: endpoint={}, model={}",
        endpoint,
        config.model
    );

    let client = build_http_client(config)?;
    let body = build_request_body(config, messages, true);

    let response = client
        .post(&endpoint)
        .headers(build_headers(config)?)
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
        let text = String::from_utf8_lossy(&read_limited_response(response).await?).to_string();
        return Err(format!(
            "API error {}\n{}\ndiagnosis={}\nresponse={}",
            status,
            provider_debug_context(config, &endpoint),
            diagnose_http_error(status, &text),
            safe_body(config, &text)
        ));
    }

    let mut full_response = String::new();
    let mut stream = response.bytes_stream();
    let mut line_buffer = Vec::new();
    let mut done = false;
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| format!("流式连接中断：{}", diagnose_request_error(&error)))?;
        for line in drain_complete_lines(&mut line_buffer, &chunk)? {
            match parse_stream_line(&line)? {
                StreamLine::Delta(content) => {
                    full_response.push_str(&content);
                    on_delta(&content);
                }
                StreamLine::Done => {
                    done = true;
                    break;
                }
                StreamLine::Ignore => {}
            }
        }
        if line_buffer.len() > 1024 * 1024 || full_response.len() > 1024 * 1024 {
            return Err("服务端响应超出允许大小。".to_string());
        }
        if done {
            break;
        }
    }
    if !done {
        if let Some(line) = drain_remaining_line(&mut line_buffer)? {
            if let StreamLine::Delta(content) = parse_stream_line(&line)? {
                full_response.push_str(&content);
                on_delta(&content);
            }
        }
    }
    if full_response.trim().is_empty() {
        return Err(format!(
            "Stream completed without content\n{}",
            provider_debug_context(config, &endpoint)
        ));
    }

    Ok(full_response)
}

async fn read_limited_response(response: reqwest::Response) -> Result<Vec<u8>, String> {
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "读取接口回复失败。".to_string())?;
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err("接口回复超出允许大小。".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub async fn test_request(
    config: &ProviderConfig,
    messages: &[AiChatMessage],
) -> Result<String, String> {
    let endpoint = build_endpoint(&config.base_url);
    let client = build_http_client(config)?;
    let body = build_request_body(config, messages, false);

    let response = client
        .post(&endpoint)
        .headers(build_headers(config)?)
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
            safe_body(config, &text)
        ));
    }

    let bytes = read_limited_response(response).await?;
    let body: ChatResponse = serde_json::from_slice(&bytes)
        .map_err(|_| "接口返回成功状态，但内容不是有效的 Chat Completions JSON。".to_string())?;
    if !body.choices.iter().any(|choice| {
        choice
            .message
            .as_ref()
            .is_some_and(|message| !message.content.trim().is_empty())
    }) {
        return Err("接口没有返回有效的文字回复，请检查模型和接口地址。".to_string());
    }
    Ok("已收到有效的文字回复。".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(id: &str) -> ProviderConfig {
        ProviderConfig {
            provider_id: id.into(),
            provider_type: super::super::AI_PROTOCOL.into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "test-model".into(),
            api_key: String::new(),
            temperature: 0.8,
            max_output_tokens: 1200,
            timeout_ms: 60000,
        }
    }

    #[test]
    fn only_default_deepseek_disables_thinking_without_adding_vendor_fields_to_custom() {
        let deepseek =
            serde_json::to_value(build_request_body(&config("deepseek"), &[], true)).unwrap();
        assert_eq!(deepseek["thinking"]["type"], "disabled");
        let profile_id = format!("deepseek-profile-{}", uuid::Uuid::new_v4());
        let mut profile = config(&profile_id);
        profile.model = "deepseek-v4-pro".into();
        let body = serde_json::to_value(build_request_body(&profile, &[], true)).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
        assert_eq!(body["model"], "deepseek-v4-pro");
        let custom =
            serde_json::to_value(build_request_body(&config("custom"), &[], true)).unwrap();
        assert!(custom.get("thinking").is_none());
    }

    #[test]
    fn local_endpoint_omits_authorization_and_invalid_key_returns_error() {
        let mut local = config("custom");
        assert!(!build_headers(&local)
            .unwrap()
            .contains_key(reqwest::header::AUTHORIZATION));
        local.api_key = "invalid\nkey".into();
        assert!(build_headers(&local).is_err());
    }

    async fn fixture(body: &str) -> ProviderConfig {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut configuration = config("custom");
        configuration.base_url = format!("http://{}", listener.local_addr().unwrap());
        let body = body.as_bytes().to_vec();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            loop {
                let n = socket.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(headers.as_bytes()).await.unwrap();
            // Split inside UTF-8 characters to exercise the real HTTP adapter.
            for byte in body {
                socket.write_all(&[byte]).await.unwrap();
                tokio::task::yield_now().await;
            }
            socket.shutdown().await.unwrap();
        });
        configuration
    }

    #[tokio::test]
    async fn test_connection_requires_actual_chat_completions_content() {
        for body in [
            "<html>OK</html>",
            r#"{"choices":[]}"#,
            r#"{"choices":[{"message":{"role":"assistant","content":""}}]}"#,
            r#"{"error":{"message":"failed"}}"#,
        ] {
            assert!(test_request(&fixture(body).await, &[]).await.is_err());
        }
        assert!(test_request(
            &fixture(r#"{"choices":[{"message":{"role":"assistant","content":"你好"}}]}"#).await,
            &[]
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn sse_stream_handles_unicode_no_space_prefix_and_done_without_waiting_for_eof() {
        let configuration = fixture(
            "data:{\"choices\":[{\"delta\":{\"content\":\"你好🐱\"}}]}\n\ndata: [DONE]\n\n",
        )
        .await;
        let mut chunks = String::new();
        let reply = stream_chat(&configuration, &[], |chunk| chunks.push_str(chunk))
            .await
            .unwrap();
        assert_eq!(reply, "你好🐱");
        assert_eq!(chunks, reply);
    }

    #[test]
    fn error_body_never_echoes_the_api_key() {
        let mut configuration = config("custom");
        configuration.api_key = "test-sensitive-only".into();
        assert_eq!(
            safe_body(&configuration, "error test-sensitive-only"),
            "error [REDACTED]"
        );
    }

    #[tokio::test]
    async fn cancelling_drops_the_http_connection_without_waiting_for_server_eof() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::sync::oneshot;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut configuration = config("custom");
        configuration.base_url = format!("http://{}", listener.local_addr().unwrap());
        let (closed_tx, closed_rx) = oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            loop {
                let n = socket.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
                if n == 0 {
                    return;
                }
            }
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 99999\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n").await.unwrap();
            let closed = match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                socket.read(&mut chunk),
            )
            .await
            {
                Ok(Ok(0)) | Ok(Err(_)) => true,
                _ => false,
            };
            let _ = closed_tx.send(closed);
        });
        let (ready_tx, ready_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut ready_tx = Some(ready_tx);
            tokio::select! {
                _ = cancel_rx => (),
                _ = stream_chat(&configuration, &[], |_| { if let Some(sender) = ready_tx.take() { let _ = sender.send(()); } }) => panic!("server should stay open"),
            }
        });
        tokio::time::timeout(std::time::Duration::from_secs(3), ready_rx)
            .await
            .unwrap()
            .unwrap();
        cancel_tx.send(()).unwrap();
        task.await.unwrap();
        assert!(closed_rx.await.unwrap());
    }
}
