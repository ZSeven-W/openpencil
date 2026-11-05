use super::*;
use crate::history_snapshot::SharedDoc;
use crate::test_support::{rect, state_with};
use crate::EditorCommand;

fn user() -> CommentAuthor {
    CommentAuthor {
        kind: CommentAuthorKind::User,
        name: "Ada".into(),
    }
}

fn agent() -> CommentAuthor {
    CommentAuthor {
        kind: CommentAuthorKind::Agent,
        name: "Claude".into(),
    }
}

fn node_anchor(id: &str) -> CommentAnchor {
    CommentAnchor {
        node_id: Some(id.into()),
        ..CommentAnchor::default()
    }
}

fn add(state: &mut EditorState, id: &str, anchor: CommentAnchor, text: &str) -> bool {
    state.apply(EditorCommand::Comment(CommentCommand::Add {
        id: id.into(),
        anchor,
        author: user(),
        text: text.into(),
        at_ms: 10,
    }))
}

fn set_status(state: &mut EditorState, id: &str, status: CommentStatus) -> bool {
    state.apply(EditorCommand::Comment(CommentCommand::SetStatus {
        comment_id: id.into(),
        status,
        at_ms: 30,
    }))
}

#[test]
fn add_reply_and_status_build_a_thread() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    assert!(add(&mut s, "c1", node_anchor("n1"), "  Make it bigger "));
    assert!(s.apply(EditorCommand::Comment(CommentCommand::Reply {
        comment_id: "c1".into(),
        author: agent(),
        text: "Done".into(),
        at_ms: 20,
    })));
    assert!(set_status(&mut s, "c1", CommentStatus::Addressed));

    let c = s.comment("c1").expect("thread");
    assert_eq!(c.status, CommentStatus::Addressed);
    assert_eq!(c.messages.len(), 2);
    assert_eq!(c.messages[0].text, "Make it bigger");
    assert_eq!(c.messages[1].author.kind, CommentAuthorKind::Agent);
    assert_eq!(c.updated_at, Some(30));
    assert!(s.is_dirty());
}

#[test]
fn add_validates_anchor_text_and_id() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    let point = CommentAnchor {
        x: Some(5.0),
        y: Some(6.0),
        ..CommentAnchor::default()
    };
    assert_eq!(
        s.check_comment_command(&CommentCommand::Add {
            id: "c1".into(),
            anchor: CommentAnchor::default(),
            author: user(),
            text: "hi".into(),
            at_ms: 0,
        }),
        Err(CommentError::EmptyAnchor)
    );
    assert!(!add(&mut s, "c1", node_anchor("missing"), "hi"));
    assert!(!add(&mut s, "c1", point.clone(), "   "));
    assert!(!add(&mut s, "", point.clone(), "hi"));
    assert!(s.comments().is_empty());
    assert!(add(&mut s, "c1", point.clone(), "hi"));
    assert!(!add(&mut s, "c1", point, "again"), "duplicate id refused");
}

#[test]
fn lifecycle_refuses_archived_to_addressed_and_reopen_returns_pending() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    assert!(add(&mut s, "c1", node_anchor("n1"), "hi"));
    assert!(set_status(&mut s, "c1", CommentStatus::Addressed));
    assert!(set_status(&mut s, "c1", CommentStatus::Archived));
    assert_eq!(
        s.check_comment_command(&CommentCommand::SetStatus {
            comment_id: "c1".into(),
            status: CommentStatus::Addressed,
            at_ms: 0,
        }),
        Err(CommentError::InvalidTransition {
            from: CommentStatus::Archived,
            to: CommentStatus::Addressed
        })
    );
    assert!(set_status(&mut s, "c1", CommentStatus::Pending), "reopen");
    assert_eq!(s.comment("c1").unwrap().status, CommentStatus::Pending);
}

#[test]
fn delete_removes_thread_and_clears_empty_field() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    assert!(add(&mut s, "c1", node_anchor("n1"), "hi"));
    assert!(s.apply(EditorCommand::Comment(CommentCommand::Delete {
        comment_id: "c1".into()
    })));
    assert!(s.doc.comments.is_none());
    assert!(!s.apply(EditorCommand::Comment(CommentCommand::Delete {
        comment_id: "c1".into()
    })));
}

#[test]
fn next_comment_id_disambiguates_same_millisecond() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    let first = s.next_comment_id(255);
    assert_eq!(first, "cm_ff");
    assert!(add(&mut s, &first, node_anchor("n1"), "hi"));
    assert_eq!(s.next_comment_id(255), "cm_ff_2");
}

#[test]
fn comments_survive_save_and_reload() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    assert!(add(&mut s, "c1", node_anchor("n1"), "hi"));
    assert!(set_status(&mut s, "c1", CommentStatus::Addressed));
    // Saves serialize the shared history snapshot, not PenDocument itself.
    let saved = serde_json::to_string(&SharedDoc::capture(&s.doc, None)).expect("serialize");
    let loaded = jian_ops_schema::load_str(&saved).expect("reload").value;
    assert_eq!(loaded.comments, s.doc.comments);
}

#[test]
fn undo_reverts_a_comment_like_any_document_edit() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 10.0, 10.0)]);
    s.commit_history();
    assert!(add(&mut s, "c1", node_anchor("n1"), "hi"));
    assert!(s.undo());
    assert!(s.comments().is_empty());
}
