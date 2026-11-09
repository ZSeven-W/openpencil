use super::*;

const CREDENTIAL: &str = "Done.\n\n\
• Checked layout, overflow, structure — 10 auto-repair(s) applied, no issues left\n  \
▸ repairs: layout 3, structure 7\n  \
▸ structure · hero-bleed · Home [n1] · gap 0 → 16";

#[test]
fn a_credential_becomes_a_receipt_that_leads_with_its_result() {
    let (prose, receipts) = split_receipts(CREDENTIAL);
    assert_eq!(prose, "Done.");
    assert_eq!(receipts.len(), 1);
    let receipt = &receipts[0];
    assert_eq!(receipt.title, "10 auto-repair(s) applied, no issues left");
    assert_eq!(receipt.status, Some(ParsedStepStatus::Done));
    assert_eq!(
        receipt.details,
        [
            "Checked layout, overflow, structure",
            "repairs: layout 3, structure 7",
            "structure · hero-bleed · Home [n1] · gap 0 → 16",
        ]
    );
}

#[test]
fn plain_bullets_stay_prose() {
    let text = "Here is the plan:\n• Header\n• Cards — three in a row\n\nThat's it.";
    let (prose, receipts) = split_receipts(text);
    assert_eq!(prose, text);
    assert!(receipts.is_empty());
}

#[test]
fn a_receipt_without_a_dash_keeps_its_title() {
    let (_, receipts) = split_receipts("• Subtask header\n  ▸ skills: cjk-typography");
    assert_eq!(receipts[0].title, "Subtask header");
    assert_eq!(receipts[0].details, ["skills: cjk-typography"]);
}

#[test]
fn receipts_lay_out_collapsed_after_the_body_with_their_own_toggle_slot() {
    let (_, receipts) = split_receipts(CREDENTIAL);
    let mut msg = ChatMessage::assistant_streaming();
    msg.streaming = false;
    let mut y = 100.0;
    let steps = layout_receipts(
        &receipts,
        &msg,
        op_editor_core::Locale::EnUs,
        10.0,
        &mut y,
        300.0,
        60,
        2,
    );
    assert_eq!(steps.len(), 1);
    let step = &steps[0];
    assert_eq!(
        (step.source_index, step.expanded, step.done),
        (2, false, true)
    );
    assert_eq!(step.rect.origin.y, 100.0);
    assert!(y > 100.0);
    // The user's toggle opens it.
    msg.action_step_expanded_overrides = vec![None, None, Some(true)];
    let mut y = 0.0;
    let opened = layout_receipts(
        &receipts,
        &msg,
        op_editor_core::Locale::EnUs,
        0.0,
        &mut y,
        300.0,
        60,
        2,
    );
    assert!(opened[0].expanded && opened[0].details.len() >= 3);
}

fn finished(content: &str) -> ChatMessage {
    let mut msg = ChatMessage::assistant_streaming();
    msg.content = content.to_owned();
    msg.streaming = false;
    msg
}

fn laid_out(msg: &ChatMessage) -> crate::widgets::ai_chat_transcript::TranscriptItem {
    let body = Rect::xywh(0.0, 0.0, 320.0, 2000.0);
    crate::widgets::ai_chat_transcript::build_transcript(
        std::slice::from_ref(msg),
        body,
        op_editor_core::Locale::EnUs,
    )
    .remove(0)
}

fn painted_text(item: &crate::widgets::ai_chat_transcript::TranscriptItem) -> String {
    item.bubble
        .iter()
        .chain(&item.flow_bubbles)
        .flat_map(|bubble| bubble.lines.iter().cloned())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_credential_trails_a_plain_reply_as_a_collapsed_step() {
    let item = laid_out(&finished(CREDENTIAL));
    let text = painted_text(&item);
    assert!(text.contains("Done."), "{text}");
    assert!(
        !text.contains('▸') && !text.contains("auto-repair"),
        "{text}"
    );
    let receipt = item.steps.last().expect("receipt step");
    assert_eq!(receipt.label, "10 auto-repair(s) applied, no issues left");
    assert!(receipt.done && !receipt.expanded);
    let bubble = item.bubble.as_ref().expect("prose bubble");
    assert!(receipt.rect.origin.y >= bubble.rect.origin.y + bubble.rect.size.y);
}

#[test]
fn the_credential_trails_an_interleaved_tool_flow_too() {
    let narration = "Building the screen.\n\n";
    let mut msg = finished(&format!(
        "{narration}All set.{}",
        &CREDENTIAL["Done.".len()..]
    ));
    msg.tool_calls = vec![op_editor_core::chat::ChatToolCall {
        name: "batch_design".into(),
        args: "{}".into(),
        content_offset: Some(narration.len() as u32),
    }];
    let item = laid_out(&msg);
    assert!(
        !item.flow_panels.is_empty(),
        "the tool call keeps its flow panel"
    );
    let text = painted_text(&item);
    assert!(text.contains("All set."), "{text}");
    assert!(
        !text.contains('▸') && !text.contains("auto-repair"),
        "{text}"
    );
    let receipt = item.steps.last().expect("receipt step");
    assert_eq!(receipt.label, "10 auto-repair(s) applied, no issues left");
    let flow_bottom = item
        .flow_bubbles
        .iter()
        .map(|bubble| bubble.rect.origin.y + bubble.rect.size.y)
        .fold(0.0, f32::max);
    assert!(
        receipt.rect.origin.y >= flow_bottom,
        "receipt sits below the flow"
    );
}
