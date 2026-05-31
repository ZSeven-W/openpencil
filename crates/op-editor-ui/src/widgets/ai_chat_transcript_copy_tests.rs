use super::*;

#[test]
fn transcript_hit_resolves_design_json_copy_button_like_ts() {
    let code = r#"[{"id":"frame-1","type":"Frame"}]"#;
    let message = ChatMessage::assistant(format!(
        r#"```json
{code}
```"#
    ));
    let messages = std::slice::from_ref(&message);
    let block =
        &build_transcript(messages, body(), op_editor_core::Locale::EnUs)[0].design_blocks[0];
    let x = block.header.origin.x + block.header.size.x - 38.0;
    let y = block.header.origin.y + block.header.size.y / 2.0;

    assert_eq!(
        transcript_hit(messages, body(), x, y, op_editor_core::Locale::EnUs),
        Some(TranscriptHit::CopyDesignBlock(code.to_string()))
    );
}

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}
