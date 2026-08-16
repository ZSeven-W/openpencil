//! Small synchronous helpers shared by the VS Code bridge handlers.

use super::*;

pub(super) fn read_triple<C: RepaintContext>(inner: &Rc<RefCell<C>>) -> Option<(u64, u64, bool)> {
    let b = inner.try_borrow().ok()?;
    let s = b.host().editor_state();
    Some((s.document_generation(), s.document_revision(), s.is_dirty()))
}

/// Serialize the live document and editor metadata atomically.
pub(super) fn snapshot_state<C: RepaintContext>(
    inner: &Rc<RefCell<C>>,
) -> Option<BridgeDocumentSnapshot> {
    let b = inner.try_borrow().ok()?;
    BridgeDocumentSnapshot::capture(b.host().editor_state())
}

/// Try to claim the shared push single-flight latch. `true` when acquired.
pub(super) fn acquire_push_busy(sync: &SharedSync) -> bool {
    match sync.try_borrow_mut() {
        Ok(mut s) if !s.push_busy => {
            s.push_busy = true;
            true
        }
        _ => false,
    }
}

pub(super) fn release_push_busy(sync: &SharedSync) {
    if let Ok(mut s) = sync.try_borrow_mut() {
        s.push_busy = false;
    }
}

/// Post a codec string to the locked host origin (falling back to `*` only
/// before the origin is known — by which point every real reply is sent).
pub(super) fn post_to_parent(json: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(parent) = window.parent().ok().flatten() else {
        return;
    };
    let target = BRIDGE_ORIGIN
        .with(|o| o.borrow().clone())
        .unwrap_or_else(|| "*".to_string());
    let _ = parent.post_message(&JsValue::from_str(json), &target);
}

/// One-shot `setTimeout`. `once_into_js` self-frees after firing.
pub(super) fn schedule_once<F: FnOnce() + 'static>(delay_ms: i32, f: F) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::once_into_js(f);
    let _ =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), delay_ms);
}
