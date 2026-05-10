//! Pure W3C `FocusEvent` → `jian_core::gesture::FocusEvent` mapping.
//!
//! `gained` reflects W3C `focus`/`focusin` (true) vs `blur`/`focusout`
//! (false). `node_id_hint` is the WidgetId-derived id of the node
//! that gained or lost focus, when the host can identify it (Phase D
//! DOM mirror lets us correlate the W3C target back to a WidgetId).
//! `related_node_id_hint` mirrors W3C `FocusEvent.relatedTarget` —
//! the node on the other side of the transition.
//!
//! Pure: no DOM access. C2 reads the W3C event's target /
//! relatedTarget, looks them up in the host's WidgetId registry,
//! and calls into here.

use openpencil_shell_core::FocusEvent;

pub fn map_focus(
    gained: bool,
    node_id_hint: Option<u64>,
    related_node_id_hint: Option<u64>,
) -> FocusEvent {
    FocusEvent {
        gained,
        node_id_hint,
        related_node_id_hint,
    }
}
