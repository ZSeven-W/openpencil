//! Native image blocks for joined, plain builtin vision turns.

use op_ai::chat_provider::ChatAttachment;
use op_editor_core::BuiltinAgentKind;
use serde_json::{json, Value};

pub(super) fn user_content(
    kind: &BuiltinAgentKind,
    prompt: &str,
    attachments: &[ChatAttachment],
) -> Result<Value, String> {
    if attachments.is_empty() {
        return Ok(Value::String(prompt.to_string()));
    }
    let mut blocks = vec![json!({"type": "text", "text": prompt})];
    for attachment in attachments {
        if attachment.data.is_empty()
            || !matches!(
                attachment.media_type.as_str(),
                "image/jpeg" | "image/png" | "image/webp" | "image/gif"
            )
        {
            // Do not silently downgrade an unsupported attachment into a
            // text-only judgment, or ask the remote model to read local paths.
            return Err("joined vision turn requires nonempty supported image attachments".into());
        }
        let data = crate::chat_attachment::attachment_to_base64(attachment);
        blocks.push(match kind {
            BuiltinAgentKind::OpenAiCompat => json!({
                "type": "image_url",
                "image_url": {"url": format!("data:{};base64,{data}", attachment.media_type)},
            }),
            BuiltinAgentKind::Anthropic => json!({
                "type": "image",
                "source": {"type": "base64", "media_type": attachment.media_type, "data": data},
            }),
        });
    }
    Ok(Value::Array(blocks))
}
