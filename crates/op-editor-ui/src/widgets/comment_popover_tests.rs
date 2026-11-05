use super::*;
use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
use crate::widgets::test_capture_backend::CaptureBackend;
use op_editor_core::comment_ui;
use op_editor_core::comments::{CommentAnchor, CommentAuthor, CommentCommand};
use op_editor_core::EditorCommand;

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

fn state_with_thread() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[{"type":"rectangle","id":"card","name":"Card","x":100,"y":100,"width":200,"height":120}]}"#,
    )
    .unwrap()
    .value;
    let mut s = EditorState::from_document(doc);
    assert!(s.apply(EditorCommand::Comment(CommentCommand::Add {
        id: "c1".into(),
        anchor: CommentAnchor {
            node_id: Some("card".into()),
            ..CommentAnchor::default()
        },
        author: CommentAuthor {
            kind: CommentAuthorKind::Agent,
            name: "Claude".into(),
        },
        text: "Padding updated".into(),
        at_ms: 1,
    })));
    s
}

fn paint(s: &mut EditorState) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    paint_comment_popover(
        &mut PaintCx {
            backend: &mut backend,
        },
        s,
        &scene(),
        1440.0,
        900.0,
    );
    backend
}

#[test]
fn thread_shows_agent_author_and_status_buttons() {
    let mut s = state_with_thread();
    s.editor_ui.locale = op_i18n::Locale::EnUs;
    comment_ui::open_comment_thread(&mut s, "c1");
    let backend = paint(&mut s);
    let texts: Vec<&str> = backend.texts.iter().map(|(t, _)| t.as_str()).collect();
    assert!(texts.contains(&"Claude · Agent"), "{texts:?}");
    assert!(texts.contains(&"Padding updated"), "{texts:?}");
    assert!(texts.contains(&"Mark addressed"), "{texts:?}");
    assert!(texts.contains(&"Reply…"), "{texts:?}");
    assert!(s.editor_ui.comments.popover_rect.is_some());
}

#[test]
fn a_thread_that_vanished_closes_the_popover() {
    let mut s = state_with_thread();
    comment_ui::open_comment_thread(&mut s, "c1");
    assert!(s.apply(EditorCommand::Comment(CommentCommand::Delete {
        comment_id: "c1".into()
    })));
    paint(&mut s);
    assert!(s.editor_ui.comments.open.is_none());
    assert!(s.editor_ui.comments.popover_rect.is_none());
}

#[test]
fn archived_threads_offer_only_reopen() {
    assert_eq!(
        status_actions(CommentStatus::Archived),
        &[(CommentStatus::Pending, "comments.reopen")]
    );
}
