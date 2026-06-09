//! State-layer mirror of the icon-picker panel's buttons.
//!
//! The panel's click enum (`IconPickerHit`) carries owned `String`
//! collection/name ids for a row select, so it isn't `Copy`. The row
//! list is index-addressable (paint + hit-test share the same row
//! order), so `Row(usize)` captures the hovered row without strings.
//! Same wasm32-clean discipline as the other `*_state` mirrors.

/// Which icon-picker target the cursor is over. `None` = no hover wash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconPickerButton {
    /// The header `✕` close button.
    Close,
    /// An icon row, by its index into the visible list.
    Row(usize),
    /// The "load more" row at the foot of a remote result set.
    LoadMore,
}
