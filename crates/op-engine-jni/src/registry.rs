//! Engine-handle registry.
//!
//! Every JNI native receives an opaque `jlong` handle and MUST validate it
//! before touching engine state: a closed or unknown handle returns
//! [`STATUS_CLOSING`](crate::STATUS_CLOSING) instead of dereferencing freed
//! memory. Handles are monotonic ids (never reused — no ABA), so a
//! destroyed engine's slot stays as a TOMBSTONE that still answers
//! `nativeLastError`, and a never-allocated id is simply unknown; both are
//! rejected identically.
//!
//! The generic [`Registry`] core carries no JNI or engine types, so it is
//! compiled and unit-tested on the host.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// Reserved handle: `nativeCreate` returns `0` for failure, and the
/// create-failure error text is read back through this id.
pub const HANDLE_FAILURE: i64 = 0;

/// One registry slot. `payload` is `None` once the engine is torn down
/// (tombstone); `error` outlives the payload so `nativeLastError` works after
/// teardown.
struct Slot<T> {
    payload: Option<T>,
    error: String,
}

struct RegistryState<T> {
    /// Monotonic id source; the first live handle is `1` (`0` is reserved).
    next_id: i64,
    slots: HashMap<i64, Slot<T>>,
    /// Error text for the last `nativeCreate` that produced no handle.
    create_error: String,
}

/// A tombstoning handle table. `T` is the per-engine payload (host tests use
/// a plain value; Android uses the engine record).
pub struct Registry<T> {
    state: Mutex<RegistryState<T>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RegistryState {
                next_id: 1,
                slots: HashMap::new(),
                create_error: String::new(),
            }),
        }
    }

    /// Locks the state, RECOVERING from poison. `with`'s closure and the
    /// `Into<String>` conversions in `set_error`/`set_create_error` DO run
    /// under this guard, so a panic there could poison it; recovering
    /// guarantees a later registry access invoked from a JNI native never
    /// itself panics and crosses the non-unwinding boundary.
    fn locked(&self) -> MutexGuard<'_, RegistryState<T>> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Registers a live engine, returning its handle (always `>= 1`).
    pub fn insert(&self, payload: T) -> i64 {
        let mut state = self.locked();
        let id = state.next_id;
        state.next_id += 1;
        state.slots.insert(
            id,
            Slot {
                payload: Some(payload),
                error: String::new(),
            },
        );
        id
    }

    /// Runs `f` against the live payload, or returns `None` when the handle
    /// is unknown or tombstoned — the caller maps `None` to `STATUS_CLOSING`.
    pub fn with<R>(&self, handle: i64, f: impl FnOnce(&T) -> R) -> Option<R> {
        let state = self.locked();
        let payload = state.slots.get(&handle)?.payload.as_ref()?;
        Some(f(payload))
    }

    /// Runs `f` against the live payload with mutable access, or returns
    /// `None` when the handle is unknown or tombstoned.
    pub fn with_mut<R>(&self, handle: i64, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let mut state = self.locked();
        let payload = state.slots.get_mut(&handle)?.payload.as_mut()?;
        Some(f(payload))
    }

    /// Records the last error text for a live handle (no-op for an
    /// unknown/tombstoned handle — a tombstone keeps the error it died with).
    pub fn set_error(&self, handle: i64, message: impl Into<String>) {
        let mut state = self.locked();
        if let Some(slot) = state.slots.get_mut(&handle) {
            slot.error = message.into();
        }
    }

    /// The last error text for a handle — tombstoned slots keep their error.
    pub fn error(&self, handle: i64) -> Option<String> {
        let state = self.locked();
        state.slots.get(&handle).map(|slot| slot.error.clone())
    }

    /// Error text from the last `nativeCreate` that produced no handle.
    pub fn create_error(&self) -> String {
        self.locked().create_error.clone()
    }

    /// Records the create failure text (read back via
    /// `nativeLastError(HANDLE_FAILURE)`).
    pub fn set_create_error(&self, message: impl Into<String>) {
        self.locked().create_error = message.into();
    }

    /// Tombstones a live engine: drops the payload (engine + thread teardown
    /// has already run) but keeps the error text readable.
    pub fn remove(&self, handle: i64) {
        let mut state = self.locked();
        if let Some(slot) = state.slots.get_mut(&handle) {
            slot.payload = None;
        }
    }

    /// Removes a live engine record for teardown, returning its payload
    /// (the caller runs the teardown). `None` for an unknown or already
    /// tombstoned handle.
    pub fn take(&self, handle: i64) -> Option<T> {
        let mut state = self.locked();
        let slot = state.slots.get_mut(&handle)?;
        slot.payload.take()
    }

    /// The last error text for a handle — tombstoned slots keep their error;
    /// `HANDLE_FAILURE` reads the create-failure text.
    pub fn last_error(&self, handle: i64) -> String {
        if handle == HANDLE_FAILURE {
            return self.create_error();
        }
        self.error(handle).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_monotonic_and_never_reused() {
        let registry = Registry::new();
        let a = registry.insert(10u32);
        let b = registry.insert(20u32);
        assert!(a >= 1 && b > a);
        registry.remove(a);
        // The freed id is a tombstone, not a free slot.
        assert_eq!(registry.with(a, |v| *v), None);
        let c = registry.insert(30u32);
        assert!(c > b);
    }

    #[test]
    fn with_and_with_mut_see_live_payloads_only() {
        let registry = Registry::new();
        let handle = registry.insert(vec![1, 2, 3]);
        assert_eq!(registry.with(handle, |v| v.len()), Some(3));
        registry.with_mut(handle, |v| v.push(4));
        assert_eq!(registry.with(handle, |v| v.len()), Some(4));
        registry.remove(handle);
        assert_eq!(registry.with(handle, |v| v.len()), None);
        // Unknown ids behave identically to tombstones.
        assert_eq!(registry.with(999, |v| v.len()), None);
    }

    #[test]
    fn errors_outlive_payloads_and_creates() {
        let registry = Registry::new();
        registry.set_create_error("bad document");
        assert_eq!(registry.create_error(), "bad document");

        let handle = registry.insert(1u32);
        registry.set_error(handle, "gpu failure");
        assert_eq!(registry.error(handle).as_deref(), Some("gpu failure"));
        registry.remove(handle);
        assert_eq!(registry.error(handle).as_deref(), Some("gpu failure"));
        assert_eq!(registry.error(999), None);
    }
}
