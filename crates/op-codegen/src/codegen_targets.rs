//! Per-framework markup generators — the React / Vue / Svelte / HTML
//! / Flutter / SwiftUI / Compose / React Native targets carved off
//! `lib.rs` to keep both files under the 800-line cap.
//!
//! All generators consume the canonical `jian_ops_schema::PenDocument`
//! and walk `PenNode` through the flat-view accessors in the crate
//! root (`node_origin` / `node_size` / `node_fill_css` /
//! …) so the per-target emit logic stays variant-agnostic.

use crate::{
    color_to_css, fmt_num, html_escape, node_children, node_corner_radius, node_fill_css,
    node_hidden, node_is_ellipse, node_is_text, node_origin, node_rotation_deg, node_size,
    node_stroke_css, node_text, parse_color, root_nodes, Codegen, CssVariables,
};
use jian_ops_schema::node::{BoolOrExpression, NumberOrExpression, PenNode};
use jian_ops_schema::PenDocument;

// --- First-class widget element mapping ----------------------------
//
// The 10 form/widget node kinds (text_input / text_area / number_input
// / select / radio_group / switch / checkbox / slider / progress /
// tabs) map to real HTML form elements instead of falling through to
// the generic `<div>` / `<span>` heuristic in `emit_node_html` /
// `emit_node_jsx`. These helpers return the inner element markup; the
// per-target emitters keep owning the positioned wrapper. Generated
// markup is valid in both HTML and JSX (self-closed voids end `/>`,
// boolean attributes are bare — which JSX accepts), so the two targets
// share one mapping. `tabs` is the only container: it emits a button
// tablist from `tabs[]` and recurses children as panels, so its inner
// recursion is target-specific and handled inline by the caller.

/// Flatten a `BoolOrExpression` to a literal `true`. Expression-bound
/// checked states can't be resolved statically, so they read as
/// unchecked — same degrade the other generators apply for `$var`.
fn bool_or_expr_true(b: &Option<BoolOrExpression>) -> bool {
    matches!(b, Some(BoolOrExpression::Bool(true)))
}

/// Flatten a `NumberOrExpression` to its literal value, when concrete.
/// Expression-bound values read as `None` (no static value attribute).
fn number_or_expr(n: &Option<NumberOrExpression>) -> Option<f64> {
    match n {
        Some(NumberOrExpression::Number(v)) => Some(*v),
        _ => None,
    }
}

/// Emit a single HTML attribute `key="value"` (value html-escaped),
/// skipping when `value` is `None`. The leading space lets callers
/// concatenate attributes onto an open tag.
fn opt_attr(key: &str, value: Option<&str>) -> String {
    match value {
        Some(v) => format!(" {}=\"{}\"", key, html_escape(v)),
        None => String::new(),
    }
}

/// Emit a numeric attribute `key="n"` (formatted via `fmt_num`),
/// skipping when `value` is `None`.
fn opt_num_attr(key: &str, value: Option<f64>) -> String {
    match value {
        Some(v) => format!(" {}=\"{}\"", key, fmt_num(v)),
        None => String::new(),
    }
}

/// Map a first-class widget node to its form-element markup. Returns
/// `None` for non-widget variants (so the caller falls back to the
/// generic div/span heuristic). `tabs` is handled by the caller
/// because its panel children recurse target-specifically; this
/// helper covers the 9 leaf widget kinds.
fn widget_markup(node: &PenNode) -> Option<String> {
    Some(match node {
        PenNode::TextInput(n) => format!(
            "<input type=\"text\"{}{} />",
            opt_attr("placeholder", n.placeholder.as_deref()),
            opt_attr("value", n.value.as_deref()),
        ),
        PenNode::TextArea(n) => format!(
            "<textarea{}>{}</textarea>",
            opt_attr("placeholder", n.placeholder.as_deref()),
            html_escape(n.value.as_deref().unwrap_or_default()),
        ),
        PenNode::NumberInput(n) => format!(
            "<input type=\"number\"{}{}{}{} />",
            opt_num_attr("min", n.min),
            opt_num_attr("max", n.max),
            opt_num_attr("step", n.step),
            opt_num_attr("value", number_or_expr(&n.value)),
        ),
        PenNode::Select(n) => {
            let mut s = String::from("<select>");
            for opt in n.options.iter().flatten() {
                let selected = if n.value.as_deref() == Some(opt.value.as_str()) {
                    " selected"
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<option value=\"{}\"{}>{}</option>",
                    html_escape(&opt.value),
                    selected,
                    html_escape(&opt.label),
                ));
            }
            s.push_str("</select>");
            s
        }
        PenNode::RadioGroup(n) => {
            let group = n.base.id.as_str();
            let mut s = String::new();
            for opt in n.options.iter().flatten() {
                let checked = if n.value.as_deref() == Some(opt.value.as_str()) {
                    " checked"
                } else {
                    ""
                };
                s.push_str(&format!(
                    "<label><input type=\"radio\" name=\"{}\" value=\"{}\"{} />{}</label>",
                    html_escape(group),
                    html_escape(&opt.value),
                    checked,
                    html_escape(&opt.label),
                ));
            }
            s
        }
        PenNode::Switch(n) => format!(
            "<input type=\"checkbox\" role=\"switch\"{} />",
            if bool_or_expr_true(&n.checked) {
                " checked"
            } else {
                ""
            },
        ),
        PenNode::Checkbox(n) => format!(
            "<input type=\"checkbox\"{} /> <label>{}</label>",
            if bool_or_expr_true(&n.checked) {
                " checked"
            } else {
                ""
            },
            html_escape(n.label.as_deref().unwrap_or_default()),
        ),
        PenNode::Slider(n) => format!(
            "<input type=\"range\"{}{}{}{} />",
            opt_num_attr("min", n.min),
            opt_num_attr("max", n.max),
            opt_num_attr("step", n.step),
            opt_num_attr("value", number_or_expr(&n.value)),
        ),
        PenNode::Progress(n) => format!(
            "<progress{}{}></progress>",
            opt_num_attr("value", number_or_expr(&n.value)),
            opt_num_attr("max", n.max),
        ),
        _ => return None,
    })
}

/// Emit the tablist `<nav>` of buttons for a `tabs` node — one button
/// per `tabs[]` entry, the active one (matching `value`) flagged
/// `aria-selected`. The panel children are recursed by the caller so
/// HTML / JSX each keep their own child-emit routine.
fn tabs_nav_markup(node: &PenNode) -> Option<String> {
    let PenNode::Tabs(n) = node else {
        return None;
    };
    let mut s = String::from("<nav role=\"tablist\">");
    for tab in n.tabs.iter().flatten() {
        let selected = if n.value.as_deref() == Some(tab.value.as_str()) {
            " aria-selected=\"true\""
        } else {
            ""
        };
        s.push_str(&format!(
            "<button role=\"tab\" value=\"{}\"{}>{}</button>",
            html_escape(&tab.value),
            selected,
            html_escape(&tab.label),
        ));
    }
    s.push_str("</nav>");
    Some(s)
}

/// HTML + inline-CSS generator. Walks the document's node tree and
/// emits `<div>` per Rect/Frame/Group with absolute positioning and
/// inline style; ellipses get `border-radius: 50%`; text nodes emit
/// `<span>`. Bare minimum useful — refining to semantic tags + flex
/// containers is a follow-up.
pub struct Html;

impl Codegen for Html {
    fn target_label(&self) -> &'static str {
        "html"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("<!-- Generated by OpenPencil — codegen::Html -->\n");
        out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"></head>\n<body>\n");
        for node in root_nodes(doc) {
            emit_node_html(&mut out, node, 1);
        }
        out.push_str("</body></html>\n");
        out
    }
}

fn emit_node_html(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let tag = if node_is_text(node) { "span" } else { "div" };
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    let mut style = format!(
        "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px",
        fmt_num(x),
        fmt_num(y),
        fmt_num(w),
        fmt_num(h),
    );
    if let Some(fill) = node_fill_css(node) {
        style.push_str(&format!(";background:{}", color_to_css(&fill)));
    }
    if let Some((color, width)) = node_stroke_css(node) {
        style.push_str(&format!(
            ";border:{}px solid {}",
            width,
            color_to_css(&color)
        ));
    }
    let radius = node_corner_radius(node);
    if radius > 0.0 {
        style.push_str(&format!(";border-radius:{}px", fmt_num(radius)));
    }
    if node_is_ellipse(node) {
        style.push_str(";border-radius:50%");
    }
    let rotation = node_rotation_deg(node);
    if rotation.abs() > f64::EPSILON {
        style.push_str(&format!(";transform:rotate({}deg)", fmt_num(rotation)));
    }
    // First-class widgets win over the generic div/span heuristic: nest
    // the real form element inside the positioned wrapper so absolute
    // layout is preserved. `tabs` also recurses its panel children.
    if let Some(widget) = widget_markup(node) {
        out.push_str(&format!("{indent}<div style=\"{style}\">{widget}</div>\n"));
        return;
    }
    if let Some(nav) = tabs_nav_markup(node) {
        out.push_str(&format!("{indent}<div style=\"{style}\">{nav}"));
        let children = node_children(node);
        if children.is_empty() {
            out.push_str("</div>\n");
        } else {
            out.push('\n');
            for c in children {
                emit_node_html(out, c, depth + 1);
            }
            out.push_str(&format!("{indent}</div>\n"));
        }
        return;
    }
    let body = node_text(node).unwrap_or_default();
    let children = node_children(node);
    if children.is_empty() && body.is_empty() {
        out.push_str(&format!("{indent}<{tag} style=\"{style}\"></{tag}>\n"));
    } else {
        out.push_str(&format!("{indent}<{tag} style=\"{style}\">"));
        if !body.is_empty() {
            out.push_str(&html_escape(&body));
        }
        if !children.is_empty() {
            out.push('\n');
            for c in children {
                emit_node_html(out, c, depth + 1);
            }
            out.push_str(&indent);
        }
        out.push_str(&format!("</{tag}>\n"));
    }
}

/// Vue 3 Single File Component generator. Wraps the node tree in a
/// `<template>` block (HTML-like) + an empty `<script setup>`
/// placeholder + a `<style>` block carrying any
/// design-variables-derived custom properties. Mirrors TS
/// `pen-codegen::vue-generator`.
pub struct Vue;

impl Codegen for Vue {
    fn target_label(&self) -> &'static str {
        "vue"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("<!-- Generated by OpenPencil — codegen::Vue -->\n");
        out.push_str("<template>\n");
        for n in root_nodes(doc) {
            emit_node_html(&mut out, n, 1);
        }
        out.push_str("</template>\n\n");
        out.push_str("<script setup lang=\"ts\">\n// generated\n</script>\n\n");
        out.push_str("<style scoped>\n");
        out.push_str(&CssVariables.generate(doc));
        out.push_str("</style>\n");
        out
    }
}

/// Svelte component generator. Markup + `<style>` block — Svelte
/// allows top-level markup without a wrapping template tag. Same
/// design-variables-as-CSS-vars pattern as Vue.
pub struct Svelte;

impl Codegen for Svelte {
    fn target_label(&self) -> &'static str {
        "svelte"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("<!-- Generated by OpenPencil — codegen::Svelte -->\n");
        out.push_str("<script lang=\"ts\">\n// generated\n</script>\n\n");
        for n in root_nodes(doc) {
            emit_node_html(&mut out, n, 0);
        }
        out.push_str("\n<style>\n");
        out.push_str(&CssVariables.generate(doc));
        out.push_str("</style>\n");
        out
    }
}

/// React + inline-style JSX generator. Emits one functional
/// component named `Page` returning a fragment of absolute-
/// positioned `<div>`s + text spans. Same per-node mapping as the
/// HTML target; the wrapper is JSX. Mirrors TS
/// `pen-codegen::react-generator` minus Tailwind class generation.
pub struct React;

impl Codegen for React {
    fn target_label(&self) -> &'static str {
        "react"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("// Generated by OpenPencil — codegen::React\n");
        out.push_str("import React from 'react';\n\n");
        out.push_str("export default function Page() {\n");
        out.push_str("  return (\n    <>\n");
        for n in root_nodes(doc) {
            emit_node_jsx(&mut out, n, 3);
        }
        out.push_str("    </>\n  );\n}\n");
        out
    }
}

fn emit_node_jsx(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let tag = if node_is_text(node) { "span" } else { "div" };
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    let radius = node_corner_radius(node);
    // JSX style is an object literal — semicolons → commas, camelCase keys.
    let style = format!(
        "position: 'absolute', left: {}, top: {}, width: {}, height: {}{}{}{}{}",
        fmt_num(x),
        fmt_num(y),
        fmt_num(w),
        fmt_num(h),
        node_fill_css(node)
            .map(|c| format!(", background: '{}'", color_to_css(&c)))
            .unwrap_or_default(),
        node_stroke_css(node)
            .map(|(c, sw)| format!(", border: '{}px solid {}'", sw, color_to_css(&c)))
            .unwrap_or_default(),
        if radius > 0.0 {
            format!(", borderRadius: {}", fmt_num(radius))
        } else {
            String::new()
        },
        if node_is_ellipse(node) {
            ", borderRadius: '50%'".to_string()
        } else {
            String::new()
        },
    );
    // First-class widgets win over the generic div/span heuristic. The
    // shared form-element markup is JSX-valid (self-closed voids, bare
    // boolean attributes), so it nests directly inside the positioned
    // wrapper. `tabs` also recurses its panel children.
    if let Some(widget) = widget_markup(node) {
        out.push_str(&format!(
            "{indent}<div style={{{{{style}}}}}>{widget}</div>\n"
        ));
        return;
    }
    if let Some(nav) = tabs_nav_markup(node) {
        out.push_str(&format!("{indent}<div style={{{{{style}}}}}>{nav}"));
        let children = node_children(node);
        if children.is_empty() {
            out.push_str("</div>\n");
        } else {
            out.push('\n');
            for c in children {
                emit_node_jsx(out, c, depth + 1);
            }
            out.push_str(&format!("{indent}</div>\n"));
        }
        return;
    }
    let body = node_text(node).unwrap_or_default();
    let children = node_children(node);
    if children.is_empty() && body.is_empty() {
        out.push_str(&format!("{indent}<{tag} style={{{{{style}}}}} />\n"));
    } else {
        out.push_str(&format!("{indent}<{tag} style={{{{{style}}}}}>"));
        if !body.is_empty() {
            out.push_str(&html_escape(&body));
        }
        if !children.is_empty() {
            out.push('\n');
            for c in children {
                emit_node_jsx(out, c, depth + 1);
            }
            out.push_str(&indent);
        }
        out.push_str(&format!("</{tag}>\n"));
    }
}

/// Flutter / Dart generator. Emits a `Stack` whose children are
/// `Positioned`-wrapped `Container`s sized + coloured per node.
/// Text uses `Text(...)` with a `TextStyle`. Mirrors TS
/// `pen-codegen::flutter-generator`.
pub struct Flutter;

impl Codegen for Flutter {
    fn target_label(&self) -> &'static str {
        "flutter"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("// Generated by OpenPencil — codegen::Flutter\n");
        out.push_str("import 'package:flutter/material.dart';\n\n");
        out.push_str("Widget buildPage() {\n  return Stack(children: [\n");
        for n in root_nodes(doc) {
            emit_node_flutter(&mut out, n, 2);
        }
        out.push_str("  ]);\n}\n");
        out
    }
}

/// Flatten a colour string into `Color.fromARGB(a,r,g,b)`. Falls
/// back to `null` when the colour is not a parseable hex / rgba.
fn flutter_color(color: &str) -> String {
    match parse_color(color) {
        Some((r, g, b, a)) => {
            let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!(
                "Color.fromARGB({},{},{},{})",
                to_u8(a),
                to_u8(r),
                to_u8(g),
                to_u8(b)
            )
        }
        None => "null".into(),
    }
}

fn emit_node_flutter(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    let inner = if node_is_text(node) {
        let body = node_text(node).unwrap_or_default();
        format!("Text({:?})", body)
    } else {
        let color = node_fill_css(node)
            .map(|c| flutter_color(&c))
            .unwrap_or_else(|| "null".into());
        format!(
            "Container(width: {}, height: {}, decoration: BoxDecoration(color: {}))",
            fmt_num(w),
            fmt_num(h),
            color
        )
    };
    out.push_str(&format!(
        "{indent}Positioned(left: {}, top: {}, child: {}),\n",
        fmt_num(x),
        fmt_num(y),
        inner
    ));
    for c in node_children(node) {
        emit_node_flutter(out, c, depth);
    }
}

/// SwiftUI generator. Emits a `ZStack` of `Rectangle`/`Ellipse`/
/// `Text` views with `.frame` + `.position` modifiers. Mirrors TS
/// `pen-codegen::swiftui-generator`.
pub struct SwiftUi;

impl Codegen for SwiftUi {
    fn target_label(&self) -> &'static str {
        "swiftui"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("// Generated by OpenPencil — codegen::SwiftUi\n");
        out.push_str("import SwiftUI\n\nstruct Page: View {\n  var body: some View {\n");
        out.push_str("    ZStack {\n");
        for n in root_nodes(doc) {
            emit_node_swiftui(&mut out, n, 3);
        }
        out.push_str("    }\n  }\n}\n");
        out
    }
}

fn emit_node_swiftui(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let view = if node_is_text(node) {
        let body = node_text(node).unwrap_or_default();
        format!("Text({:?})", body)
    } else if node_is_ellipse(node) {
        "Ellipse()".to_string()
    } else {
        "Rectangle()".to_string()
    };
    let color = node_fill_css(node)
        .and_then(|c| parse_color(&c))
        .map(|(r, g, b, a)| {
            format!(
                ".foregroundColor(Color(red: {:.3}, green: {:.3}, blue: {:.3}, opacity: {:.3}))",
                r, g, b, a
            )
        })
        .unwrap_or_default();
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    out.push_str(&format!(
        "{indent}{view}{color}.frame(width: {}, height: {}).position(x: {}, y: {})\n",
        fmt_num(w),
        fmt_num(h),
        fmt_num(cx),
        fmt_num(cy)
    ));
    for c in node_children(node) {
        emit_node_swiftui(out, c, depth);
    }
}

/// Android Jetpack Compose generator. Emits a `Box` with
/// `Modifier.offset`/`size` per node + a single fill color.
/// Mirrors TS `pen-codegen::compose-generator`.
pub struct Compose;

impl Codegen for Compose {
    fn target_label(&self) -> &'static str {
        "compose"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("// Generated by OpenPencil — codegen::Compose\n");
        out.push_str("import androidx.compose.foundation.*\n");
        out.push_str("import androidx.compose.foundation.layout.*\n");
        out.push_str("import androidx.compose.material3.Text\n");
        out.push_str("import androidx.compose.runtime.Composable\n");
        out.push_str("import androidx.compose.ui.Modifier\n");
        out.push_str("import androidx.compose.ui.graphics.Color\n");
        out.push_str("import androidx.compose.ui.unit.dp\n\n");
        out.push_str("@Composable\nfun Page() {\n  Box(modifier = Modifier.fillMaxSize()) {\n");
        for n in root_nodes(doc) {
            emit_node_compose(&mut out, n, 2);
        }
        out.push_str("  }\n}\n");
        out
    }
}

fn emit_node_compose(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    if node_is_text(node) {
        let body = node_text(node).unwrap_or_default();
        out.push_str(&format!(
            "{indent}Text(text = {:?}, modifier = Modifier.offset(x = {}.dp, y = {}.dp))\n",
            body,
            fmt_num(x),
            fmt_num(y)
        ));
    } else {
        let color = node_fill_css(node)
            .and_then(|c| parse_color(&c))
            .map(|(r, g, b, a)| {
                let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                format!(
                    ".background(Color({}, {}, {}, {}))",
                    to_u8(r),
                    to_u8(g),
                    to_u8(b),
                    to_u8(a)
                )
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "{indent}Box(modifier = Modifier.offset(x = {}.dp, y = {}.dp).size(width = {}.dp, height = {}.dp){})\n",
            fmt_num(x),
            fmt_num(y),
            fmt_num(w),
            fmt_num(h),
            color
        ));
    }
    for c in node_children(node) {
        emit_node_compose(out, c, depth);
    }
}

/// React Native generator. Emits a default-exported functional
/// component with absolute-positioned `<View>`s + `<Text>`s using
/// React Native's `StyleSheet.flatten`-friendly inline style.
/// Mirrors TS `pen-codegen::react-native-generator`.
pub struct ReactNative;

impl Codegen for ReactNative {
    fn target_label(&self) -> &'static str {
        "react-native"
    }
    fn generate(&self, doc: &PenDocument) -> String {
        let mut out = String::new();
        out.push_str("// Generated by OpenPencil — codegen::ReactNative\n");
        out.push_str("import React from 'react';\n");
        out.push_str("import { View, Text } from 'react-native';\n\n");
        out.push_str(
            "export default function Page() {\n  return (\n    <View style={{ flex: 1 }}>\n",
        );
        for n in root_nodes(doc) {
            emit_node_rn(&mut out, n, 3);
        }
        out.push_str("    </View>\n  );\n}\n");
        out
    }
}

fn emit_node_rn(out: &mut String, node: &PenNode, depth: usize) {
    if node_hidden(node) {
        return;
    }
    let indent = "  ".repeat(depth);
    let (x, y) = node_origin(node);
    let (w, h) = node_size(node);
    let radius = node_corner_radius(node);
    let style = format!(
        "position: 'absolute', left: {}, top: {}, width: {}, height: {}{}{}",
        fmt_num(x),
        fmt_num(y),
        fmt_num(w),
        fmt_num(h),
        node_fill_css(node)
            .map(|c| format!(", backgroundColor: '{}'", color_to_css(&c)))
            .unwrap_or_default(),
        if radius > 0.0 {
            format!(", borderRadius: {}", fmt_num(radius))
        } else {
            String::new()
        },
    );
    if node_is_text(node) {
        let body = node_text(node).unwrap_or_default();
        out.push_str(&format!(
            "{indent}<Text style={{{{{style}}}}}>{}</Text>\n",
            html_escape(&body)
        ));
    } else {
        let children = node_children(node);
        if children.is_empty() {
            out.push_str(&format!("{indent}<View style={{{{{style}}}}} />\n"));
        } else {
            out.push_str(&format!("{indent}<View style={{{{{style}}}}}>\n"));
            for c in children {
                emit_node_rn(out, c, depth + 1);
            }
            out.push_str(&format!("{indent}</View>\n"));
        }
    }
}
