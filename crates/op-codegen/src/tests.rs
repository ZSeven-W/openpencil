//! Generator test suite — CSS-variable emission plus the per-framework
//! markup targets, carved off `lib.rs` to keep both files under the
//! 800-line cap.

use super::*;
use jian_ops_schema::node::{
    BoolOrExpression, CheckboxNode, EllipseNode, NumberInputNode, NumberOrExpression, PenNodeBase,
    ProgressNode, RadioGroupNode, RectangleNode, SelectNode, SelectOption, SliderNode, SwitchNode,
    TabsNode, TextAreaNode, TextContent, TextInputNode, TextNode,
};
use jian_ops_schema::page::PenPage;
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, SolidFillBody};
use jian_ops_schema::variable::{
    ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
};
use std::collections::BTreeMap;

/// Bare empty document — no variables, no pages, no children.
fn empty_doc() -> PenDocument {
    PenDocument {
        version: "1.0.0".into(),
        name: None,
        themes: None,
        variables: None,
        pages: None,
        children: Vec::new(),
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
        design_md: None,
        conversion: None,
        responsive: None,
    }
}

/// Replace the document's node forest with a single page holding
/// `nodes`, mirroring the old `page.children` test helper.
fn doc_with_nodes(nodes: Vec<PenNode>) -> PenDocument {
    let mut doc = empty_doc();
    doc.pages = Some(vec![PenPage {
        id: "p1".into(),
        name: "Page 1".into(),
        children: nodes,
        background_color: None,
        state: None,
        lifecycle: None,
    }]);
    doc
}

fn axis(name: &str, value: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(name.to_string(), value.to_string());
    m
}

fn solid(color: &str) -> Vec<PenFill> {
    vec![PenFill::Solid(SolidFillBody {
        color: color.into(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })]
}

/// A rectangle at `(x,y)` sized `w×h` with an optional fill.
fn rect(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Option<&str>) -> PenNode {
    let container = jian_ops_schema::node::ContainerProps {
        width: Some(SizingBehavior::Number(w)),
        height: Some(SizingBehavior::Number(h)),
        fill: fill.map(solid),
        ..Default::default()
    };
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.into(),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        container,
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

fn ellipse(id: &str, x: f64, y: f64, w: f64, h: f64) -> PenNode {
    PenNode::Ellipse(EllipseNode {
        base: PenNodeBase {
            id: id.into(),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(w)),
        height: Some(SizingBehavior::Number(h)),
        corner_radius: None,
        inner_radius: None,
        start_angle: None,
        sweep_angle: None,
        fill: None,
        stroke: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    })
}

fn text(id: &str, x: f64, y: f64, w: f64, h: f64, body: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.into(),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(w)),
        height: Some(SizingBehavior::Number(h)),
        content: TextContent::Plain(body.into()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    })
}

fn color_def(value: &str) -> VariableDefinition {
    VariableDefinition {
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str(value.into())),
    }
}

#[test]
fn css_variables_emits_root_scalars() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert("primary".to_string(), color_def("#0066ff"));
    vars.insert(
        "spacing".to_string(),
        VariableDefinition {
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(12.0)),
        },
    );
    doc.variables = Some(vars);
    let css = CssVariables.generate(&doc);
    assert!(css.contains(":root {"));
    assert!(css.contains("--primary: #0066ff;"));
    assert!(css.contains("--spacing: 12;"));
}

#[test]
fn css_variables_emits_per_theme_block() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert(
        "bg".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: Some(axis("mode", "light")),
                },
                ThemedValue {
                    value: VariableScalar::Str("#000000".into()),
                    theme: Some(axis("mode", "dark")),
                },
            ]),
        },
    );
    doc.variables = Some(vars);
    let css = CssVariables.generate(&doc);
    assert!(css.contains(":root[data-mode=\"light\"]"));
    assert!(css.contains(":root[data-mode=\"dark\"]"));
    assert!(css.contains("--bg: #ffffff;"));
    assert!(css.contains("--bg: #000000;"));
}

#[test]
fn css_variables_sanitises_non_ident_chars() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert("primary.color".to_string(), color_def("#f00"));
    doc.variables = Some(vars);
    let css = CssVariables.generate(&doc);
    // Dot replaced with dash so the result is a valid CSS ident.
    assert!(css.contains("--primary-color: #f00;"));
}

#[test]
fn html_emits_doctype_and_body() {
    let doc = empty_doc();
    let html = Html.generate(&doc);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<body>"));
    assert!(html.contains("</body>"));
    assert!(html.contains("OpenPencil"));
}

#[test]
fn html_emits_div_per_node_with_position_and_fill() {
    let doc = doc_with_nodes(vec![rect("n10", 20.0, 30.0, 100.0, 50.0, Some("#ff0000"))]);
    let html = Html.generate(&doc);
    assert!(html.contains("left:20px"));
    assert!(html.contains("top:30px"));
    assert!(html.contains("width:100px"));
    assert!(html.contains("rgb(255,0,0)"));
}

#[test]
fn html_emits_span_for_text_and_escapes_body() {
    let doc = doc_with_nodes(vec![text("n10", 0.0, 0.0, 100.0, 24.0, "Hello & <world>")]);
    let html = Html.generate(&doc);
    assert!(html.contains("<span"));
    assert!(html.contains("Hello &amp; &lt;world&gt;"));
}

#[test]
fn vue_emits_template_script_style_blocks() {
    let doc = empty_doc();
    let s = Vue.generate(&doc);
    assert!(s.contains("<template>"));
    assert!(s.contains("</template>"));
    assert!(s.contains("<script setup"));
    assert!(s.contains("<style scoped>"));
}

#[test]
fn svelte_emits_script_then_markup_then_style() {
    let doc = empty_doc();
    let s = Svelte.generate(&doc);
    let script_pos = s.find("<script").unwrap();
    let style_pos = s.find("<style>").unwrap();
    // Script before style — Svelte SFC convention.
    assert!(script_pos < style_pos);
}

#[test]
fn vue_includes_variable_css_in_style_block() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert("primary".to_string(), color_def("#abc"));
    doc.variables = Some(vars);
    let s = Vue.generate(&doc);
    assert!(s.contains("--primary: #abc;"));
}

#[test]
fn react_emits_functional_component_with_jsx_fragment() {
    let doc = empty_doc();
    let s = React.generate(&doc);
    assert!(s.contains("import React from 'react';"));
    assert!(s.contains("export default function Page()"));
    assert!(s.contains("<>"));
    assert!(s.contains("</>"));
}

#[test]
fn flutter_emits_stack_with_positioned_children() {
    let doc = doc_with_nodes(vec![rect("n10", 5.0, 10.0, 100.0, 50.0, Some("#ff0000"))]);
    let s = Flutter.generate(&doc);
    assert!(s.contains("import 'package:flutter/material.dart';"));
    assert!(s.contains("Stack(children:"));
    assert!(s.contains("Positioned(left: 5, top: 10"));
    assert!(s.contains("Container(width: 100, height: 50"));
    assert!(s.contains("Color.fromARGB(255,255,0,0)"));
}

#[test]
fn swiftui_emits_zstack_with_positioned_views() {
    let doc = doc_with_nodes(vec![rect("n10", 0.0, 0.0, 100.0, 50.0, None)]);
    let s = SwiftUi.generate(&doc);
    assert!(s.contains("import SwiftUI"));
    assert!(s.contains("ZStack {"));
    assert!(s.contains("Rectangle()"));
    assert!(s.contains(".frame(width: 100, height: 50)"));
}

#[test]
fn swiftui_emits_ellipse_for_ellipse_kind() {
    let doc = doc_with_nodes(vec![ellipse("n10", 0.0, 0.0, 20.0, 20.0)]);
    let s = SwiftUi.generate(&doc);
    assert!(s.contains("Ellipse()"));
}

#[test]
fn compose_emits_composable_box_with_offset_size() {
    let doc = doc_with_nodes(vec![rect("n10", 5.0, 10.0, 100.0, 50.0, Some("#ff0000"))]);
    let s = Compose.generate(&doc);
    assert!(s.contains("@Composable"));
    assert!(s.contains("Modifier.offset(x = 5.dp, y = 10.dp)"));
    assert!(s.contains("size(width = 100.dp, height = 50.dp)"));
    assert!(s.contains("Color(255, 0, 0, 255)"));
}

#[test]
fn react_native_emits_view_tree_with_absolute_positions() {
    let doc = doc_with_nodes(vec![rect("n10", 0.0, 0.0, 80.0, 40.0, None)]);
    let s = ReactNative.generate(&doc);
    assert!(s.contains("import { View, Text } from 'react-native';"));
    assert!(s.contains("export default function Page()"));
    assert!(s.contains("<View style={{"));
    assert!(s.contains("position: 'absolute'"));
    assert!(s.contains("width: 80, height: 40"));
}

#[test]
fn html_skips_hidden_nodes() {
    let mut hidden = rect("n10", 0.0, 0.0, 10.0, 10.0, None);
    hidden.base_mut().visible = Some(false);
    hidden.base_mut().name = Some("hidden".into());
    let doc = doc_with_nodes(vec![hidden]);
    let html = Html.generate(&doc);
    assert!(!html.contains("hidden"));
}

#[test]
fn css_variables_empty_doc_emits_header_only() {
    let doc = empty_doc();
    let css = CssVariables.generate(&doc);
    assert!(css.contains("Generated by OpenPencil"));
    assert!(!css.contains(":root"));
}

#[test]
fn falls_back_to_bare_children_when_unpaged() {
    let mut doc = empty_doc();
    doc.children = vec![rect("n10", 1.0, 2.0, 30.0, 40.0, None)];
    let html = Html.generate(&doc);
    assert!(html.contains("left:1px"));
    assert!(html.contains("width:30px"));
}

// --- First-class widget node builders ------------------------------
//
// The widget structs derive `Default`, so the builders only set the
// fields a test cares about and lean on `..Default::default()` for
// the rest (id, sizing, fills, bindings, …).

fn base(id: &str) -> PenNodeBase {
    PenNodeBase {
        id: id.into(),
        ..Default::default()
    }
}

fn option(value: &str, label: &str) -> SelectOption {
    SelectOption {
        value: value.into(),
        label: label.into(),
    }
}

#[test]
fn html_emits_text_input_element() {
    let node = PenNode::TextInput(TextInputNode {
        base: base("w1"),
        placeholder: Some("Email".into()),
        value: Some("a@b.c".into()),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<input type=\"text\""));
    assert!(html.contains("placeholder=\"Email\""));
    assert!(html.contains("value=\"a@b.c\""));
}

#[test]
fn html_emits_textarea_with_value_body() {
    let node = PenNode::TextArea(TextAreaNode {
        base: base("w1"),
        placeholder: Some("Bio".into()),
        value: Some("Hi <there>".into()),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<textarea"));
    assert!(html.contains("placeholder=\"Bio\""));
    // Value lands in the body and is html-escaped like text nodes.
    assert!(html.contains(">Hi &lt;there&gt;</textarea>"));
}

#[test]
fn html_emits_number_input_with_min_max_step() {
    let node = PenNode::NumberInput(NumberInputNode {
        base: base("w1"),
        min: Some(0.0),
        max: Some(10.0),
        step: Some(2.0),
        value: Some(NumberOrExpression::Number(4.0)),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<input type=\"number\""));
    assert!(html.contains("min=\"0\""));
    assert!(html.contains("max=\"10\""));
    assert!(html.contains("step=\"2\""));
    assert!(html.contains("value=\"4\""));
}

#[test]
fn html_emits_select_with_options_and_selected() {
    let node = PenNode::Select(SelectNode {
        base: base("w1"),
        value: Some("b".into()),
        options: Some(vec![option("a", "Apple"), option("b", "Banana")]),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<select>"));
    assert!(html.contains("<option value=\"a\">Apple</option>"));
    // The current value gets the `selected` flag.
    assert!(html.contains("<option value=\"b\" selected>Banana</option>"));
}

#[test]
fn html_emits_radio_group_with_checked_match() {
    let node = PenNode::RadioGroup(RadioGroupNode {
        base: base("grp"),
        value: Some("y".into()),
        options: Some(vec![option("y", "Yes"), option("n", "No")]),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    // name groups the radios; the matching value is checked.
    assert!(html.contains("type=\"radio\" name=\"grp\" value=\"y\" checked"));
    assert!(html.contains("type=\"radio\" name=\"grp\" value=\"n\" />No"));
}

#[test]
fn html_emits_switch_checkbox_with_role_and_checked() {
    let node = PenNode::Switch(SwitchNode {
        base: base("w1"),
        checked: Some(BoolOrExpression::Bool(true)),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<input type=\"checkbox\" role=\"switch\" checked />"));
}

#[test]
fn html_emits_checkbox_with_label() {
    let node = PenNode::Checkbox(CheckboxNode {
        base: base("w1"),
        checked: Some(BoolOrExpression::Bool(false)),
        label: Some("Accept".into()),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    // Unchecked → no `checked` attribute; label text follows.
    assert!(html.contains("<input type=\"checkbox\" /> <label>Accept</label>"));
}

#[test]
fn html_emits_slider_range_input() {
    let node = PenNode::Slider(SliderNode {
        base: base("w1"),
        min: Some(0.0),
        max: Some(100.0),
        step: Some(5.0),
        value: Some(NumberOrExpression::Number(25.0)),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<input type=\"range\""));
    assert!(html.contains("min=\"0\""));
    assert!(html.contains("max=\"100\""));
    assert!(html.contains("step=\"5\""));
    assert!(html.contains("value=\"25\""));
}

#[test]
fn html_emits_progress_with_value_and_max() {
    let node = PenNode::Progress(ProgressNode {
        base: base("w1"),
        value: Some(NumberOrExpression::Number(30.0)),
        max: Some(100.0),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<progress value=\"30\" max=\"100\"></progress>"));
}

#[test]
fn html_emits_tabs_tablist_and_panels() {
    let panel = rect("p1", 0.0, 0.0, 10.0, 10.0, None);
    let node = PenNode::Tabs(TabsNode {
        base: base("w1"),
        tabs: Some(vec![option("one", "One"), option("two", "Two")]),
        value: Some("two".into()),
        children: Some(vec![panel]),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<nav role=\"tablist\">"));
    assert!(html.contains("<button role=\"tab\" value=\"one\">One</button>"));
    // The active tab carries aria-selected.
    assert!(html.contains("<button role=\"tab\" value=\"two\" aria-selected=\"true\">Two</button>"));
    // Panel child still recurses (rect → positioned div).
    assert!(html.contains("width:10px"));
}

#[test]
fn react_emits_text_input_element() {
    let node = PenNode::TextInput(TextInputNode {
        base: base("w1"),
        placeholder: Some("Email".into()),
        ..Default::default()
    });
    let s = React.generate(&doc_with_nodes(vec![node]));
    assert!(s.contains("<input type=\"text\""));
    assert!(s.contains("placeholder=\"Email\""));
}

#[test]
fn react_emits_select_with_options() {
    let node = PenNode::Select(SelectNode {
        base: base("w1"),
        value: Some("a".into()),
        options: Some(vec![option("a", "Apple")]),
        ..Default::default()
    });
    let s = React.generate(&doc_with_nodes(vec![node]));
    assert!(s.contains("<select>"));
    assert!(s.contains("<option value=\"a\" selected>Apple</option>"));
}

#[test]
fn react_emits_checkbox_and_progress() {
    let checkbox = PenNode::Checkbox(CheckboxNode {
        base: base("c1"),
        checked: Some(BoolOrExpression::Bool(true)),
        label: Some("On".into()),
        ..Default::default()
    });
    let progress = PenNode::Progress(ProgressNode {
        base: base("p1"),
        value: Some(NumberOrExpression::Number(50.0)),
        ..Default::default()
    });
    let s = React.generate(&doc_with_nodes(vec![checkbox, progress]));
    assert!(s.contains("<input type=\"checkbox\" checked /> <label>On</label>"));
    assert!(s.contains("<progress value=\"50\"></progress>"));
}

#[test]
fn expression_bound_values_degrade_to_no_attribute() {
    // A `$var`-bound slider value can't resolve statically, so no
    // `value` attribute is emitted (same degrade as other generators).
    let node = PenNode::Slider(SliderNode {
        base: base("w1"),
        value: Some(NumberOrExpression::Expression("$progress".into())),
        ..Default::default()
    });
    let html = Html.generate(&doc_with_nodes(vec![node]));
    assert!(html.contains("<input type=\"range\""));
    assert!(!html.contains("value="));
    assert!(!html.contains("$progress"));
}

/// B1: `--`-prefixed shadcn variable names must emit as CSS custom
/// properties verbatim (`--primary`), never triple-dash (`---primary`).
#[test]
fn css_variables_emit_leading_dash_names_verbatim() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert("--primary".to_string(), color_def("#2563eb"));
    vars.insert("--muted-foreground".to_string(), color_def("#64748b"));
    doc.variables = Some(vars);
    let css = CssVariables.generate(&doc);
    assert!(css.contains("--primary: #2563eb;"), "css: {css}");
    assert!(css.contains("--muted-foreground: #64748b;"), "css: {css}");
    assert!(!css.contains("---"), "triple-dash leak: {css}");
}

/// B1: the shadcn `.dark` class convention — themed `Mode=Dark` entries
/// also emit under a `.dark` selector so shadcn component classes
/// consume the generated globals.css directly.
#[test]
fn css_variables_emit_dark_class_block() {
    let mut doc = empty_doc();
    let mut vars = BTreeMap::new();
    vars.insert(
        "--background".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#f8fafc".into()),
                    theme: Some(axis("Mode", "Light")),
                },
                ThemedValue {
                    value: VariableScalar::Str("#0f172a".into()),
                    theme: Some(axis("Mode", "Dark")),
                },
            ]),
        },
    );
    doc.variables = Some(vars);
    let css = CssVariables.generate(&doc);
    assert!(css.contains(":root[data-Mode=\"Dark\"]"), "css: {css}");
    assert!(css.contains(".dark"), "missing .dark selector: {css}");
    // The .dark selector shares the dark block's declarations.
    let dark_block = css.split(".dark").nth(1).expect(".dark block");
    assert!(dark_block.contains("--background: #0f172a;"), "css: {css}");
}
