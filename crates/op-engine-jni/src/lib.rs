//! Android JNI marshalling layer for the OpenPencil engine player.
//!
//! `engine_thread` is the host-testable queue core and `registry` is the
//! host-testable handle table; the JNI bindings, callback trampolines, and
//! window ownership are Android-only modules.

pub mod engine_thread;
pub mod registry;

#[cfg(target_os = "android")]
pub mod alog;
#[cfg(target_os = "android")]
pub mod bindings;
#[cfg(all(target_os = "android", feature = "editor"))]
mod bindings_editor;
#[cfg(target_os = "android")]
mod bindings_media;
#[cfg(target_os = "android")]
mod bindings_text;
#[cfg(target_os = "android")]
pub mod callbacks;
#[cfg(target_os = "android")]
pub mod window;

pub use engine_thread::{Dispatch, EngineThread, STATUS_CLOSING};
pub use registry::Registry;
