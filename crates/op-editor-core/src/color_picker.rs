//! Colour / fill mutators + the HSV colour-picker state machine —
//! ported from shell-core's `document/color_picker.rs` plus the
//! fill-related parts of `document/mutators.rs`
//! (`set_selected_color` / `set_selected_fill_type` /
//! `add_drop_shadow_to_selected`).
//!
//! ## Fill model
//!
//! shell-core's flat `Node` had `fill: Option<Color>` — a single
//! literal colour. The canonical `PenNode` carries
//! `fill: Option<Vec<PenFill>>` (Solid / gradient / Image variants,
//! hex `String` colours). These mutators only ever touch "the first
//! solid fill's hex" — the [`crate::fills`] helpers do that read /
//! write while preserving any gradient / image fills verbatim.
//!
//! ## Colour-picker history
//!
//! shell-core stored the pre-edit snapshot inside `ColorPickerState`.
//! Here the snapshot lives in `ui.pending_color_history` (parallel to
//! the pen tool's `pending_pen_history`) so `ColorPickerState` stays
//! a plain value type. `close_color_picker` pushes that snapshot onto
//! undo only when the colour actually changed.

use crate::color_picker_snapshot::{
    effect_color_hex, gradient_stop_hex, scalar_as_hex, snapshot_active_children,
    snapshot_variable_hex, splice_alpha, variant_scalar,
};
use crate::fills::{
    first_solid_fill_hex, first_solid_stroke_hex, push_drop_shadow, set_primary_fill_hex,
    set_primary_stroke_hex,
};
use crate::state::EditorState;
use crate::ui_draft::{ColorPickerDrag, ColorPickerState, ColorTarget};
use crate::walkers::find_node_mut;

// The colour maths is canonical in `jian_core::color` (one copy, shared with
// jian-widgets' painting). Re-export it through this module so existing
// `crate::color_picker::{hsv_to_rgb, parse_hex_rgb, …}` callers (and the
// `lib.rs` crate-root re-export) keep resolving.
pub use jian_core::color::{hsv_to_rgb, parse_hex_alpha, parse_hex_rgb, rgb_to_hex, rgb_to_hsv};

/// Hex of the solid fill at `index`, when that fill is a solid. Falls
/// back to the first solid fill for index 0 (so the primary-fill
/// picker seeds the same way it always did). Used by the colour picker
/// to seed HSV from / detect changes on a specific fill row.
fn indexed_solid_fill_hex(node: &jian_ops_schema::node::PenNode, index: usize) -> Option<String> {
    use jian_ops_schema::style::PenFill;
    let fills = crate::fills::node_fills(node)?;
    match fills.get(index) {
        Some(PenFill::Solid(b)) => Some(b.color.clone()),
        Some(_) => None,
        None if index == 0 => first_solid_fill_hex(node).map(str::to_string),
        None => None,
    }
}

impl EditorState {
    // --- Fill / stroke colour ---------------------------------------

    /// Write a `#rrggbb` hex to the anchor node's fill (`is_fill`) or
    /// stroke colour. Editable-gated. Returns true when the write
    /// landed. Mirrors shell-core's `set_selected_color`.
    pub fn set_selected_color(&mut self, is_fill: bool, hex: &str) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        if is_fill {
            set_primary_fill_hex(node, hex)
        } else {
            set_primary_stroke_hex(node, hex)
        }
    }

    /// Write the anchor node's primary-fill opacity, in `[0.0, 1.0]`.
    /// Editable-gated. Drives the Fill section's `100 %` input.
    pub fn set_selected_fill_opacity(&mut self, opacity: f32) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        crate::fills::set_primary_fill_opacity(node, opacity)
    }

    /// Append a default drop-shadow effect to the anchor node.
    /// Editable-gated. Mirrors shell-core's
    /// `add_drop_shadow_to_selected`.
    pub fn add_drop_shadow_to_selected(&mut self) -> bool {
        self.add_effect_to_selected(push_drop_shadow)
    }

    /// Append a default Gaussian layer-blur effect to the anchor node.
    /// Editable-gated companion to [`Self::add_drop_shadow_to_selected`].
    pub fn add_layer_blur_to_selected(&mut self) -> bool {
        self.add_effect_to_selected(crate::fills::push_layer_blur)
    }

    /// Shared effect-add path: history is snapshotted ONLY when the
    /// anchor node exists and actually carries an effects list, so a
    /// Ref / IconFont / missing target never leaves an empty undo +
    /// dirty state.
    fn add_effect_to_selected(
        &mut self,
        push: fn(&mut jian_ops_schema::node::PenNode) -> bool,
    ) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let supported = crate::walkers::find_node(self.active_children(), &sel)
            .map(crate::fills::node_supports_effects)
            .unwrap_or(false);
        if !supported {
            return false;
        }
        // Snapshot before mutating so the add is undoable.
        self.commit_history();
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        push(node)
    }

    // --- HSV colour picker ------------------------------------------

    /// Open the floating colour picker on the given target. Seeds HSV
    /// from the anchor node's current fill / stroke colour. Captures
    /// a pre-edit history snapshot. Returns false when there is no
    /// editable selection to edit. A `Fill` target opens against the
    /// primary fill (index 0); use [`Self::open_color_picker_for_fill`]
    /// to target a specific fill row.
    pub fn open_color_picker(&mut self, target: ColorTarget, anchor_y: f32) -> bool {
        self.open_color_picker_for_fill(target, 0, anchor_y)
    }

    /// Like [`Self::open_color_picker`] but, for a `Fill` target, binds
    /// the picker to fill `fill_index` (the Fill section stacks one row
    /// per fill, so each swatch writes back to its own fill). The index
    /// is ignored for non-`Fill` targets.
    pub fn open_color_picker_for_fill(
        &mut self,
        target: ColorTarget,
        fill_index: usize,
        anchor_y: f32,
    ) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = self.selected_node() else {
            return false;
        };
        // A Ref anchor seeds from the merged instance display node so
        // the picker opens on the override-effective colour instead of
        // the Ref's (always-empty) own fill (#000000 fallback).
        let display_node;
        let node = match crate::instance_override::resolve_instance_display_node(&self.doc, node) {
            Some(display) => {
                display_node = display;
                &display_node
            }
            None => node,
        };
        let current_hex = match target {
            ColorTarget::Fill => indexed_solid_fill_hex(node, fill_index),
            ColorTarget::Stroke => first_solid_stroke_hex(node).map(str::to_string),
            ColorTarget::GradientStop(i) => gradient_stop_hex(node, i),
            ColorTarget::EffectColor(i) => effect_color_hex(node, i),
        }
        .unwrap_or_else(|| "#000000".to_string());
        let (h, s, v) = rgb_to_hsv(parse_hex_rgb(&current_hex).unwrap_or((0.0, 0.0, 0.0)));
        // Preserve per-stop / per-effect alpha across picker edits.
        // Fill / stroke ignore alpha (they carry it in a separate
        // opacity input) so this only matters for `GradientStop`
        // and `EffectColor`.
        let alpha = match target {
            ColorTarget::GradientStop(_) | ColorTarget::EffectColor(_) => {
                parse_hex_alpha(&current_hex)
            }
            _ => 1.0,
        };
        self.ui.pending_color_history = Some(self.snapshot_for_history());
        self.ui.color_picker = Some(ColorPickerState {
            target,
            hue: h,
            sat: s,
            val: v,
            drag: None,
            anchor_y,
            anchor_x: None,
            variable: None,
            variable_theme: None,
            alpha,
            fill_index,
            hex_focused: false,
            hex_input: jian_core::text_input::TextInputState::default(),
            rgb_focus: None,
            rgb_input: jian_core::text_input::TextInputState::default(),
        });
        true
    }

    /// Open the picker rooted at a Color **variable** instead of the
    /// selected node. Seeds HSV from the variable's currently-resolved
    /// scalar under the active theme. Returns false when no variable
    /// of that name exists, it isn't Color-kind, or its scalar isn't a
    /// parseable hex. The picker's commit path (live HSV → RGB) then
    /// routes through `set_variable_color` instead of
    /// `set_selected_color`. Ported from shell-core's
    /// `Document::open_color_picker_for_variable`.
    pub fn open_color_picker_for_variable(
        &mut self,
        variable: impl Into<String>,
        anchor_y: f32,
    ) -> bool {
        self.open_color_picker_for_variable_with_anchor(variable, None, None, anchor_y)
    }

    /// Same as [`Self::open_color_picker_for_variable`], but anchors
    /// the floating picker near the clicked variable swatch.
    pub fn open_color_picker_for_variable_at(
        &mut self,
        variable: impl Into<String>,
        anchor_x: f32,
        anchor_y: f32,
    ) -> bool {
        self.open_color_picker_for_variable_with_anchor(variable, None, Some(anchor_x), anchor_y)
    }

    /// Variant-column-targeted picker open (#19). Seeds HSV from the
    /// variable's value under exactly `(axis, theme_value)` — NOT the
    /// active theme — and arms the commit path to write that themed
    /// entry. Mirrors TS `variable-row.tsx` ColorCell + setValueForTheme.
    pub fn open_color_picker_for_variable_theme_at(
        &mut self,
        variable: impl Into<String>,
        axis: impl Into<String>,
        theme_value: impl Into<String>,
        anchor_x: f32,
        anchor_y: f32,
    ) -> bool {
        self.open_color_picker_for_variable_with_anchor(
            variable,
            Some((axis.into(), theme_value.into())),
            Some(anchor_x),
            anchor_y,
        )
    }

    fn open_color_picker_for_variable_with_anchor(
        &mut self,
        variable: impl Into<String>,
        variable_theme: Option<(String, String)>,
        anchor_x: Option<f32>,
        anchor_y: f32,
    ) -> bool {
        let name = variable.into();
        // Resolve the variable's current colour to seed HSV. Reject
        // unknown names, non-Color kinds, and non-hex scalars.
        let Some(def) = self.find_variable(&name) else {
            return false;
        };
        if !matches!(def.kind, jian_ops_schema::variable::VariableKind::Color) {
            return false;
        }
        // Variant-targeted opens seed from THAT column's value (TS
        // ColorCell shows the per-column value); the active-theme
        // resolve only serves the plain (row-level) open.
        let scalar = match &variable_theme {
            Some((axis, value)) => match variant_scalar(&def.value, axis, value) {
                Some(jian_ops_schema::variable::VariableScalar::Str(s)) => s.clone(),
                _ => return false,
            },
            None => match self.resolve_variable(&name) {
                Some(jian_ops_schema::variable::VariableScalar::Str(s)) => s.clone(),
                _ => return false,
            },
        };
        let Some(rgb) = parse_hex_rgb(&scalar) else {
            return false;
        };
        let (h, s, v) = rgb_to_hsv(rgb);
        self.ui.pending_color_history = Some(self.snapshot_for_history());
        self.ui.color_picker = Some(ColorPickerState {
            // `target` is unused on the variable path but must carry a
            // sane default for any code that pattern-matches on it
            // without checking `variable` first.
            target: ColorTarget::Fill,
            hue: h,
            sat: s,
            val: v,
            drag: None,
            anchor_y,
            anchor_x,
            variable: Some(name),
            variable_theme,
            alpha: 1.0,
            fill_index: 0,
            hex_focused: false,
            hex_input: jian_core::text_input::TextInputState::default(),
            rgb_focus: None,
            rgb_input: jian_core::text_input::TextInputState::default(),
        });
        true
    }

    /// Update the picker HSV and live-apply the resulting RGB. In
    /// node mode this writes the anchor node's target colour; in
    /// variable mode (`ColorPickerState::variable` is `Some`) it
    /// writes through to the named Color variable so paint reflects
    /// the change everywhere the variable resolves. Tolerates
    /// `color_picker = None` so hosts can pipe move events
    /// unconditionally.
    pub fn color_picker_set_hsv(&mut self, hue: f32, sat: f32, val: f32) -> bool {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return false;
        };
        state.hue = hue.rem_euclid(360.0);
        state.sat = sat.clamp(0.0, 1.0);
        state.val = val.clamp(0.0, 1.0);
        let target = state.target;
        let fill_index = state.fill_index;
        let variable = state.variable.clone();
        let variable_theme = state.variable_theme.clone();
        let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
        let hex = rgb_to_hex(r, g, b);
        if let Some(name) = variable {
            // Variable-mode commit — write through the variable so
            // every node referencing it repaints. A variant-targeted
            // open (#19) writes exactly the clicked theme column;
            // otherwise `set_variable_color` applies the active-theme
            // routing discipline.
            if let Some((axis, value)) = variable_theme {
                self.set_variable_color_for_theme(&name, &axis, &value, &hex);
            } else {
                self.set_variable_color(&name, &hex);
            }
            return true;
        }
        match target {
            ColorTarget::Fill => {
                // The primary fill (index 0) keeps `set_selected_color`,
                // which prepends a fresh solid when the node has no solid
                // fill (and is colour-variable-aware). A non-primary fill
                // row writes its own solid fill in place.
                if fill_index == 0 {
                    self.set_selected_color(true, &hex);
                } else {
                    let _ = self.set_selected_fill_hex_at(fill_index, &hex);
                }
            }
            ColorTarget::Stroke => {
                self.set_selected_color(false, &hex);
            }
            ColorTarget::GradientStop(i) => {
                let hex_with_alpha = splice_alpha(&hex, self.picker_alpha());
                let _ = self.set_selected_gradient_stop_hex(i, &hex_with_alpha);
            }
            ColorTarget::EffectColor(i) => {
                let hex_with_alpha = splice_alpha(&hex, self.picker_alpha());
                let sel = self.selection.anchor.clone();
                if sel.is_real() {
                    let _ = self.apply(crate::EditorCommand::SetEffectColor {
                        node_id: sel,
                        index: i as u32,
                        hex: hex_with_alpha,
                    });
                }
            }
        }
        true
    }

    /// Read the picker's preserved alpha (0..=1) — defaults to 1.0
    /// when no picker is open.
    fn picker_alpha(&self) -> f32 {
        self.ui
            .color_picker
            .as_ref()
            .map(|s| s.alpha.clamp(0.0, 1.0))
            .unwrap_or(1.0)
    }

    /// Set the active drag kind so `apply_cursor_move` can route a
    /// move event to the right control.
    pub fn color_picker_set_drag(&mut self, drag: Option<ColorPickerDrag>) {
        if let Some(state) = self.ui.color_picker.as_mut() {
            state.drag = drag;
        }
    }

    /// Close the picker. Pushes the pre-edit snapshot onto the undo
    /// stack when the colour actually changed; drops it otherwise.
    /// Returns true when a picker was open.
    pub fn close_color_picker(&mut self) -> bool {
        let Some(state) = self.ui.color_picker.take() else {
            return false;
        };
        let snap = self.ui.pending_color_history.take();
        let Some(snap) = snap else {
            return true;
        };
        // Variable-mode close: compare the variable's resolved scalar
        // in the pre-edit snapshot's `doc` against the live one. The
        // snapshot carries a full `PenDocument` (variables included),
        // so undo restores the variable for free — see `history.rs`.
        let changed = if let Some(name) = &state.variable {
            // The active-theme selection is rebuilt-on-load transient
            // state, stable across the (short-lived) picker session, so
            // resolve the pre-edit snapshot's variable under the SAME
            // active theme as the live `resolve_variable` uses.
            let before = snapshot_variable_hex(&snap, name, &self.ui.variables.active_theme);
            let after = self.resolve_variable(name).and_then(scalar_as_hex);
            before != after
        } else if matches!(
            self.selected_node(),
            Some(jian_ops_schema::node::PenNode::Ref(_))
        ) {
            // Instance anchors keep their colours in `descendants`
            // overrides — the per-target colour readers see `None` on
            // a Ref, so compare the whole node instead. An override
            // edit must still land exactly one undo entry.
            let sel = self.selection.anchor.clone();
            let snap_children = snapshot_active_children(&snap);
            crate::walkers::find_node(snap_children, &sel) != self.selected_node()
        } else {
            let sel = self.selection.anchor.clone();
            let snap_children = snapshot_active_children(&snap);
            let fill_index = state.fill_index;
            let before =
                crate::walkers::find_node(snap_children, &sel).and_then(|n| match state.target {
                    ColorTarget::Fill => indexed_solid_fill_hex(n, fill_index),
                    ColorTarget::Stroke => first_solid_stroke_hex(n).map(str::to_string),
                    ColorTarget::GradientStop(i) => gradient_stop_hex(n, i),
                    ColorTarget::EffectColor(i) => effect_color_hex(n, i),
                });
            let after = self.selected_node().and_then(|n| match state.target {
                ColorTarget::Fill => indexed_solid_fill_hex(n, fill_index),
                ColorTarget::Stroke => first_solid_stroke_hex(n).map(str::to_string),
                ColorTarget::GradientStop(i) => gradient_stop_hex(n, i),
                ColorTarget::EffectColor(i) => effect_color_hex(n, i),
            });
            before != after
        };
        if changed {
            self.history_push_past(snap);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_id::NodeId;
    use crate::test_support::{rect, state_with};
    use crate::ui_draft::ColorTarget;

    fn doc_with_rect() -> EditorState {
        let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 40.0, 30.0)]);
        s.set_single_selection(NodeId::new("n1"));
        s
    }

    #[test]
    fn set_selected_color_writes_first_solid_fill() {
        let mut s = doc_with_rect();
        assert!(s.set_selected_color(true, "#ff0000"));
        let node = s.selected_node().unwrap();
        assert_eq!(crate::fills::first_solid_fill_hex(node), Some("#ff0000"));
    }

    #[test]
    fn set_selected_color_writes_stroke() {
        let mut s = doc_with_rect();
        assert!(s.set_selected_color(false, "#00ff00"));
        let node = s.selected_node().unwrap();
        assert_eq!(crate::fills::first_solid_stroke_hex(node), Some("#00ff00"));
    }

    #[test]
    fn set_selected_color_no_op_without_selection() {
        let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
        assert!(!s.set_selected_color(true, "#ffffff"));
    }

    #[test]
    fn add_drop_shadow_appends_effect() {
        let mut s = doc_with_rect();
        assert!(s.add_drop_shadow_to_selected());
        // A second call appends a second shadow.
        assert!(s.add_drop_shadow_to_selected());
    }

    #[test]
    fn add_layer_blur_appends_blur_effect() {
        use jian_ops_schema::style::PenEffect;
        let mut s = doc_with_rect();
        assert!(s.add_layer_blur_to_selected());
        let node = s.selected_node().expect("selection");
        let effects = crate::node_effects(node);
        assert!(
            matches!(effects.last(), Some(PenEffect::Blur(_))),
            "add_layer_blur must append a Blur effect, got {effects:?}"
        );
    }

    #[test]
    fn add_effect_on_effectless_node_creates_no_undo_state() {
        // An icon_font node carries no `effects` list. Adding an
        // effect must be a no-op that leaves NO empty undo/dirty state.
        let src = r#"{"version":"0.8.0","children":[
            {"type":"icon_font","id":"i1","name":"Icon",
             "x":0,"y":0,"width":20,"height":20,"iconFontName":"star"}
        ]}"#;
        let doc = jian_ops_schema::load_str(src)
            .expect("fixture parses")
            .value;
        let mut s = EditorState::from_document(doc);
        s.set_single_selection(NodeId::new("i1"));
        assert!(
            !s.add_layer_blur_to_selected(),
            "effect-less node must reject the add"
        );
        assert!(
            !s.undo(),
            "rejected add must not have pushed an empty undo state"
        );
    }

    #[test]
    fn add_layer_blur_is_undoable() {
        let mut s = doc_with_rect();
        let before = crate::node_effects(s.selected_node().unwrap()).len();
        assert!(s.add_layer_blur_to_selected());
        assert_eq!(
            crate::node_effects(s.selected_node().unwrap()).len(),
            before + 1
        );
        assert!(s.undo(), "add layer blur must be undoable");
        assert_eq!(
            crate::node_effects(s.selected_node().unwrap()).len(),
            before,
            "undo must remove the added blur"
        );
    }

    #[test]
    fn open_picker_seeds_hsv_from_fill() {
        let mut s = doc_with_rect();
        s.set_selected_color(true, "#ff8800");
        assert!(s.open_color_picker(ColorTarget::Fill, 120.0));
        let state = s.ui.color_picker.as_ref().unwrap();
        // Orange #ff8800 → hue near 32°.
        assert!(state.hue > 20.0 && state.hue < 45.0, "hue {}", state.hue);
        assert!(state.sat > 0.95);
        assert!(state.val > 0.95);
        assert!(s.ui.pending_color_history.is_some());
    }

    #[test]
    fn picker_set_hsv_writes_through_to_node() {
        let mut s = doc_with_rect();
        assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
        // Pure red: H=0 S=1 V=1.
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
        let node = s.selected_node().unwrap();
        assert_eq!(crate::fills::first_solid_fill_hex(node), Some("#ff0000"));
    }

    #[test]
    fn close_picker_pushes_history_only_on_change() {
        let mut s = doc_with_rect();
        let depth = s.history.past.len();
        assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
        // No HSV change → close does not push history.
        assert!(s.close_color_picker());
        assert_eq!(s.history.past.len(), depth);

        // Re-open + drag + close → history grows by one.
        assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
        assert!(s.color_picker_set_hsv(180.0, 1.0, 1.0));
        assert!(s.close_color_picker());
        assert_eq!(s.history.past.len(), depth + 1);
    }

    #[test]
    fn undo_after_picker_edit_restores_color() {
        let mut s = doc_with_rect();
        s.set_selected_color(true, "#ff8800");
        assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
        assert!(s.close_color_picker());
        assert_eq!(
            crate::fills::first_solid_fill_hex(s.selected_node().unwrap()),
            Some("#ff0000")
        );
        assert!(s.undo());
        assert_eq!(
            crate::fills::first_solid_fill_hex(s.selected_node().unwrap()),
            Some("#ff8800")
        );
    }

    #[test]
    fn hsv_roundtrip_is_stable() {
        for &hex in &["#ff0000", "#00ff00", "#0000ff", "#808080", "#ff8800"] {
            let rgb = parse_hex_rgb(hex).unwrap();
            let (h, s, v) = rgb_to_hsv(rgb);
            let (r, g, b) = hsv_to_rgb(h, s, v);
            assert_eq!(rgb_to_hex(r, g, b), hex, "roundtrip {hex}");
        }
    }

    // --- Variable-mode picker (Gap 1) -------------------------------

    use jian_ops_schema::variable::{VariableKind, VariableScalar};

    /// A state holding one Color variable and no nodes.
    fn state_with_color_var(name: &str, hex: &str) -> EditorState {
        let mut s = state_with(vec![]);
        s.create_variable(name, VariableKind::Color, VariableScalar::Str(hex.into()));
        s
    }

    #[test]
    fn open_picker_for_variable_seeds_hsv_from_resolved_color() {
        let mut s = state_with_color_var("brand", "#ff8800");
        assert!(s.open_color_picker_for_variable("brand", 100.0));
        let state = s.ui.color_picker.as_ref().expect("picker open");
        assert_eq!(state.variable.as_deref(), Some("brand"));
        assert!(
            s.ui.pending_color_history.is_some(),
            "undo snapshot captured"
        );
        // #ff8800 (orange) → hue near 32°.
        assert!(state.hue > 20.0 && state.hue < 45.0, "hue {}", state.hue);
        assert!(state.sat > 0.95, "sat {}", state.sat);
        assert!(state.val > 0.95, "val {}", state.val);
    }

    #[test]
    fn open_picker_for_variable_fails_on_missing_or_wrong_kind() {
        let mut s = state_with(vec![]);
        // Unknown name → false, no picker.
        assert!(!s.open_color_picker_for_variable("nope", 0.0));
        assert!(s.ui.color_picker.is_none());
        // Number-kind variable → not a colour → false.
        s.create_variable("spacing", VariableKind::Number, VariableScalar::Num(16.0));
        assert!(!s.open_color_picker_for_variable("spacing", 0.0));
        assert!(s.ui.color_picker.is_none());
    }

    #[test]
    fn picker_set_hsv_writes_through_variable_path() {
        // Open on a variable, push pure-red HSV, confirm the variable
        // flips and no node fill is touched (there are no nodes).
        let mut s = state_with_color_var("brand", "#ff8800");
        assert!(s.open_color_picker_for_variable("brand", 0.0));
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
        match s.resolve_variable("brand") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff0000"),
            other => panic!("expected red, got {other:?}"),
        }
    }

    // --- Variant-targeted picker (#19) -------------------------------

    /// State with one Color variable and a 2-variant Theme-1 axis,
    /// active theme pinned to Light.
    fn state_with_two_variant_color(hex: &str) -> EditorState {
        let mut s = state_with_color_var("brand", hex);
        let mut themes = std::collections::BTreeMap::new();
        themes.insert(
            "Theme-1".to_string(),
            vec!["Light".to_string(), "Dark".to_string()],
        );
        s.doc.themes = Some(themes);
        s.ui.variables
            .active_theme
            .insert("Theme-1".into(), "Light".into());
        s
    }

    #[test]
    fn variant_targeted_picker_writes_the_clicked_column() {
        // Click the Dark column swatch while the canvas renders Light:
        // the commit must land in Dark, leaving Light untouched (this
        // was the #19 wrong-cell-write bug).
        let mut s = state_with_two_variant_color("#ff8800");
        assert!(s.open_color_picker_for_variable_theme_at("brand", "Theme-1", "Dark", 10.0, 20.0));
        let state = s.ui.color_picker.as_ref().expect("picker open");
        assert_eq!(
            state.variable_theme,
            Some(("Theme-1".to_string(), "Dark".to_string()))
        );
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
                                                        // Active theme (Light) still resolves the original colour.
        assert_eq!(
            s.resolve_variable("brand"),
            Some(&VariableScalar::Str("#ff8800".into()))
        );
        // The Dark entry carries the new red.
        s.ui.variables
            .active_theme
            .insert("Theme-1".into(), "Dark".into());
        assert_eq!(
            s.resolve_variable("brand"),
            Some(&VariableScalar::Str("#ff0000".into()))
        );
    }

    #[test]
    fn variant_targeted_picker_seeds_hsv_from_that_column() {
        let mut s = state_with_two_variant_color("#ff0000");
        // Materialize a green Dark entry, keep Light red.
        assert!(s.set_variable_color_for_theme("brand", "Theme-1", "Dark", "#00ff00"));
        assert!(s.open_color_picker_for_variable_theme_at("brand", "Theme-1", "Dark", 0.0, 0.0));
        let state = s.ui.color_picker.as_ref().expect("picker open");
        // Green hue ≈ 120°, not red's 0° (which an active-theme seed
        // would have produced).
        assert!(
            (state.hue - 120.0).abs() < 1.0,
            "expected green seed, hue {}",
            state.hue
        );
    }

    #[test]
    fn close_picker_after_variable_edit_pushes_history_only_on_change() {
        let mut s = state_with_color_var("brand", "#ff8800");
        let depth = s.history.past.len();
        // No HSV change → close does not push history.
        assert!(s.open_color_picker_for_variable("brand", 0.0));
        assert!(s.close_color_picker());
        assert_eq!(s.history.past.len(), depth);
        // Re-open + drag + close → history grows by one.
        assert!(s.open_color_picker_for_variable("brand", 0.0));
        assert!(s.color_picker_set_hsv(180.0, 1.0, 1.0));
        assert!(s.close_color_picker());
        assert_eq!(s.history.past.len(), depth + 1);
    }

    #[test]
    fn undo_after_variable_picker_edit_restores_color() {
        // The picker's pre-edit snapshot carries the whole PenDocument
        // (variables included), so undo round-trips the variable.
        let mut s = state_with_color_var("brand", "#ff8800");
        assert!(s.open_color_picker_for_variable("brand", 0.0));
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
        assert!(s.close_color_picker());
        match s.resolve_variable("brand") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff0000"),
            other => panic!("expected red post-edit, got {other:?}"),
        }
        assert!(s.undo());
        match s.resolve_variable("brand") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff8800"),
            other => panic!("undo must restore #ff8800, got {other:?}"),
        }
    }

    #[test]
    fn gradient_stop_picker_preserves_alpha() {
        // Open the picker on a transparent stop (`#00000000`) and
        // drag SV → the resulting stop must still carry the original
        // alpha, not silently flip to opaque.
        use jian_ops_schema::node::PenNode;
        use jian_ops_schema::style::{GradientStop, LinearGradientBody, PenFill};
        let mut node = rect("n1", "r", 0.0, 0.0, 40.0, 30.0);
        // Seed a 2-stop gradient where stop 1 is fully transparent.
        let body = PenFill::LinearGradient(LinearGradientBody {
            angle: Some(0.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: "#ffffff".into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: "#00000000".into(),
                },
            ],
            explain: None,
            opacity: None,
            blend_mode: None,
        });
        if let PenNode::Rectangle(r) = &mut node {
            r.container.fill = Some(vec![body]);
        } else {
            panic!("expected rectangle");
        }
        let mut s = state_with(vec![node]);
        s.set_single_selection(NodeId::new("n1"));
        assert!(s.open_color_picker(ColorTarget::GradientStop(1), 100.0));
        assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
        let _ = s.close_color_picker();
        let node = s.selected_node().expect("rect");
        let stops = match crate::fills::node_fills(node)
            .and_then(|f| f.first())
            .expect("first fill")
        {
            PenFill::LinearGradient(b) => &b.stops,
            other => panic!("expected linear, got {other:?}"),
        };
        let written = &stops[1].color;
        assert!(
            written.eq_ignore_ascii_case("#ff000000"),
            "alpha must round-trip; got {written}"
        );
        // Stop 0 (opaque) must be untouched.
        assert!(stops[0].color.eq_ignore_ascii_case("#ffffff"));
    }
}
