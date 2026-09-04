//! Completion reconciliation helpers for the OpenCode HTTP transport.

use std::time::Duration;

/// Cleanup is best-effort and must never delay the primary turn result for a
/// full control-plane timeout. Both endpoints are localhost-only.
const SESSION_CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) fn latest_assistant_text(messages: &serde_json::Value) -> Option<String> {
    let assistant = messages.as_array()?.iter().rev().find(|message| {
        message
            .get("info")
            .and_then(|info| info.get("role"))
            .and_then(|role| role.as_str())
            == Some("assistant")
    })?;
    let text = assistant
        .get("parts")?
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(|value| value.as_str()) == Some("text"))
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

/// Abort an unfinished turn when requested, then delete the temporary session
/// OpenPencil created for it. Cleanup failures are diagnostics only: they must
/// not replace the provider error (or successful answer) already produced.
pub(super) async fn cleanup_session(
    client: &reqwest::Client,
    base: &str,
    session_id: &str,
    abort_first: bool,
) {
    if abort_first {
        run_cleanup_request(
            "abort",
            client
                .post(format!("{base}/session/{session_id}/abort"))
                .json(&serde_json::json!({})),
        )
        .await;
    }
    run_cleanup_request(
        "delete",
        client.delete(format!("{base}/session/{session_id}")),
    )
    .await;
}

async fn run_cleanup_request(operation: &str, request: reqwest::RequestBuilder) {
    match tokio::time::timeout(SESSION_CLEANUP_TIMEOUT, request.send()).await {
        Ok(Ok(response)) if response.status().is_success() => {}
        Ok(Ok(response)) => eprintln!(
            "[AI] OpenCode session {operation} failed with HTTP {}",
            response.status()
        ),
        Ok(Err(error)) => eprintln!("[AI] OpenCode session {operation} failed: {error}"),
        Err(_) => eprintln!(
            "[AI] OpenCode session {operation} timed out after {} seconds",
            SESSION_CLEANUP_TIMEOUT.as_secs()
        ),
    }
}
