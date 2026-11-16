//! Compile standalone HTML tap handlers and screen transitions.
//!
//! The exported browser program consumes data only. Route expressions are
//! resolved here when they are string literals in the authored screen set.

use std::collections::{BTreeMap, BTreeSet};

use jian_core::motion::screen_transition::{
    NavigationKind, ResolvedScreenTransition, ScreenTransitionSpec, TransitionScreen,
};
use jian_core::motion::MotionControl;
use jian_ops_schema::events::Action;
use jian_ops_schema::motion::{MotionPreference, MotionTrigger};
use jian_ops_schema::{node::PenNode, PenDocument};
use op_editor_core::interaction_edit::unquote_literal;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

use crate::{fmt_num, html_escape, node_children, node_hidden, node_size, root_nodes};

const RUNTIME: &str = include_str!("codegen_html_prototype.js");
const TRANSITION_STEPS: u32 = 120;

enum TapAction {
    Motion {
        control: MotionControl,
    },
    Push {
        path: String,
        effect: usize,
        back: usize,
    },
    Replace {
        path: String,
        effect: usize,
        back: usize,
    },
    Pop {
        effect: Option<usize>,
    },
}

impl TapAction {
    fn to_wire(&self) -> Value {
        match self {
            Self::Motion { control } => json!({
                "op": "motion", "target": control.target, "animation": control.animation, "mode": control.mode
            }),
            Self::Push { path, effect, back } => json!({
                "op": "push", "path": path, "effect": effect, "back": back
            }),
            Self::Replace { path, effect, back } => json!({
                "op": "replace", "path": path, "effect": effect, "back": back
            }),
            Self::Pop { effect } => json!({ "op": "pop", "effect": effect }),
        }
    }
}

pub(super) struct HtmlPrototype {
    actions: BTreeMap<String, Vec<TapAction>>,
    targets: BTreeSet<String>,
    effects: Vec<ResolvedScreenTransition>,
    entry: Option<String>,
    reduced: bool,
}

impl HtmlPrototype {
    pub(super) fn compile(doc: &PenDocument) -> Self {
        let roots = root_nodes(doc);
        let screens: BTreeSet<&str> = roots
            .iter()
            .filter(|root| !node_hidden(root))
            .filter_map(|root| root.screen())
            .filter(|path| !path.is_empty())
            .collect();
        let mut prototype = Self {
            actions: BTreeMap::new(),
            targets: BTreeSet::new(),
            effects: Vec::new(),
            entry: None,
            reduced: doc.motion == Some(MotionPreference::Reduced),
        };
        let mut clips = BTreeMap::new();
        for root in roots {
            collect_clips(root, &mut clips);
        }
        for root in roots {
            prototype.collect_actions(root, &screens, &clips);
        }
        if prototype
            .actions
            .values()
            .flatten()
            .any(|action| matches!(action, TapAction::Push { .. } | TapAction::Replace { .. }))
        {
            prototype.entry = doc
                .routes
                .as_ref()
                .map(|routes| routes.entry.as_str())
                .filter(|entry| screens.contains(entry))
                .or_else(|| screens.get("/").copied())
                .or_else(|| {
                    roots
                        .iter()
                        .filter(|root| !node_hidden(root))
                        .filter_map(|root| root.screen())
                        .find(|path| screens.contains(path))
                })
                .map(str::to_owned);
        }
        if prototype.entry.is_none() {
            for actions in prototype.actions.values_mut() {
                actions.retain(|action| matches!(action, TapAction::Motion { .. }));
            }
            prototype.effects.clear();
        }
        prototype
    }

    pub(super) fn attributes(&self, node: &PenNode) -> String {
        let mut attrs = String::new();
        if self.targets.contains(node.id_str()) {
            attrs.push_str(&format!(
                " data-op-target=\"{}\"",
                html_escape(node.id_str())
            ));
        }
        if let Some(actions) = self.actions.get(node.id_str()) {
            let actions: Vec<_> = actions.iter().map(TapAction::to_wire).collect();
            let json = serde_json::to_string(&actions).expect("serializable tap data");
            attrs.push_str(&format!(" data-op-tap=\"{}\"", html_escape(&json)));
            if !actions.is_empty() {
                attrs.push_str(" role=\"button\" tabindex=\"0\"");
            }
        }
        attrs
    }

    pub(super) fn render_roots(
        &self,
        out: &mut String,
        roots: &[PenNode],
        mut emit: impl FnMut(&mut String, &PenNode, usize, bool),
    ) {
        let Some(entry) = self.entry.as_deref() else {
            for root in roots {
                emit(out, root, 1, false);
            }
            return;
        };
        let Some(root) = roots
            .iter()
            .find(|n| !node_hidden(n) && n.screen() == Some(entry))
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
        for root in roots.iter().filter(|root| !node_hidden(root)) {
            let Some(path) = root.screen().filter(|path| emitted.insert(*path)) else {
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
        for root in roots.iter().filter(|root| root.screen().is_none()) {
            emit(out, root, 1, false);
        }
    }

    pub(super) fn render_script(&self, out: &mut String) {
        if !self.actions.values().any(|actions| !actions.is_empty()) {
            return;
        }
        let profiles: Vec<_> = self.effects.iter().map(encode_profile).collect();
        let json = serde_json::to_string(&profiles).expect("serializable transition samples");
        out.push_str(&format!(
            "<script type=\"application/json\" id=\"op-screen-motion\">{json}</script>\n"
        ));
        out.push_str("<script>\n");
        out.push_str(RUNTIME);
        out.push_str("</script>\n");
    }

    fn collect_actions(
        &mut self,
        node: &PenNode,
        screens: &BTreeSet<&str>,
        clips: &BTreeMap<String, BTreeSet<String>>,
    ) {
        if node_hidden(node) {
            return;
        }
        if let Some(actions) = node.events().and_then(|events| {
            let disabled = events
                .extra
                .get("disabledEvents")
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("onTap")));
            events
                .on_tap
                .as_ref()
                .filter(|actions| !disabled && !actions.is_empty())
        }) {
            let actions = actions
                .iter()
                .map(|action| self.compile_action(action, screens, clips))
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default();
            for action in &actions {
                if let TapAction::Motion { control } = action {
                    self.targets.insert(control.target.clone());
                }
            }
            self.actions.insert(node.id_str().to_owned(), actions);
        }
        for child in node_children(node) {
            self.collect_actions(child, screens, clips);
        }
    }

    fn compile_action(
        &mut self,
        action: &Action,
        screens: &BTreeSet<&str>,
        clips: &BTreeMap<String, BTreeSet<String>>,
    ) -> Option<TapAction> {
        let (verb, body) = action.single()?;
        match verb {
            "play_motion" => {
                let control: MotionControl = serde_json::from_value(body.clone()).ok()?;
                clips
                    .get(&control.target)
                    .filter(|ids| ids.contains(&control.animation))?;
                Some(TapAction::Motion { control })
            }
            "push" | "replace" if screens.len() > 1 => {
                let expression = body.as_str().or_else(|| body.get("path")?.as_str())?;
                let path = unquote_literal(expression);
                if path == expression.trim() || !screens.contains(path.as_str()) {
                    return None;
                }
                let spec = transition_spec(body)?;
                let push = verb == "push";
                let navigation = if push {
                    NavigationKind::Push
                } else {
                    NavigationKind::Replace
                };
                let effect = self.effect(spec.as_ref(), navigation);
                let back = self.effect(spec.as_ref(), NavigationKind::Pop);
                Some(if push {
                    TapAction::Push { path, effect, back }
                } else {
                    TapAction::Replace { path, effect, back }
                })
            }
            "pop" if screens.len() > 1 => {
                let spec = transition_spec(body)?;
                Some(TapAction::Pop {
                    effect: spec
                        .as_ref()
                        .map(|spec| self.effect(Some(spec), NavigationKind::Pop)),
                })
            }
            _ => None,
        }
    }

    fn effect(&mut self, spec: Option<&ScreenTransitionSpec>, navigation: NavigationKind) -> usize {
        let effect = ResolvedScreenTransition::resolve(spec, navigation);
        if let Some(index) = self.effects.iter().position(|existing| *existing == effect) {
            return index;
        }
        self.effects.push(effect);
        self.effects.len() - 1
    }
}

fn transition_spec(body: &Value) -> Option<Option<ScreenTransitionSpec>> {
    match body.get("transition").filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone()).ok().map(Some),
        None => Some(None),
    }
}

fn collect_clips(
    node: &PenNode,
    index: &mut BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    if node_hidden(node) {
        return BTreeSet::new();
    }
    let mut ids: BTreeSet<_> = node
        .motion_declarations()
        .1
        .into_iter()
        .flatten()
        .filter(|clip| clip.trigger == MotionTrigger::Manual)
        .filter_map(|clip| clip.id.clone())
        .collect();
    for child in node_children(node) {
        ids.extend(collect_clips(child, index));
    }
    if !ids.is_empty() {
        index.insert(node.id_str().to_owned(), ids.clone());
    }
    ids
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
