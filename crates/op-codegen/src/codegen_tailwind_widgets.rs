//! First-class widget nodes (`text_input`, `select`, `switch`, …) →
//! shadcn/ui primitives for the `react-tailwind` target. The widget's
//! authored state (placeholder, value, options, checked) becomes the
//! component's uncontrolled `default*` props; its box placement and
//! size become Tailwind classes. Visual styling is the component's own.

use jian_ops_schema::node::{BoolOrExpression, NumberOrExpression, PenNode};

use super::jsx::{emit_node, jsx_attr, jsx_text, line};
use super::shadcn::dom_id;
use super::style::{self, Classes, Parent};
use super::Ctx;
use crate::fmt_num;

fn frame_classes(ctx: &mut Ctx, node: &PenNode, parent: Parent) -> Classes {
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    classes
}

fn opt_str_attr(key: &str, value: Option<&str>) -> String {
    value
        .map(|v| format!(" {key}=\"{}\"", jsx_attr(v)))
        .unwrap_or_default()
}

fn opt_num_attr(key: &str, value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite())
        .map(|v| format!(" {key}={{{}}}", fmt_num(v)))
        .unwrap_or_default()
}

fn literal_number(value: &Option<NumberOrExpression>) -> Option<f64> {
    match value {
        Some(NumberOrExpression::Number(n)) => Some(*n),
        _ => None,
    }
}

fn checked_attr(value: &Option<BoolOrExpression>) -> &'static str {
    if matches!(value, Some(BoolOrExpression::Bool(true))) {
        " defaultChecked"
    } else {
        ""
    }
}

/// Emit `node` as a shadcn/ui widget. Returns `false` for non-widget
/// variants.
pub(super) fn emit(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) -> bool {
    match node {
        PenNode::TextInput(n) => {
            ctx.ui("Input");
            let cls = frame_classes(ctx, node, parent).attr();
            let kind = if n.secure == Some(true) {
                " type=\"password\""
            } else {
                ""
            };
            line(
                out,
                depth,
                &format!(
                    "<Input{kind}{}{}{cls} />",
                    opt_str_attr("placeholder", n.placeholder.as_deref()),
                    opt_str_attr("defaultValue", n.value.as_deref()),
                ),
            );
        }
        PenNode::TextArea(n) => {
            ctx.ui("Textarea");
            let cls = frame_classes(ctx, node, parent).attr();
            line(
                out,
                depth,
                &format!(
                    "<Textarea{}{}{cls} />",
                    opt_str_attr("placeholder", n.placeholder.as_deref()),
                    opt_str_attr("defaultValue", n.value.as_deref()),
                ),
            );
        }
        PenNode::NumberInput(n) => {
            ctx.ui("Input");
            let cls = frame_classes(ctx, node, parent).attr();
            line(
                out,
                depth,
                &format!(
                    "<Input type=\"number\"{}{}{}{}{cls} />",
                    opt_num_attr("min", n.min),
                    opt_num_attr("max", n.max),
                    opt_num_attr("step", n.step),
                    opt_num_attr("defaultValue", literal_number(&n.value)),
                ),
            );
        }
        PenNode::Switch(n) => {
            ctx.ui("Switch");
            let cls = frame_classes(ctx, node, parent).attr();
            line(
                out,
                depth,
                &format!("<Switch{}{cls} />", checked_attr(&n.checked)),
            );
        }
        PenNode::Checkbox(n) => {
            ctx.ui("Checkbox");
            let id = dom_id(node);
            let checked = checked_attr(&n.checked);
            match n.label.as_deref().filter(|l| !l.is_empty()) {
                Some(label) => {
                    ctx.ui("Label");
                    let mut row = Classes::default();
                    row.push("flex");
                    row.push("items-center");
                    row.push("gap-2");
                    row.extend(frame_classes(ctx, node, parent));
                    line(out, depth, &format!("<div{}>", row.attr()));
                    line(
                        out,
                        depth + 1,
                        &format!("<Checkbox id=\"{id}\"{checked} />"),
                    );
                    line(
                        out,
                        depth + 1,
                        &format!("<Label htmlFor=\"{id}\">{}</Label>", jsx_text(label)),
                    );
                    line(out, depth, "</div>");
                }
                None => {
                    let cls = frame_classes(ctx, node, parent).attr();
                    line(out, depth, &format!("<Checkbox{checked}{cls} />"));
                }
            }
        }
        PenNode::Slider(n) => {
            ctx.ui("Slider");
            let cls = frame_classes(ctx, node, parent).attr();
            let value = literal_number(&n.value).or(n.min).unwrap_or(0.0);
            line(
                out,
                depth,
                &format!(
                    "<Slider defaultValue={{[{}]}}{}{}{}{cls} />",
                    fmt_num(value),
                    opt_num_attr("min", n.min),
                    opt_num_attr("max", n.max),
                    opt_num_attr("step", n.step),
                ),
            );
        }
        PenNode::Progress(n) => {
            ctx.ui("Progress");
            let cls = frame_classes(ctx, node, parent).attr();
            let max = n.max.filter(|m| *m > 0.0).unwrap_or(100.0);
            let percent = literal_number(&n.value)
                .map(|v| ((v / max) * 100.0).clamp(0.0, 100.0).round())
                .unwrap_or(0.0);
            line(
                out,
                depth,
                &format!("<Progress value={{{}}}{cls} />", fmt_num(percent)),
            );
        }
        PenNode::Select(n) => emit_select(ctx, out, node, n, parent, depth),
        PenNode::RadioGroup(n) => {
            for name in ["Label", "RadioGroup", "RadioGroupItem"] {
                ctx.ui(name);
            }
            let cls = frame_classes(ctx, node, parent).attr();
            line(
                out,
                depth,
                &format!(
                    "<RadioGroup{}{cls}>",
                    opt_str_attr("defaultValue", n.value.as_deref())
                ),
            );
            let group = dom_id(node);
            for option in n.options.iter().flatten() {
                let id = format!(
                    "{group}-{}",
                    option
                        .value
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                        .collect::<String>()
                );
                line(
                    out,
                    depth + 1,
                    "<div className=\"flex items-center gap-2\">",
                );
                line(
                    out,
                    depth + 2,
                    &format!(
                        "<RadioGroupItem value=\"{}\" id=\"{id}\" />",
                        jsx_attr(&option.value)
                    ),
                );
                line(
                    out,
                    depth + 2,
                    &format!(
                        "<Label htmlFor=\"{id}\">{}</Label>",
                        jsx_text(&option.label)
                    ),
                );
                line(out, depth + 1, "</div>");
            }
            line(out, depth, "</RadioGroup>");
        }
        PenNode::Tabs(n) => {
            for name in ["Tabs", "TabsContent", "TabsList", "TabsTrigger"] {
                ctx.ui(name);
            }
            let tabs: Vec<_> = n.tabs.iter().flatten().collect();
            let default = n
                .value
                .clone()
                .or_else(|| tabs.first().map(|t| t.value.clone()));
            let cls = frame_classes(ctx, node, parent).attr();
            line(
                out,
                depth,
                &format!(
                    "<Tabs{}{cls}>",
                    opt_str_attr("defaultValue", default.as_deref())
                ),
            );
            line(out, depth + 1, "<TabsList>");
            for tab in &tabs {
                line(
                    out,
                    depth + 2,
                    &format!(
                        "<TabsTrigger value=\"{}\">{}</TabsTrigger>",
                        jsx_attr(&tab.value),
                        jsx_text(&tab.label)
                    ),
                );
            }
            line(out, depth + 1, "</TabsList>");
            // Panel i belongs to tab i; surplus panels stay visible.
            let panels = n.children.as_deref().unwrap_or_default();
            for (index, panel) in panels.iter().enumerate() {
                match tabs.get(index) {
                    Some(tab) => {
                        line(
                            out,
                            depth + 1,
                            &format!("<TabsContent value=\"{}\">", jsx_attr(&tab.value)),
                        );
                        emit_node(ctx, out, panel, Parent::Root, depth + 2);
                        line(out, depth + 1, "</TabsContent>");
                    }
                    None => emit_node(ctx, out, panel, Parent::Root, depth + 1),
                }
            }
            line(out, depth, "</Tabs>");
        }
        _ => return false,
    }
    true
}

fn emit_select(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    select: &jian_ops_schema::node::SelectNode,
    parent: Parent,
    depth: usize,
) {
    for name in [
        "Select",
        "SelectContent",
        "SelectItem",
        "SelectTrigger",
        "SelectValue",
    ] {
        ctx.ui(name);
    }
    let cls = frame_classes(ctx, node, parent).attr();
    line(
        out,
        depth,
        &format!(
            "<Select{}>",
            opt_str_attr("defaultValue", select.value.as_deref())
        ),
    );
    line(out, depth + 1, &format!("<SelectTrigger{cls}>"));
    line(
        out,
        depth + 2,
        &format!(
            "<SelectValue{} />",
            opt_str_attr("placeholder", select.placeholder.as_deref())
        ),
    );
    line(out, depth + 1, "</SelectTrigger>");
    let options: Vec<_> = select.options.iter().flatten().collect();
    if options.is_empty() {
        line(out, depth + 1, "<SelectContent />");
    } else {
        line(out, depth + 1, "<SelectContent>");
        for option in options {
            line(
                out,
                depth + 2,
                &format!(
                    "<SelectItem value=\"{}\">{}</SelectItem>",
                    jsx_attr(&option.value),
                    jsx_text(&option.label)
                ),
            );
        }
        line(out, depth + 1, "</SelectContent>");
    }
    line(out, depth, "</Select>");
}
