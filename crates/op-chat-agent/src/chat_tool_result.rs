//! Semantic result classification shared by in-process chat tool surfaces.

use op_ai::chat_provider::ChatToolResult;

pub fn rolled_back_transaction_message(data: &serde_json::Value) -> Option<String> {
    let object = data.as_object()?;
    if object.get("applied") != Some(&serde_json::Value::Bool(false)) {
        return None;
    }
    let errors = object.get("errors")?.as_array()?;
    if errors.is_empty() {
        return None;
    }
    Some(
        object
            .get("hint")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(
                "Tool transaction rolled back; inspect data.errors and resend the corrected batch",
            )
            .to_string(),
    )
}

pub fn structured_error_result(message: String, data: serde_json::Value) -> ChatToolResult {
    let envelope = serde_json::json!({
        "success": false,
        "error": message,
        "data": data,
    });
    ChatToolResult {
        content: envelope.to_string(),
        is_error: true,
    }
}
