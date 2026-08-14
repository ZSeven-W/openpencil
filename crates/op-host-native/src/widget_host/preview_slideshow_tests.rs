//! Deck slideshow host lifecycle: entering, arrow keys, and Escape.
//!
//! Windows-gated for the same reason the other preview host tests are:
//! entering preview solves layout through `jian_skia::SkiaMeasure`, which
//! aborts the process under Windows CI's DirectWrite.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::SlideshowToolbarButton;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, PreviewDeviceKind};
use op_editor_ui::widgets::SlideshowToolbar;
use op_editor_ui::Point2D;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

/// Three 16:9 boards side by side — the shape a generated deck has.
const THREE_BOARD_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn host_with(source: &str, scenario: Option<TemplateScene>) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(source)
        .expect("parse slideshow fixture")
        .value;
    let mut host = WidgetHostNative::new();
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = scenario;
    host.install_imported_state(state);
    host
}

fn board_on_screen(host: &WidgetHostNative) -> Option<String> {
    host.editor_state
        .preview_slideshow()
        .and_then(|slideshow| slideshow.current_board())
        .map(str::to_string)
}

#[test]
fn a_deck_presents_from_board_zero_and_arrow_keys_move_through_it() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));

    assert!(host.enter_preview((1200.0, 800.0)));
    assert!(host.preview_slideshow_active(), "a deck presents");
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));
    assert_eq!(
        host.editor_state.editor_ui.preview.device,
        Some(PreviewDeviceKind::Canvas),
        "a slide has no phone or desktop silhouette"
    );

    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));

    // The end of the deck holds, it does not wrap to the title slide.
    assert!(host.preview_dispatch_key("ArrowRight", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));

    assert!(host.preview_dispatch_key("ArrowLeft", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));

    // Escape leaves preview through the existing ladder, ending the
    // presentation with it.
    assert!(host.apply_escape());
    assert!(!host.preview_slideshow_active());
    assert!(!host.editor_state.editor_ui.preview.mode);
    assert!(host.editor_state.preview_slideshow().is_none());
}

#[test]
fn the_presented_board_is_framed_by_the_viewport_fit() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));

    host.enter_preview((1920.0, 1080.0));
    let first_zoom = host.editor_state.viewport.zoom;
    let first_pan_x = host.editor_state.viewport.pan_x;
    // A 1920x1080 board in a 1920x1080 canvas fits at 1:1, centred.
    assert!((first_zoom - 1.0).abs() < 1e-3, "zoom {first_zoom}");
    assert!(first_pan_x.abs() < 1e-3, "pan_x {first_pan_x}");

    // Advancing re-frames onto the next board, which sits 2100px away.
    // Paint owns the canvas size in production; the test drives the same
    // framing call paint makes.
    assert!(host.preview_slideshow_step(1));
    assert!(host.frame_slideshow_board((1920.0, 1080.0)));
    assert!(
        (host.editor_state.viewport.pan_x + 2100.0).abs() < 1e-3,
        "pan_x {}",
        host.editor_state.viewport.pan_x
    );
    assert!((host.editor_state.viewport.zoom - first_zoom).abs() < 1e-3);
}

#[test]
fn an_untagged_document_previews_interactively_as_before() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, None);

    assert!(host.enter_preview((1200.0, 800.0)));

    assert!(!host.preview_slideshow_active());
    assert!(
        !host.preview_slideshow_step(1),
        "slideshow keys do nothing outside a presentation"
    );
}

/// The real shipped deck, not just a hand-written fixture: the template a
/// user actually opens has to present, and its board count has to match the
/// slides it was authored with.
#[test]
fn the_shipped_slide_deck_template_presents_every_board() {
    let _guard = test_lock();
    let source = op_editor_core::scene_template_catalog::scene_template_document("slide-deck")
        .expect("the deck template ships");
    let mut host = host_with(source, Some(TemplateScene::Slides));
    let authored_boards = host.editor_state.active_children().len();
    assert!(authored_boards > 1, "the deck has several slides");

    assert!(host.enter_preview((1200.0, 800.0)));

    let slideshow = host
        .editor_state
        .preview_slideshow()
        .expect("the deck presents");
    assert_eq!(slideshow.len(), authored_boards);
    assert_eq!(slideshow.counter_label(), format!("1 / {authored_boards}"));

    // Walking to the end lands on the last board and holds there.
    for _ in 0..authored_boards {
        host.preview_dispatch_key("ArrowRight", false);
    }
    let slideshow = host
        .editor_state
        .preview_slideshow()
        .expect("still present");
    assert_eq!(slideshow.index(), authored_boards - 1);
}

/// A deck tag on a page with no boards must not panic or trap the user in
/// an empty presentation — preview behaves exactly as it did before.
#[test]
fn a_deck_with_no_boards_falls_back_to_ordinary_preview() {
    let _guard = test_lock();
    let mut host = host_with(
        r#"{"version":"1.0.0","children":[]}"#,
        Some(TemplateScene::Slides),
    );

    assert!(host.enter_preview((1200.0, 800.0)));

    assert!(!host.preview_slideshow_active());
    assert!(!host.preview_dispatch_key("ArrowRight", false));
    assert!(host.apply_escape());
}

// ── presenting toolbar + click-to-advance ─────────────────────────────────

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

fn presenting_host() -> WidgetHostNative {
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));
    // `apply_cursor_move` resolves overlays against the cached viewport, the
    // way the runner leaves it after a frame.
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host.enter_preview((VW, VH));
    assert!(host.preview_slideshow_active(), "fixture presents");
    host
}

fn toolbar_point(host: &WidgetHostNative, button: SlideshowToolbarButton) -> Point2D {
    let canvas = host.preview_canvas_rect(VW, VH);
    let label = host
        .editor_state
        .preview_slideshow()
        .expect("presenting")
        .counter_label();
    let rect = SlideshowToolbar::button_rects(canvas, &label)
        .into_iter()
        .find(|(candidate, _)| *candidate == button)
        .expect("every control has a rect")
        .1;
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// A point on the presented board, clear of the toolbar.
fn board_point(host: &WidgetHostNative) -> Point2D {
    let canvas = host.preview_canvas_rect(VW, VH);
    Point2D::new(
        canvas.origin.x + canvas.size.x / 2.0,
        canvas.origin.y + canvas.size.y / 4.0,
    )
}

/// Press and release at one point, with the cursor tracked the way the
/// runner tracks it — the release reads the maintained hover.
fn click(host: &mut WidgetHostNative, point: Point2D) {
    host.apply_cursor_move(point.x, point.y);
    host.apply_press(point.x, point.y, VW, VH);
    host.apply_release_with_viewport(VW, VH);
}

#[test]
fn the_toolbar_steps_the_deck_and_exits() {
    let _guard = test_lock();
    let mut host = presenting_host();

    let next = toolbar_point(&host, SlideshowToolbarButton::Next);
    click(&mut host, next);
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));

    let previous = toolbar_point(&host, SlideshowToolbarButton::Previous);
    click(&mut host, previous);
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));

    // Exit takes the same path Escape does.
    let exit = toolbar_point(&host, SlideshowToolbarButton::Exit);
    click(&mut host, exit);
    assert!(!host.preview_slideshow_active());
    assert!(!host.editor_state.editor_ui.preview.mode);
}

#[test]
fn pressing_the_counter_is_swallowed_without_advancing() {
    let _guard = test_lock();
    let mut host = presenting_host();
    let canvas = host.preview_canvas_rect(VW, VH);
    let label = host
        .editor_state
        .preview_slideshow()
        .expect("presenting")
        .counter_label();
    let pill = SlideshowToolbar::pill_rect(canvas, &label);
    let counter = Point2D::new(
        pill.origin.x + pill.size.x / 2.0,
        pill.origin.y + pill.size.y / 2.0,
    );
    assert_eq!(
        SlideshowToolbar::hit_test(canvas, &label, counter),
        None,
        "the fixture point really is the counter, not a button"
    );

    click(&mut host, counter);

    assert_eq!(
        board_on_screen(&host).as_deref(),
        Some("slide-1"),
        "a press on the toolbar must never reach the board underneath"
    );
}

#[test]
fn clicking_the_board_advances_and_holds_at_the_last_slide() {
    let _guard = test_lock();
    let mut host = presenting_host();
    let point = board_point(&host);

    click(&mut host, point);
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
    click(&mut host, point);
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));

    // No wrap, and no exit-on-click: an accidental exit mid-talk costs the
    // presenter more than a dead click does.
    click(&mut host, point);
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));
    assert!(host.preview_slideshow_active());
}

#[test]
fn a_vertical_drag_across_the_board_does_not_advance() {
    let _guard = test_lock();
    let mut host = presenting_host();
    let start = board_point(&host);

    host.apply_cursor_move(start.x, start.y);
    host.apply_press(start.x, start.y, VW, VH);
    host.apply_cursor_move(start.x + 40.0, start.y + 240.0);
    host.apply_release_with_viewport(VW, VH);

    assert_eq!(
        board_on_screen(&host).as_deref(),
        Some("slide-1"),
        "a drag is not a click"
    );
}

#[test]
fn every_presenter_key_moves_the_deck() {
    let _guard = test_lock();
    let mut host = presenting_host();

    assert!(host.preview_dispatch_key("ArrowDown", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
    assert!(host.preview_dispatch_key("ArrowUp", false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));

    assert!(host.preview_slideshow_to_end(true));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));
    assert!(
        !host.preview_slideshow_to_end(true),
        "already at the last board"
    );
    assert!(host.preview_slideshow_to_end(false));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));
}

/// The highest-risk surface of the presenting press path: a preview that is
/// NOT presenting must route presses exactly as it did before — into the
/// live runtime, with none of the slideshow bookkeeping armed.
#[test]
fn a_non_presenting_preview_still_routes_presses_to_the_runtime() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, None);
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    host.enter_preview((VW, VH));
    assert!(!host.preview_slideshow_active());
    let point = board_point(&host);

    // Let the enter animation finish: while it plays, preview discards
    // pointer input on purpose, which would hide what this test is checking.
    host.set_now_ms(host.now_ms + 5_000);
    host.apply_cursor_move(point.x, point.y);
    assert!(host.apply_press(point.x, point.y, VW, VH));

    assert!(
        host.preview_press_active,
        "the runtime owns the gesture outside a presentation"
    );
    assert!(host.slideshow_press_screen.is_none());
    assert!(host
        .editor_state
        .editor_ui
        .preview
        .toolbar_pressed
        .is_none());

    assert!(host.apply_release_with_viewport(VW, VH));
    assert!(!host.preview_press_active, "the runtime got its pointer up");

    // Chrome is untouched too: ordinary preview keeps its rails, so the
    // stage stays the editing canvas region and the StatusBar still answers.
    let stage = host.preview_canvas_rect(VW, VH);
    assert!(
        stage.origin.x > 0.0 && stage.size.x < VW,
        "an ordinary preview is still bounded by the rails"
    );
    let status =
        op_editor_ui::widgets::host_canvas_geometry::status_bar_rect(&host.editor_state, VW, VH)
            .expect("the status bar is painted in ordinary preview");
    // Aim at a real control rather than the pill's middle, which is the
    // zoom readout and deliberately inert.
    let bar = op_editor_ui::widgets::StatusBar::for_editor(&host.editor_state);
    let control = (0..status.size.x as i32)
        .map(|dx| {
            Point2D::new(
                status.origin.x + dx as f32,
                status.origin.y + status.size.y / 2.0,
            )
        })
        .find(|point| bar.control_at(status, *point).is_some())
        .expect("the status bar has controls");
    let zoom_before = host.editor_state.viewport.zoom;
    host.apply_press(control.x, control.y, VW, VH);
    assert_ne!(
        host.editor_state.viewport.zoom, zoom_before,
        "its zoom controls still work outside a presentation"
    );
}

// ── presenting hides the editing chrome ───────────────────────────────────

/// The stage a presentation paints on: everything under the TopBar.
///
/// This is the whole mechanism — no panel state changes, so the rails come
/// back on their own the moment the presentation ends.
#[test]
fn presenting_takes_the_full_stage_and_gives_it_back_on_exit() {
    let _guard = test_lock();
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));
    host.last_viewport_w = VW;
    host.last_viewport_h = VH;
    let editing_stage = host.preview_canvas_rect(VW, VH);
    assert!(
        editing_stage.origin.x > 0.0 && editing_stage.size.x < VW,
        "the editing canvas is bounded by the rails"
    );

    host.enter_preview((VW, VH));
    let presenting_stage = host.preview_canvas_rect(VW, VH);
    assert_eq!(presenting_stage.origin.x, 0.0);
    assert_eq!(presenting_stage.size.x, VW);
    assert!(presenting_stage.origin.y > 0.0, "the TopBar keeps its band");

    host.apply_escape();

    assert_eq!(
        host.preview_canvas_rect(VW, VH).origin.x,
        editing_stage.origin.x,
        "leaving the presentation restores the editing stage untouched"
    );
    assert_eq!(
        host.preview_canvas_rect(VW, VH).size.x,
        editing_stage.size.x
    );
    assert!(
        host.editor_state.editor_ui.sidebar_open,
        "the rails were hidden by paint policy, never by closing them"
    );
}

/// A press where a hidden widget used to be belongs to the deck.
///
/// The StatusBar's tier sits ABOVE preview in the press ladder, so without
/// the presenting gate its controls would still answer — a dead patch over
/// the slide that silently zoomed the canvas instead of advancing.
#[test]
fn a_press_where_the_hidden_status_bar_sat_advances_the_deck() {
    let _guard = test_lock();
    let mut host = presenting_host();
    let status =
        op_editor_ui::widgets::host_canvas_geometry::status_bar_rect(&host.editor_state, VW, VH)
            .expect("the editor would paint a status bar here");
    let point = Point2D::new(
        status.origin.x + status.size.x / 2.0,
        status.origin.y + status.size.y / 2.0,
    );
    let zoom_before = host.editor_state.viewport.zoom;

    click(&mut host, point);

    assert_eq!(
        board_on_screen(&host).as_deref(),
        Some("slide-2"),
        "the press fell through to the board"
    );
    assert_eq!(
        host.editor_state.viewport.zoom, zoom_before,
        "and never reached the hidden zoom controls"
    );
}

/// Same for the left rail's band.
#[test]
fn a_press_over_the_hidden_layer_rail_advances_the_deck() {
    let _guard = test_lock();
    let mut host = presenting_host();
    let rail =
        op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(&host.editor_state, VH);
    let point = Point2D::new(
        rail.origin.x + rail.size.x / 2.0,
        rail.origin.y + rail.size.y / 2.0,
    );

    click(&mut host, point);

    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
}

/// The chrome really is unpainted, not merely unclickable: the left rail's
/// band shows the slide while presenting, and the panel again after exit.
#[test]
fn the_rails_are_not_painted_while_presenting() {
    use crate::backend::{NativeBackend, NativeFrameBackend};

    let _guard = test_lock();
    const W: i32 = 800;
    const H: i32 = 600;

    fn rail_pixel(host: &mut WidgetHostNative, backend: &mut NativeBackend) -> [u8; 3] {
        let mut surface =
            skia_safe::surfaces::raster_n32_premul((W, H)).expect("raster surface allocated");
        surface.canvas().clear(skia_safe::Color::BLACK);
        {
            let mut frame = NativeFrameBackend::new(backend, surface.canvas());
            host.paint(&mut frame, W as f32, H as f32);
        }
        let stride = (W * 4) as usize;
        let mut pixels = vec![0u8; stride * H as usize];
        let info = skia_safe::ImageInfo::new(
            (W, H),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        assert!(surface.read_pixels(&info, &mut pixels, stride, (0, 0)));
        // Inside the left rail, well below the TopBar.
        let offset = 300 * stride + 40 * 4;
        [pixels[offset], pixels[offset + 1], pixels[offset + 2]]
    }

    let mut backend = NativeBackend::with_dpi(1.0);
    let mut host = host_with(THREE_BOARD_DECK, Some(TemplateScene::Slides));
    host.last_viewport_w = W as f32;
    host.last_viewport_h = H as f32;
    host.enter_preview((W as f32, H as f32));
    host.mark_paint_dirty_for_test();
    let presenting = rail_pixel(&mut host, &mut backend);

    host.apply_escape();
    host.mark_paint_dirty_for_test();
    let editing = rail_pixel(&mut host, &mut backend);

    // The deck's slides are white; the layer rail is not.
    assert!(
        presenting.iter().all(|channel| *channel > 200),
        "the slide should reach the rail's band while presenting, got {presenting:?}"
    );
    assert_ne!(
        presenting, editing,
        "the rail must be painted again once the presentation ends"
    );
}

#[test]
fn touch_account_and_collaboration_overlays_are_neither_painted_nor_hit_while_presenting() {
    use crate::backend::{NativeBackend, NativeFrameBackend};
    use op_editor_ui::widgets::login_modal::LoginModal;
    use op_editor_ui::widgets::CollabPanel;

    let _guard = test_lock();
    let mut host = presenting_host();
    host.editor_state.editor_ui.touch = true;
    host.editor_state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;

    fn center_pixel(host: &mut WidgetHostNative) -> [u8; 4] {
        let mut surface = skia_safe::surfaces::raster_n32_premul((VW as i32, VH as i32))
            .expect("raster surface allocated");
        let mut backend = NativeBackend::with_dpi(1.0);
        {
            let mut frame = NativeFrameBackend::new(&mut backend, surface.canvas());
            host.paint(&mut frame, VW, VH);
        }
        let stride = VW as usize * 4;
        let mut pixels = vec![0u8; stride * VH as usize];
        let info = skia_safe::ImageInfo::new(
            (VW as i32, VH as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );
        assert!(surface.read_pixels(&info, &mut pixels, stride, (0, 0)));
        let offset = VH as usize / 2 * stride + VW as usize / 2 * 4;
        pixels[offset..offset + 4].try_into().unwrap()
    }

    let baseline = center_pixel(&mut host);
    host.editor_state.editor_ui.login_modal_open = true;
    host.editor_state.editor_ui.collab.panel.open = true;
    host.editor_state.editor_ui.agent_settings_open = true;
    assert_eq!(
        center_pixel(&mut host),
        baseline,
        "stale touch overlays must not alter a presentation frame"
    );

    host.editor_state.editor_ui.collab.panel.open = false;
    let login = LoginModal::for_editor(&host.editor_state);
    let login_rect = login.rect(VW, VH);
    click(
        &mut host,
        Point2D::new(login_rect.origin.x + 24.0, login_rect.origin.y + 120.0),
    );
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-2"));
    assert!(host.editor_state.editor_ui.login_modal_open);
    assert!(!host.editor_state.editor_ui.login_modal_stub_hint_shown);

    host.editor_state.editor_ui.login_modal_open = false;
    host.editor_state.editor_ui.collab.panel.open = true;
    let panel = CollabPanel::for_editor_ui(&host.editor_state.editor_ui).unwrap();
    let rect = op_editor_ui::widgets::touch_overlay_geometry::collaboration_panel_rect(
        &host.editor_state,
        &panel,
        VW,
        VH,
    );
    click(
        &mut host,
        Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        ),
    );
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-3"));
    assert!(host.editor_state.editor_ui.collab.panel.open);
    assert_eq!(host.editor_state.editor_ui.collab.pending_action, None);
}
