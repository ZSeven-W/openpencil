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

use crate::fills::{
    first_solid_fill_hex, first_solid_stroke_hex, push_drop_shadow, set_primary_fill_hex,
    set_primary_stroke_hex,
};
use crate::state::EditorState;
use crate::ui_draft::{ColorPickerDrag, ColorPickerState, ColorTarget};
use crate::walkers::find_node_mut;

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
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &sel) else {
            return false;
        };
        push_drop_shadow(node)
    }

    // --- HSV colour picker ------------------------------------------

    /// Open the floating colour picker on the given target. Seeds HSV
    /// from the anchor node's current fill / stroke colour. Captures
    /// a pre-edit history snapshot. Returns false when there is no
    /// editable selection to edit.
    pub fn open_color_picker(&mut self, target: ColorTarget, anchor_y: f32) -> bool {
        self.open_color_picker_with_anchor(target, None, anchor_y)
    }

    /// Same as [`Self::open_color_picker`], but horizontally anchors
    /// the floating picker near the click point instead of the right
    /// property rail.
    pub fn open_color_picker_at(
        &mut self,
        target: ColorTarget,
        anchor_x: f32,
        anchor_y: f32,
    ) -> bool {
        self.open_color_picker_with_anchor(target, Some(anchor_x), anchor_y)
    }

    fn open_color_picker_with_anchor(
        &mut self,
        target: ColorTarget,
        anchor_x: Option<f32>,
        anchor_y: f32,
    ) -> bool {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return false;
        }
        let Some(node) = self.selected_node() else {
            return false;
        };
        let current_hex = match target {
            ColorTarget::Fill => first_solid_fill_hex(node).map(str::to_string),
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
            anchor_x,
            variable: None,
            alpha,
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
        self.open_color_picker_for_variable_with_anchor(variable, None, anchor_y)
    }

    /// Same as [`Self::open_color_picker_for_variable`], but anchors
    /// the floating picker near the clicked variable swatch.
    pub fn open_color_picker_for_variable_at(
        &mut self,
        variable: impl Into<String>,
        anchor_x: f32,
        anchor_y: f32,
    ) -> bool {
        self.open_color_picker_for_variable_with_anchor(variable, Some(anchor_x), anchor_y)
    }

    fn open_color_picker_for_variable_with_anchor(
        &mut self,
        variable: impl Into<String>,
        anchor_x: Option<f32>,
        anchor_y: f32,
    ) -> bool {
        let name = variable.into();
        // Resolve the variable's current colour to seed HSV. Reject
        // unknown names, non-Color kinds, and non-hex scalars.
        let is_color = matches!(
            self.find_variable(&name).map(|d| &d.kind),
            Some(jian_ops_schema::variable::VariableKind::Color)
        );
        if !is_color {
            return false;
        }
        let scalar = match self.resolve_variable(&name) {
            Some(jian_ops_schema::variable::VariableScalar::Str(s)) => s.clone(),
            _ => return false,
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
            alpha: 1.0,
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
        let variable = state.variable.clone();
        let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
        let hex = rgb_to_hex(r, g, b);
        if let Some(name) = variable {
            // Variable-mode commit — write through the variable so
            // every node referencing it repaints. `set_variable_color`
            // applies the same theme-routing discipline variable edits
            // land through.
            self.set_variable_color(&name, &hex);
            return true;
        }
        match target {
            ColorTarget::Fill => {
                self.set_selected_color(true, &hex);
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
        } else {
            let sel = self.selection.anchor.clone();
            let snap_children = snapshot_active_children(&snap);
            let before =
                crate::walkers::find_node(snap_children, &sel).and_then(|n| match state.target {
                    ColorTarget::Fill => first_solid_fill_hex(n).map(str::to_string),
                    ColorTarget::Stroke => first_solid_stroke_hex(n).map(str::to_string),
                    ColorTarget::GradientStop(i) => gradient_stop_hex(n, i),
                    ColorTarget::EffectColor(i) => effect_color_hex(n, i),
                });
            let after = self.selected_node().and_then(|n| match state.target {
                ColorTarget::Fill => first_solid_fill_hex(n).map(str::to_string),
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

/// Reduce a resolved variable scalar to a `#rrggbb` hex string, if it
/// is a `Str` scalar. Used by the variable-mode `close_color_picker`
/// change check.
fn scalar_as_hex(s: &jian_ops_schema::variable::VariableScalar) -> Option<String> {
    match s {
        jian_ops_schema::variable::VariableScalar::Str(hex) => Some(hex.clone()),
        _ => None,
    }
}

/// Re-attach an alpha (0..=1) to a `#RRGGBB` hex. When the alpha
/// would round to fully opaque the 6-char form is preserved so the
/// canonical schema stays compact.
fn splice_alpha(hex: &str, alpha: f32) -> String {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        hex.to_string()
    } else {
        format!("{}{:02X}", hex, a)
    }
}

/// Read the colour hex of the Shadow effect at `index` on `node`.
/// `None` when the node has no effects, the index is out of range,
/// or the effect isn't a Shadow.
fn effect_color_hex(node: &jian_ops_schema::node::PenNode, index: usize) -> Option<String> {
    use jian_ops_schema::style::PenEffect;
    let effects = crate::fills::node_effects(node);
    match effects.get(index)? {
        PenEffect::Shadow(s) => Some(s.color.clone()),
        _ => None,
    }
}

/// Read one stop's hex from the node's primary gradient body.
/// `None` when the first fill isn't a gradient or `index` is out of
/// range — the same gating `set_primary_gradient_stop_hex` applies
/// on the write path.
fn gradient_stop_hex(node: &jian_ops_schema::node::PenNode, index: usize) -> Option<String> {
    use jian_ops_schema::style::PenFill;
    let fills = crate::fills::node_fills(node)?;
    let first = fills.first()?;
    let stops = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return None,
    };
    stops.get(index).map(|s| s.color.clone())
}

/// Resolve a Color variable's hex scalar from a history snapshot's
/// `doc` under the supplied active-theme selection. The snapshot's
/// `EditorSnapshot` carries the full `PenDocument` (variables
/// included) but not the transient `ui.variables.active_theme`, so the
/// caller threads the live active theme in — it is stable across the
/// short-lived picker session.
fn snapshot_variable_hex(
    snap: &crate::history::EditorSnapshot,
    name: &str,
    active_theme: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let def = snap.doc.variables.as_ref()?.get(name)?;
    if !matches!(def.kind, jian_ops_schema::variable::VariableKind::Color) {
        return None;
    }
    let scalar = resolve_snapshot_value(&def.value, active_theme)?;
    scalar_as_hex(scalar)
}

/// Resolve a `VariableValue` under `active_theme` — picks a `Scalar`
/// directly, or a `Themed` list's subset-matching entry, falling back
/// to the `theme: None` default. Mirrors `variables.rs::resolve_value`.
fn resolve_snapshot_value<'a>(
    value: &'a jian_ops_schema::variable::VariableValue,
    active_theme: &std::collections::BTreeMap<String, String>,
) -> Option<&'a jian_ops_schema::variable::VariableScalar> {
    use jian_ops_schema::variable::VariableValue;
    match value {
        VariableValue::Scalar(s) => Some(s),
        VariableValue::Themed(entries) => {
            for e in entries {
                if let Some(t) = &e.theme {
                    if t.iter().all(|(k, v)| active_theme.get(k) == Some(v)) {
                        return Some(&e.value);
                    }
                }
            }
            entries.iter().find(|e| e.theme.is_none()).map(|e| &e.value)
        }
    }
}

/// The active page's children inside a history snapshot — mirrors
/// [`EditorState::active_children`] but reads from a snapshot.
fn snapshot_active_children(
    snap: &crate::history::EditorSnapshot,
) -> &[jian_ops_schema::node::PenNode] {
    match snap.doc.pages.as_ref() {
        Some(pages) => match pages.get(snap.active_page_index) {
            Some(page) => &page.children,
            None => &[],
        },
        None => &snap.doc.children,
    }
}

// --- HSV / hex helpers -----------------------------------------------

/// HSV → RGB, h 0..360, s/v 0..1. Each channel 0..1.
/// Ported verbatim from shell-core's `hsv_to_rgb`.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let hh = h / 60.0;
    let x = c * (1.0 - (hh.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

/// RGB (0..1) → HSV (h 0..360, s 0..1, v 0..1).
/// Ported verbatim from shell-core's `rgb_to_hsv`.
pub fn rgb_to_hsv(rgb: (f32, f32, f32)) -> (f32, f32, f32) {
    let (r, g, b) = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let delta = max - min;
    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` into RGB floats (0..1).
/// Lenient on case; requires the leading `#`.
pub fn parse_hex_rgb(s: &str) -> Option<(f32, f32, f32)> {
    let s = s.trim().strip_prefix('#')?;
    let (r, g, b) = match s.len() {
        3 => (
            u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?,
        ),
        6 | 8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ),
        _ => return None,
    };
    Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}

/// Parse the alpha channel out of `#rrggbbaa` — defaults to `1.0`
/// when the hex is 6-char (no alpha authored) or unparseable. Used
/// by the gradient-stop colour picker so dragging SV / hue doesn't
/// drop the stop's authored transparency.
pub fn parse_hex_alpha(s: &str) -> f32 {
    let Some(stripped) = s.trim().strip_prefix('#') else {
        return 1.0;
    };
    if stripped.len() != 8 {
        return 1.0;
    }
    u8::from_str_radix(&stripped[6..8], 16)
        .map(|a| a as f32 / 255.0)
        .unwrap_or(1.0)
}

/// Format RGB floats (0..1) as a `#rrggbb` hex string.
pub fn rgb_to_hex(r: f32, g: f32, b: f32) -> String {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
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
