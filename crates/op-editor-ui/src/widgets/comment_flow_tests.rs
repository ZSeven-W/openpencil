use super::*;
use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
use crate::widgets::comment_pins::{comment_pins, doc_to_screen, PIN_SIZE};
use crate::widgets::comment_popover::paint_comment_popover;
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::PaintCx;
use crate::Rect;
use op_editor_core::comment_ui::{CommentPopover, CommentPopoverHit};
use op_editor_core::comments::CommentStatus;

const VW: f32 = 1440.0;
const VH: f32 = 900.0;

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[{"type":"rectangle","id":"card","name":"Card","x":100,"y":100,"width":200,"height":120}]}"#,
    )
    .expect("fixture")
    .value;
    EditorState::from_document(doc)
}

fn scene() -> LayoutScene {
    let mut card = SceneNode::leaf("card", NodeKind::Rect);
    card.bounds = Rect::xywh(100.0, 100.0, 200.0, 120.0);
    LayoutScene {
        pages: vec![ScenePage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![card],
        }],
        active_page_index: 0,
    }
}

fn paint(state: &mut EditorState) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    paint_comment_popover(
        &mut PaintCx {
            backend: &mut backend,
        },
        state,
        &scene(),
        VW,
        VH,
    );
    backend
}

fn centre(rect: [f32; 4]) -> Point2D {
    Point2D::new(rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0)
}

fn post(state: &mut EditorState, at_doc: Point2D, text: &str) -> String {
    let screen = doc_to_screen(state, at_doc);
    let press = press_canvas(state, &scene(), screen, VW, VH);
    assert!(press.consumed);
    for c in text.chars() {
        comment_ui::comment_text(state, c, 1);
    }
    assert_eq!(comment_ui::comment_submit(state, 7), Some(Ok(true)));
    match &state.editor_ui.comments.open {
        Some(CommentPopover::Thread { id }) => id.clone(),
        other => panic!("expected the posted thread, got {other:?}"),
    }
}

#[test]
fn comment_mode_press_pins_a_draft_to_the_node_under_the_cursor() {
    let mut s = state();
    comment_ui::toggle_comment_mode(&mut s);
    let id = post(&mut s, Point2D::new(150.0, 130.0), "Tighter");
    let anchor = &s.comment(&id).unwrap().anchor;
    assert_eq!(anchor.node_id.as_deref(), Some("card"));
    assert_eq!((anchor.x, anchor.y), (Some(50.0), Some(30.0)));

    let pins = comment_pins(&s, &scene(), VW, VH);
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].tip, doc_to_screen(&s, Point2D::new(150.0, 130.0)));
}

#[test]
fn pin_press_opens_thread_and_outside_press_closes_it_without_consuming() {
    let mut s = state();
    comment_ui::toggle_comment_mode(&mut s);
    let id = post(&mut s, Point2D::new(500.0, 400.0), "Empty area");
    assert!(s.comment(&id).unwrap().anchor.node_id.is_none());
    comment_ui::toggle_comment_mode(&mut s);
    comment_ui::close_comment_popover(&mut s);

    let pin = comment_pins(&s, &scene(), VW, VH)[0].rect;
    let inside = Point2D::new(pin.origin.x + 4.0, pin.origin.y + PIN_SIZE - 4.0);
    assert!(press_canvas(&mut s, &scene(), inside, VW, VH).consumed);
    assert_eq!(
        s.editor_ui.comments.open,
        Some(CommentPopover::Thread { id })
    );

    let elsewhere = doc_to_screen(&s, Point2D::new(150.0, 130.0));
    let press = press_canvas(&mut s, &scene(), elsewhere, VW, VH);
    assert!(!press.consumed, "selection still gets the press");
    assert!(press.changed);
    assert!(s.editor_ui.comments.open.is_none());
}

#[test]
fn popover_buttons_drive_the_lifecycle_and_archived_pins_disappear() {
    let mut s = state();
    comment_ui::toggle_comment_mode(&mut s);
    let id = post(&mut s, Point2D::new(150.0, 130.0), "Fix");
    paint(&mut s);
    let addressed = s
        .editor_ui
        .comments
        .popover_hits
        .iter()
        .find(|(hit, _)| *hit == CommentPopoverHit::SetStatus(CommentStatus::Addressed))
        .map(|(_, r)| *r)
        .expect("pending thread offers Mark addressed");
    assert!(press_popover(&mut s, centre(addressed), 9).consumed);
    assert_eq!(s.comment(&id).unwrap().status, CommentStatus::Addressed);

    paint(&mut s);
    let archive = s
        .editor_ui
        .comments
        .popover_hits
        .iter()
        .find(|(hit, _)| *hit == CommentPopoverHit::SetStatus(CommentStatus::Archived))
        .map(|(_, r)| *r)
        .expect("addressed thread offers Archive");
    assert!(press_popover(&mut s, centre(archive), 10).consumed);
    assert_eq!(s.comment(&id).unwrap().status, CommentStatus::Archived);
    assert!(comment_pins(&s, &scene(), VW, VH).is_empty());
}

#[test]
fn popover_press_outside_the_card_falls_through() {
    let mut s = state();
    comment_ui::toggle_comment_mode(&mut s);
    post(&mut s, Point2D::new(150.0, 130.0), "Fix");
    paint(&mut s);
    assert!(!press_popover(&mut s, Point2D::new(1.0, 1.0), 0).consumed);
}
