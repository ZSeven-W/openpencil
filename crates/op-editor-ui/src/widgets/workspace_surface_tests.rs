//! Layout + hit-test coverage for the generation-workspace chrome.

use super::*;
use op_editor_core::WorkspacePhase;

fn layout_1440_900(family: HomeFamily, collapsed: bool, boards: usize) -> WorkspaceLayout {
    layout_for(1440.0, 900.0, family, 320.0, collapsed, boards)
}

#[test]
fn app_layout_docks_the_canvas_beside_the_320_chat() {
    let layout = layout_1440_900(HomeFamily::AppUi, false, 4);
    assert_eq!(layout.header, Rect::xywh(0.0, 0.0, 1440.0, 64.0));
    assert_eq!(
        layout.toolbar,
        Rect::xywh(320.0, 64.0, 1120.0, 44.0),
        "the toolbar starts at the dock edge"
    );
    assert_eq!(
        layout.dock,
        Some(Rect::xywh(0.0, 64.0, 320.0, 836.0)),
        "the dock runs the full height below the header"
    );
    assert_eq!(layout.canvas, Rect::xywh(320.0, 108.0, 1120.0, 792.0));
    assert_eq!(layout.strip, None, "App has no deck strip");
    assert_eq!(layout.view_segments.len(), 2, "全部 / 单屏");
}

#[test]
fn deck_layout_sits_the_canvas_above_the_strip() {
    let layout = layout_1440_900(HomeFamily::Presentation, false, 5);
    let strip = layout.strip.expect("presentation paints the strip");
    assert_eq!(strip, Rect::xywh(320.0, 790.0, 1120.0, 110.0));
    assert_eq!(
        layout.canvas,
        Rect::xywh(320.0, 108.0, 1120.0, 682.0),
        "the canvas bottom must equal the strip top"
    );
    assert_eq!(layout.thumbs.len(), 5);
    assert!(layout.play.is_some());
}

#[test]
fn collapsed_dock_gives_the_canvas_the_full_width() {
    let layout = layout_1440_900(HomeFamily::AppUi, true, 3);
    assert_eq!(layout.dock, None);
    assert_eq!(layout.dock_handle, None);
    assert_eq!(layout.canvas.origin.x, 0.0);
    assert_eq!(layout.canvas, Rect::xywh(0.0, 108.0, 1440.0, 792.0));
}

#[test]
fn narrow_viewport_keeps_the_same_band_structure() {
    let layout = layout_for(1180.0, 820.0, HomeFamily::Web, 320.0, false, 1);
    assert_eq!(layout.header.size.y, 64.0);
    assert_eq!(layout.toolbar, Rect::xywh(320.0, 64.0, 860.0, 44.0));
    assert_eq!(layout.canvas, Rect::xywh(320.0, 108.0, 860.0, 712.0));
    assert_eq!(layout.view_segments.len(), 1, "长页 is the only segment");
}

#[test]
fn header_actions_stay_right_aligned() {
    let layout = layout_1440_900(HomeFamily::AppUi, false, 2);
    assert_eq!(
        layout.professional.origin.x + layout.professional.size.x,
        1440.0 - 14.0
    );
    assert_eq!(
        layout.export.origin.x + layout.export.size.x,
        1440.0 - 14.0 - 104.0 - 8.0
    );
    assert_eq!(layout.back, Rect::xywh(80.0, 14.0, 36.0, 36.0));
}

/// An editor whose active page holds `boards` frames — the shape a
/// generated document has (same `load_str` seeding the preview host
/// tests use).
fn editor_with_boards(boards: usize) -> op_editor_core::EditorState {
    let mut children = String::new();
    for index in 0..boards {
        children.push_str(&format!(
            r#"{{ "type": "frame", "id": "board-{index}", "x": {x}, "y": 0,
                "width": 375, "height": 812, "children": [] }},"#,
            x = index * 420
        ));
    }
    let children = children.trim_end_matches(',');
    let source = format!(r#"{{ "version": "1.0.0", "children": [{children}] }}"#);
    let document = jian_ops_schema::load_str(&source)
        .expect("parse workspace fixture")
        .value;
    op_editor_core::EditorState::from_document(document)
}

fn surface_hit(
    family: HomeFamily,
    view: WorkspaceView,
    collapsed: bool,
    boards: usize,
    point: Point2D,
) -> Option<WorkspaceHit> {
    let mut editor = editor_with_boards(boards);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        family,
        view,
        // A finished result: Present is only live once the run is Done.
        phase: WorkspacePhase::Done,
        ..op_editor_core::WorkspaceState::default()
    };
    // The dock IS the left panel: the surface reads the panel's own
    // width / open flag, so the fixture sets those, not workspace
    // fields. 320 keeps every hard-coded coordinate below valid.
    editor.editor_ui.layer_panel_width = 320.0;
    editor.editor_ui.sidebar_open = !collapsed;
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    surface.hit_test(1440.0, 900.0, point)
}

#[test]
fn hit_test_resolves_the_chrome_targets() {
    let layout = layout_1440_900(HomeFamily::AppUi, false, 4);
    let at = |rect: Rect| Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0);
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.back)
        ),
        Some(WorkspaceHit::Back)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.export)
        ),
        Some(WorkspaceHit::Export)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.professional)
        ),
        Some(WorkspaceHit::Professional)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.zoom_in)
        ),
        Some(WorkspaceHit::ZoomIn)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.zoom_out)
        ),
        Some(WorkspaceHit::ZoomOut)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.zoom_fit)
        ),
        Some(WorkspaceHit::ZoomFit)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.view_segments[0])
        ),
        Some(WorkspaceHit::View(WorkspaceView::AllBoards))
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            at(layout.view_segments[1])
        ),
        Some(WorkspaceHit::View(WorkspaceView::Single { index: 0 }))
    );
}

#[test]
fn hit_test_resolves_the_deck_strip_targets() {
    let layout = layout_1440_900(HomeFamily::Presentation, false, 5);
    let view = WorkspaceView::Single { index: 0 };
    let at = |rect: Rect| Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0);
    assert_eq!(
        surface_hit(
            HomeFamily::Presentation,
            view,
            false,
            5,
            at(layout.thumbs[2])
        ),
        Some(WorkspaceHit::Thumb(2))
    );
    assert_eq!(
        surface_hit(
            HomeFamily::Presentation,
            view,
            false,
            5,
            at(layout.overview)
        ),
        Some(WorkspaceHit::Overview)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::Presentation,
            view,
            false,
            5,
            at(layout.play.unwrap())
        ),
        Some(WorkspaceHit::Play)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::Presentation,
            view,
            false,
            5,
            at(layout.prev.unwrap())
        ),
        Some(WorkspaceHit::Prev)
    );
    assert_eq!(
        surface_hit(
            HomeFamily::Presentation,
            view,
            false,
            5,
            at(layout.next.unwrap())
        ),
        Some(WorkspaceHit::Next)
    );
}

#[test]
fn canvas_and_dock_points_fall_through_to_lower_tiers() {
    let layout = layout_1440_900(HomeFamily::AppUi, false, 4);
    let canvas_point = Point2D::new(
        layout.canvas.origin.x + 100.0,
        layout.canvas.origin.y + 100.0,
    );
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            canvas_point
        ),
        None,
        "the canvas belongs to the canvas tier, not the chrome"
    );
    let chat_point = Point2D::new(160.0, 400.0);
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            chat_point
        ),
        None,
        "the dock body belongs to the chat panel"
    );
    let handle = layout.dock_handle.unwrap();
    assert_eq!(
        surface_hit(
            HomeFamily::AppUi,
            WorkspaceView::AllBoards,
            false,
            4,
            Point2D::new(handle.origin.x + 2.0, 500.0)
        ),
        Some(WorkspaceHit::DockResize),
        "the dock edge is the resize handle"
    );
}

#[test]
fn failed_phase_surfaces_the_banner_buttons() {
    let mut editor = editor_with_boards(2);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        phase: WorkspacePhase::Failed,
        ..op_editor_core::WorkspaceState::default()
    };
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    let (retry, return_edit) = surface.banner_buttons(&layout).expect("failed banner");
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(retry.origin.x + 2.0, retry.origin.y + 2.0)
        ),
        Some(WorkspaceHit::Retry)
    );
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(return_edit.origin.x + 2.0, return_edit.origin.y + 2.0)
        ),
        Some(WorkspaceHit::ReturnEdit)
    );
    // Non-failed phases show no banner.
    let mut ok = editor;
    ok.editor_ui.workspace.phase = WorkspacePhase::Done;
    let ok_surface = WorkspaceSurface::for_editor_at(&ok, 0).expect("workspace visible");
    let ok_layout = ok_surface.layout(1440.0, 900.0);
    assert!(ok_surface.banner_buttons(&ok_layout).is_none());
}

#[test]
fn a_waiting_template_draft_surfaces_its_banner_action_over_the_canvas() {
    let mut editor = editor_with_boards(2);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        ..op_editor_core::WorkspaceState::default()
    };
    editor
        .editor_ui
        .workspace
        .adopt_template_draft("knowledge-carousel", false);
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    let button = surface.draft_banner_button(&layout).expect("draft banner");
    assert!(layout.canvas.contains(Point2D::new(
        button.origin.x + button.size.x / 2.0,
        button.origin.y + button.size.y / 2.0
    )));
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(button.origin.x + 2.0, button.origin.y + 2.0)
        ),
        Some(WorkspaceHit::DraftAction)
    );

    // The banner paints in the canvas pass, not with the chrome: painted
    // under the canvas background its button was invisible.
    let mut chrome = crate::widgets::test_capture_backend::CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut chrome,
        };
        surface.paint(&mut cx, Rect::xywh(0.0, 0.0, 1440.0, 900.0));
    }
    assert!(!chrome
        .round_fills
        .iter()
        .any(|(rect, _, _)| *rect == button));
    let mut banners = crate::widgets::test_capture_backend::CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut banners,
        };
        surface.paint_canvas_banners(&mut cx, Rect::xywh(0.0, 0.0, 1440.0, 900.0));
    }
    assert!(banners
        .round_fills
        .iter()
        .any(|(rect, _, _)| *rect == button));
    let connect = op_i18n::translate(editor.editor_ui.locale, "home.submit.connect");
    assert!(banners.texts.iter().any(|(text, _)| text == connect));

    // Once the refine is under way the banner retires.
    editor.editor_ui.workspace.begin_draft_refine();
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    assert!(surface.draft_banner_button(&layout).is_none());
}

#[test]
fn the_header_back_circle_clears_the_macos_traffic_lights() {
    // The frameless desktop window draws the close/minimize/zoom buttons
    // over the app's own header at roughly x 12..72. Anything the user
    // must click has to start after that band or the window buttons
    // swallow the press (measured 2026-09-13: 返回首页 was unclickable at
    // inset 14 — the press reached the window, not the surface).
    const TRAFFIC_LIGHT_RIGHT_EDGE: f32 = 72.0;
    let mut editor = editor_with_boards(2);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        ..op_editor_core::WorkspaceState::default()
    };
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    for (vw, vh) in [(1440.0_f32, 900.0_f32), (1180.0, 820.0)] {
        let layout = surface.layout(vw, vh);
        assert!(
            layout.back.origin.x >= TRAFFIC_LIGHT_RIGHT_EDGE,
            "back circle at {} overlaps the traffic lights",
            layout.back.origin.x
        );
        assert!(layout.back.origin.x + layout.back.size.x < layout.doc_tile.origin.x);
        assert_eq!(
            surface.hit_test_layout(
                &layout,
                Point2D::new(
                    layout.back.origin.x + layout.back.size.x / 2.0,
                    layout.back.origin.y + layout.back.size.y / 2.0,
                ),
            ),
            Some(WorkspaceHit::Back)
        );
    }
}

#[test]
fn the_strip_tiles_share_the_thumbnail_rows_geometry() {
    // 总览 / 放映 are cells of the same row as the thumbnails, so all
    // three kinds of cell must share a top edge and a height. Sizing the
    // tiles on their own put them on a different baseline from the
    // thumbnails they sit beside.
    let mut editor = editor_with_boards(5);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        family: HomeFamily::Presentation,
        ..op_editor_core::WorkspaceState::default()
    };
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    let strip = layout.strip.expect("presentation paints a strip");
    let play = layout.play.expect("presentation paints 放映");
    let thumb = *layout.thumbs.first().expect("at least one thumbnail");
    for cell in [layout.overview, play] {
        assert_eq!(cell.origin.y, thumb.origin.y, "cells share a top edge");
        assert_eq!(cell.size.y, thumb.size.y, "cells share a height");
        assert!(cell.origin.y >= strip.origin.y);
        assert!(cell.origin.y + cell.size.y <= strip.origin.y + strip.size.y);
    }
    // The thumbnails live strictly between the two tiles.
    let last = *layout.thumbs.last().expect("thumbnails");
    assert!(layout.overview.origin.x + layout.overview.size.x < thumb.origin.x);
    assert!(last.origin.x + last.size.x < play.origin.x);
}

#[test]
fn stopped_phase_offers_retry_too() {
    let mut editor = editor_with_boards(2);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        phase: WorkspacePhase::Stopped,
        ..op_editor_core::WorkspaceState::default()
    };
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    let (retry, _) = surface
        .banner_buttons(&layout)
        .expect("a stopped run must offer a way to try again");
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(retry.origin.x + 2.0, retry.origin.y + 2.0)
        ),
        Some(WorkspaceHit::Retry)
    );
}

#[test]
fn present_is_inert_while_the_run_is_still_generating() {
    let mut editor = editor_with_boards(3);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        family: HomeFamily::Presentation,
        phase: WorkspacePhase::Generating,
        ..op_editor_core::WorkspaceState::default()
    };
    editor.editor_ui.layer_panel_width = 320.0;
    editor.editor_ui.sidebar_open = true;
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    let play = layout.play.expect("deck strip carries Present");
    assert!(!surface.play_enabled());
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(play.origin.x + 2.0, play.origin.y + 2.0)
        ),
        None,
        "the tile paints disabled, so the press must not present a half-drawn deck"
    );
}

#[test]
fn the_thumbnail_window_follows_the_selection() {
    assert_eq!(
        thumb_window_start(5, 8, 4),
        0,
        "a deck that fits never slides"
    );
    assert_eq!(thumb_window_start(20, 6, 0), 0);
    assert_eq!(thumb_window_start(20, 6, 10), 7, "centred on the selection");
    assert_eq!(thumb_window_start(20, 6, 19), 14, "clamped at the end");
    assert_eq!(thumb_window_start(20, 0, 10), 0);
}

#[test]
fn a_long_deck_keeps_its_late_slides_reachable_from_the_strip() {
    // Measured: the strip dropped every board past what fit, so on a long
    // deck the later slides had no thumbnail to press.
    let boards = 30;
    let mut editor = editor_with_boards(boards);
    editor.editor_ui.workspace = op_editor_core::WorkspaceState {
        visible: true,
        active: true,
        family: HomeFamily::Presentation,
        phase: WorkspacePhase::Done,
        selected: 27,
        ..op_editor_core::WorkspaceState::default()
    };
    editor.editor_ui.layer_panel_width = 320.0;
    editor.editor_ui.sidebar_open = true;
    let surface = WorkspaceSurface::for_editor_at(&editor, 0).expect("workspace visible");
    let layout = surface.layout(1440.0, 900.0);
    assert!(
        layout.thumbs.len() < boards,
        "the fixture must overflow the strip"
    );
    let rect = layout
        .thumb_rect(27)
        .expect("the selected late slide has a thumbnail slot");
    assert_eq!(
        surface.hit_test_layout(
            &layout,
            Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0)
        ),
        Some(WorkspaceHit::Thumb(27))
    );
    assert!(
        layout.thumb_rect(0).is_none(),
        "the head scrolled out of the window"
    );
}

#[test]
fn the_pager_keeps_room_for_its_position_label() {
    // The "3 / 12" label paints between the arrows; with a 2 px gap it was
    // drawn under the chevrons and only a sliver showed.
    let layout = layout_1440_900(HomeFamily::Presentation, false, 12);
    let (prev, next) = (layout.prev.unwrap(), layout.next.unwrap());
    let gap = next.origin.x - (prev.origin.x + prev.size.x);
    assert!(gap >= 40.0, "gap {gap}");
    assert!(next.origin.x + next.size.x <= layout.zoom_out.origin.x);
}
