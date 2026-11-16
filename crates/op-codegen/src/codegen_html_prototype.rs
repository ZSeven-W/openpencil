//! Standalone HTML presentation and compiled interaction data.

use std::collections::{BTreeMap, BTreeSet};

use jian_core::motion::screen_transition::{
    NavigationKind, ResolvedScreenTransition, ScreenTransitionSpec, TransitionScreen,
};
use jian_ops_schema::events::ActionList;
use jian_ops_schema::motion::MotionPreference;
use jian_ops_schema::{node::PenNode, page::PenPage, PenDocument};
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

use crate::{fmt_num, html_escape, node_children, node_hidden, node_size, root_nodes};

#[path = "html_prototype/actions.rs"]
mod actions;
#[path = "html_prototype/expression.rs"]
mod expression;

const EXPRESSIONS: &str = include_str!("html_prototype/expression.js");
const RUNTIME: &str = include_str!("codegen_html_prototype.js");
const TRANSITION_STEPS: u32 = 120;

struct NodeProgram {
    page: String,
    events: BTreeMap<&'static str, Vec<Value>>,
    bindings: BTreeMap<String, Value>,
    state: Value,
}

pub(super) struct HtmlPrototype {
    nodes: BTreeMap<String, NodeProgram>,
    pages: BTreeMap<String, Value>,
    state: Value,
    vars: Value,
    effects: Vec<ResolvedScreenTransition>,
    defaults: [usize; 3],
    entry: Option<String>,
    interactive: bool,
    navigates: bool,
    reduced: bool,
}

impl HtmlPrototype {
    pub(super) fn compile(doc: &PenDocument) -> Self {
        let pages = doc.pages.as_deref().unwrap_or_default();
        let entry = marked_roots(doc)
            .filter_map(|(root, _)| screen_path(root))
            .find(|path| *path == "/")
            .or_else(|| marked_roots(doc).find_map(|(root, _)| screen_path(root)))
            .map(str::to_owned);
        let mut program = Self {
            nodes: BTreeMap::new(),
            pages: BTreeMap::new(),
            state: serde_json::to_value(&doc.state).map_or(Value::Null, |value| defaults(&value)),
            vars: Value::Null,
            effects: Vec::new(),
            defaults: [0; 3],
            entry,
            interactive: false,
            navigates: false,
            reduced: doc.motion == Some(MotionPreference::Reduced),
        };
        for (root, page) in marked_roots(doc) {
            program.collect_root(root, page, root.id_str());
        }
        let fallback = pages.first();
        for root in root_nodes(doc) {
            if screen_path(root).is_none() {
                program.collect_root(root, fallback, fallback.map_or("", |page| page.id.as_str()));
            }
        }
        if program.navigates {
            program.defaults = program.profiles_for(None);
        } else {
            program.entry = None;
        }
        if program.interactive && doc.variables.is_some() {
            program.vars = initial_variables(doc);
        }
        program
    }

    pub(super) fn interactive(&self) -> bool {
        self.interactive
    }

    pub(super) fn attributes(&self, node: &PenNode) -> String {
        if !self.interactive {
            return String::new();
        }
        let mut attrs = format!(" data-op-target=\"{}\"", html_escape(node.id_str()));
        if self
            .nodes
            .get(node.id_str())
            .is_some_and(|node| node.events.contains_key("onTap"))
        {
            attrs.push_str(" data-op-tap role=\"button\" tabindex=\"0\"");
        }
        if node_hidden(node) {
            attrs.push_str(" hidden");
        }
        attrs
    }

    pub(super) fn motion_roots<'a>(&self, doc: &'a PenDocument) -> Vec<&'a PenNode> {
        if self.entry.is_none() {
            return root_nodes(doc).iter().collect();
        }
        let mut paths = BTreeSet::new();
        marked_roots(doc)
            .filter_map(|(root, _)| {
                screen_path(root)
                    .filter(|path| paths.insert(*path))
                    .map(|_| root)
            })
            .chain(
                root_nodes(doc)
                    .iter()
                    .filter(|root| screen_path(root).is_none()),
            )
            .collect()
    }

    pub(super) fn render_roots(
        &self,
        out: &mut String,
        doc: &PenDocument,
        mut emit: impl FnMut(&mut String, &PenNode, usize, bool),
    ) {
        let Some(entry) = self.entry.as_deref() else {
            for root in root_nodes(doc) {
                emit(out, root, 1, false);
            }
            return;
        };
        let Some((root, _)) = marked_roots(doc).find(|(node, _)| screen_path(node) == Some(entry))
        else {
            return;
        };
        let (width, height) = node_size(root);
        let reduced = if self.reduced { " data-op-reduced" } else { "" };
        out.push_str(&format!(
            "  <div data-op-stage{reduced} style=\"position:relative;overflow:hidden;width:{}px;height:{}px\">\n",
            fmt_num(width), fmt_num(height),
        ));
        let mut emitted = BTreeSet::new();
        for (root, _) in marked_roots(doc) {
            let Some(path) = screen_path(root).filter(|path| emitted.insert(*path)) else {
                continue;
            };
            let (width, height) = node_size(root);
            let hidden = if path == entry { "" } else { " hidden inert" };
            out.push_str(&format!(
                "    <div data-op-screen=\"{}\"{hidden} style=\"position:absolute;top:0;left:0;width:{}px;height:{}px\">\n",
                html_escape(path), fmt_num(width), fmt_num(height),
            ));
            emit(out, root, 3, true);
            out.push_str("    </div>\n");
        }
        out.push_str("  </div>\n");
        for root in root_nodes(doc)
            .iter()
            .filter(|root| screen_path(root).is_none())
        {
            emit(out, root, 1, false);
        }
    }

    pub(super) fn render_script(&self, out: &mut String) {
        if !self.interactive {
            return;
        }
        let profiles: Vec<_> = self.effects.iter().map(encode_profile).collect();
        let nodes: BTreeMap<_, _> = self.nodes.iter().map(|(id, node)| (id, json!({
            "page": node.page, "events": node.events, "bindings": node.bindings, "state": node.state
        }))).collect();
        let program = json!({
            "nodes": nodes, "pages": self.pages, "state": self.state, "vars": self.vars,
            "profiles": profiles, "defaults": self.defaults, "reduced": self.reduced
        });
        // A JSON script is still an HTML raw-text element. Escape '<' so
        // authored text cannot end it or introduce executable markup.
        let json = program.to_string().replace('<', "\\u003c");
        out.push_str(&format!(
            "<script type=\"application/json\" id=\"op-prototype\">{json}</script>\n"
        ));
        out.push_str("<script>\n(() => {\n");
        out.push_str(EXPRESSIONS);
        out.push_str(RUNTIME);
        out.push_str("})();\n</script>\n");
    }

    fn collect_root(&mut self, node: &PenNode, page: Option<&PenPage>, scope: &str) {
        if !self.pages.contains_key(scope) {
            let state = page
                .and_then(|page| serde_json::to_value(&page.state).ok())
                .map_or_else(|| json!({}), |value| defaults(&value));
            self.pages.insert(scope.to_owned(), state);
        }
        let raw = serde_json::to_value(node).expect("serializable node");
        self.collect_node(node, &raw, scope);
    }

    fn collect_node(&mut self, node: &PenNode, raw: &Value, page: &str) {
        let mut events = BTreeMap::new();
        if let Some(authored) = node.events() {
            let disabled = authored
                .extra
                .get("disabledEvents")
                .and_then(Value::as_array);
            for (name, actions) in [
                ("onTap", &authored.on_tap),
                ("onHoverEnter", &authored.on_hover_enter),
                ("onHoverLeave", &authored.on_hover_leave),
                ("onChange", &authored.on_change),
                ("onSubmit", &authored.on_submit),
                ("onFocus", &authored.on_focus),
                ("onBlur", &authored.on_blur),
                ("onKey", &authored.on_key),
                ("onScroll", &authored.on_scroll),
            ] {
                if let Some(actions) = actions.as_ref().filter(|_| {
                    !disabled
                        .is_some_and(|list| list.iter().any(|value| value.as_str() == Some(name)))
                }) {
                    self.insert_event(&mut events, name, actions);
                }
            }
        }
        if let Some(hooks) = node.lifecycle() {
            for (name, actions) in [
                ("onMount", &hooks.on_mount),
                ("onUnmount", &hooks.on_unmount),
            ] {
                if let Some(actions) = actions.as_ref().filter(|_| {
                    !hooks
                        .disabled_events
                        .as_ref()
                        .is_some_and(|list| list.iter().any(|key| key == name))
                }) {
                    self.insert_event(&mut events, name, actions);
                }
            }
        }
        let bindings: BTreeMap<_, _> = raw["bindings"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(name, _)| {
                matches!(
                    name.as_str(),
                    "content"
                        | "visible"
                        | "opacity"
                        | "fill"
                        | "stroke"
                        | "cornerRadius"
                        | "x"
                        | "y"
                        | "width"
                        | "height"
                        | "value"
                        | "bind:value"
                        | "disabled"
                )
            })
            .filter_map(|(name, source)| Some((name.clone(), expression::from_value(source)?)))
            .collect();
        self.interactive |= !events.is_empty() || !bindings.is_empty();
        self.nodes.insert(
            node.id_str().to_owned(),
            NodeProgram {
                page: page.to_owned(),
                events,
                bindings,
                state: defaults(&raw["state"]),
            },
        );
        if let Some(children) = raw["children"].as_array() {
            for (child, value) in node_children(node).iter().zip(children) {
                self.collect_node(child, value, page);
            }
        }
    }

    fn insert_event(
        &mut self,
        events: &mut BTreeMap<&'static str, Vec<Value>>,
        name: &'static str,
        actions: &ActionList,
    ) {
        let actions = self.compile_actions(actions);
        if !actions.is_empty() {
            events.insert(name, actions);
        }
    }

    fn profiles_for(&mut self, spec: Option<&ScreenTransitionSpec>) -> [usize; 3] {
        [
            NavigationKind::Push,
            NavigationKind::Replace,
            NavigationKind::Pop,
        ]
        .map(|kind| {
            let effect = ResolvedScreenTransition::resolve(spec, kind);
            if let Some(index) = self.effects.iter().position(|existing| *existing == effect) {
                index
            } else {
                self.effects.push(effect);
                self.effects.len() - 1
            }
        })
    }
}

fn screen_path(root: &PenNode) -> Option<&str> {
    root.screen()
        .filter(|path| !path.is_empty() && !node_hidden(root))
}

fn marked_roots(doc: &PenDocument) -> impl Iterator<Item = (&PenNode, Option<&PenPage>)> {
    let pages = doc.pages.as_deref().unwrap_or_default();
    pages
        .iter()
        .flat_map(|page| page.children.iter().map(move |root| (root, Some(page))))
        .chain(
            doc.children
                .iter()
                .filter(move |_| pages.is_empty())
                .map(|root| (root, None)),
        )
        .filter(|(root, _)| screen_path(root).is_some())
}

fn defaults(schema: &Value) -> Value {
    Value::Object(
        schema
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, entry)| (key.clone(), entry["default"].clone()))
            .collect(),
    )
}

fn initial_variables(doc: &PenDocument) -> Value {
    jian_core::Runtime::new_from_document(doc.clone())
        .map(|runtime| json!(runtime.state.vars_snapshot()))
        .unwrap_or_else(|_| json!({}))
}

fn encode_profile(effect: &ResolvedScreenTransition) -> Value {
    let initial = effect.layers_at(0.0, (1.0, 1.0));
    let outgoing_on_top = initial[1].screen == TransitionScreen::Outgoing;
    let mut frames = [Vec::new(), Vec::new()];
    if effect.duration_ms != 0 {
        for step in 0..=TRANSITION_STEPS {
            let layers = effect.layers_at(step as f32 / TRANSITION_STEPS as f32, (1.0, 1.0));
            for layer in layers {
                let index = usize::from(layer.screen == TransitionScreen::Entering);
                frames[index].push([layer.dx, layer.dy, layer.alpha]);
            }
        }
    }
    json!({"duration": effect.duration_ms, "outgoing_on_top": outgoing_on_top, "frames": frames})
}

#[cfg(test)]
#[path = "codegen_html_prototype_tests.rs"]
mod tests;
