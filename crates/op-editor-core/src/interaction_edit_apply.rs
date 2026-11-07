//! Structured edits on a node's `events` block (see `interaction_edit`)
//! plus the `EditorState` entry point the property panel dispatches to.

use crate::interaction_edit::{
    build_action, events_map, sanitize_state_key, strip_state_prefix, InteractionActionKind,
    InteractionParamKind, InteractionTrigger,
};
use crate::pen_node_ext::PenNodeExt;
use crate::{EditorCommand, EditorState};
use jian_ops_schema::node::PenNode;
use serde_json::{Map, Value};

/// One structured change to the selected node's interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionEdit {
    /// Add `trigger` with one default action (no-op when it exists —
    /// then a default action is appended instead).
    AddTrigger(InteractionTrigger),
    /// Move `from`'s actions to `to` (appended when `to` already exists).
    ChangeTrigger {
        from: InteractionTrigger,
        to: InteractionTrigger,
    },
    /// Drop a whole trigger by its wire key (also covers read-only keys).
    RemoveTrigger(String),
    /// Append a default action to `trigger`.
    AddAction(InteractionTrigger),
    /// Swap action `index` to a different kind (param reseeded).
    SetActionKind {
        trigger: InteractionTrigger,
        index: usize,
        kind: InteractionActionKind,
    },
    /// Rewrite action `index`'s parameter, keeping its kind.
    SetActionParam {
        trigger: InteractionTrigger,
        index: usize,
        value: String,
    },
    RemoveAction {
        trigger: InteractionTrigger,
        index: usize,
    },
    /// Move action `index` one slot earlier (`up`) or later.
    MoveAction {
        trigger: InteractionTrigger,
        index: usize,
        up: bool,
    },
}

/// Which Interactions-section dropdown is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionMenuTarget {
    /// The trailing "+ Add interaction" row: pick a trigger.
    AddTrigger,
    /// A trigger header row, by wire key (unknown keys allowed).
    Trigger(String),
    /// An action row: change kind / reorder / remove.
    Action {
        trigger: InteractionTrigger,
        index: usize,
    },
    /// A picker parameter row (screen or layer).
    Param {
        trigger: InteractionTrigger,
        index: usize,
    },
}

/// A layer a Show/Hide/Scroll action can target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionTargetNode {
    pub id: String,
    pub name: String,
    pub hidden: bool,
}

/// Picker sources used to seed new actions with a useful parameter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionContext {
    pub screens: Vec<String>,
    /// The `screen` of the top-level frame holding the selection.
    pub own_screen: Option<String>,
    pub nodes: Vec<InteractionTargetNode>,
}

impl InteractionContext {
    fn default_param(&self, kind: InteractionActionKind) -> String {
        use InteractionActionKind as K;
        match kind {
            K::Navigate => self
                .screens
                .iter()
                .find(|s| Some(s.as_str()) != self.own_screen.as_deref())
                .or_else(|| self.screens.first())
                .cloned()
                .unwrap_or_else(|| "/".into()),
            K::Show | K::ToggleVisibility => self
                .nodes
                .iter()
                .find(|n| n.hidden)
                .or_else(|| self.nodes.first())
                .map(|n| n.id.clone())
                .unwrap_or_default(),
            K::Hide | K::ScrollTo => self
                .nodes
                .iter()
                .find(|n| !n.hidden)
                .or_else(|| self.nodes.first())
                .map(|n| n.id.clone())
                .unwrap_or_default(),
            K::Toast => "Done".into(),
            K::OpenUrl => "https://".into(),
            K::Copy => String::new(),
            K::ToggleState => "open".into(),
            K::Delay => "300".into(),
            K::Back | K::Haptic | K::DismissKeyboard => String::new(),
        }
    }

    /// The default action for a brand-new trigger.
    fn default_kind(&self) -> InteractionActionKind {
        if self.screens.is_empty() {
            InteractionActionKind::Toast
        } else {
            InteractionActionKind::Navigate
        }
    }

    fn default_action(&self, kind: InteractionActionKind) -> Value {
        build_action(kind, &self.default_param(kind), None)
    }
}

fn list_mut(map: &mut Map<String, Value>, trigger: InteractionTrigger) -> &mut Vec<Value> {
    let entry = map
        .entry(trigger.key().to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    entry.as_array_mut().expect("normalized to array")
}

fn one_key(action: &Value) -> Option<(&str, &Value)> {
    action
        .as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.iter().next())
        .map(|(k, v)| (k.as_str(), v))
}

/// Apply `edit` to an `events` map. Returns `false` when it addressed
/// nothing (bad index, unchanged). Empty triggers are dropped.
pub fn apply_interaction_edit(
    map: &mut Map<String, Value>,
    edit: &InteractionEdit,
    ctx: &InteractionContext,
) -> bool {
    let before = map.clone();
    match edit {
        InteractionEdit::AddTrigger(trigger) | InteractionEdit::AddAction(trigger) => {
            let kind = ctx.default_kind();
            list_mut(map, *trigger).push(ctx.default_action(kind));
        }
        InteractionEdit::ChangeTrigger { from, to } => {
            if from == to {
                return false;
            }
            let Some(moved) = map.remove(from.key()) else {
                return false;
            };
            let moved = moved.as_array().cloned().unwrap_or_default();
            list_mut(map, *to).extend(moved);
        }
        InteractionEdit::RemoveTrigger(key) => {
            map.remove(key);
        }
        InteractionEdit::SetActionKind {
            trigger,
            index,
            kind,
        } => {
            let Some(slot) = list_mut(map, *trigger).get_mut(*index) else {
                return false;
            };
            let current = one_key(slot)
                .and_then(|(k, b)| InteractionActionKind::classify(k, b))
                .filter(|k| k == kind);
            if current.is_none() {
                *slot = ctx.default_action(*kind);
            }
        }
        InteractionEdit::SetActionParam {
            trigger,
            index,
            value,
        } => {
            let Some(slot) = list_mut(map, *trigger).get_mut(*index) else {
                return false;
            };
            let Some((key, body)) = one_key(slot) else {
                return false;
            };
            let Some(kind) = InteractionActionKind::classify(key, body) else {
                return false;
            };
            if kind.param_kind() == InteractionParamKind::None {
                return false;
            }
            let verb = key.to_string();
            *slot = build_action(kind, value, Some(&verb));
        }
        InteractionEdit::RemoveAction { trigger, index } => {
            let list = list_mut(map, *trigger);
            if *index >= list.len() {
                return false;
            }
            list.remove(*index);
        }
        InteractionEdit::MoveAction { trigger, index, up } => {
            let list = list_mut(map, *trigger);
            let other = if *up {
                index.checked_sub(1)
            } else {
                Some(index + 1)
            };
            match other {
                Some(other) if other < list.len() && *index < list.len() => {
                    list.swap(*index, other)
                }
                _ => return false,
            }
        }
    }
    map.retain(|_, v| !v.as_array().is_some_and(Vec::is_empty));
    *map != before
}

/// `$app` keys toggled anywhere in `map`.
fn toggled_state_keys(map: &Map<String, Value>) -> Vec<String> {
    let mut keys = Vec::new();
    for list in map.values().filter_map(Value::as_array) {
        for action in list {
            if let Some(("toggle", Value::String(path))) = one_key(action) {
                let key = sanitize_state_key(strip_state_prefix(path));
                if !key.is_empty() && !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }
    keys
}

fn collect_targets(node: &PenNode, skip: &str, out: &mut Vec<InteractionTargetNode>) {
    let Some(children) = node.children() else {
        return;
    };
    for child in children {
        if out.len() >= MAX_TARGETS {
            return;
        }
        let base = child.base();
        if base.id != skip {
            out.push(InteractionTargetNode {
                id: base.id.clone(),
                name: base.name.clone().unwrap_or_else(|| base.id.clone()),
                hidden: base.visible == Some(false),
            });
        }
        collect_targets(child, skip, out);
    }
}

/// Cap on picker rows so the popover stays on screen.
pub const MAX_TARGETS: usize = 24;

fn contains_id(node: &PenNode, id: &str) -> bool {
    node.id_str() == id
        || node
            .children()
            .is_some_and(|c| c.iter().any(|child| contains_id(child, id)))
}

impl EditorState {
    /// Picker sources for the current selection: the page's screens, the
    /// selection's own screen, and the layers of its top-level frame.
    pub fn interaction_context(&self) -> InteractionContext {
        let screens = self
            .active_children()
            .iter()
            .filter_map(|n| n.screen().map(str::to_string))
            .collect();
        let anchor = self.selection.anchor.as_str().to_string();
        let root = self
            .active_children()
            .iter()
            .find(|n| contains_id(n, &anchor));
        let mut nodes = Vec::new();
        if let Some(root) = root {
            collect_targets(root, &anchor, &mut nodes);
        }
        InteractionContext {
            screens,
            own_screen: root.and_then(|r| r.screen().map(str::to_string)),
            nodes,
        }
    }

    /// Apply one structured interaction edit to the selected node as a
    /// single undo step. A `toggle` on an undeclared `$app` key also
    /// declares it (bool, default `false`) so the Preview runtime has a
    /// value to flip.
    pub fn apply_selected_interaction_edit(&mut self, edit: &InteractionEdit) -> bool {
        self.apply_interaction_edit_inner(edit, true)
    }

    /// [`Self::apply_selected_interaction_edit`] without its own history
    /// entry — for callers (the property-draft commit) that already
    /// bracket the write in one undo step.
    pub fn apply_selected_interaction_edit_untracked(&mut self, edit: &InteractionEdit) -> bool {
        self.apply_interaction_edit_inner(edit, false)
    }

    fn apply_interaction_edit_inner(&mut self, edit: &InteractionEdit, track: bool) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        let Some(node) = self.selected_node() else {
            return false;
        };
        let mut map = events_map(node.events());
        let ctx = self.interaction_context();
        if !apply_interaction_edit(&mut map, edit, &ctx) {
            return false;
        }
        let declared = self.doc.state.clone().unwrap_or_default();
        let new_state: std::collections::BTreeMap<_, _> = toggled_state_keys(&map)
            .into_iter()
            .filter(|k| !declared.contains_key(k))
            .map(|k| {
                (
                    k,
                    jian_ops_schema::state::StateEntry {
                        kind: jian_ops_schema::state::StateType::Primitive(
                            jian_ops_schema::state::PrimitiveType::Bool,
                        ),
                        default: Some(Value::Bool(false)),
                        description: None,
                        persist: None,
                    },
                )
            })
            .collect();
        let events = if map.is_empty() {
            Value::Null
        } else {
            Value::Object(map)
        };
        let patch_json = serde_json::json!({ "events": events }).to_string();
        if track {
            self.commit_history();
        }
        let changed = self.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
        if changed && !new_state.is_empty() {
            let _ = self.apply(EditorCommand::MergeAppState {
                plan_idx: 0,
                state: new_state,
            });
        }
        changed
    }
}

#[cfg(test)]
#[path = "interaction_edit_apply_tests.rs"]
mod tests;
