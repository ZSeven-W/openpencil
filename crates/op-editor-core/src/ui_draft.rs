//! Transient editor UI state — the draft buffers + focus + overlay
//! toggles that are rebuilt on load and never serialized.
//!
//! This models the *editor-state* subset of
//! `openpencil-shell-core::document::UiState`. shell-core's `UiState`
//! also carries a large amount of *widget-layer* state (hover targets
//! typed as `crate::widgets::*`, export-dialog format enums, the
//! agent-settings modal struct, etc.). Those are UI concerns that
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
    /// Container flex-layout gap in document px.
    LayoutGap,
    /// Container padding values, shown in top/right/bottom/left order.
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    Opacity,
    FillHex,
    /// Fill section's `100 %` opacity input — percentage (0..100).
    FillOpacity,
    /// LinearGradient angle input — degrees in the canonical `.op`
    /// convention (0° = bottom→top, 90° = left→right). Only valid
    /// while the selected node's first fill is `LinearGradient`.
    GradientAngle,
    /// One gradient stop's `#RRGGBB` colour input — indexed into the
    /// first fill's `stops` list. Valid for both Linear and Radial
    /// fills; an out-of-range index is treated as no-op at commit.
    GradientStopHex(usize),
    /// One gradient stop's offset input — percentage in `0..=100`
    /// (stored as 0.0..=1.0 in the canonical schema). Same indexing
    /// + scope rules as [`GradientStopHex`].
    GradientStopOffset(usize),
    /// Polygon side count input. UI clamps to `3..=100`.
    PolygonSides,
    /// Ellipse arc start angle, in degrees.
    EllipseStart,
    /// Ellipse arc sweep angle, in degrees.
    EllipseSweep,
    /// Ellipse donut-hole radius, shown as a percentage.
    EllipseInnerRadius,
    /// Text font size in document px.
    FontSize,
    /// Text font weight, numeric 1..=1000.
    FontWeight,
    /// Text line height, shown as a percentage.
    LineHeight,
    /// Text letter spacing in document px.
    LetterSpacing,
    StrokeHex,
    StrokeWidth,
}

impl PropertyFocus {
    /// True when the focused row carries hex colour input — drives
    /// the `#`-sticky keyboard validation. Hex focuses cap the draft
    /// at 7 chars (`#RRGGBB`).
    pub fn is_hex(self) -> bool {
        matches!(
            self,
            PropertyFocus::FillHex | PropertyFocus::StrokeHex | PropertyFocus::GradientStopHex(_)
        )
    }

    /// True when the focused row accepts a decimal point — used by
    /// the numeric input validator. Position / Size / corner radius
    /// reject `.` (integer doc px); rotation / opacity / stroke width
    /// / gradient angle / gradient offset accept it.
    pub fn accepts_decimal(self) -> bool {
        matches!(
            self,
            PropertyFocus::Rotation
                | PropertyFocus::PositionR
                | PropertyFocus::Opacity
                | PropertyFocus::FillOpacity
                | PropertyFocus::LayoutGap
                | PropertyFocus::PaddingTop
                | PropertyFocus::PaddingRight
                | PropertyFocus::PaddingBottom
                | PropertyFocus::PaddingLeft
                | PropertyFocus::StrokeWidth
                | PropertyFocus::GradientAngle
                | PropertyFocus::GradientStopOffset(_)
                | PropertyFocus::EllipseStart
                | PropertyFocus::EllipseSweep
                | PropertyFocus::EllipseInnerRadius
                | PropertyFocus::FontSize
                | PropertyFocus::LineHeight
                | PropertyFocus::LetterSpacing
        )
    }
}

/// Which colour a draft edit / picker is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Fill,
    Stroke,
    /// One stop on the primary gradient body — indexed against
    /// `stops`. Out-of-range writes are silently dropped at commit.
    GradientStop(usize),
    /// The colour of the visual effect at `index` (Shadow only).
    /// Out-of-range writes silently drop at commit; the picker stays
    /// open against a missing target so the user can re-pick.
    EffectColor(usize),
}

/// What an inline rename / context action is acting on. Ported from
/// shell-core's `LayerContextTarget`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerContextTarget {
    Layer(NodeId),
    Page(usize),
}

/// Right-click context menu on a path anchor / handle (Select tool).
/// Mirrors TS `PathAnchorContextMenu` (`skia-canvas.tsx:276-332`):
/// Corner Point / Symmetric Curve / Free Curve / Reset Handles.
#[derive(Debug, Clone, PartialEq)]
pub struct PathAnchorMenuState {
    /// The Path node whose anchor was right-clicked.
    pub node_id: NodeId,
    /// Index of the right-clicked anchor (or the anchor owning the
    /// right-clicked handle).
    pub anchor_index: usize,
    /// Menu anchor in viewport coords (the right-click position).
    pub x: f32,
    pub y: f32,
    /// Hovered row index for the menu paint; `None` = no row hovered.
    pub hovered_row: Option<u8>,
}

/// Inline rename in progress on a layer or page row.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRenameState {
    pub target: LayerContextTarget,
    pub input: jian_core::text_input::TextInputState,
}

/// Which control of the HSV colour picker a drag is currently
/// driving. Ported from shell-core's `ColorPickerDrag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerDrag {
    /// The 2-D saturation / value box.
    SvBox,
    /// The 1-D hue strip.
    HueSlider,
}

/// Floating HSV colour-picker state. The picker keeps `(hue, sat,
/// val)` as the source of truth so dragging a control doesn't
/// visibly snap when round-trip RGB rounding pulls a slightly
/// different hex out of the same HSV.
///
/// shell-core's `ColorPickerState` carried a `pre_snap:
/// Option<DocumentSnapshot>`; here the pre-edit `EditorSnapshot` is
/// held in [`UiDraftState::pending_color_history`] instead — parallel
/// to how the pen tool stashes `pending_pen_history` — so the picker
/// state stays a plain value type.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorPickerState {
    /// Whether the picker edits the node's fill or stroke colour.
    pub target: ColorTarget,
    /// Hue in degrees, 0..360.
    pub hue: f32,
    /// Saturation, 0..1.
    pub sat: f32,
    /// Value (brightness), 0..1.
    pub val: f32,
    /// The control currently being dragged; `None` while idle.
    pub drag: Option<ColorPickerDrag>,
    /// Viewport-y of the click that opened the picker — anchors the
    /// floating panel.
    pub anchor_y: f32,
    /// Optional viewport-x of the click that opened the picker. When
    /// absent, the widget keeps the legacy right-rail anchor.
    pub anchor_x: Option<f32>,
    /// When `Some(name)`, the picker edits the named Color **variable**
    /// instead of the selected node's fill / stroke. The commit path
    /// (`color_picker_set_hsv`) then routes through
    /// [`crate::state::EditorState::set_variable_color`]. `target` is
    /// unused on the variable path but still carries a sane default for
    /// any code that pattern-matches on it without checking `variable`.
    /// Ported from shell-core's `ColorPickerState::variable`.
    pub variable: Option<String>,
    /// When `Some((axis, value))` alongside `variable`, the picker
    /// edits that variable's THEMED value for exactly the clicked
    /// variant column instead of routing through the active theme.
    /// Mirrors TS `variable-row.tsx setValueForTheme` — clicking the
    /// "Dark" column swatch writes the Dark entry even while the
    /// canvas renders Light. Unused in node mode.
    pub variable_theme: Option<(String, String)>,
    /// Alpha (`0..=1`) the picker preserves on every commit. Seeded
    /// from the target's current colour at open time so adjusting
    /// SV / hue on a transparent gradient stop (`#00000000`) keeps
    /// it transparent instead of silently turning it opaque. Fill /
    /// stroke / variable targets always seed `1.0` because their
    /// alpha lives in a separate opacity field.
    pub alpha: f32,
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
    /// Draft, caret, selection, and blink state for the focused
    /// property-panel input.
    pub property_input: jian_core::text_input::TextInputState,
    /// Legacy compatibility mirror for focused property / variables
    /// inputs while older paint and preset-name paths are being
    /// migrated. New focused property edits live in `property_input`.
    pub property_input_draft: String,
    /// Legacy compatibility caret mirror for `property_input_draft`.
    pub property_caret_pos: usize,
    /// Legacy compatibility caret-blink anchor for older paint paths.
    pub property_caret_anchor_ms: u64,
    /// Legacy compatibility select-all mirror for `property_input_draft`.
    pub property_draft_select_all: bool,
    /// Inline rename in progress on a layer or page row.
    pub layer_rename: Option<LayerRenameState>,
    /// Canvas Text node in inline text-edit mode.
    pub text_editing: Option<NodeId>,
    /// Draft, caret, selection, IME composition, and blink state for
    /// the inline canvas text editor.
    pub text_edit_input: jian_core::text_input::TextInputState,
    /// In-progress Pen-tool path. `Some(id)` while the user is
    /// click-adding anchors; cleared on Enter / Escape / tool change.
    pub pen_in_progress: Option<NodeId>,
    /// Cursor position in document coords for the Pen-tool rubber band.
    pub pen_cursor_doc: Option<Point2D>,
    /// True between a Pen-tool press and its release — cursor moves
    /// mint mirrored bezier handles on the just-placed anchor instead
    /// of tracking the rubber band (TS `penDraggingHandle`,
    /// `skia-pen-tool.ts:97-110`).
    pub pen_dragging_handle: bool,
    /// Last Pen-tool press `(now_ms, viewport_x, viewport_y)` — drives
    /// the double-click-finish detection (TS finishes the path on
    /// `dblclick`; the host detects two presses < 400 ms / < 4 px apart).
    pub pen_last_press: Option<(u64, f32, f32)>,
    /// Pre-pen history snapshot — captured BEFORE `start_pen_path`
    /// mutates the tree, pushed onto the undo stack only when the
    /// finished path has ≥ 2 anchors. Dropped on a 1-anchor cancel.
    pub pending_pen_history: Option<crate::history::EditorSnapshot>,
    /// Open right-click menu on a path anchor; `None` when closed.
    pub path_anchor_menu: Option<PathAnchorMenuState>,
    /// Active HSV colour picker overlay; `None` when closed.
    pub color_picker: Option<ColorPickerState>,
    /// Pre-edit snapshot captured when the colour picker opens;
    /// pushed onto undo on close only when the colour changed.
    pub pending_color_history: Option<crate::history::EditorSnapshot>,
    /// Pre-edit snapshot for the inline text-edit session; opened
    /// lazily on the first keystroke, dropped if the text is
    /// unchanged at commit. See `rename.rs`.
    pub pending_text_edit_history: Option<crate::history::EditorSnapshot>,
    /// Timestamp (ms) of the last text-edit keystroke — drives the
    /// 500 ms history-coalescing window.
    pub text_edit_last_ms: u64,
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
