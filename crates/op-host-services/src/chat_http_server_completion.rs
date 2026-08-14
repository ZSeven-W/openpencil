//! Completion reconciliation helpers for the OpenCode HTTP transport.

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

pub(super) async fn abort_session(client: &reqwest::Client, base: &str, session_id: &str) {
    let _ = client
        .post(format!("{base}/session/{session_id}/abort"))
        .json(&serde_json::json!({}))
        .send()
        .await;
}
