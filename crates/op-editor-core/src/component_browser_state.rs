//! State-layer mirror of the component-browser panel's buttons.
//!
//! The panel's click enum (`ComponentBrowserHit`) carries owned
//! `String` ids for a card insert, so it isn't `Copy`. For the hover
//! wash we only need to identify WHICH target the cursor is over, and
//! the card grid is index-addressable (paint + hit-test share the same
//! `filtered()` order), so `Card(usize)` suffices — no string keys.
//! Same wasm32-clean discipline as the other `*_state` mirrors.

use crate::uikit::ComponentCategory;

/// Which component-browser target the cursor is over. `None` =
/// no hover wash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentBrowserButton {
    /// The header `✕` close button.
    Close,
    /// A category filter pill (`None` = the "All" pill).
    Category(Option<ComponentCategory>),
    /// A component card, by its index into the filtered grid.
    Card(usize),
}
