//! Interaction-integrity check for a finished design.
//!
//! The visual/structural finalize passes say nothing about whether the
//! prototype actually works. A generated product page passed every one of
//! them while its quantity showed `null`, its total `¥0`, and its name
//! input never reached state: the document declared no `$app` state and
//! bound the input to a target the runtime did not write. This module
//! inspects the wiring statically — no evaluation — and reports:
//!
//! - `$app` / `$state` keys that bindings or actions use but the document
//!   never declares (they start as `null` in Preview);
//! - `bind:value` sources that are not a writable single app-state key,
//!   and `bindings` property names the runtime has no target for;
//! - `push` / `replace` / `reset` to a route no screen declares;
//! - `show` / `hide` / `toggle_visibility` / `scroll_to` naming a node id
//!   that does not exist.
//!
//! It does not invent defaults: a missing declaration is reported, never
//! silently filled, because the right initial value is product intent.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde_json::Value;
use std::collections::BTreeSet;

/// One wiring defect, located by node id and schema site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionIssue {
    pub node_id: String,
    /// `bindings.content`, `events.onTap[0]`, …
    pub site: String,
    pub kind: InteractionIssueKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionIssueKind {
    UndeclaredState {
        key: String,
    },
    UnsupportedBindTarget {
        source: String,
        reason: String,
    },
    UnknownRoute {
        path: String,
    },
    UnknownTarget {
        target: String,
    },
    /// A `bindings` key the runtime has no target for (`selected`, …).
    UnsupportedBindingProperty {
        property: String,
    },
}

impl InteractionIssueKind {
    /// Stable advisory code for MCP consumers.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndeclaredState { .. } => "interaction-undeclared-state",
            Self::UnsupportedBindTarget { .. } => "interaction-unsupported-bind-target",
            Self::UnsupportedBindingProperty { .. } => "interaction-unsupported-binding-property",
            Self::UnknownRoute { .. } => "interaction-unknown-route",
            Self::UnknownTarget { .. } => "interaction-unknown-target",
        }
    }
}

impl std::fmt::Display for InteractionIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let at = format!("{} {}", self.node_id, self.site);
        match &self.kind {
            InteractionIssueKind::UndeclaredState { key } => write!(
                f,
                "interaction: `$app.{key}` is used at {at} but never declared in document state (starts as null)"
            ),
            InteractionIssueKind::UnsupportedBindTarget { source, reason } => {
                write!(f, "interaction: bind:value `{source}` at {at}: {reason}")
            }
            InteractionIssueKind::UnknownRoute { path } => {
                write!(f, "interaction: navigation to `{path}` at {at} matches no screen")
            }
            InteractionIssueKind::UnsupportedBindingProperty { property } => write!(
                f,
                "interaction: `bindings.{property}` at {at} is not a property the runtime can bind"
            ),
            InteractionIssueKind::UnknownTarget { target } => {
                write!(f, "interaction: action at {at} targets missing node `{target}`")
            }
        }
    }
}

/// Inspect every page of `doc`.
pub fn check_interactions(doc: &PenDocument) -> Vec<InteractionIssue> {
    let forests: Vec<&[PenNode]> = match doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages.iter().map(|p| p.children.as_slice()).collect(),
        _ => vec![doc.children.as_slice()],
    };
    let mut ctx = Context::default();
    if let Some(state) = doc.state.as_ref() {
        ctx.declared.extend(state.keys().cloned());
    }
    let values: Vec<Value> = forests
        .iter()
        .flat_map(|forest| forest.iter())
        .filter_map(|node| serde_json::to_value(node).ok())
        .collect();
    for value in &values {
        ctx.index(value);
    }
    let mut issues = Vec::new();
    for value in &values {
        ctx.check(value, &mut issues);
    }
    issues
}

#[derive(Default)]
struct Context {
    declared: BTreeSet<String>,
    node_ids: BTreeSet<String>,
    screens: BTreeSet<String>,
}

impl Context {
    fn index(&mut self, node: &Value) {
        let Some(map) = node.as_object() else {
            return;
        };
        if let Some(id) = map.get("id").and_then(Value::as_str) {
            self.node_ids.insert(id.to_owned());
        }
        if let Some(screen) = map.get("screen").and_then(Value::as_str) {
            self.screens.insert(screen.to_owned());
        }
        // Node-level `state` is hoisted to the document by generation, but
        // a hand-authored file may still carry it — it declares too.
        if let Some(state) = map.get("state").and_then(Value::as_object) {
            self.declared.extend(state.keys().cloned());
        }
        for child in children(map) {
            self.index(child);
        }
    }

    fn check(&self, node: &Value, issues: &mut Vec<InteractionIssue>) {
        let Some(map) = node.as_object() else {
            return;
        };
        let id = map
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_owned();
        let mut push = |site: String, kind: InteractionIssueKind| {
            let issue = InteractionIssue {
                node_id: id.clone(),
                site,
                kind,
            };
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        };
        if let Some(bindings) = map.get("bindings").and_then(Value::as_object) {
            for (prop, source) in bindings {
                let Some(source) = source.as_str() else {
                    continue;
                };
                let site = format!("bindings.{prop}");
                if prop != "bind:value" && jian_core::binding::BindingTarget::parse(prop).is_none()
                {
                    push(
                        site.clone(),
                        InteractionIssueKind::UnsupportedBindingProperty {
                            property: prop.clone(),
                        },
                    );
                }
                if prop == "bind:value" {
                    if let Err(error) = jian_core::state::parse_app_bind_key(source) {
                        push(
                            site.clone(),
                            InteractionIssueKind::UnsupportedBindTarget {
                                source: source.to_owned(),
                                reason: error.to_string(),
                            },
                        );
                    }
                }
                for key in app_keys(source) {
                    if !self.declared.contains(&key) {
                        push(site.clone(), InteractionIssueKind::UndeclaredState { key });
                    }
                }
            }
        }
        if let Some(events) = map.get("events").and_then(Value::as_object) {
            for (trigger, list) in events {
                let Some(actions) = list.as_array() else {
                    continue;
                };
                for (index, action) in actions.iter().enumerate() {
                    let site = format!("events.{trigger}[{index}]");
                    self.check_action(action, &site, &mut push);
                }
            }
        }
        for child in children(map) {
            self.check(child, issues);
        }
    }

    fn check_action(
        &self,
        action: &Value,
        site: &str,
        push: &mut impl FnMut(String, InteractionIssueKind),
    ) {
        for key in app_keys_in_value(action) {
            if !self.declared.contains(&key) {
                push(
                    site.to_owned(),
                    InteractionIssueKind::UndeclaredState { key },
                );
            }
        }
        let Some((verb, body)) = action
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.iter().next())
        else {
            return;
        };
        match verb.as_str() {
            "push" | "replace" | "reset" => {
                if let Some(path) = body.as_str().and_then(string_literal) {
                    // No screens at all is the WORST case (every route is
                    // missing), not a reason to skip the check.
                    if !self.screens.contains(&path) {
                        push(site.to_owned(), InteractionIssueKind::UnknownRoute { path });
                    }
                }
            }
            "show" | "hide" | "toggle_visibility" | "scroll_to" => {
                let target = body
                    .as_str()
                    .or_else(|| body.get("target").and_then(Value::as_str))
                    .map(str::trim);
                if let Some(target) = target.filter(|t| !t.is_empty()) {
                    if !self.node_ids.contains(target) {
                        push(
                            site.to_owned(),
                            InteractionIssueKind::UnknownTarget {
                                target: target.to_owned(),
                            },
                        );
                    }
                }
            }
            "if" | "parallel" | "delay" | "confirm" | "for_each" | "race" => {
                for nested in nested_action_lists(body) {
                    for (index, inner) in nested.iter().enumerate() {
                        self.check_action(inner, &format!("{site}.{verb}[{index}]"), push);
                    }
                }
            }
            _ => {}
        }
    }
}

fn children(map: &serde_json::Map<String, Value>) -> impl Iterator<Item = &Value> {
    map.get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// The action lists nested in a control-flow body: `parallel: [a, b]`
/// is ONE list of actions, `race: [[a], [b]]` is several, and
/// `if` / `confirm` / `for_each` carry lists under named fields.
fn nested_action_lists(body: &Value) -> Vec<&Vec<Value>> {
    match body {
        Value::Array(items) if items.iter().all(Value::is_object) => vec![body.as_array().unwrap()],
        Value::Array(items) => items.iter().filter_map(Value::as_array).collect(),
        Value::Object(map) => map
            .values()
            .filter(|v| is_action_list(v))
            .filter_map(Value::as_array)
            .collect(),
        _ => Vec::new(),
    }
}

/// `"\"/detail\""` / `"'/detail'"` → `/detail`; other expressions → None.
fn string_literal(source: &str) -> Option<String> {
    let s = source.trim();
    if s.len() >= 2 {
        if let Ok(Value::String(text)) = serde_json::from_str::<Value>(s) {
            return Some(text);
        }
        if s.starts_with('\'') && s.ends_with('\'') && !s[1..s.len() - 1].contains('\'') {
            return Some(s[1..s.len() - 1].to_owned());
        }
    }
    None
}

/// Every `$app.<key>` / `$state.<key>` head key an expression READS,
/// taken from the runtime compiler's own scope-path pool — so template
/// text without `${…}` and quoted strings never count, and the checker
/// can't drift from what Preview evaluates. A source that does not
/// compile reads nothing here (expression validity is checked at insert).
fn app_keys(source: &str) -> Vec<String> {
    let Ok(expression) = jian_core::expression::Expression::compile(source) else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    for path in &expression.chunk.scope_paths {
        let rest = path
            .strip_prefix("$app.")
            .or_else(|| path.strip_prefix("$state."));
        let Some(rest) = rest else { continue };
        let key: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !key.is_empty() && !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

/// A JSON value shaped like an action list (`[{verb: body}, …]`) or a
/// list of them (`race: [[…], […]]`).
fn is_action_list(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|item| item.is_object() || item.as_array().is_some())
    })
}

/// State keys an action body reads or writes: every string (expression
/// or `set` target) and object key, skipping nested action lists — those
/// are checked with their own site so a defect is reported once.
fn app_keys_in_value(value: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    collect_keys(value, &mut keys);
    keys
}

fn collect_keys(value: &Value, keys: &mut Vec<String>) {
    let add = |found: Vec<String>, keys: &mut Vec<String>| {
        for key in found {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    };
    match value {
        Value::String(s) => add(app_keys(s), keys),
        Value::Array(items) if !is_action_list(value) => {
            items.iter().for_each(|v| collect_keys(v, keys))
        }
        Value::Object(map) => {
            for (key, v) in map {
                add(app_keys(key), keys);
                if !is_action_list(v) {
                    collect_keys(v, keys);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "interaction_integrity_tests.rs"]
mod tests;
