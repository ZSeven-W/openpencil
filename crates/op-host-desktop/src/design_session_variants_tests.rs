//! Tests for the design pump's side-by-side directions handling.

use super::*;
use op_editor_core::{HomeFamily, TaskDraft, WorkspaceVariant};

fn landed(index: usize) -> WorkspaceVariant {
    WorkspaceVariant {
        index,
        name: format!("方案 {}", op_editor_core::variant_letter(index)),
        style_guide: "zen-paper-light".into(),
        style_label: "Zen Paper Light".into(),
        name_prefix: String::new(),
        root_ids: vec![format!("r{index}")],
        variables: None,
        themes: None,
    }
}

#[test]
fn landed_directions_are_recorded_on_the_active_workspace() {
    let mut state = EditorState::new();
    let progress = vec![
        Progress::VariantReady(landed(1)),
        Progress::CleanupDone,
        Progress::VariantReady(landed(0)),
    ];
    // No workspace: nothing to record on.
    assert!(!fold_variant_progress(&mut state, &progress));

    state.editor_ui.workspace.open_for_generation(
        HomeFamily::AppUi,
        "brief",
        TaskDraft::default(),
        0,
        1,
        None,
    );
    state.editor_ui.workspace.begin_variants(3);
    assert!(fold_variant_progress(&mut state, &progress));
    let slots: Vec<usize> = state
        .editor_ui
        .workspace
        .variants
        .iter()
        .map(|v| v.index)
        .collect();
    assert_eq!(slots, vec![0, 1]);
}

#[test]
fn landings_and_failures_are_narrated() {
    let mut msg = ChatMessage::assistant_streaming();
    assert!(narrate_variant(
        &mut msg,
        &Progress::VariantReady(landed(0)),
        Locale::ZhCn
    ));
    assert!(narrate_variant(
        &mut msg,
        &Progress::VariantFailed {
            index: 1,
            name: "方案 B".into(),
            error: "provider quota".into(),
        },
        Locale::ZhCn
    ));
    assert!(
        msg.content.contains("方案 A 已完成（Zen Paper Light）"),
        "{}",
        msg.content
    );
    assert!(
        msg.content.contains("方案 B 没有生成出来：provider quota"),
        "{}",
        msg.content
    );
    assert!(!narrate_variant(
        &mut msg,
        &Progress::CleanupDone,
        Locale::ZhCn
    ));
}
