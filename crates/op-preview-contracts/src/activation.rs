//! User activation ids (R3/R4 contracts).
//!
//! An activation id is an opaque handle a host stamps on an input
//! envelope to certify "this dispatch carries fresh user intent". The
//! Preview session stores the activation ONLY for the synchronous
//! ActionList that input spawns and expires it before delayed/async
//! work — a later task can never inherit a stale gesture's consent.

use serde::{Deserialize, Serialize};

/// Opaque, monotonically allocated activation handle. Hosts mint ids
/// (any monotonic counter works); the session only ever compares and
/// expires them — it never interprets the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserActivationId(u64);

impl UserActivationId {
    /// Wrap a host-allocated counter value. The value's meaning is
    /// host-defined; sessions must treat ids as opaque handles.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The wrapped host counter value, for FFI/JNI/NAPI marshalling.
    pub const fn raw(self) -> u64 {
        self.0
    }
}
