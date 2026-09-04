//! Track C-3 integration tests: `reconcile` classifying + starting a
//! [`super::transition::ScreenTransition`] on a real screen switch, and
//! [`PreviewSession::paint_framed_animated`] compositing while one plays.
//!
//! Split out of `tests_app_mode.rs` (which already carries the
//! `TWO_SCREEN_DOC_JSON` fixture this file reuses) to keep both files
//! under the repo's 800-line-per-file cap.

#![cfg(test)]

use super::transition::TransitionKind;
use super::{test_measure, PreviewSession};
use jian_core::action::services::Router;
use op_editor_ui::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

/// Same two-screen fixture `tests_app_mode.rs` uses: entry "/" (a "go"
/// button pushing "/detail") and "/detail" (a plain switch).
fn two_screen_doc() -> jian_ops_schema::PenDocument {
    serde_json::from_str(super::tests_app_mode::TWO_SCREEN_DOC_JSON).unwrap()
}

fn go_button_center(session: &PreviewSession) -> (f32, f32) {
    let (x, y, w, h) = session.node_rect("go").expect("go button laid out");
    (x + w / 2.0, y + h / 2.0)
}

#[test]
fn push_reconcile_starts_a_push_transition() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    let (bx, by) = go_button_center(&session);
    session.dispatch_tap(bx, by);
    assert!(session.reconcile(1_000).switched);
    assert_eq!(
        session.transition_kind_for_test(),
        Some(TransitionKind::Push)
    );
    assert!(session.transition_active_for_test(1_100));
    assert!(
        !session.transition_active_for_test(1_400),
        "240ms push must finish"
    );
}

#[test]
fn pop_reconcile_starts_a_pop_transition() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    session.router_for_test().push("/detail");
    session.reconcile(1_000);
    session.router_for_test().pop();
    assert!(session.reconcile(2_000).switched);
    assert_eq!(
        session.transition_kind_for_test(),
        Some(TransitionKind::Pop)
    );
    assert!(session.transition_active_for_test(2_100));
    assert!(
        !session.transition_active_for_test(2_400),
        "240ms pop must finish"
    );
}

#[test]
fn same_depth_replace_starts_a_replace_transition() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    // `replace` keeps the stack at depth 1 — a tab-switch shape, not a push.
    session.router_for_test().replace("/detail");
    assert!(session.reconcile(1_000).switched);
    assert_eq!(
        session.transition_kind_for_test(),
        Some(TransitionKind::Replace)
    );
    assert!(session.transition_active_for_test(1_100));
    assert!(
        !session.transition_active_for_test(1_200),
        "160ms replace must finish"
    );
}

#[test]
fn mid_animation_switch_replaces_the_transition_outright() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    let (bx, by) = go_button_center(&session);
    session.dispatch_tap(bx, by);
    assert!(session.reconcile(1_000).switched, "push to /detail");
    assert_eq!(
        session.transition_kind_for_test(),
        Some(TransitionKind::Push)
    );

    // Pop back to "/" WHILE the 240ms push is still playing (at +100ms).
    session.router_for_test().pop();
    assert!(
        session.reconcile(1_100).switched,
        "pop while push still active"
    );
    assert_eq!(
        session.transition_kind_for_test(),
        Some(TransitionKind::Pop),
        "a fresh switch replaces the in-flight transition outright, not queued behind it"
    );
    // The replacement's OWN clock starts fresh at 1_100, not 1_000.
    assert!(session.transition_active_for_test(1_300));
    assert!(!session.transition_active_for_test(1_341));
}

/// Records `clip_rect` calls (one per composited layer) and rejects
/// nothing else — same minimal-fake-backend pattern as
/// `scene_paint_options.rs`'s `FillCaptureBackend`.
#[derive(Default)]
struct ClipCountBackend {
    clip_calls: u32,
}

impl RenderBackend for ClipCountBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {
        self.clip_calls += 1;
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
    fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn framed_paint_args() -> (Rect, Point2D, f32) {
    let content_clip = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(200.0, 200.0),
    };
    let content_origin = Point2D::new(0.0, 0.0);
    (content_clip, content_origin, 1.0)
}

#[test]
fn paint_framed_animated_composites_two_layers_while_active() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    let (bx, by) = go_button_center(&session);
    session.dispatch_tap(bx, by);
    session.reconcile(1_000);
    assert!(session.transition_active_for_test(1_100));

    let (root_id, _) = session.framed_root().expect("framed root");
    let (content_clip, content_origin, fit) = framed_paint_args();
    let mut backend = ClipCountBackend::default();
    session.paint_framed_animated(
        &mut backend,
        &root_id,
        content_clip,
        content_origin,
        fit,
        None,
        None,
        1_100,
    );
    assert_eq!(
        backend.clip_calls, 4,
        "an active push composites outgoing + entering — double the single-layer clip count"
    );
}

#[test]
fn paint_framed_animated_falls_back_to_single_layer_once_finished() {
    let doc = two_screen_doc();
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .unwrap();
    let (bx, by) = go_button_center(&session);
    session.dispatch_tap(bx, by);
    session.reconcile(1_000);
    assert!(!session.transition_active_for_test(1_400));

    let (root_id, _) = session.framed_root().expect("framed root");
    let (content_clip, content_origin, fit) = framed_paint_args();
    let mut backend = ClipCountBackend::default();
    session.paint_framed_animated(
        &mut backend,
        &root_id,
        content_clip,
        content_origin,
        fit,
        None,
        None,
        1_400,
    );
    assert_eq!(
        backend.clip_calls, 2,
        "once finished, paint_framed_animated routes to the plain single-layer paint_framed"
    );
}
