//! Per-section summary types carried by [`super::NodeSnapshot`] —
//! widget / text / icon / padding / fill / gradient-stop / effect.
//!
//! Split out of `property_panel_snapshot.rs` to keep both files under
//! the openpencil 800-line cap.

use super::color_from_hex;
use crate::widgets::property_panel_action::{
    TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};
use crate::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseArcSummary {
    pub start_deg: f32,
    pub sweep_deg: f32,
    pub inner_percent: f32,
}

/// Which form-widget variant the selected node is — drives the
/// Widget section's per-kind field set. Mirrors the schema's widget
/// `PenNode` variants (`states` overrides are out of scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    TextInput,
    TextArea,
    NumberInput,
    Select,
    RadioGroup,
    Switch,
    Checkbox,
    Slider,
    Progress,
    Tabs,
}

impl WidgetKind {
    /// Whether the kind exposes a `placeholder` field.
    pub fn has_placeholder(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput
                | WidgetKind::TextArea
                | WidgetKind::NumberInput
                | WidgetKind::Select
        )
    }

    /// Whether the kind exposes a text-typed `value` field the panel
    /// edits as a string (numeric `value` on Slider / NumberInput /
    /// Progress is edited through the min/max/step + numeric inputs,
    /// not the text value row).
    pub fn has_text_value(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput
                | WidgetKind::TextArea
                | WidgetKind::Select
                | WidgetKind::RadioGroup
        )
    }

    /// Whether the kind carries a `checked` toggle.
    pub fn has_checked(self) -> bool {
        matches!(self, WidgetKind::Switch | WidgetKind::Checkbox)
    }

    /// Whether the kind carries a `label` text field (Checkbox only).
    pub fn has_label(self) -> bool {
        matches!(self, WidgetKind::Checkbox)
    }

    /// Whether the kind exposes the min / max / step numeric trio.
    pub fn has_range(self) -> bool {
        matches!(self, WidgetKind::Slider | WidgetKind::NumberInput)
    }

    /// Whether the kind exposes `leadingIcon` / `trailingIcon` name
    /// fields (icon-bearing input widgets, Phase 1).
    pub fn has_icons(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput | WidgetKind::TextArea | WidgetKind::NumberInput
        )
    }

    /// Whether the kind exposes the `bind:value` state-key field. The
    /// schema carries `bindings` on every form widget; Phase 1 surfaces
    /// the editor only on the text-bearing input kinds (parity with
    /// where the value/placeholder rows show).
    pub fn has_bind_value(self) -> bool {
        matches!(
            self,
            WidgetKind::TextInput | WidgetKind::TextArea | WidgetKind::NumberInput
        )
    }
}

/// Snapshot of a form-widget node's editable props, formatted for
/// the Widget section. `None` on the snapshot when the selection
/// isn't a widget. The `options` / `tabs` list-editor is deferred
/// (see `property_panel_widget`); the lists are surfaced read-only
/// as a count so the user can see what's authored.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetSummary {
    pub kind: WidgetKind,
    /// `placeholder`, formatted for the input (empty when unset).
    pub placeholder: String,
    /// String `value` (text widgets) — empty when unset / numeric.
    pub value: String,
    /// `label` (Checkbox) — empty when unset.
    pub label: String,
    /// `checked` literal (Switch / Checkbox). `false` for an
    /// expression-bound or unset value.
    pub checked: bool,
    /// `min` / `max` / `step`, formatted (empty when unset).
    pub min: String,
    pub max: String,
    pub step: String,
    /// Count of authored `options` (Select / RadioGroup) — read-only
    /// affordance; the row editor is a follow-up.
    pub option_count: usize,
    /// Count of authored `tabs` (Tabs) — read-only affordance.
    pub tab_count: usize,
    /// `leadingIcon` / `trailingIcon` lucide glyph names (input
    /// widgets) — empty when unset. The Widget section edits these.
    pub leading_icon: String,
    pub trailing_icon: String,
    /// The `bind:value` state key, with the `$state.` prefix stripped
    /// for display (e.g. `email` for `bindings."bind:value" =
    /// "$state.email"`). Empty when no value binding is authored.
    pub bind_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSummary {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height_percent: f32,
    pub letter_spacing: f32,
    pub align: TextAlignValue,
    pub vertical_align: TextVerticalAlignValue,
    pub growth: TextGrowthValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IconSummary {
    pub family: String,
    pub name: String,
    pub icon_id: String,
}

/// Image-node video metadata formatted for the Video property section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoSummary {
    pub src: String,
    pub poster_src: String,
    pub autoplay: bool,
    pub loop_video: bool,
    pub muted: bool,
    pub hold_last_frame: bool,
    pub click_to_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutPaddingSummary {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl LayoutPaddingSummary {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn value_for(self, focus: op_editor_core::PropertyFocus) -> Option<f32> {
        use op_editor_core::PropertyFocus as F;
        match focus {
            F::PaddingTop => Some(self.top),
            F::PaddingRight => Some(self.right),
            F::PaddingBottom => Some(self.bottom),
            F::PaddingLeft => Some(self.left),
            _ => None,
        }
    }
}

/// One fill's head-row summary for the multi-fill Fill section.
/// The Fill section stacks one row per `PenFill`; each carries its
/// own type / colour / opacity so the row paints its head + body
/// independently. Built from `node_fills` in authored order.
#[derive(Debug, Clone)]
pub struct FillSummary {
    /// The fill kind — drives the per-row type dropdown + which body
    /// (solid hex / gradient stops / image) paints below the head row.
    pub fill_type: op_editor_core::FillType,
    /// Representative paint colour for the head-row swatch + (for a
    /// Solid fill) the hex input. Solid → its colour; gradient → the
    /// first stop's colour; image → white placeholder.
    pub color: Color,
    /// This fill's opacity in `[0.0, 1.0]` — the head row's `%` input
    /// paints `opacity * 100`.
    pub opacity: f32,
    /// Per-paint compositing. `None` is the canonical Normal default.
    pub blend_mode: Option<jian_ops_schema::style::BlendMode>,
    /// Bound colour-variable name, when this fill's colour follows a
    /// `$ref`. `None` for a literal colour. Only meaningful for the
    /// primary (index 0) fill today (the variable subsystem keys off
    /// the primary fill); carried per-fill for forward compatibility.
    pub variable_ref: Option<String>,
}

/// One gradient stop summary for the Fill section.
#[derive(Debug, Clone)]
pub struct GradientStopSummary {
    /// Offset 0.0..=1.0 — the Fill panel paints `offset * 100` as
    /// the per-stop `%` input.
    pub offset: f32,
    /// Schema hex string (`#RRGGBB` or `#RRGGBBAA`). The panel
    /// paints this verbatim so a freshly-typed user value isn't
    /// silently re-cased by `format_color_hex` round-trips.
    pub hex: String,
    /// Parsed paint colour for the per-row swatch. Falls back to
    /// black when the hex fails to parse.
    pub color: Color,
}

/// Which visual-effect variant a row represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Shadow,
    LayerBlur,
    BackgroundBlur,
}

impl EffectKind {
    /// Human-readable row label.
    pub fn label(self) -> &'static str {
        match self {
            EffectKind::Shadow => "Shadow",
            EffectKind::LayerBlur => "Layer Blur",
            EffectKind::BackgroundBlur => "Background Blur",
        }
    }
}

/// One effect's editable scalar parameters, formatted for the
/// Effects section. Shadow uses all four; the blur kinds use `blur`
/// as the radius and leave offset / spread at 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectSummary {
    pub kind: EffectKind,
    pub visible: bool,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    /// Effect colour — Shadow carries an authored hex string; the
    /// blur kinds don't have a colour field and use transparent.
    pub color: Color,
}

impl EffectSummary {
    /// Current value of one editable parameter — Blur / BackgroundBlur
    /// keep their radius in `blur`, so `Blur` and `Radius` both read
    /// that field.
    pub fn param_value(&self, field: op_editor_core::EffectField) -> f32 {
        use op_editor_core::EffectField as F;
        match field {
            F::OffsetX => self.offset_x,
            F::OffsetY => self.offset_y,
            F::Blur | F::Radius => self.blur,
            F::Spread => self.spread,
        }
    }

    /// Summarise a canonical `PenEffect` for the panel.
    pub(super) fn from_pen_effect(e: &jian_ops_schema::style::PenEffect) -> Self {
        use jian_ops_schema::style::PenEffect;
        match e {
            PenEffect::Shadow(s) => EffectSummary {
                kind: EffectKind::Shadow,
                visible: s.visible != Some(false),
                offset_x: s.offset_x,
                offset_y: s.offset_y,
                blur: s.blur,
                spread: s.spread,
                color: color_from_hex(&s.color).unwrap_or(Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.25,
                }),
            },
            PenEffect::Blur(b) => EffectSummary {
                kind: EffectKind::LayerBlur,
                visible: b.visible != Some(false),
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
                color: Color::TRANSPARENT,
            },
            PenEffect::BackgroundBlur(b) => EffectSummary {
                kind: EffectKind::BackgroundBlur,
                visible: b.visible != Some(false),
                offset_x: 0.0,
                offset_y: 0.0,
                blur: b.radius,
                spread: 0.0,
                color: Color::TRANSPARENT,
            },
        }
    }
}
