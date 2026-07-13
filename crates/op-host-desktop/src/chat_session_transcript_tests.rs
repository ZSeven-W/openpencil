//! Transcript-only result compaction tests.

use super::*;
use op_editor_core::ChatToolCall;

#[test]
fn production_screenshot_envelope_keeps_metadata_but_drops_base64_from_transcript() {
    let mut chat = ChatState::default();
    let mut message = ChatMessage::assistant_streaming();
    message.tool_calls.push(ChatToolCall {
        name: "get_screenshot".into(),
        args: r#"{"level":"read","args":{"nodeId":"root"},"status":"running"}"#.into(),
        content_offset: None,
    });
    chat.messages.push(message);

    let original = serde_json::json!({
        "success": true,
        "data": {
            "image_base64": "QUJDRA==",
            "format": "png"
        }
    })
    .to_string();
    let result = ChatToolResult {
        content: original.clone(),
        is_error: false,
    };

    assert!(attach_tool_result_to_transcript(
        &mut chat,
        "get_screenshot",
        &result
    ));

    let card: serde_json::Value =
        serde_json::from_str(&chat.messages[0].tool_calls[0].args).unwrap();
    assert_eq!(card["status"], "done");
    assert_eq!(card["result"]["success"], true);
    assert_eq!(card["result"]["data"]["format"], "png");
    assert_eq!(card["result"]["data"]["image_base64_chars"], 8);
    assert_eq!(card["result"]["data"]["image_bytes"], 4);
    assert!(card["result"]["data"]["image_summary"]
        .as_str()
        .is_some_and(|summary| summary.contains("omitted from the UI transcript")));
    assert!(card["result"]["data"].get("image_base64").is_none());
    assert!(!chat.messages[0].tool_calls[0].args.contains("QUJDRA=="));

    // Transcript compaction is display-only: the executor acknowledgement
    // consumed by the in-flight agent loop remains byte-for-byte unchanged.
    assert_eq!(result.content, original);
}
