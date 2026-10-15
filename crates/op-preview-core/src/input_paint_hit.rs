//! The painted scene owns the hit target when engine and canvas stacking differ.

use crate::PreviewSession;
use jian_core::{
    gesture::{PointerEvent, SemanticEvent},
    spatial::NodeBBox,
};

impl PreviewSession {
    pub(crate) fn dispatch_pointer_with_painted_target(
        &mut self,
        event: PointerEvent,
        painted_id: Option<&str>,
    ) -> Vec<SemanticEvent> {
        let Some(document) = self.runtime.document.as_ref() else {
            return self.runtime.dispatch_pointer(event);
        };
        let painted = painted_id.and_then(|id| document.tree.by_id.get(id).copied());
        let engine =
            jian_core::gesture::hit_test(&self.runtime.spatial, document, event.position).topmost();
        if painted == engine {
            return self.runtime.dispatch_pointer(event);
        }
        // The design painter uses front-to-back siblings, while the generic
        // engine prefers deeper/later nodes. A nested map background must not
        // steal a tap from its painted overlay. Supply only the painted leaf;
        // the engine still builds its normal ancestor path, gesture arena,
        // bubbling, disabled checks and capture. Do not fire actions directly.
        let target = painted.and_then(|key| {
            self.runtime
                .layout
                .node_rect(key)
                .map(|rect| NodeBBox { key, rect })
        });
        self.runtime.spatial.rebuild(target);
        let events = self.runtime.dispatch_pointer(event);
        // Actions can mutate/reload the runtime. Rebuild from its CURRENT
        // document, never restore an index saved before those actions.
        self.runtime.rebuild_spatial();
        events
    }
}
