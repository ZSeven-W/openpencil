//! The ABI engine handle ([`OpEngine`]) and its call/destroy dispatch —
//! the owner-thread + re-entrancy + panic contract around every C ABI
//! call. Carved out of `lifecycle.rs` as pure code motion to keep that
//! file under the 800-line cap.

use crate::error::FfiResult;
use crate::OpStatus;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(feature = "editor")]
use super::settings_persistence_active;
use super::Session;

/// The engine handle handed across the ABI. Owned by exactly one thread:
/// every call checks `owner` and refuses to re-enter while a call is in
/// flight, and any panic crossing the boundary poisons the engine.
pub struct OpEngine {
    owner: std::thread::ThreadId,
    in_call: Cell<bool>,
    poisoned: Cell<bool>,
    last_error: RefCell<String>,
    // `pub(super)` for the lifecycle test siblings, which inspect the
    // session directly (they lived in the same module before the split).
    pub(super) session: UnsafeCell<Session>,
}

impl OpEngine {
    /// Wrap a session as an ABI handle owned by the calling thread.
    pub(crate) fn new(session: Session) -> OpEngine {
        OpEngine {
            owner: std::thread::current().id(),
            in_call: Cell::new(false),
            poisoned: Cell::new(false),
            last_error: RefCell::new(String::new()),
            session: UnsafeCell::new(session),
        }
    }

    /// Clone of the last call error (for `op_last_error`).
    pub(crate) fn last_error(&self) -> String {
        self.last_error.borrow().clone()
    }

    #[cfg(all(test, feature = "editor"))]
    pub(crate) fn session_mut_for_test(&mut self) -> &mut Session {
        // Unit tests own the engine on this thread and never overlap an ABI
        // call with this inspection seam.
        unsafe { &mut *self.session.get() }
    }
}

/// Dispatch one engine call with the thread + re-entrancy + panic
/// contract. The session's per-call diagnostics are emitted afterwards.
///
/// # Safety
///
/// `pointer` must be a live `OpEngine` from `op_create` on the calling
/// thread.
pub(crate) unsafe fn call_session(
    pointer: *mut OpEngine,
    call: impl FnOnce(&mut Session) -> FfiResult<()>,
) -> OpStatus {
    if pointer.is_null() {
        return OpStatus::InvalidArg;
    }
    let engine = unsafe { &*pointer };
    if engine.owner != std::thread::current().id() || engine.in_call.get() {
        return OpStatus::WrongThread;
    }
    if engine.poisoned.get() {
        return OpStatus::Poisoned;
    }
    engine.in_call.set(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let session = unsafe { &mut *engine.session.get() };
        // Desktop persists settings by fingerprinting around every window
        // event; the ABI boundary is the mobile analogue of that single
        // chokepoint, so a settings edit can never miss its save. The
        // fingerprint is a cheap field snapshot and the save only runs
        // when a persisted field actually changed.
        #[cfg(feature = "editor")]
        let settings_before = if settings_persistence_active() {
            session
                .editor()
                .map(|host| op_editor_host_core::settings_io::fingerprint(host.editor_state()))
        } else {
            None
        };
        let result = call(session);
        #[cfg(feature = "editor")]
        if let (Some(before), Some(host)) = (settings_before, session.editor()) {
            op_editor_host_core::settings_io::save_if_changed(host.editor_state(), before);
        }
        if let Err(error) = &result {
            session.emit_runtime_error(2, &error.message, "op-engine-ffi");
        }
        result
    }));
    engine.in_call.set(false);
    match outcome {
        Ok(Ok(())) => OpStatus::Ok,
        Ok(Err(error)) => {
            *engine.last_error.borrow_mut() = error.message;
            error.status
        }
        Err(_) => {
            engine.poisoned.set(true);
            *engine.last_error.borrow_mut() =
                "panic crossed the OpenPencil ABI boundary".to_owned();
            OpStatus::Poisoned
        }
    }
}

pub(crate) unsafe fn destroy_engine(pointer: *mut OpEngine) -> OpStatus {
    if pointer.is_null() {
        return OpStatus::InvalidArg;
    }
    let engine = unsafe { &*pointer };
    if engine.owner != std::thread::current().id() || engine.in_call.get() {
        return OpStatus::WrongThread;
    }
    engine.in_call.set(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| unsafe {
        // Desktop `exiting` parity: flush a focused-but-uncommitted
        // settings draft (e.g. the MCP port field), then persist.
        #[cfg(feature = "editor")]
        if settings_persistence_active() {
            let session = &mut *engine.session.get();
            if let Some(host) = session.editor.as_mut() {
                host.flush_settings_input();
                op_editor_host_core::settings_io::save(host.editor_state());
            }
        }
        #[cfg(feature = "editor")]
        (&mut *engine.session.get()).shutdown_editor_collab();
        #[cfg(feature = "editor")]
        crate::editor_auth::shutdown(&mut *engine.session.get());
        drop(Box::from_raw(pointer));
    }));
    match outcome {
        Ok(()) => OpStatus::Ok,
        Err(_) => OpStatus::Poisoned,
    }
}
