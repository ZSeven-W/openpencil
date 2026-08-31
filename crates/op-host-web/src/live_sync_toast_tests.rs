//! End-to-end guards for the two paths that own the editor toast in the
//! browser host: the online conflict auto-accept raises it, and an account
//! switch clears it.
//!
//! These drive the real functions against a real `WidgetHost` rather than
//! asserting on source text — the toast exists precisely because nobody was
//! watching this path, so its wiring has to be exercised, not pinned.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsValue;

use crate::repaint_ctx::RepaintContext;
use crate::widget_host::WidgetHost;

/// Minimal host context: everything the conflict + identity paths touch is
/// `host_mut().editor_state_mut()` plus a repaint tally.
struct ToastContext {
    host: WidgetHost,
    repaints: usize,
}

impl RepaintContext for ToastContext {
    fn host(&self) -> &WidgetHost {
        &self.host
    }

    fn host_mut(&mut self) -> &mut WidgetHost {
        &mut self.host
    }

    fn viewport_size(&self) -> (f32, f32) {
        (1440.0, 900.0)
    }

    fn register_system_font(&mut self, _family: &str, _bytes: &[u8]) -> bool {
        false
    }

    fn register_imported_font(&mut self, _family: &str, _bytes: &[u8]) -> bool {
        false
    }

    fn register_imported_font_from_bytes(&mut self, _bytes: &[u8]) -> Option<String> {
        None
    }

    fn imported_family_list(&self) -> Vec<String> {
        Vec::new()
    }

    fn remove_imported_font(&mut self, _family: &str) {}

    fn repaint(&mut self) -> Result<(), JsValue> {
        self.repaints += 1;
        Ok(())
    }
}

fn context() -> Rc<RefCell<ToastContext>> {
    Rc::new(RefCell::new(ToastContext {
        host: WidgetHost::new(),
        repaints: 0,
    }))
}

fn toast_key(inner: &Rc<RefCell<ToastContext>>) -> Option<String> {
    inner
        .borrow()
        .host
        .editor_state()
        .editor_ui
        .editor_toast
        .as_ref()
        .map(|toast| toast.i18n_key.clone())
}

#[test]
fn an_online_conflict_accept_raises_the_recovery_toast() {
    let inner = context();
    // The online daemon is the sequencer — the deployment where the
    // collaboration panel that would otherwise carry this notice is
    // structurally unreachable.
    super::live_sync_conflict::set_server_authoritative_for_test(true);

    assert!(super::live_sync_conflict::preserve_local_document(
        &inner, 1_000
    ));

    assert_eq!(
        toast_key(&inner).as_deref(),
        Some("collab.status.localEditPreserved"),
        "the user must be told their document was replaced and how to get it back"
    );
    let level = inner
        .borrow()
        .host
        .editor_state()
        .editor_ui
        .editor_toast
        .as_ref()
        .map(|toast| toast.level);
    assert_eq!(
        level,
        Some(op_editor_core::editor_toast::EditorToastLevel::Warn),
        "a document replaced under the user is not an Info-level event"
    );
}

#[test]
fn a_session_deployment_leaves_the_toast_alone() {
    // Desktop / LAN sessions show the same sentence in the collaboration
    // panel's notice strip. Raising a toast as well would say it twice.
    let inner = context();
    super::live_sync_conflict::set_server_authoritative_for_test(false);

    assert!(super::live_sync_conflict::preserve_local_document(
        &inner, 1_000
    ));

    assert_eq!(toast_key(&inner), None);
    assert!(
        inner
            .borrow()
            .host
            .editor_state()
            .editor_ui
            .collab
            .notice
            .is_some(),
        "the panel notice is still raised on every deployment"
    );
}

#[test]
fn switching_accounts_clears_a_toast_raised_for_the_previous_one() {
    // A toast names an undo that belongs to the previous account's document.
    // Leaving it up would show one user a sentence about another user's data.
    let inner = context();
    super::live_sync_conflict::set_server_authoritative_for_test(true);
    assert!(super::live_sync_conflict::preserve_local_document(
        &inner, 1_000
    ));
    assert!(
        toast_key(&inner).is_some(),
        "a toast is up before the switch"
    );
    {
        let mut context = inner.borrow_mut();
        context.host.editor_state_mut().editor_ui.pending_locale = Some(op_editor_core::Locale::Ja);
        context
            .host
            .editor_state_mut()
            .editor_ui
            .locale_persistence_override = Some(op_editor_core::Locale::Ja);
    }
    let repaints_before_reset = inner.borrow().repaints;

    super::live_sync_identity::reset_for_new_identity(&inner);

    assert_eq!(
        toast_key(&inner),
        None,
        "the reset must not carry one account's notice into the next"
    );
    let context = inner.borrow();
    assert_eq!(context.repaints, repaints_before_reset);
    assert_eq!(context.host.editor_state().editor_ui.pending_locale, None);
    assert_eq!(
        context
            .host
            .editor_state()
            .editor_ui
            .locale_persistence_override,
        None
    );
}
