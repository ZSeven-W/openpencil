//! Design-session cancellation and transcript cleanup, without queued writes.

use super::{workers, DesignSession};
use op_host_native::WidgetHostNative;

pub(super) fn discard_stale_commands(session: &mut DesignSession) -> bool {
    if crate::chat_session::hosted_epoch_is_current(session.hosted_session_epoch()) {
        return false;
    }
    session.abort();
    // Drop acknowledgements along with stale commands. The following
    // progress pump retires the session and its original transcript tab.
    drop(session.drain_cmd_requests());
    true
}

pub(super) fn retire_stale_session(
    host: &mut WidgetHostNative,
    current: &mut Option<DesignSession>,
    running_tab: Option<usize>,
) -> bool {
    if !current.as_ref().is_some_and(|session| {
        !crate::chat_session::hosted_epoch_is_current(session.hosted_session_epoch())
    }) {
        return false;
    }
    *current = None;
    let locale = host.editor_state().editor_ui.locale;
    workers::finish_disconnected_design_messages(
        &mut host
            .editor_state_mut()
            .chat
            .run_tab_mut(running_tab)
            .messages,
        locale,
    );
    // The auth guard can retire a turn before its first design progress
    // event establishes worker/transcript metadata. Stop that initial
    // assistant bubble as well, without applying any queued output.
    for message in &mut host
        .editor_state_mut()
        .chat
        .run_tab_mut(running_tab)
        .messages
    {
        message.streaming = false;
    }
    host.mark_editor_state_dirty();
    true
}

/// Finalize transcript rows for an explicit Stop before the design session is
/// dropped. The widget already cleared every `streaming` bit, so worker
/// metadata and active activities — not streaming state — identify the turn.
pub(crate) fn stop_design_transcript(
    host: &mut WidgetHostNative,
    running_tab: Option<usize>,
) -> bool {
    let locale = host.editor_state().editor_ui.locale;
    let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
    let changed = workers::stop_design_messages(&mut chat.messages, locale);
    if changed {
        host.mark_editor_state_dirty();
    }
    changed
}
