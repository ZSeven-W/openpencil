//! Check explicitly requested map controls before accepting a navigation viewport.
//! This is a structural contract, not proof that the prototype behaves correctly.

use crate::{plan::Subtask, types::DesignRequest};
use jian_ops_schema::{node::PenNode, state::StateSchema};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
enum Control {
    Layers,
    Locate,
    End,
}

impl Control {
    fn name(self) -> &'static str {
        match self {
            Self::Layers => "图层/layers",
            Self::Locate => "定位/locate",
            Self::End => "结束导航/end navigation",
        }
    }

    fn matches(self, text: &str) -> bool {
        let text = text.to_lowercase().replace(['-', '_'], " ");
        let words: &[&str] = match self {
            Self::Layers => &["图层", "layer"],
            Self::Locate => &[
                "定位",
                "locate",
                "location",
                "recenter",
                "re centre",
                "crosshair",
            ],
            Self::End => &[
                "结束导航",
                "退出导航",
                "停止导航",
                "end navigation",
                "stop navigation",
                "exit navigation",
            ],
        };
        words.iter().any(|word| text.contains(word))
    }
}

fn requested(req: &DesignRequest, subtask: &Subtask) -> Vec<Control> {
    if !crate::map_viewport::owns(subtask) {
        return Vec::new();
    }
    let brief = req.prompt.to_lowercase();
    let explicit = [
        "可操作",
        "可点击",
        "可交互",
        "interactive",
        "clickable",
        "functional",
    ]
    .iter()
    .any(|word| brief.contains(word));
    let static_only = [
        "不可操作",
        "不可点击",
        "不可交互",
        "无需交互",
        "不需要交互",
        "仅视觉",
        "static only",
        "non-interactive",
        "not interactive",
    ]
    .iter()
    .any(|word| brief.contains(word));
    if !explicit || static_only {
        return Vec::new();
    }
    [Control::Layers, Control::Locate, Control::End]
        .into_iter()
        .filter(|control| control.matches(&brief))
        .collect()
}

pub(crate) fn instruction(req: &DesignRequest, subtask: &Subtask) -> String {
    let controls = requested(req, subtask);
    if controls.is_empty() {
        return String::new();
    }
    format!("\n\nOPERABLE MAP CONTROLS — required: {}. Each requested enabled button needs its own non-empty events.onTap and a visible state-driven result. Declare typed state on the viewport root, write $app.<key> with set/toggle and read it in bindings on visible content. Every set VALUE is an expression STRING: use \"true\"/\"false\", never raw booleans. A toast alone or an unused state variable does NOT implement an action. Prefer inline bound feedback; toast text, if used, must be a quoted expression string. For example, layers opens/closes a small layer chooser; locate resets a simulated map view or shows an explicitly labelled demo-location marker; end navigation changes the trip panel to an ended state with a way to resume. This is an illustrative local prototype: do not claim live GPS or a real navigation service. Ending navigation means stopped, not arrived. Bind text/CTA in place for active/ended states; do not stack hidden alternative rows in the trip panel. Transient panels must be absolute overlays inside the viewport, with authored visible:false plus a visible binding for the closed state; opacity:0 alone still intercepts taps and reserves space. Static properties must match initial state so the exported design is also correct. Keep these states compact; preserve the initial layout, supplied copy, map and controls. Return the complete section, not an interaction-only fragment.", controls.iter().map(|c| c.name()).collect::<Vec<_>>().join(", "))
}

fn visible(node: &Value) -> bool {
    node["visible"].as_bool() != Some(false)
        && node["enabled"].as_bool() != Some(false)
        && node["opacity"].as_f64() != Some(0.0)
}

fn children(node: &Value) -> &[Value] {
    node["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn identity(node: &Value) -> String {
    ["name", "content", "iconFontName"]
        .iter()
        .filter_map(|key| node[*key].as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn controls<'a>(node: &'a Value, kind: Control, out: &mut Vec<&'a Value>) {
    if !visible(node) {
        return;
    }
    // Only a control or its immediate label/icon identifies it. A large map
    // wrapper containing three independent buttons must not satisfy all three.
    let name = node["name"].as_str().unwrap_or("").to_lowercase();
    let affordance = node["role"].as_str().is_some_and(|r| r.contains("button"))
        || name.contains("button")
        || name.contains("按钮")
        || node["type"] == "icon_font"
        || (node["height"].as_f64().is_some_and(|h| h <= 64.0)
            && children(node).iter().any(|c| c["type"] == "icon_font"));
    let matches = kind.matches(&identity(node))
        || children(node).iter().filter(|c| visible(c)).any(|c| {
            matches!(c["type"].as_str(), Some("text" | "icon_font")) && kind.matches(&identity(c))
        });
    if affordance && matches {
        out.push(node);
        return;
    }
    for child in children(node) {
        controls(child, kind, out);
    }
}

fn state_reads(node: &Value, reads: &mut BTreeSet<String>, declared: &mut BTreeSet<String>) {
    if let Some(state) = node["state"].as_object() {
        declared.extend(state.keys().map(|key| format!("$app.{key}")));
    }
    if let Some(bindings) = node["bindings"].as_object() {
        // A hidden panel may become visible via a binding. Do not discard its
        // reads based on the initial visible flag.
        for expression in bindings.values().filter_map(Value::as_str) {
            reads.extend(
                expression
                    .split(|c: char| !c.is_alphanumeric() && !['_', '$', '.'].contains(&c))
                    .filter(|token| token.starts_with("$app."))
                    .map(str::to_owned),
            );
        }
    }
    for child in children(node) {
        state_reads(child, reads, declared);
    }
}

fn changes_bound_state(
    actions: &Value,
    reads: &BTreeSet<String>,
    declared: &BTreeSet<String>,
) -> bool {
    actions.as_array().is_some_and(|actions| {
        actions.iter().any(|action| {
            let bound = |path: &str| reads.contains(path) && declared.contains(path);
            action["toggle"].as_str().is_some_and(bound)
                || action["set"].as_object().is_some_and(|set| {
                    set.keys().any(|key| bound(key))
                        && set.values().all(|value| {
                            value
                                .as_str()
                                .is_some_and(|expression| !expression.trim().is_empty())
                        })
                })
                || changes_bound_state(&action["if"]["then"], reads, declared)
                || changes_bound_state(&action["if"]["else"], reads, declared)
        })
    })
}

pub(crate) fn rejection(
    req: &DesignRequest,
    subtask: &Subtask,
    nodes: &[PenNode],
    program_state: &StateSchema,
) -> Option<String> {
    let required = requested(req, subtask);
    if required.is_empty() {
        return None;
    }
    let forest = serde_json::to_value(nodes).ok()?;
    let roots = forest.as_array()?;
    let mut reads = BTreeSet::new();
    let mut declared = program_state.keys().map(|k| format!("$app.{k}")).collect();
    for root in roots {
        state_reads(root, &mut reads, &mut declared);
    }
    let missing: Vec<_> = required
        .into_iter()
        .filter_map(|kind| {
            let mut found = Vec::new();
            for root in roots {
                controls(root, kind, &mut found);
            }
            // A disabled or missing requested control cannot satisfy an explicit
            // operability request, even when other controls have working events.
            if found.is_empty() {
                return Some(kind.name().to_owned());
            }
            for node in found {
                let actions = &node["events"]["onTap"];
                // One malformed toast/branch can prevent the entire chain
                // from being installed, even when the set itself is valid.
                if let Err(error) = jian_core::action::default_registry()
                    .borrow()
                    .parse_list(actions)
                {
                    return Some(format!("{} (invalid action chain: {error})", kind.name()));
                }
                if !changes_bound_state(actions, &reads, &declared) {
                    return Some(kind.name().to_owned());
                }
            }
            None
        })
        .collect();
    (!missing.is_empty()).then(|| format!(
        "self-check failed: navigation-control-inert: {}. Regenerate the complete viewport with events.onTap on each requested button, declared $app state and bindings producing a visible local prototype change. Every set value must be an expression string (\"true\", not raw true). Toast text also needs a quoted expression, or omit the toast. Toast-only feedback and unused state do not implement the action. Preserve composition and all requested content; do not claim live GPS.", missing.join(", ")
    ))
}

#[cfg(test)]
#[path = "navigation_interaction_tests.rs"]
mod tests;
