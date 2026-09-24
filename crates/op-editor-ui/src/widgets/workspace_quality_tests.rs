//! The 质检 chip + panel: hit-test and paint walk the same geometry, and the
//! panel only ever shows a finished, audited report.

use super::*;
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::{PaintCx, Widget, WorkspaceSurface};
use op_editor_core::{
    HomeFamily, Locale, QualityItem, QualityRepairRecord, QualityReport, QualityTopic,
    WorkspacePhase, WorkspaceState, WorkspaceView,
};

fn report() -> QualityReport {
    let mut report = QualityReport::default();
    report.ingest_repairs(
        &["overflow".into(), "layout".into()],
        &[QualityRepairRecord {
            pass: "geometry-validation".into(),
            family: "overflow".into(),
            node_id: "t1".into(),
            node_name: Some("Title".into()),
            detail: "width 420 → 327".into(),
        }],
        &[],
    );
    report.ingest_audit(
        &[QualityTopic::Contrast, QualityTopic::Completeness],
        vec![
            QualityItem {
                topic: QualityTopic::Contrast,
                source: "text-bg-contrast".into(),
                node_id: Some("t2".into()),
                node_name: Some("Caption".into()),
                board_id: Some("board-0".into()),
                detail: "text/bg contrast 1.10:1 below 2.5:1".into(),
            },
            QualityItem {
                topic: QualityTopic::Contrast,
                source: "text-bg-contrast".into(),
                node_id: None,
                node_name: None,
                board_id: None,
                detail: "document-level".into(),
            },
        ],
    );
    report
}

fn editor(phase: WorkspacePhase, open: bool) -> op_editor_core::EditorState {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "board-0", "x": 0, "y": 0, "width": 375, "height": 812,
          "children": [ { "type": "text", "id": "t2", "content": "Hi" } ] } ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut editor = op_editor_core::EditorState::from_document(document);
    let mut report = report();
    report.attribute_boards(&editor, &["board-0".to_string()]);
    editor.editor_ui.locale = Locale::ZhCn;
    editor.editor_ui.workspace = WorkspaceState {
        visible: true,
        active: true,
        family: HomeFamily::AppUi,
        view: WorkspaceView::AllBoards,
        phase,
        quality: Some(report),
        quality_open: open,
        ..WorkspaceState::default()
    };
    editor.editor_ui.layer_panel_width = 320.0;
    editor.editor_ui.sidebar_open = true;
    editor
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn the_chip_sits_left_of_export_and_answers_its_own_press() {
    let editor = editor(WorkspacePhase::Done, false);
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("visible");
    let layout = surface.layout(1440.0, 900.0);
    let chip = surface
        .quality_chip(&layout)
        .expect("finished run shows the chip");
    assert!(chip.origin.x + chip.size.x < layout.export.origin.x);
    assert_eq!(chip.origin.y, layout.export.origin.y);
    assert!(chip.origin.x > layout.title.origin.x);
    assert_eq!(
        surface.hit_test_layout(&layout, centre(chip)),
        Some(WorkspaceHit::QualityChip)
    );
    let label = surface.quality_label().expect("label");
    assert!(label.contains('1'), "{label}");
    assert!(label.contains('2'), "{label}");
    assert!(surface.quality_panel(&layout).is_none(), "closed chip");
}

#[test]
fn no_chip_while_generating_or_after_a_stop() {
    for phase in [
        WorkspacePhase::Generating,
        WorkspacePhase::Stopped,
        WorkspacePhase::Failed,
    ] {
        let editor = editor(phase, true);
        let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("visible");
        let layout = surface.layout(1440.0, 900.0);
        assert!(surface.quality_chip(&layout).is_none(), "{phase:?}");
        assert!(surface.quality_panel(&layout).is_none(), "{phase:?}");
    }
}

#[test]
fn an_unaudited_report_shows_no_chip() {
    let mut editor = editor(WorkspacePhase::Done, false);
    if let Some(report) = editor.editor_ui.workspace.quality.as_mut() {
        report.audited = false;
    }
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("visible");
    let layout = surface.layout(1440.0, 900.0);
    assert!(surface.quality_chip(&layout).is_none());
}

#[test]
fn the_open_panel_lists_topics_and_routes_item_presses() {
    let editor = editor(WorkspacePhase::Done, true);
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("visible");
    let layout = surface.layout(1440.0, 900.0);
    let panel = surface.quality_panel(&layout).expect("open panel");
    assert!(!panel.truncated);
    assert!(panel.panel.origin.x + panel.panel.size.x <= 1440.0);
    let kinds: Vec<_> = panel.rows.iter().map(|row| row.kind).collect();
    let report = editor.editor_ui.workspace.quality.as_ref().expect("report");
    let contrast = report
        .topics
        .iter()
        .position(|entry| entry.topic == QualityTopic::Contrast)
        .expect("contrast topic");
    assert!(kinds.contains(&QualityRowKind::Topic { topic: contrast }));
    assert!(kinds.contains(&QualityRowKind::Remaining {
        topic: contrast,
        item: 0
    }));
    assert!(kinds.contains(&QualityRowKind::BoardsHeader));
    // Rows never overlap and stay inside the panel.
    for pair in panel.rows.windows(2) {
        assert!(pair[0].rect.origin.y + pair[0].rect.size.y <= pair[1].rect.origin.y + 0.01);
    }
    for row in &panel.rows {
        assert!(panel.panel.contains(centre(row.rect)));
    }

    let item_row = panel
        .rows
        .iter()
        .find(|row| {
            row.kind
                == QualityRowKind::Remaining {
                    topic: contrast,
                    item: 0,
                }
        })
        .expect("item row");
    assert_eq!(
        surface.hit_test_layout(&layout, centre(item_row.rect)),
        Some(WorkspaceHit::QualityItem {
            topic: contrast,
            item: 0
        })
    );
    // The panel floats over the canvas: its blank area is swallowed, not
    // passed to the canvas tier.
    let head = centre(panel.head);
    assert_eq!(
        surface.hit_test_layout(&layout, head),
        Some(WorkspaceHit::QualityPanel)
    );
}

#[test]
fn the_panel_truncates_instead_of_running_off_a_short_viewport() {
    let mut report = report();
    report.ingest_audit(
        &[QualityTopic::Spacing],
        (0..40)
            .map(|index| QualityItem {
                topic: QualityTopic::Spacing,
                source: "edge-section-padding".into(),
                node_id: Some(format!("n{index}")),
                node_name: None,
                board_id: None,
                detail: "padding".into(),
            })
            .collect(),
    );
    let chip = Rect::xywh(900.0, 16.0, 200.0, 32.0);
    let panel = quality_panel_layout(chip, 1200.0, 260.0, &report);
    assert!(panel.panel.origin.y + panel.panel.size.y <= 260.0);
    // 40 findings fold into four rows plus "36 more" when there is room;
    // here the viewport is too short even for that, so rows are cut.
    assert!(panel.truncated);
}

#[test]
fn paint_draws_the_chip_label_and_the_panel_rows_where_hit_test_finds_them() {
    let editor = editor(WorkspacePhase::Done, true);
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("visible");
    let layout = surface.layout(1440.0, 900.0);
    let chip = surface.quality_chip(&layout).expect("chip");
    let label = surface.quality_label().expect("label");

    let mut backend = CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        surface.paint(&mut cx, Rect::xywh(0.0, 0.0, 1440.0, 900.0));
    }
    let painted_chip = backend.round_fills.iter().any(|(rect, _, _)| *rect == chip);
    assert!(painted_chip, "chip plate painted at the hit-test rect");
    let (_, origin) = backend
        .texts
        .iter()
        .find(|(text, _)| *text == label)
        .expect("chip label painted");
    assert!(chip.contains(Point2D::new(origin.x + 1.0, origin.y - 4.0)));

    let panel = surface.quality_panel(&layout).expect("panel");
    let mut overlay = CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut overlay,
        };
        surface.paint_quality_overlay(&mut cx, Rect::xywh(0.0, 0.0, 1440.0, 900.0));
    }
    assert!(overlay
        .round_fills
        .iter()
        .any(|(rect, _, _)| *rect == panel.panel));
    let title = op_i18n::translate(Locale::ZhCn, "workspace.quality.title");
    assert!(overlay.texts.iter().any(|(text, _)| text == title));
    let contrast_label = op_i18n::translate(Locale::ZhCn, "workspace.quality.topic.contrast");
    let (_, topic_origin) = overlay
        .texts
        .iter()
        .find(|(text, _)| text == contrast_label)
        .expect("topic label painted");
    assert!(
        panel
            .rows
            .iter()
            .any(|row| matches!(row.kind, QualityRowKind::Topic { .. })
                && row
                    .rect
                    .contains(Point2D::new(topic_origin.x + 1.0, topic_origin.y - 4.0))),
        "topic label sits inside its hit-test row"
    );
}
