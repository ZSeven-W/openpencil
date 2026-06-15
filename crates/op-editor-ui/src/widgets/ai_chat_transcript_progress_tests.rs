use super::*;

#[test]
fn streaming_design_progress_without_content_renders_generating_block() {
    let mut message = ChatMessage::assistant_streaming();
    message.thinking = "• Planning...\n• Scaffold ready".into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        Rect::xywh(0.0, 0.0, 340.0, 300.0),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].design_blocks.len(), 1);
    assert_eq!(items[0].design_blocks[0].label, "Generating design...");
    assert!(items[0].design_blocks[0].streaming);
    assert!(
        items[0].bubble.is_none(),
        "design generation should use the TS generating row, not the generic thinking pill"
    );
}

#[test]
fn streaming_design_op_block_defaults_expanded_with_code_preview() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"```json
[{"id":"frame-1","type":"Frame","name":"Login"}]"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        Rect::xywh(0.0, 0.0, 340.0, 300.0),
        op_editor_core::Locale::EnUs,
    );
    let block = &items[0].design_blocks[0];

    assert!(block.expanded);
    assert!(block.body.size.y > 0.0);
    assert!(block
        .code_lines
        .iter()
        .any(|line| line.contains(r#""type":"Frame""#)));
}
