//! A Home brief that replaces a document hands the replaced work to the shell
//! and carries the brief's staged attachments across the swap.

use crate::widget_host::WidgetHostNative;
use op_editor_core::HomeFamily;

fn host_with_first_brief_sent() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.task = HomeFamily::AppUi;
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("做一个咖啡点单 App");
    assert!(host.queue_home_send(), "first brief sends");
    assert!(
        host.take_replaced_home_document().is_none(),
        "the untouched starter page has nothing to hand over"
    );
    host
}

/// The first run drew something: the page stops being the pristine starter
/// and the document gains an unsaved revision.
fn first_run_drew_a_design(host: &mut WidgetHostNative) {
    assert!(host
        .editor_state_mut()
        .apply(op_editor_core::EditorCommand::UpdateNode {
            node_id: op_editor_core::NodeId::new("n10".to_string()),
            x: None,
            y: None,
            width: None,
            height: None,
            name: Some("咖啡点单 首页".into()),
            fill_hex: None,
            page_id: None,
        }));
    assert!(host.editor_state().is_dirty());
}

fn come_back_with_second_brief(host: &mut WidgetHostNative) {
    host.editor_state_mut().editor_ui.workspace.visible = false;
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.task = HomeFamily::Presentation;
    host.editor_state_mut()
        .editor_ui
        .home
        .set_draft("做一份产品介绍 PPT");
}

#[test]
fn a_second_brief_hands_the_unsaved_first_design_to_the_shell() {
    let mut host = host_with_first_brief_sent();
    // The first run drew something and nobody saved it.
    first_run_drew_a_design(&mut host);
    host.editor_state_mut().chat.title = "咖啡点单 App".into();
    come_back_with_second_brief(&mut host);
    assert!(host.queue_home_send(), "second brief sends");

    let replaced = host
        .take_replaced_home_document()
        .expect("the replaced design must reach the shell, not the floor");
    assert!(replaced.had_unsaved_changes);
    assert_eq!(replaced.title, "咖啡点单 App");
    assert!(host.take_replaced_home_document().is_none(), "drained once");
}

#[test]
fn a_saved_first_design_is_handed_over_as_clean() {
    let mut host = host_with_first_brief_sent();
    first_run_drew_a_design(&mut host);
    host.editor_state_mut().mark_saved_revision();
    come_back_with_second_brief(&mut host);
    assert!(host.queue_home_send());

    let replaced = host.take_replaced_home_document().expect("handed over");
    assert!(
        !replaced.had_unsaved_changes,
        "a saved document needs no rescue copy — only its path must be dropped"
    );
}

#[test]
fn staged_attachments_cross_the_document_swap() {
    let mut host = host_with_first_brief_sent();
    first_run_drew_a_design(&mut host);
    come_back_with_second_brief(&mut host);
    assert!(host
        .editor_state_mut()
        .chat
        .add_attachment(op_editor_core::chat::ChatAttachment {
            name: "reference.png".into(),
            media_type: "image/png".into(),
            data: vec![0x89, b'P', b'N', b'G'],
        }));
    assert!(host.queue_home_send());
    assert!(
        host.take_replaced_home_document().is_some(),
        "the document really was swapped"
    );

    let sent = host
        .editor_state()
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == op_editor_core::chat::ChatRole::User)
        .expect("the brief was sent as a user turn");
    assert_eq!(
        sent.images.len(),
        1,
        "the screenshot staged for this brief must travel with it"
    );
}

#[test]
fn retrying_a_stopped_run_starts_on_a_fresh_page_and_keeps_the_partial_one() {
    let mut host = host_with_first_brief_sent();
    first_run_drew_a_design(&mut host);
    host.editor_state_mut().editor_ui.workspace.phase = op_editor_core::WorkspacePhase::Stopped;
    let brief = host.editor_state().editor_ui.workspace.brief.clone();
    let (w, h) = (1440.0, 900.0);
    let retry = {
        let surface = op_editor_ui::widgets::workspace_surface::WorkspaceSurface::for_editor_at(
            host.editor_state(),
            0,
        )
        .expect("workspace visible");
        let layout = surface.layout(w, h);
        surface
            .banner_buttons(&layout)
            .expect("a stopped run offers Retry")
            .0
    };
    assert_eq!(
        host.press_workspace(retry.origin.x + 4.0, retry.origin.y + 4.0, w, h),
        Some(true)
    );
    let replaced = host
        .take_replaced_home_document()
        .expect("the partial design is handed to the shell, not overdrawn");
    assert!(replaced.had_unsaved_changes);
    assert!(
        op_editor_core::blank_starter::active_page_is_blank_starter(host.editor_state()),
        "the retry draws on a fresh page"
    );
    // Swapping the page resets the workspace; the retry must bring it back,
    // or the run goes on behind a workspace that has vanished.
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(
        workspace.active && workspace.visible,
        "the workspace stays up"
    );
    assert_eq!(workspace.brief, brief);
    assert_eq!(workspace.phase, op_editor_core::WorkspacePhase::Generating);
}
