//! State-layer mirror of the variables panel's buttons.
//!
//! The panel's click enum (`VariablesPanelHit`) carries an owned axis
//! name + value for a dropdown pick, so it isn't `Copy`. Rows, axis
//! chips, and the open dropdown's value rows are all index-addressable
//! (paint + hit-test share the same order), so indices suffice for the
//! hover wash — no string keys. Same wasm32-clean discipline as the
//! other `*_state` mirrors.

/// Which variables-panel target the cursor is over. `None` = no hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariablesPanelButton {
    /// Close button on the floating variables manager.
    Close,
    /// Header-level add affordance.
    HeaderAdd,
    /// Preset dropdown affordance in the modal header.
    PresetMenu,
    /// Footer "add variable" affordance.
    AddVariable,
    /// A variable row, by index.
    Row(usize),
    /// An active-theme axis chip in the header, by index.
    AxisChip(usize),
    /// A value row inside the open axis dropdown, by index.
    DropdownItem(usize),
}
