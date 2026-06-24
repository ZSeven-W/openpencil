use super::*;

#[test]
fn streaming_design_progress_without_content_has_no_generating_block() {
    let mut message = ChatMessage::assistant_streaming();
    message.thinking = "• Planning...\n• Scaffold ready".into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        Rect::xywh(0.0, 0.0, 340.0, 300.0),
        op_editor_core::Locale::EnUs,
    );

    // The streaming "Generating design..." placeholder card was removed —
    // the fixed "Pencil it out" checklist conveys live progress instead.
    assert!(
        items[0].design_blocks.is_empty(),
        "no streaming placeholder design card"
    );
}
