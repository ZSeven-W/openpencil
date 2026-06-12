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
    /// Header-level add-theme affordance in the compact manager.
    AddTheme,
    /// Preset dropdown affordance in the modal header.
    PresetMenu,
    /// A row/button inside the open preset dropdown.
    PresetMenuItem(PresetMenuButton),
    /// Footer "add variable" affordance.
    AddVariable,
    /// A row inside the open add-variable dropdown.
    AddVariableMenuItem(usize),
    /// A theme tab in the compact manager header, by index.
    ThemeTab(usize),
    /// A row inside a theme-tab menu.
    ThemeMenuItem(usize),
    /// Column-header add-variant affordance.
    AddVariant,
    /// A variant column header, by index.
    VariantHeader(usize),
    /// A row inside a variant-column menu.
    VariantMenuItem(usize),
    /// The modal's default column header when no explicit axis chip exists.
    DefaultColumn,
    /// The modal's column-header add affordance.
    ColumnAdd,
    /// The variables search filter input.
    SearchBox,
    /// A variable row, by index.
    Row(usize),
    /// A variable row overflow-menu button, by source row index.
    RowMenuButton(usize),
    /// A row inside the open variable-row overflow menu
    /// (0 = Rename, 1 = Delete).
    RowMenuItem(usize),
    /// An active-theme axis chip in the header, by index.
    AxisChip(usize),
    /// A value row inside the open axis dropdown, by index.
    DropdownItem(usize),
}

/// Which control inside the open theme-preset dropdown is hovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetMenuButton {
    SaveCurrent,
    NameInput,
    Load(usize),
    Delete(usize),
    Import,
    Export,
}
