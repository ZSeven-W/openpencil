//! Composite-widget `PenNode` → `NodePayload` conversions.
//!
//! The interactive widget family (text_input / text_area / select /
//! switch / checkbox / slider / radio_group / number_input / progress /
//! tabs) degrades to a `rect` / `text` / `frame` `NodePayload` for
//! geometry + container style, but each carries a [`WidgetPayload`]
//! descriptor (props harvested from the canonical schema) so the OP
//! design canvas can paint the recognizable static visual (track +
//! knob, box + check, chevron, bar, …) instead of a bare rect.
//!
//! Carved out of `adapter.rs` to keep that file under the 800-line
//! ceiling. `adapter::node_to_payload` dispatches the widget variants
//! here; `tabs` recurses back through it for its panel children.

use std::collections::BTreeMap;

use jian_ops_schema::node::{
    BoolOrExpression, CheckboxNode, NumberInputNode, NumberOrExpression, ProgressNode,
    RadioGroupNode, SelectNode, SelectOption, SliderNode, SwitchNode, TabsNode, TextAreaNode,
    TextInputNode,
};

use crate::adapter::node_to_payload;
use crate::payload::{NodePayload, WidgetOption, WidgetPayload};
use crate::style_payload::{
    apply_container_style, assign_first_fill, base_payload, stroke_to_payload,
};

pub(crate) fn text_input_to_payload(n: &TextInputNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    p.text = n.value.clone().or_else(|| n.placeholder.clone());
    assign_first_fill(&mut p, n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.widget = Some(WidgetPayload {
        kind: "text_input".into(),
        value_str: n.value.clone(),
        placeholder: n.placeholder.clone(),
        ..Default::default()
    });
    p
}

pub(crate) fn text_area_to_payload(n: &TextAreaNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    p.text = n.value.clone().or_else(|| n.placeholder.clone());
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "text_area".into(),
        value_str: n.value.clone(),
        placeholder: n.placeholder.clone(),
        ..Default::default()
    });
    p
}

pub(crate) fn select_to_payload(n: &SelectNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    p.text = n.value.clone().or_else(|| n.placeholder.clone());
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "select".into(),
        value_str: n.value.clone(),
        placeholder: n.placeholder.clone(),
        options: select_options(n.options.as_deref()),
        ..Default::default()
    });
    p
}

pub(crate) fn switch_to_payload(n: &SwitchNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "switch".into(),
        checked: Some(bool_or_expr(n.checked.as_ref())),
        ..Default::default()
    });
    p
}

pub(crate) fn checkbox_to_payload(n: &CheckboxNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "checkbox".into(),
        checked: Some(bool_or_expr(n.checked.as_ref())),
        label: n.label.clone(),
        ..Default::default()
    });
    p
}

pub(crate) fn slider_to_payload(n: &SliderNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "slider".into(),
        value_num: n.value.as_ref().map(number_or_expr),
        min: n.min.map(|v| v as f32),
        max: n.max.map(|v| v as f32),
        step: n.step.map(|v| v as f32),
        ..Default::default()
    });
    p
}

pub(crate) fn radio_group_to_payload(n: &RadioGroupNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "radio_group".into(),
        value_str: n.value.clone(),
        options: select_options(n.options.as_deref()),
        ..Default::default()
    });
    p
}

pub(crate) fn number_input_to_payload(n: &NumberInputNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "number_input".into(),
        value_num: n.value.as_ref().map(number_or_expr),
        placeholder: n.placeholder.clone(),
        min: n.min.map(|v| v as f32),
        max: n.max.map(|v| v as f32),
        step: n.step.map(|v| v as f32),
        ..Default::default()
    });
    p
}

pub(crate) fn progress_to_payload(n: &ProgressNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "progress".into(),
        value_num: n.value.as_ref().map(number_or_expr),
        max: n.max.map(|v| v as f32),
        ..Default::default()
    });
    p
}

pub(crate) fn tabs_to_payload(n: &TabsNode, rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "frame");
    apply_container_style(
        &mut p,
        n.fill.as_deref(),
        n.stroke.as_ref(),
        n.corner_radius.as_ref(),
    );
    p.widget = Some(WidgetPayload {
        kind: "tabs".into(),
        value_str: n.value.clone(),
        options: select_options(n.tabs.as_deref()),
        ..Default::default()
    });
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload(c, rects))
        .collect();
    p
}

/// `BoolOrExpression` → concrete bool. An unresolved `$expr` reference
/// renders as `false` on the static design surface (the runtime
/// evaluates the binding; design-time shows the off state).
fn bool_or_expr(value: Option<&BoolOrExpression>) -> bool {
    matches!(value, Some(BoolOrExpression::Bool(true)))
}

/// `NumberOrExpression` → concrete f32. An unresolved `$expr` reference
/// collapses to 0.0 (range minimum) for the static visual.
fn number_or_expr(value: &NumberOrExpression) -> f32 {
    match value {
        NumberOrExpression::Number(n) => *n as f32,
        NumberOrExpression::Expression(_) => 0.0,
    }
}

/// Map schema `SelectOption`s into payload `(value, label)` rows.
fn select_options(opts: Option<&[SelectOption]>) -> Vec<WidgetOption> {
    opts.unwrap_or(&[])
        .iter()
        .map(|o| WidgetOption {
            value: o.value.clone(),
            label: o.label.clone(),
        })
        .collect()
}
