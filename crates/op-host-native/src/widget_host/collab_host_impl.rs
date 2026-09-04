//! `CollabHost` impl for the GUI host.
//!
//! The trait comes from `op-collab-host` and the type from this crate, so the
//! orphan rule forces the impl into one of the two. It lives here rather than
//! in `op-collab-host` because that crate must not depend on any host.
//!
//! ID-allocation methods forward to their inherent counterparts. The dirty
//! mark deliberately uses the host-internal cache-invalidating path because
//! collaboration overlays are baked into the canvas pan layer.

use op_collab_host::CollabHost;

use super::WidgetHostNative;

impl CollabHost for WidgetHostNative {
    fn mark_editor_state_dirty(&mut self) {
        // Collaboration presence/participant projections paint inside the
        // canvas layer. Route this trait-only dirty mark through the internal
        // cache-invalidating path without penalising unrelated public
        // `mark_editor_state_dirty` callers (chat/model/settings updates).
        self.mark_dirty();
    }

    fn enable_collaboration_ids(
        &mut self,
        namespace: op_editor_core::PeerNamespace,
    ) -> Result<(), op_editor_core::IdAllocError> {
        WidgetHostNative::enable_collaboration_ids(self, namespace)
    }

    fn disable_collaboration_ids(&mut self) {
        WidgetHostNative::disable_collaboration_ids(self);
    }
}
