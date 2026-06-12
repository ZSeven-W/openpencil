//! `EditorCommand::Batch` atomic-apply tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::sample;

fn insert(name: &str) -> EditorCommand {
    EditorCommand::InsertNode {
        kind: "rect".into(),
        name: name.into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: NodeId::NONE,
        page_id: None,
    }
}

#[test]
fn batch_applies_every_sub_command_in_order() {
    let mut s = sample();
    let before = s.active_children().len();
    assert!(s.apply(EditorCommand::Batch {
        commands: vec![
            insert("A"),
            EditorCommand::SetNodeName {
                node_id: NodeId::new("n11"),
                name: "Renamed".into(),
            },
        ],
    }));
    assert_eq!(s.active_children().len(), before + 1);
    let title = crate::walkers::find_node(s.active_children(), &NodeId::new("n11")).unwrap();
    assert_eq!(title.base().name.as_deref(), Some("Renamed"));
}

#[test]
fn batch_rolls_back_on_mid_batch_failure() {
    let mut s = sample();
    // Seed one undo entry AND one redo entry so the depths are
    // observable across the failed batch. Plain `InsertNode` pushes
    // no history at apply time (the Batch wrapper / host owns that),
    // so seed with selection-scoped edits, which do.
    assert!(s.apply(EditorCommand::NudgeSelected { dx: 1, dy: 0 }));
    assert!(s.apply(EditorCommand::NudgeSelected { dx: 0, dy: 1 }));
    assert!(s.undo());
    let before_doc = serde_json::to_string(&s.doc).unwrap();
    let before_selection = s.selection.clone();
    assert_eq!(s.history.past.len(), 1);
    assert_eq!(s.history.future.len(), 1);
    assert!(!s.apply(EditorCommand::Batch {
        commands: vec![
            insert("A"),
            // Unknown node id — rejects at apply time.
            EditorCommand::DeleteNode {
                node_id: NodeId::new("nope"),
                page_id: None,
            },
        ],
    }));
    assert_eq!(
        serde_json::to_string(&s.doc).unwrap(),
        before_doc,
        "a failed batch must leave the document untouched"
    );
    assert_eq!(
        s.selection, before_selection,
        "a failed batch must leave the selection untouched"
    );
    assert_eq!(
        s.history.past.len(),
        1,
        "a failed batch must leave the undo depth untouched"
    );
    assert_eq!(
        s.history.future.len(),
        1,
        "a failed batch must leave the redo stack untouched"
    );
}

#[test]
fn batch_rejects_empty_and_nested_batches() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::Batch { commands: vec![] }));
    let before_doc = serde_json::to_string(&s.doc).unwrap();
    assert!(!s.apply(EditorCommand::Batch {
        commands: vec![
            insert("A"),
            EditorCommand::Batch {
                commands: vec![insert("B")],
            },
        ],
    }));
    assert_eq!(
        serde_json::to_string(&s.doc).unwrap(),
        before_doc,
        "a nested batch must reject the outer batch up front"
    );
}

#[test]
fn batch_rejects_non_document_sub_commands_up_front_with_no_state_change() {
    // Commands whose primary state lives outside the rollback
    // snapshot reject the WHOLE batch before any sub-command runs —
    // including document commands listed BEFORE the offender.
    let offenders = vec![
        EditorCommand::SetActiveTool {
            tool: "hand".into(),
        },
        EditorCommand::SetViewport {
            pan_x: Some(10),
            pan_y: Some(10),
            zoom_percent: Some(200),
        },
        EditorCommand::CopySelected,
        EditorCommand::CutSelected,
        EditorCommand::PasteClipboard { offset_px: 10 },
        EditorCommand::Undo,
        EditorCommand::Redo,
        EditorCommand::SetActiveAxisValue {
            axis: "mode".into(),
            value: "dark".into(),
        },
        EditorCommand::CycleActiveAxisValue {
            axis: "mode".into(),
        },
    ];
    for offender in offenders {
        // `sample()` selects `n11`, so `CopySelected` / `CutSelected`
        // WOULD succeed standalone — proving the rejection is the
        // classification, not a sub-command failure.
        let mut s = sample();
        let before_doc = serde_json::to_string(&s.doc).unwrap();
        let before_selection = s.selection.clone();
        let before_tool = s.tool;
        let before_viewport = s.viewport;
        assert!(
            !s.apply(EditorCommand::Batch {
                commands: vec![insert("A"), offender.clone(), insert("B")],
            }),
            "batch containing {offender:?} must be rejected"
        );
        assert_eq!(
            serde_json::to_string(&s.doc).unwrap(),
            before_doc,
            "rejecting {offender:?} must not run the preceding insert"
        );
        assert_eq!(s.selection, before_selection);
        assert_eq!(s.tool, before_tool);
        assert_eq!(s.viewport, before_viewport);
        assert!(s.clipboard.is_empty(), "clipboard must stay untouched");
        assert!(s.history.past.is_empty(), "no history entry may land");
        assert!(s.history.future.is_empty());
    }
}

#[test]
fn batch_rejection_runs_nothing_even_when_the_offender_leads() {
    let mut s = sample();
    let before = s.active_children().len();
    // `SetActiveTool` would succeed standalone; as the FIRST batch
    // entry it must still be rejected up front — the tool stays put
    // and the following insert never runs.
    assert!(!s.apply(EditorCommand::Batch {
        commands: vec![
            EditorCommand::SetActiveTool {
                tool: "hand".into(),
            },
            insert("A"),
        ],
    }));
    assert_eq!(s.tool, crate::tool::Tool::Select);
    assert_eq!(s.active_children().len(), before);
    assert!(s.history.past.is_empty());
}

#[test]
fn batch_lands_as_a_single_undo_step_and_failure_preserves_redo() {
    let mut s = sample();
    let before_doc = serde_json::to_string(&s.doc).unwrap();
    assert!(s.apply(EditorCommand::Batch {
        commands: vec![insert("A"), insert("B")],
    }));
    let after_doc = serde_json::to_string(&s.doc).unwrap();
    assert_ne!(before_doc, after_doc);
    // One undo reverts the WHOLE batch.
    assert!(s.undo());
    assert_eq!(serde_json::to_string(&s.doc).unwrap(), before_doc);
    // Redo stack now holds the batch; a FAILED batch must not eat it.
    assert!(!s.apply(EditorCommand::Batch {
        commands: vec![EditorCommand::DeleteNode {
            node_id: NodeId::new("nope"),
            page_id: None,
        }],
    }));
    assert!(s.redo(), "redo must survive a rolled-back batch");
    assert_eq!(serde_json::to_string(&s.doc).unwrap(), after_doc);
}
