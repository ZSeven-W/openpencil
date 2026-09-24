//! Host wiring for the workspace's 质检 chip: the chip toggles the panel,
//! a remaining-issue row selects its node, and a press elsewhere closes it.

use super::WidgetHostNative;
use op_editor_core::{HomeFamily, QualityItem, QualityReport, QualityTopic, Tool, WorkspacePhase};
use op_editor_ui::widgets::{QualityRowKind, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn finished_host() -> WidgetHostNative {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "board-0", "x": 0, "y": 0, "width": 375, "height": 812,
          "children": [ { "type": "text", "id": "caption", "name": "Caption",
                          "x": 20, "y": 40, "content": "Hi" } ] } ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(op_editor_core::EditorState::from_document(document));
    host.set_now_ms(2_000);
    let editor = host.editor_state_mut();
    editor.tool = Tool::Hand;
    editor.editor_ui.open_workspace_for_generation(
        HomeFamily::AppUi,
        "登录页",
        op_editor_core::TaskDraft::default(),
        0,
        1_000,
        Some(Tool::Select),
    );
    let mut report = QualityReport::default();
    report.ingest_audit(
        &[QualityTopic::Contrast],
        vec![QualityItem {
            topic: QualityTopic::Contrast,
            source: "text-bg-contrast".into(),
            node_id: Some("caption".into()),
            node_name: Some("Caption".into()),
            board_id: Some("board-0".into()),
            detail: "text/bg contrast 1.10:1 below 2.5:1".into(),
        }],
    );
    editor.editor_ui.workspace.quality = Some(report);
    editor.editor_ui.workspace.phase = WorkspacePhase::Done;
    host
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn chip(host: &WidgetHostNative) -> Rect {
    let surface = WorkspaceSurface::for_editor_at(host.editor_state(), 0).expect("visible");
    let layout = surface.layout(W, H);
    surface.quality_chip(&layout).expect("chip")
}

#[test]
fn the_chip_toggles_the_report_panel() {
    let mut host = finished_host();
    let point = centre(chip(&host));
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(host.editor_state().editor_ui.workspace.quality_open);
    host.apply_release_with_viewport(W, H);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert!(!host.editor_state().editor_ui.workspace.quality_open);
}

#[test]
fn a_remaining_row_selects_its_node_and_keeps_the_panel_open() {
    let mut host = finished_host();
    host.editor_state_mut().editor_ui.workspace.quality_open = true;
    let row = {
        let surface = WorkspaceSurface::for_editor_at(host.editor_state(), 0).expect("visible");
        let layout = surface.layout(W, H);
        let panel = surface.quality_panel(&layout).expect("panel");
        panel
            .rows
            .iter()
            .find(|row| matches!(row.kind, QualityRowKind::Remaining { .. }))
            .expect("remaining row")
            .rect
    };
    let point = centre(row);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert_eq!(host.editor_state().selection.anchor.as_str(), "caption");
    assert!(host.editor_state().editor_ui.workspace.quality_open);
}

#[test]
fn a_press_outside_the_panel_closes_it() {
    let mut host = finished_host();
    host.editor_state_mut().editor_ui.workspace.quality_open = true;
    // The canvas, well clear of the panel (which hangs under the chip on
    // the right).
    host.apply_press(500.0, 700.0, W, H);
    assert!(!host.editor_state().editor_ui.workspace.quality_open);
}

#[test]
fn a_new_run_clears_the_previous_report() {
    let mut host = finished_host();
    host.editor_state_mut().editor_ui.workspace.quality_open = true;
    host.editor_state_mut()
        .editor_ui
        .workspace
        .resume_generating(7);
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(workspace.quality.is_none());
    assert!(!workspace.quality_open);
}
