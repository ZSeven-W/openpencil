//! The standard web turn's quality report: the daemon folds the run's
//! quality progress while the orchestrator streams, audits the boards it
//! drew when the run succeeds, and sends the finished report to the browser
//! as one `{"qualityReport": …}` SSE frame ahead of `done`. The browser
//! stamps it onto the Studio workspace of the run that asked for it.
//!
//! Folding and auditing are the desktop session's own helpers
//! (`crate::run_quality`), so both hosts report the same numbers.

use std::io::Write;
use std::sync::Mutex;

use op_editor_core::QualityReport;
use op_orchestrator::RunSummary;

use crate::web_canvas_server::WebCanvasState;

/// The report a successful run earned: `None` when the run never reported
/// a quality check (nothing to vouch for) or the audit left nothing to
/// show. Audits the live document the browser is pulling, not the sink's
/// mirror, so the counts match the boards on screen.
pub(super) fn finished_run_report(
    folded: Option<QualityReport>,
    state: &Mutex<WebCanvasState>,
    summary: &RunSummary,
) -> Option<QualityReport> {
    let report = folded.filter(|report| !report.audited)?;
    let guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let report = crate::run_quality::audit_run_report(report, &guard.editor, summary);
    (!report.is_empty()).then_some(report)
}

/// `data: {"qualityReport": …}` — the browser's `AiEvent::QualityReport`.
pub(super) fn write_quality_report_event<W: Write>(
    out: &mut W,
    report: &QualityReport,
) -> std::io::Result<()> {
    let payload = serde_json::json!({ "qualityReport": report });
    out.write_all(format!("data: {payload}\n\n").as_bytes())?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{EditorState, QualityRepairRecord};
    use op_orchestrator::{Progress, SubtaskOutcome};
    use serde_json::json;

    fn state_with_board() -> Mutex<WebCanvasState> {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
            "version": "1.0",
            "children": [
                {"type": "frame", "id": "home", "name": "Home", "layout": "vertical",
                 "width": 390, "height": 844,
                 "fill": [{"type": "solid", "color": "#FFFFFF"}],
                 "children": [
                    {"type": "text", "id": "title", "name": "Title", "content": "Hi",
                     "fill": [{"type": "solid", "color": "#111111"}]}
                 ]}
            ]
        }))
        .expect("doc");
        Mutex::new(WebCanvasState::new(EditorState::from_document(doc), 3100))
    }

    fn summary() -> RunSummary {
        RunSummary {
            root_frame_id: "home".into(),
            subtasks: vec![SubtaskOutcome {
                id: "s1".into(),
                node_count: 2,
                error: None,
                inserted_root_ids: vec!["title".into()],
                headline: None,
                subtask: None,
            }],
            total_nodes: 2,
            unfilled_screens: Vec::new(),
            incomplete_subtask_failure: false,
        }
    }

    fn folded_report() -> Option<QualityReport> {
        let mut slot = None;
        let event = Progress::QualityChecked {
            checks: vec!["overflow".into()],
            repairs: vec![("overflow".into(), 1)],
            records: Vec::new(),
            notes: Vec::new(),
            items: vec![QualityRepairRecord {
                pass: "geometry-validation".into(),
                family: "overflow".into(),
                node_id: "title".into(),
                node_name: Some("Title".into()),
                detail: "width 420 → 327".into(),
            }],
        };
        assert!(crate::run_quality::fold_quality_event(&mut slot, &event));
        slot
    }

    #[test]
    fn a_run_that_reported_checks_ships_an_audited_report() {
        let state = state_with_board();
        let report =
            finished_run_report(folded_report(), &state, &summary()).expect("audited report");
        assert!(report.audited);
        assert_eq!(report.total_fixed(), 1);
        assert_eq!(report.boards.len(), 1);
        assert_eq!(report.boards[0].board_id, "home");
    }

    #[test]
    fn a_run_without_quality_checks_ships_nothing() {
        let state = state_with_board();
        assert!(finished_run_report(None, &state, &summary()).is_none());
        let mut already = folded_report().expect("report");
        already.audited = true;
        assert!(finished_run_report(Some(already), &state, &summary()).is_none());
    }

    #[test]
    fn the_wire_frame_round_trips_the_report() {
        let state = state_with_board();
        let report = finished_run_report(folded_report(), &state, &summary()).expect("report");
        let mut out = Vec::new();
        write_quality_report_event(&mut out, &report).expect("write");
        let text = String::from_utf8(out).expect("utf8");
        let payload = text
            .strip_prefix("data: ")
            .and_then(|rest| rest.strip_suffix("\n\n"))
            .expect("one SSE frame");
        let value: serde_json::Value = serde_json::from_str(payload).expect("json");
        let decoded: QualityReport =
            serde_json::from_value(value["qualityReport"].clone()).expect("report");
        assert_eq!(decoded, report);
    }
}
