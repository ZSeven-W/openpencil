use futures::StreamExt;
use op_ai::chat_provider::ChatDelta;
use op_editor_core::HostedProtocol;
use tokio::sync::mpsc;

use super::{stream_protocol::ProtocolState, HostedRuntimeError};

const MAX_EVENT_BYTES: usize = 1024 * 1024;

/// Bounded SSE pump. Remote error text is never copied into UI/log channels.
pub(super) async fn pump(
    response: reqwest::Response,
    tx: &mpsc::Sender<ChatDelta>,
    protocol: HostedProtocol,
) -> Result<bool, HostedRuntimeError> {
    if !response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("text/event-stream")
        })
    {
        return Err(HostedRuntimeError::GatewayRejected);
    }
    let mut state = ProtocolState::new(protocol);
    let mut chunks = response.bytes_stream();
    let mut pending = Vec::new();
    let mut event = String::new();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|_| HostedRuntimeError::Network)?;
        if pending.len().saturating_add(chunk.len()) > MAX_EVENT_BYTES {
            return Err(HostedRuntimeError::GatewayRejected);
        }
        pending.extend_from_slice(&chunk);
        while let Some(newline) = pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = pending.drain(..=newline).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| HostedRuntimeError::GatewayRejected)?
                .trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                if emit(&mut event, tx, &mut state).await? {
                    return Ok(true);
                }
            } else if let Some(data) = line.strip_prefix("data:") {
                if event.len().saturating_add(data.len()) > MAX_EVENT_BYTES {
                    return Err(HostedRuntimeError::GatewayRejected);
                }
                if !event.is_empty() {
                    event.push('\n');
                }
                event.push_str(data.trim_start());
            }
        }
    }
    // A truncated event or missing terminal is a failed stream, never fake success.
    Ok(false)
}

async fn emit(
    event: &mut String,
    tx: &mpsc::Sender<ChatDelta>,
    state: &mut ProtocolState,
) -> Result<bool, HostedRuntimeError> {
    let deltas = state.consume(event.trim())?;
    event.clear();
    let mut done = false;
    for delta in deltas {
        let terminal = matches!(delta, ChatDelta::Done { .. });
        tx.send(delta)
            .await
            .map_err(|_| HostedRuntimeError::Cancelled)?;
        done |= terminal;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn fixture(body: String, status: &str, extra: &str) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}", body.len());
        let task = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut read = [0_u8; 4096];
            let _ = socket.read(&mut read).unwrap();
            socket.write_all(response.as_bytes()).unwrap();
        });
        (url, task)
    }

    #[tokio::test]
    async fn fixture_openai_stream_is_parsed_without_success_on_truncation() {
        let (url, task) = fixture(
            "data: {\"choices\":[{\"delta\":{\"content\":\"fixture\"},\"finish_reason\":\"stop\"}]}\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\ndata: [DONE]\n\n"
                .into(),
            "200 OK",
            "",
        );
        let response = crate::hosted_models::client()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        assert!(pump(response, &tx, HostedProtocol::OpenAiCompatible)
            .await
            .unwrap());
        assert!(matches!(rx.recv().await, Some(ChatDelta::TextDelta(text)) if text == "fixture"));
        assert!(matches!(rx.recv().await, Some(ChatDelta::Done { .. })));
        task.join().unwrap();

        let (url, task) = fixture(
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":\"stop\"}]}\n\n".into(),
            "200 OK",
            "",
        );
        let response = crate::hosted_models::client()
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap();
        assert!(!pump(response, &tx, HostedProtocol::OpenAiCompatible)
            .await
            .unwrap());
        task.join().unwrap();
    }

    #[tokio::test]
    async fn hosted_client_refuses_redirects_and_stream_errors_are_redacted() {
        let (url, task) = fixture(
            String::new(),
            "302 Found",
            "Location: http://127.0.0.1:1/secret\r\n",
        );
        let response = crate::hosted_models::client()
            .unwrap()
            .get(url)
            .bearer_auth("test-fixture-only")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        task.join().unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        let mut event =
            r#"{"error":{"message":"opllm_credential https://private.example"}}"#.to_string();
        assert_eq!(
            emit(
                &mut event,
                &tx,
                &mut ProtocolState::new(HostedProtocol::OpenAiCompatible)
            )
            .await
            .unwrap_err(),
            HostedRuntimeError::GatewayRejected
        );
        assert!(rx.try_recv().is_err());
    }
}
