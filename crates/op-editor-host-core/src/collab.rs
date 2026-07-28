//! Transport-neutral collaboration/editor actor integration.
//!
//! The owner sequencer and the live editor are driven synchronously by one
//! caller. Network threads may deliver typed frames, but never own the
//! document or global sequence.

mod guest;
#[cfg(test)]
mod guest_tests;
mod host;
mod owner;
#[cfg(test)]
mod owner_tests;

pub use guest::{
    GuestEditorError, GuestEditorLimits, GuestEditorOutput, GuestEditorSession,
    GuestLocalEditRejection, GuestLocalEditResolution,
};
pub use host::CollaborationEditorHost;
pub use owner::{
    LocalEditRejection, LocalEditResolution, OwnerEditorError, OwnerEditorLimits,
    OwnerEditorOutput, OwnerEditorSession,
};
