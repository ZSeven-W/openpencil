//! Form-widget + text extractors read by
//! [`super::NodeSnapshot::from_node`] — the Widget section's per-kind
//! prop summary and the Text section's typography summary.
//!
//! Split out of `property_panel_snapshot.rs` to keep both files under
//! the openpencil 800-line cap.

use super::{TextSummary, WidgetKind, WidgetSummary};
use crate::widgets::property_panel_action::{
    TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};
use jian_ops_schema::node::base::{BoolOrExpression, NumberOrExpression};
use jian_ops_schema::node::text::{FontWeight, TextAlign, TextAlignVertical, TextGrowth};
use jian_ops_schema::node::PenNode;

/// Format an optional `f64` widget prop for an input box, dropping a
/// trailing `.0` so `5.0` paints as `5`. Empty when unset.
fn fmt_widget_num(v: Option<f64>) -> String {
    match v {
        Some(n) if n.fract() == 0.0 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => String::new(),
    }
}

/// Read a `NumberOrExpression` value as a display string — a literal
/// number drops trailing `.0`; an expression paints verbatim (the
/// panel can't edit a binding numerically, but should still show it).
fn fmt_widget_value(v: Option<&NumberOrExpression>) -> String {
    match v {
        Some(NumberOrExpression::Number(n)) if n.fract() == 0.0 => format!("{}", *n as i64),
        Some(NumberOrExpression::Number(n)) => format!("{n}"),
        Some(NumberOrExpression::Expression(s)) => s.clone(),
        None => String::new(),
    }
}

fn checked_literal(v: Option<&BoolOrExpression>) -> bool {
    matches!(v, Some(BoolOrExpression::Bool(true)))
}

/// Display key for the `bind:value` two-way binding — the raw
/// expression with the `$state.` prefix stripped so the panel input
/// shows `email` for `$state.email`. Empty when no value binding is
/// authored. A non-`$state` expression is shown verbatim.
fn bind_value_key(bindings: Option<&jian_ops_schema::events::Bindings>) -> String {
    bindings
        .and_then(|b| b.get("bind:value"))
        .map(|e| e.0.trim().trim_start_matches("$state.").to_string())
        .unwrap_or_default()
}

/// Build the Widget-section summary for a form-widget `PenNode`.
/// `None` for every non-widget kind so the section stays hidden.
pub(super) fn widget_summary_of(node: &PenNode) -> Option<WidgetSummary> {
    let base = |kind: WidgetKind| WidgetSummary {
        kind,
        placeholder: String::new(),
        value: String::new(),
        label: String::new(),
        checked: false,
        min: String::new(),
        max: String::new(),
        step: String::new(),
        option_count: 0,
        tab_count: 0,
        leading_icon: String::new(),
        trailing_icon: String::new(),
        bind_key: String::new(),
    };
    match node {
        PenNode::TextInput(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::TextInput)
        }),
        PenNode::TextArea(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::TextArea)
        }),
        PenNode::NumberInput(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: fmt_widget_value(n.value.as_ref()),
            min: fmt_widget_num(n.min),
            max: fmt_widget_num(n.max),
            step: fmt_widget_num(n.step),
            leading_icon: n.leading_icon.clone().unwrap_or_default(),
            trailing_icon: n.trailing_icon.clone().unwrap_or_default(),
            bind_key: bind_value_key(n.bindings.as_ref()),
            ..base(WidgetKind::NumberInput)
        }),
        PenNode::Select(n) => Some(WidgetSummary {
            placeholder: n.placeholder.clone().unwrap_or_default(),
            value: n.value.clone().unwrap_or_default(),
            option_count: n.options.as_ref().map_or(0, |o| o.len()),
            ..base(WidgetKind::Select)
        }),
        PenNode::RadioGroup(n) => Some(WidgetSummary {
            value: n.value.clone().unwrap_or_default(),
            option_count: n.options.as_ref().map_or(0, |o| o.len()),
            ..base(WidgetKind::RadioGroup)
        }),
        PenNode::Switch(n) => Some(WidgetSummary {
            checked: checked_literal(n.checked.as_ref()),
            ..base(WidgetKind::Switch)
        }),
        PenNode::Checkbox(n) => Some(WidgetSummary {
            label: n.label.clone().unwrap_or_default(),
            checked: checked_literal(n.checked.as_ref()),
            ..base(WidgetKind::Checkbox)
        }),
        PenNode::Slider(n) => Some(WidgetSummary {
            value: fmt_widget_value(n.value.as_ref()),
            min: fmt_widget_num(n.min),
            max: fmt_widget_num(n.max),
            step: fmt_widget_num(n.step),
            ..base(WidgetKind::Slider)
        }),
        PenNode::Progress(n) => Some(WidgetSummary {
            value: fmt_widget_value(n.value.as_ref()),
            max: fmt_widget_num(n.max),
            ..base(WidgetKind::Progress)
        }),
        PenNode::Tabs(n) => Some(WidgetSummary {
            value: n.value.clone().unwrap_or_default(),
            tab_count: n.tabs.as_ref().map_or(0, |t| t.len()),
            ..base(WidgetKind::Tabs)
        }),
        _ => None,
    }
}

pub(super) fn text_summary_of(node: &PenNode) -> Option<TextSummary> {
    let PenNode::Text(t) = node else {
        return None;
    };
    Some(TextSummary {
        font_family: t
            .font_family
            .clone()
            .unwrap_or_else(|| "Inter, sans-serif".to_string()),
        font_size: t.font_size_px().unwrap_or(16.0) as f32,
        font_weight: font_weight_value(t.font_weight.as_ref()),
        line_height_percent: (t.line_height.unwrap_or(1.2) * 100.0) as f32,
        letter_spacing: t.letter_spacing.unwrap_or(0.0) as f32,
        align: match t.text_align.as_ref().unwrap_or(&TextAlign::Left) {
            TextAlign::Left => TextAlignValue::Left,
            TextAlign::Center => TextAlignValue::Center,
            TextAlign::Right => TextAlignValue::Right,
            TextAlign::Justify => TextAlignValue::Justify,
        },
        vertical_align: match t
            .text_align_vertical
            .as_ref()
            .unwrap_or(&TextAlignVertical::Top)
        {
            TextAlignVertical::Top => TextVerticalAlignValue::Top,
            TextAlignVertical::Middle => TextVerticalAlignValue::Middle,
            TextAlignVertical::Bottom => TextVerticalAlignValue::Bottom,
        },
        growth: match t.text_growth.as_ref().unwrap_or(&TextGrowth::FixedWidth) {
            TextGrowth::Auto => TextGrowthValue::Auto,
            TextGrowth::FixedWidth => TextGrowthValue::FixedWidth,
            TextGrowth::FixedWidthHeight => TextGrowthValue::FixedWidthHeight,
        },
    })
}

fn font_weight_value(weight: Option<&FontWeight>) -> u16 {
    match weight {
        Some(FontWeight::Number(n)) => (*n).clamp(1, 1000) as u16,
        Some(FontWeight::Keyword(s)) => match s.as_str() {
            "thin" => 100,
            "light" => 300,
            "medium" => 500,
            "semibold" => 600,
            "bold" => 700,
            "black" => 900,
            _ => 400,
        },
        None => 400,
    }
}
