//! Host plumbing for the quality report: progress events in, an audited
//! per-board report plus one transcript line out.

use std::collections::BTreeMap;

use super::*;
use op_editor_core::{ChatMessage, QualityRepairRecord, QualityReport, QualityTopic};
use op_orchestrator::SubtaskOutcome;
use serde_json::json;

fn state_with_boards() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [
            {"type": "frame", "id": "home", "name": "Home", "layout": "vertical",
             "width": 390, "height": 844,
             "fill": [{"type": "solid", "color": "#FFFFFF"}],
             "children": [
                {"type": "text", "id": "title", "name": "Title", "content": "Hi",
                 "fill": [{"type": "solid", "color": "#111111"}]},
                {"type": "text", "id": "faint", "name": "Caption", "content": "Hello",
                 "fill": [{"type": "solid", "color": "#F4F4F4"}]}
             ]},
            {"type": "frame", "id": "detail", "name": "Detail", "layout": "vertical",
             "width": 390, "height": 844,
             "children": [
                {"type": "text", "id": "body", "name": "Body", "content": "Text",
                 "fill": [{"type": "solid", "color": "#111111"}]}
             ]}
        ]
    }))
    .expect("doc");
    let mut state = EditorState::from_document(doc);
    state
        .chat
        .messages
        .push(ChatMessage::user("design two screens"));
    let mut assistant = ChatMessage::assistant_streaming();
    assistant.content = "Planned 2 screens.".into();
    state.chat.messages.push(assistant);
    state
}

fn quality_event() -> Progress {
    Progress::QualityChecked {
        checks: vec!["overflow".into(), "layout".into()],
        repairs: vec![("overflow".into(), 1), ("layout".into(), 1)],
        records: vec![
            "overflow · geometry-validation · Title [title] · width 420 → 327".into(),
            "layout · unify-section-margins · Body [body] · padding 0 → 16".into(),
        ],
        notes: Vec::new(),
        items: vec![
            QualityRepairRecord {
                pass: "geometry-validation".into(),
                family: "overflow".into(),
                node_id: "title".into(),
                node_name: Some("Title".into()),
                detail: "width 420 → 327".into(),
            },
            QualityRepairRecord {
                pass: "unify-section-margins".into(),
                family: "layout".into(),
                node_id: "body".into(),
                node_name: Some("Body".into()),
                detail: "padding 0 → 16".into(),
            },
        ],
    }
}

fn summary() -> RunSummary {
    let outcome = |id: &str, root: &str| SubtaskOutcome {
        id: id.into(),
        node_count: 3,
        error: None,
        inserted_root_ids: vec![root.into()],
        headline: None,
        subtask: None,
    };
    RunSummary {
        root_frame_id: "home".into(),
        subtasks: vec![outcome("s1", "title"), outcome("s2", "detail")],
        total_nodes: 6,
        unfilled_screens: Vec::new(),
        incomplete_subtask_failure: false,
    }
}

#[test]
fn progress_folds_into_the_report_and_the_run_end_audits_it() {
    let mut state = state_with_boards();
    let mut by_category = BTreeMap::new();
    by_category.insert("text-bg-contrast".to_string(), 1);
    let events = vec![
        Progress::CleanupDone,
        quality_event(),
        Progress::ValidationPreCheckDone {
            applied: 1,
            by_category,
        },
    ];
    assert!(fold_quality_progress(&mut state, &events));
    let live = state.editor_ui.workspace.quality.clone().expect("report");
    assert!(!live.audited, "remaining is unknown until the run ends");
    assert_eq!(live.total_fixed(), 3);

    assert!(finish_quality_report(
        &mut state,
        &summary(),
        None,
        Locale::ZhCn
    ));
    let report = state.editor_ui.workspace.quality.clone().expect("report");
    assert!(report.audited);
    assert_eq!(report.total_fixed(), 3);
    let contrast = report
        .topics
        .iter()
        .find(|entry| entry.topic == QualityTopic::Contrast)
        .expect("contrast topic");
    assert_eq!(contrast.remaining.len(), 1, "{:?}", contrast.remaining);
    assert_eq!(contrast.remaining[0].node_id.as_deref(), Some("faint"));
    // Per board: the overflow fix and the contrast finding are on Home,
    // the spacing fix on Detail.
    assert_eq!(report.boards.len(), 2);
    assert_eq!(
        (report.boards[0].board_id.as_str(), report.boards[0].fixed),
        ("home", 1)
    );
    assert_eq!(report.boards[0].remaining, 1);
    assert_eq!(
        (report.boards[1].board_id.as_str(), report.boards[1].fixed),
        ("detail", 1)
    );

    let transcript = &state.chat.messages[1].content;
    assert!(
        transcript.contains(&report.transcript_line(Locale::ZhCn)),
        "{transcript}"
    );
    // Finishing twice never appends a second line.
    assert!(!finish_quality_report(
        &mut state,
        &summary(),
        None,
        Locale::ZhCn
    ));
}

#[test]
fn worker_scoped_quality_events_are_folded_too() {
    let mut state = state_with_boards();
    let identity = op_orchestrator::agent_identity::AgentIdentity {
        name: "Ada".into(),
        color: "#336699".into(),
    };
    let scoped = Progress::worker_scoped(1, "Detail", identity, quality_event());
    assert!(fold_quality_progress(&mut state, &[scoped]));
    assert_eq!(
        state
            .editor_ui
            .workspace
            .quality
            .as_ref()
            .map(QualityReport::total_fixed),
        Some(2)
    );
}

#[test]
fn a_run_without_quality_events_gets_no_report_or_line() {
    let mut state = state_with_boards();
    assert!(!fold_quality_progress(&mut state, &[Progress::CleanupDone]));
    assert!(!finish_quality_report(
        &mut state,
        &summary(),
        None,
        Locale::EnUs
    ));
    assert!(state.editor_ui.workspace.quality.is_none());
    assert_eq!(state.chat.messages[1].content, "Planned 2 screens.");
}

#[test]
fn a_new_run_never_adds_to_a_finished_report() {
    let mut state = state_with_boards();
    fold_quality_progress(&mut state, &[quality_event()]);
    finish_quality_report(&mut state, &summary(), None, Locale::EnUs);
    fold_quality_progress(&mut state, &[quality_event()]);
    let report = state.editor_ui.workspace.quality.as_ref().expect("report");
    assert!(!report.audited);
    assert_eq!(report.total_fixed(), 2, "fresh report, not 4");
}

#[test]
fn run_boards_prefer_the_boards_the_summary_names() {
    let state = state_with_boards();
    let mut only_home = summary();
    only_home.subtasks.truncate(1);
    assert_eq!(run_boards(&state, &only_home), vec!["home".to_string()]);
    let mut none = summary();
    none.root_frame_id.clear();
    none.subtasks.clear();
    assert_eq!(
        run_boards(&state, &none),
        vec!["home".to_string(), "detail".to_string()]
    );
}
