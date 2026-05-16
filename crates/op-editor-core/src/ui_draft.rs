//! Transient editor UI state — the draft buffers + focus + overlay
//! toggles that are rebuilt on load and never serialized.
//!
//! This models the *editor-state* subset of
//! `openpencil-shell-core::document::UiState`. shell-core's `UiState`
//! also carries a large amount of *widget-layer* state (hover targets
//! typed as `crate::widgets::*`, export-dialog format enums, the
//! agent-settings modal struct, etc.). Those are chrome concerns that
//! belong to a later widget-layer crate, not the editor-state layer —
//! `op-editor-core` deliberately has no widget dependency. So this
//! struct ports the parts that are genuinely editor state:
//!
//!   - the active page index (page-scoped editing — see `SelectionState`)
//!   - focused property field + its draft buffer + caret anchor
//!   - inline-edit drafts (layer rename, canvas text edit)
//!   - the pen-tool in-progress path + rubber-band cursor
//!   - the **transient variable/theme state** (spec §5.2): the active
//!     theme selection + the `fill_refs` / `stroke_refs` resolution
//!     caches. The *persisted* `variables` + `themes` live on
//!     `PenDocument`; only this rebuilt-on-load state lives here.

use crate::node_id::NodeId;
use crate::render_backend::Point2D;
use std::collections::{BTreeMap, HashMap};

/// Identifier for a property-panel input row. Ported verbatim from
/// shell-core's `PropertyFocus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyFocus {
    PositionX,
    PositionY,
    Rotation,
    PositionR,
    SizeW,
    SizeH,
    Opacity,
    FillHex,
    StrokeHex,
    StrokeWidth,
}

/// Which colour a draft edit / picker is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Fill,
    Stroke,
}

/// What an inline rename / context action is acting on. Ported from
/// shell-core's `LayerContextTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerContextTarget {
    Layer(NodeId),
    Page(usize),
}

/// Inline rename in progress on a layer or page row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRenameState {
    pub target: LayerContextTarget,
    pub draft: String,
}

/// Transient variable/theme editor state (spec §5.2).
///
/// shell-core's `VariableTable` mixed *persisted* data (`variables`,
/// `themes`) with this transient state. The persisted half now lives
/// on `PenDocument`; this struct holds only what is rebuilt on load
/// and never serialized.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableUiState {
    /// Current selection per theme axis. e.g. `{"mode": "dark"}`.
    /// Rebuilt to a default on load; not part of the `.op` file.
    pub active_theme: BTreeMap<String, String>,
    /// Resolution cache: `node id → variable name` for nodes whose
    /// fill is a `$ref`. Paint reads this first, then falls back to
    /// the node's literal fill. Rebuilt by scanning the tree on load.
    pub fill_refs: HashMap<NodeId, String>,
    /// Resolution cache: `node id → variable name` for nodes whose
    /// stroke colour follows a `$ref`. Parallel to `fill_refs`.
    pub stroke_refs: HashMap<NodeId, String>,
}

/// Transient editor UI state — draft buffers, focus, overlay toggles.
#[derive(Debug, Clone, Default)]
pub struct UiDraftState {
    /// Index into the document's pages for the page currently shown.
    /// Page-scoped editing keys off this (see `SelectionState`).
    pub active_page_index: usize,
    /// Property-panel input with keyboard focus; `None` = no focus.
    pub property_focus: Option<PropertyFocus>,
    /// Draft for the focused property input; committed on Enter,
    /// discarded on Escape.
    pub property_input_draft: String,
    /// Caret-blink anchor (ms) for the focused property input — reset
    /// on focus and on every keystroke.
    pub property_caret_anchor_ms: u64,
    /// Select-all-on-focus flag — next keystroke clears the seeded draft.
    pub property_draft_select_all: bool,
    /// Inline rename in progress on a layer or page row.
    pub layer_rename: Option<LayerRenameState>,
    /// Canvas Text node in inline text-edit mode.
    pub text_editing: Option<NodeId>,
    /// Caret-blink anchor (ms) for the inline text editor.
    pub text_edit_caret_anchor_ms: u64,
    /// In-progress Pen-tool path. `Some(id)` while the user is
    /// click-adding anchors; cleared on Enter / Escape / tool change.
    pub pen_in_progress: Option<NodeId>,
    /// Cursor position in document coords for the Pen-tool rubber band.
    pub pen_cursor_doc: Option<Point2D>,
    /// Transient variable/theme state (active theme + ref caches).
    pub variables: VariableUiState,
}

impl UiDraftState {
    /// A fresh draft state — no focus, no drafts, page 0 active.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_draft_state_is_quiescent() {
        let ui = UiDraftState::new();
        assert_eq!(ui.active_page_index, 0);
        assert!(ui.property_focus.is_none());
        assert!(ui.property_input_draft.is_empty());
        assert!(ui.pen_in_progress.is_none());
        assert!(ui.variables.active_theme.is_empty());
        assert!(ui.variables.fill_refs.is_empty());
    }
}
