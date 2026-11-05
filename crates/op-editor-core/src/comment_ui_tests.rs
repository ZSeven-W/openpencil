use super::*;
use crate::test_support::{rect, state_with};

fn typed(state: &mut EditorState, text: &str) {
    for c in text.chars() {
        assert_eq!(comment_text(state, c, 1), Some(true));
    }
}

#[test]
fn draft_posts_a_pending_thread_and_switches_to_it() {
    let mut s = state_with(vec![rect("n1", "Card", 10.0, 10.0, 50.0, 50.0)]);
    toggle_comment_mode(&mut s);
    let anchor = comment_anchor_at(&s, Some(("n1", [10.0, 10.0])), [14.0, 13.0]);
    assert_eq!(anchor.x, Some(4.0));
    begin_comment_draft(&mut s, anchor);
    assert_eq!(comment_submit(&mut s, 5), Some(Ok(false)), "empty draft");
    typed(&mut s, "Bigger");
    assert_eq!(comment_submit(&mut s, 5), Some(Ok(true)));

    let thread = &s.comments()[0];
    assert_eq!(thread.status, CommentStatus::Pending);
    assert_eq!(thread.anchor.node_id.as_deref(), Some("n1"));
    assert_eq!(thread.messages[0].author.kind, CommentAuthorKind::User);
    assert_eq!(
        s.editor_ui.comments.open,
        Some(CommentPopover::Thread {
            id: thread.id.clone()
        })
    );
    assert!(s.editor_ui.comments.input.text().is_empty());
}

#[test]
fn reply_status_and_delete_from_the_popover() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 50.0, 50.0)]);
    let anchor = comment_anchor_at(&s, None, [3.0, 4.0]);
    begin_comment_draft(&mut s, anchor);
    typed(&mut s, "Hi");
    comment_submit(&mut s, 1);
    let id = s.comments()[0].id.clone();

    typed(&mut s, "More");
    assert_eq!(comment_backspace(&mut s, 2), Some(true));
    assert_eq!(comment_submit(&mut s, 2), Some(Ok(true)));
    assert_eq!(s.comment(&id).unwrap().messages[1].text, "Mor");

    assert_eq!(
        set_open_comment_status(&mut s, CommentStatus::Addressed, 3),
        Ok(true)
    );
    assert_eq!(
        set_open_comment_status(&mut s, CommentStatus::Archived, 4),
        Ok(true)
    );
    assert!(s.editor_ui.comments.open.is_none(), "archiving closes it");

    open_comment_thread(&mut s, &id);
    assert_eq!(
        set_open_comment_status(&mut s, CommentStatus::Pending, 5),
        Ok(true)
    );
    assert_eq!(delete_open_comment(&mut s), Ok(true));
    assert!(s.comments().is_empty());
}

#[test]
fn each_gui_edit_is_one_undo_step() {
    let mut s = state_with(vec![rect("n1", "Card", 0.0, 0.0, 50.0, 50.0)]);
    let anchor = comment_anchor_at(&s, None, [3.0, 4.0]);
    begin_comment_draft(&mut s, anchor);
    typed(&mut s, "Hi");
    comment_submit(&mut s, 1);
    set_open_comment_status(&mut s, CommentStatus::Addressed, 2).unwrap();
    assert!(s.undo());
    assert_eq!(s.comments()[0].status, CommentStatus::Pending);
    assert!(s.undo());
    assert!(s.comments().is_empty());
}

#[test]
fn escape_closes_popover_then_leaves_mode() {
    let mut s = state_with(vec![]);
    toggle_comment_mode(&mut s);
    let anchor = comment_anchor_at(&s, None, [0.0, 0.0]);
    begin_comment_draft(&mut s, anchor);
    assert!(comment_escape(&mut s));
    assert!(s.editor_ui.comments.open.is_none());
    assert!(s.editor_ui.comments.mode);
    assert!(comment_escape(&mut s));
    assert!(!s.editor_ui.comments.mode);
    assert!(!comment_escape(&mut s));
    assert_eq!(
        comment_text(&mut s, 'x', 0),
        None,
        "closed popover ignores keys"
    );
}

#[test]
fn popover_hit_uses_painted_regions() {
    let mut s = state_with(vec![]);
    let anchor = comment_anchor_at(&s, None, [0.0, 0.0]);
    begin_comment_draft(&mut s, anchor);
    let ui = &mut s.editor_ui.comments;
    ui.popover_rect = Some([100.0, 100.0, 200.0, 120.0]);
    ui.popover_hits = vec![(CommentPopoverHit::Close, [270.0, 104.0, 24.0, 24.0])];
    assert_eq!(ui.popover_hit(280.0, 110.0), Some(CommentPopoverHit::Close));
    assert_eq!(ui.popover_hit(150.0, 150.0), Some(CommentPopoverHit::Body));
    assert_eq!(ui.popover_hit(10.0, 10.0), None);
}
