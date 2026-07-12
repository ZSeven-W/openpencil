//! Instance inspection + override editing (GAP #10).
//!
//! TS `property-panel.tsx:28-135` resolves a selected `RefNode` into a
//! merged DISPLAY node (`{ ...component, ...descendants[ref],
//! ...definedRefProps }`) so an instance exposes the full inspector,
//! and routes panel writes either onto the `RefNode` itself
//! (`INSTANCE_DIRECT_PROPS`) or into `descendants[ref.target]` as a
//! partial-override object.
//!
//! ## Chosen choke point (DOCUMENTED)
//!
//! The Rust panel writes through ~30 anchor-keyed `EditorState`
//! mutators (`set_selected_color`, `commit_property_edit`,
//! `cmd_set_node_*`, layout/text/effect commands, …) — there is no
//! single function every write funnels through, and refactoring each
//! mutator would collide with the concurrent variables-panel WIP. The
//! one invariant they DO share: every write resolves its target via
//! `find_node_mut(active_children_mut(), selection.anchor)`.
//!
//! So the choke point is a **swap–run–diff–restore scope** around the
//! host's property-panel dispatch entry points:
//!
//! 1. [`EditorState::begin_instance_write`] swaps the effective display
//!    node (Ref id for a root, `refId__childId` for a virtual child;
//!    children stripped) into the Ref's tree slot — every existing
//!    mutator then "just works" against it.
//! 2. The host runs the unmodified mutator(s).
//! 3. [`EditorState::finish_instance_write`] diffs the display node's
//!    top-level JSON before/after and restores the original `RefNode`.
//!    Root changes route `INSTANCE_DIRECT_PROPS` onto the Ref base and
//!    everything else to `descendants[ref.target]`; child changes all
//!    route to `descendants[original child id]`.
//!
//! Host integration points (op-host-native): `apply_property_action`,
//! `commit_property_focus_if_any`, `commit_effect_param_focus_if_any`,
//! the color-picker HSV drag sites and the keyboard property-step
//! path. Mutators that push history inside the scope would snapshot
//! the swapped tree; `finish_instance_write` repairs any snapshot
//! captured during the scope by routing each captured display state back
//! onto the original Ref.

use crate::history::EditorSnapshot;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::{find_node, find_node_mut};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde_json::{Map, Value};

pub use crate::instance_child_override::{
    resolve_instance_display_node_for_anchor, split_instance_child_anchor,
};

/// Properties stored directly on the `RefNode` (instance-level), not
/// as overrides — mirrors TS `INSTANCE_DIRECT_PROPS`.
///
/// DELIBERATE DIVERGENCE: TS also lists `width` / `height`, but the
/// canonical Rust `RefNode` schema has no size fields (only
/// `PenNodeBase`), so size writes route through
/// `descendants[ref.target]` instead. They still render: the Rust
/// `expand_ref` applies the target's top-level overrides onto the
/// merged node (see `ref_resolve.rs`).
pub const INSTANCE_DIRECT_PROPS: &[&str] = &[
    "x", "y", "name", "visible", "locked", "rotation", "opacity", "flipX", "flipY", "enabled",
    "theme",
];

/// Keys that identify the node / route its overrides — never diffed,
/// never routed.
const STRUCTURAL_KEYS: &[&str] = &["id", "type", "ref", "descendants", "children", "reusable"];

/// Resolve the merged DISPLAY node for a selected instance — the TS
/// property-panel merge: component base → `descendants[ref.target]`
/// top-level overrides → the RefNode's own defined props. The result
/// keeps the REF's id (so selection still resolves), takes the
/// component's type, and carries the component subtree with child
/// overrides applied (original child ids — display only, never
/// persisted). Returns `None` for a dangling / non-Ref node.
pub fn resolve_instance_display_node(doc: &PenDocument, ref_node: &PenNode) -> Option<PenNode> {
    let PenNode::Ref(reference) = ref_node else {
        return None;
    };
    let component = crate::ref_resolve::find_component_node(doc, &reference.target)?;
    let component_value = serde_json::to_value(&component).ok()?;
    let ref_value = serde_json::to_value(ref_node).ok()?;
    let (Value::Object(component_map), Value::Object(ref_map)) = (component_value, ref_value)
    else {
        return None;
    };
    let mut merged = component_map.clone();
    // Top-level overrides live under the component's own id (TS
    // `descendants?.[node.ref]`).
    if let Some(top) = ref_map
        .get("descendants")
        .and_then(Value::as_object)
        .and_then(|d| d.get(reference.target.as_str()))
        .and_then(Value::as_object)
    {
        for (key, value) in top {
            if !matches!(key.as_str(), "id" | "type" | "children") {
                merged.insert(key.clone(), value.clone());
            }
        }
    }
    // The RefNode's own defined props win (id included — the display
    // node must answer to the instance's id).
    for (key, value) in &ref_map {
        if matches!(key.as_str(), "type" | "ref" | "descendants" | "children") {
            continue;
        }
        if !value.is_null() {
            merged.insert(key.clone(), value.clone());
        }
    }
    if let Some(t) = component_map.get("type") {
        merged.insert("type".into(), t.clone());
    }
    if merged.get("name").is_none_or(Value::is_null) {
        if let Some(name) = component_map.get("name") {
            merged.insert("name".into(), name.clone());
        }
    }
    // The display node is an instance view, not a component definition.
    merged.remove("reusable");
    // Children: the component subtree with `descendants` child
    // overrides applied at their ORIGINAL ids (display only). Mirror
    // `expand_ref`'s slot fallback when an empty master gets inline
    // instance children.
    let overrides = ref_map.get("descendants").and_then(Value::as_object);
    let children_source = match component_map.get("children") {
        Some(Value::Array(children)) if !children.is_empty() => Some(children),
        _ => ref_map.get("children").and_then(Value::as_array),
    };
    if let Some(children) = children_source {
        let with_overrides: Vec<Value> = children
            .iter()
            .map(|child| crate::ref_resolve::apply_overrides(child, overrides))
            .collect();
        merged.insert("children".into(), Value::Array(with_overrides));
    }
    serde_json::from_value(Value::Object(merged)).ok()
}

/// State carried across a swap–run–diff–restore instance write. See
/// the module docs for the protocol.
#[derive(Debug)]
pub struct InstanceWriteScope {
    ref_id: NodeId,
    display_id: NodeId,
    target_id: String,
    route_direct_props: bool,
    original_ref: PenNode,
    display_before: Map<String, Value>,
    history_push_count_before: u64,
}

impl EditorState {
    /// Begin an instance-write scope for the selection anchor when it
    /// is a resolvable `Ref` root or virtual instance child. `None`
    /// (no scope, run mutators as-is) for every other selection.
    pub fn begin_instance_write_for_anchor(&mut self) -> Option<InstanceWriteScope> {
        if self.instance_write_virtual_anchor.is_some() {
            return None;
        }
        let anchor = self.selection.anchor.clone();
        if !anchor.is_real() {
            return None;
        }
        self.begin_instance_write(&anchor)
    }

    /// Swap the effective display node for `anchor` into its authored
    /// Ref's tree slot so anchor-keyed panel mutators write into it. A
    /// virtual child keeps its virtual id; a Ref root keeps the Ref id.
    /// The display node is swapped in WITHOUT children so component ids
    /// never appear twice in the live tree. Must be paired with
    /// [`EditorState::finish_instance_write`].
    pub fn begin_instance_write(&mut self, anchor: &NodeId) -> Option<InstanceWriteScope> {
        let (ref_id, target_id, mut display, route_direct_props) =
            if let Some((ref_id, child_id)) = split_instance_child_anchor(anchor, &self.doc) {
                let ref_node = find_node(self.active_children(), &ref_id)?;
                let base = ref_node.base();
                if !matches!(ref_node, PenNode::Ref(_))
                    || !base.visible.unwrap_or(true)
                    || base.locked.unwrap_or(false)
                {
                    return None;
                }
                (
                    ref_id,
                    child_id.as_str().to_string(),
                    resolve_instance_display_node_for_anchor(&self.doc, anchor)?,
                    false,
                )
            } else {
                let node = find_node(self.active_children(), anchor)?;
                let PenNode::Ref(reference) = node else {
                    return None;
                };
                let base = node.base();
                if !base.visible.unwrap_or(true) || base.locked.unwrap_or(false) {
                    return None;
                }
                (
                    anchor.clone(),
                    reference.target.clone(),
                    resolve_instance_display_node(&self.doc, node)?,
                    true,
                )
            };
        let display_id = NodeId::new_opt(display.id_str())?;
        // Strip the subtree: panel writes only ever target the anchor
        // node's own props, and the component's original child ids
        // must not coexist with the live component subtree.
        if let Some(children) = display.children_mut() {
            children.clear();
        }
        let display_before = match serde_json::to_value(&display) {
            Ok(Value::Object(map)) => map,
            _ => return None,
        };
        let history_push_count_before = self.history_push_count;
        let slot = find_node_mut(self.active_children_mut(), &ref_id)?;
        let original_ref = std::mem::replace(slot, display);
        if !route_direct_props {
            self.instance_write_virtual_anchor = Some(display_id.clone());
        }
        Some(InstanceWriteScope {
            ref_id,
            display_id,
            target_id,
            route_direct_props,
            original_ref,
            display_before,
            history_push_count_before,
        })
    }

    /// Diff the display node against its pre-write state, restore the
    /// original `RefNode`, and route every changed top-level key:
    /// direct props onto the Ref base, everything else into
    /// `descendants[target]`. Returns true when any write was routed.
    pub fn finish_instance_write(&mut self, scope: InstanceWriteScope) -> bool {
        let routed = self.route_display_diff(&scope);
        if self.instance_write_virtual_anchor.as_ref() == Some(&scope.display_id) {
            self.instance_write_virtual_anchor = None;
        }
        self.repair_scope_snapshots(&scope);
        routed
    }

    fn route_display_diff(&mut self, scope: &InstanceWriteScope) -> bool {
        let Some(slot) = find_node_mut(self.active_children_mut(), &scope.display_id) else {
            // The node vanished during the scope (host-level delete);
            // nothing to restore into.
            return false;
        };
        let (updated, routed) = route_display_state(
            &scope.original_ref,
            &scope.target_id,
            scope.route_direct_props,
            &scope.display_before,
            slot,
        );
        *slot = updated;
        routed
    }

    /// Any history snapshot pushed DURING the scope captured the
    /// swapped display node in place of the Ref — undo would restore a
    /// silently-detached instance. Route each snapshot's own display state
    /// back into a Ref so compound edits preserve every intermediate state.
    fn repair_scope_snapshots(&mut self, scope: &InstanceWriteScope) {
        if !matches!(scope.original_ref, PenNode::Ref(_)) {
            return;
        }
        let pushes_in_scope = self
            .history_push_count
            .saturating_sub(scope.history_push_count_before);
        let repair_count = pushes_in_scope.min(self.history.past.len() as u64) as usize;
        for snap in self.history.past.iter_mut().rev().take(repair_count) {
            repair_scope_snapshot(
                snap,
                &scope.ref_id,
                &scope.display_id,
                &scope.target_id,
                scope.route_direct_props,
                &scope.original_ref,
                &scope.display_before,
            );
        }
        // Pending pre-edit snapshots (colour picker / text edit) taken
        // inside the scope carry the same contamination signature — a
        // non-Ref node at the instance id — and the same repair.
        if let Some(snap) = self.ui.pending_color_history.as_mut() {
            repair_scope_snapshot(
                snap,
                &scope.ref_id,
                &scope.display_id,
                &scope.target_id,
                scope.route_direct_props,
                &scope.original_ref,
                &scope.display_before,
            );
        }
        if let Some(snap) = self.ui.pending_text_edit_history.as_mut() {
            repair_scope_snapshot(
                snap,
                &scope.ref_id,
                &scope.display_id,
                &scope.target_id,
                scope.route_direct_props,
                &scope.original_ref,
                &scope.display_before,
            );
        }
    }
}

/// Route one display-node state onto the original Ref. This pure helper is
/// shared by the live scope finish and history-snapshot repair so both apply
/// identical direct-prop and descendants-override semantics.
fn route_display_state(
    original_ref: &PenNode,
    target_id: &str,
    route_direct_props: bool,
    before: &Map<String, Value>,
    display_after: &PenNode,
) -> (PenNode, bool) {
    let after = match serde_json::to_value(display_after) {
        Ok(Value::Object(map)) => map,
        _ => return (original_ref.clone(), false),
    };
    let mut direct: Vec<(String, Option<Value>)> = Vec::new();
    let mut overrides: Map<String, Value> = Map::new();
    let mut keys: Vec<&String> = before.keys().chain(after.keys()).collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        if STRUCTURAL_KEYS.contains(&key.as_str()) {
            continue;
        }
        let old = before.get(key);
        let new = after.get(key);
        if old == new {
            continue;
        }
        if route_direct_props && INSTANCE_DIRECT_PROPS.contains(&key.as_str()) {
            direct.push((key.clone(), new.cloned()));
        } else {
            // Removed keys persist as explicit nulls so the override can
            // clear a component value.
            overrides.insert(key.clone(), new.cloned().unwrap_or(Value::Null));
        }
    }
    if direct.is_empty() && overrides.is_empty() {
        return (original_ref.clone(), false);
    }
    let mut ref_map = match serde_json::to_value(original_ref) {
        Ok(Value::Object(map)) => map,
        _ => return (original_ref.clone(), false),
    };
    for (key, value) in direct {
        match value {
            Some(value) => {
                ref_map.insert(key, value);
            }
            None => {
                ref_map.remove(&key);
            }
        }
    }
    if !overrides.is_empty() {
        let descendants = ref_map
            .entry("descendants")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(descendants) = descendants {
            let entry = descendants
                .entry(target_id.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(existing) = entry {
                for (key, value) in overrides {
                    existing.insert(key, value);
                }
            }
        }
    }
    match serde_json::from_value::<PenNode>(Value::Object(ref_map)) {
        Ok(updated) if matches!(updated, PenNode::Ref(_)) => (updated, true),
        _ => (original_ref.clone(), false),
    }
}

fn repair_scope_snapshot(
    snapshot: &mut EditorSnapshot,
    ref_id: &NodeId,
    display_id: &NodeId,
    target_id: &str,
    route_direct_props: bool,
    pre_scope_ref: &PenNode,
    display_before: &Map<String, Value>,
) {
    let snapshot_node_id = if snapshot
        .doc
        .snapshot_find_node(snapshot.active_page_index, display_id)
        .is_some()
    {
        display_id
    } else if snapshot
        .doc
        .snapshot_find_node(snapshot.active_page_index, ref_id)
        .is_some()
    {
        ref_id
    } else {
        return;
    };
    let replacement = {
        let display_at_snapshot = snapshot
            .doc
            .snapshot_find_node(snapshot.active_page_index, snapshot_node_id)
            .expect("snapshot node checked above");
        if matches!(display_at_snapshot, PenNode::Ref(_)) {
            return;
        }
        route_display_state(
            pre_scope_ref,
            target_id,
            route_direct_props,
            display_before,
            display_at_snapshot,
        )
        .0
    };
    snapshot.doc.repair_swap(snapshot_node_id, &replacement);
}

/// Run `write` (the same prop-write the panel would do) routed as an
/// instance override on `ref_id`. Convenience wrapper over the
/// begin/finish scope — `None` when `ref_id` isn't a resolvable Ref.
pub fn apply_instance_override<R>(
    state: &mut EditorState,
    ref_id: &NodeId,
    write: impl FnOnce(&mut EditorState) -> R,
) -> Option<R> {
    let scope = state.begin_instance_write(ref_id)?;
    let result = write(state);
    state.finish_instance_write(scope);
    Some(result)
}

#[cfg(test)]
#[path = "instance_override_history_tests.rs"]
mod history_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ref_resolve::resolve_refs_for_canvas;

    const DOC: &str = r##"{
      "version":"0.8.0",
      "children":[
        {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#222222"}],
         "children":[
           {"type":"text","id":"title","name":"Title","content":"Hello"},
           {"type":"icon_font","id":"icon","name":"home","iconFontName":"home",
            "width":24,"height":24,"fill":[{"type":"solid","color":"#111111"}]},
           {"type":"rectangle","id":"surface","name":"Surface","width":80,"height":32,
            "cornerRadius":4}
         ]},
        {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
      ]
    }"##;

    fn state() -> EditorState {
        let doc = jian_ops_schema::load_str(DOC)
            .expect("fixture parses")
            .value;
        let mut s = EditorState::from_document(doc);
        s.set_single_selection(NodeId::new("inst1"));
        s
    }

    fn ref_node(state: &EditorState) -> &jian_ops_schema::node::RefNode {
        match find_node(state.active_children(), &NodeId::new("inst1")) {
            Some(PenNode::Ref(r)) => r,
            other => panic!("inst1 must stay a Ref, got {other:?}"),
        }
    }

    #[test]
    fn split_child_anchor_accepts_only_a_real_instance_descendant() {
        let s = state();
        assert_eq!(
            split_instance_child_anchor(&NodeId::new("inst1__title"), &s.doc),
            Some((NodeId::new("inst1"), NodeId::new("title")))
        );
        assert_eq!(
            split_instance_child_anchor(&NodeId::new("inst1__missing"), &s.doc),
            None,
            "a delimiter alone must not manufacture an editable child"
        );
    }

    #[test]
    fn child_display_merges_master_value_with_descendant_override() {
        let mut s = state();
        if let Some(PenNode::Ref(r)) = find_node_mut(s.active_children_mut(), &NodeId::new("inst1"))
        {
            r.descendants = Some(std::collections::BTreeMap::from([(
                "title".to_string(),
                serde_json::json!({"fill":[{"type":"solid","color":"#ff8800"}]}),
            )]));
        }
        let display =
            resolve_instance_display_node_for_anchor(&s.doc, &NodeId::new("inst1__title"))
                .expect("virtual child resolves for display");
        assert_eq!(display.id_str(), "inst1__title");
        let PenNode::Text(text) = &display else {
            panic!("title keeps the master's Text kind");
        };
        assert_eq!(
            text.content,
            jian_ops_schema::node::TextContent::Plain("Hello".into())
        );
        assert_eq!(
            crate::fills::first_solid_fill_hex(&display),
            Some("#ff8800")
        );
    }

    #[test]
    fn display_node_merges_component_base_overrides_and_instance_props() {
        let mut s = state();
        // Author a top-level fill override the panel merge must show.
        if let Some(PenNode::Ref(r)) = find_node_mut(s.active_children_mut(), &NodeId::new("inst1"))
        {
            r.descendants = Some(std::collections::BTreeMap::from([(
                "card".to_string(),
                serde_json::json!({"fill":[{"type":"solid","color":"#ff8800"}]}),
            )]));
        }
        let node = s.selected_node().unwrap();
        let display = resolve_instance_display_node(&s.doc, node).expect("resolves");
        let PenNode::Frame(frame) = &display else {
            panic!("display node takes the component's Frame type");
        };
        assert_eq!(frame.base.id, "inst1", "display answers to the ref id");
        assert_eq!(frame.base.x, Some(300.0), "instance position wins");
        assert_eq!(
            crate::fills::first_solid_fill_hex(&display),
            Some("#ff8800"),
            "descendants[target] top-level override wins over the component fill"
        );
        assert_eq!(frame.reusable, None, "display is not a definition");
    }

    #[test]
    fn fill_write_on_instance_routes_to_descendants_and_renders() {
        let mut s = state();
        let routed = apply_instance_override(&mut s, &NodeId::new("inst1"), |s| {
            s.set_selected_color(true, "#ff0000")
        });
        assert_eq!(routed, Some(true));
        let r = ref_node(&s);
        let over = r
            .descendants
            .as_ref()
            .and_then(|d| d.get("card"))
            .expect("override object under the target id");
        assert_eq!(
            over.pointer("/fill/0/color").and_then(|v| v.as_str()),
            Some("#ff0000")
        );
        // Render-time expansion reflects the override.
        let resolved = resolve_refs_for_canvas(&s.doc);
        let expanded = &resolved.children[1];
        assert_eq!(
            crate::fills::first_solid_fill_hex(expanded),
            Some("#ff0000"),
            "resolve_refs_for_canvas applies the top-level override"
        );
    }

    #[test]
    fn fill_write_on_instance_child_routes_to_child_override_and_renders() {
        let mut s = state();
        let child = NodeId::new("inst1__title");
        s.set_single_selection(child.clone());
        let routed =
            apply_instance_override(&mut s, &child, |s| s.set_selected_color(true, "#ff0000"));
        assert_eq!(routed, Some(true));
        let over = ref_node(&s)
            .descendants
            .as_ref()
            .and_then(|d| d.get("title"))
            .expect("override object under the original child id");
        assert_eq!(
            over.pointer("/fill/0/color").and_then(Value::as_str),
            Some("#ff0000")
        );
        let resolved = resolve_refs_for_canvas(&s.doc);
        let expanded_child = find_node(&resolved.children, &child).expect("expanded child");
        assert_eq!(
            crate::fills::first_solid_fill_hex(expanded_child),
            Some("#ff0000")
        );
    }

    #[test]
    fn stroke_width_write_on_instance_child_routes_and_renders() {
        let mut s = state();
        let child = NodeId::new("inst1__icon");
        s.set_single_selection(child.clone());
        assert!(s.commit_property_edit(crate::PropertyFocus::StrokeWidth, 5.0));
        let over = ref_node(&s)
            .descendants
            .as_ref()
            .and_then(|d| d.get("icon"))
            .expect("stroke override under the original child id");
        assert_eq!(
            over.pointer("/stroke/thickness").and_then(Value::as_f64),
            Some(5.0)
        );
        let resolved = resolve_refs_for_canvas(&s.doc);
        let expanded_child = find_node(&resolved.children, &child).expect("expanded child");
        assert_eq!(crate::fills::node_stroke_width(expanded_child), Some(5.0));
    }

    #[test]
    fn corner_radius_write_on_instance_child_routes_through_position_r() {
        let mut s = state();
        let child = NodeId::new("inst1__surface");
        s.set_single_selection(child);
        assert!(s.commit_property_edit(crate::PropertyFocus::PositionR, 12.0));
        assert_eq!(
            ref_node(&s)
                .descendants
                .as_ref()
                .and_then(|d| d.get("surface"))
                .and_then(|v| v.get("cornerRadius"))
                .and_then(Value::as_f64),
            Some(12.0)
        );
    }

    #[test]
    fn locked_instance_rejects_virtual_child_write() {
        let mut s = state();
        if let Some(PenNode::Ref(r)) = find_node_mut(s.active_children_mut(), &NodeId::new("inst1"))
        {
            r.base.locked = Some(true);
        }
        let before = ref_node(&s).clone();
        let child = NodeId::new("inst1__icon");
        s.set_single_selection(child.clone());
        assert!(
            !s.set_selected_color(true, "#ff0000"),
            "the locked Ref must reject child writes"
        );
        assert_eq!(ref_node(&s), &before);
    }

    #[test]
    fn locked_master_child_is_editable_when_instance_is_unlocked() {
        let mut s = state();
        let icon = NodeId::new("icon");
        find_node_mut(s.active_children_mut(), &icon)
            .expect("master icon")
            .base_mut()
            .locked = Some(true);
        let child = NodeId::new("inst1__icon");
        s.set_single_selection(child.clone());
        assert!(s.set_selected_color(true, "#ff0000"));
        assert_eq!(
            ref_node(&s)
                .descendants
                .as_ref()
                .and_then(|d| d.get("icon"))
                .and_then(|v| v.pointer("/fill/0/color"))
                .and_then(Value::as_str),
            Some("#ff0000")
        );
    }

    #[test]
    fn virtual_child_is_editable_and_keeps_the_property_panel_visible() {
        let mut s = state();
        let child = NodeId::new("inst1__icon");
        s.set_single_selection(child.clone());
        assert!(s.is_editable(&child));
        assert!(s.property_panel_visible());
    }

    #[test]
    fn position_write_on_instance_lands_direct_on_the_ref() {
        let mut s = state();
        let routed = apply_instance_override(&mut s, &NodeId::new("inst1"), |s| {
            s.commit_property_edit(crate::PropertyFocus::PositionX, 400.0)
        });
        assert_eq!(routed, Some(true));
        let r = ref_node(&s);
        assert_eq!(r.base.x, Some(400.0), "x is an INSTANCE_DIRECT_PROP");
        assert!(
            r.descendants.is_none(),
            "no override written for a direct prop"
        );
    }

    #[test]
    fn size_write_routes_to_descendants_schema_divergence() {
        // TS lists width in INSTANCE_DIRECT_PROPS, but the Rust
        // RefNode schema has no width field — size routes through the
        // descendants override and still renders (documented in
        // `INSTANCE_DIRECT_PROPS`).
        let mut s = state();
        let routed = apply_instance_override(&mut s, &NodeId::new("inst1"), |s| {
            s.commit_property_edit(crate::PropertyFocus::SizeW, 320.0)
        });
        assert_eq!(routed, Some(true));
        let r = ref_node(&s);
        let over = r
            .descendants
            .as_ref()
            .and_then(|d| d.get("card"))
            .expect("size override under target id");
        assert_eq!(over.pointer("/width").and_then(|v| v.as_f64()), Some(320.0));
        let resolved = resolve_refs_for_canvas(&s.doc);
        let PenNode::Frame(expanded) = &resolved.children[1] else {
            panic!("frame");
        };
        assert_eq!(
            expanded.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(320.0)),
            "the size override renders"
        );
    }

    #[test]
    fn child_scope_history_is_repaired_to_hold_the_ref() {
        let mut s = state();
        let child = NodeId::new("inst1__icon");
        s.set_single_selection(child.clone());
        apply_instance_override(&mut s, &child, |s| {
            s.commit_history();
            s.set_selected_color(true, "#00ff00")
        });
        let snap = s.history.past.back().expect("history entry pushed");
        let in_snap = snap
            .doc
            .snapshot_find_node(0, &NodeId::new("inst1"))
            .expect("authored Ref id restored in snapshot");
        assert!(matches!(in_snap, PenNode::Ref(_)));
        assert!(
            snap.doc.snapshot_find_node(0, &child).is_none(),
            "the virtual display node must not leak into history"
        );
    }

    #[test]
    fn no_op_write_restores_the_ref_untouched() {
        let mut s = state();
        let before = ref_node(&s).clone();
        let routed = apply_instance_override(&mut s, &NodeId::new("inst1"), |_| {});
        assert_eq!(routed, Some(()));
        assert_eq!(ref_node(&s), &before, "no diff → ref restored verbatim");
    }

    #[test]
    fn detach_instance_materializes_top_level_overrides() {
        let mut s = state();
        apply_instance_override(&mut s, &NodeId::new("inst1"), |s| {
            s.set_selected_color(true, "#ff0000")
        });
        let new_id = s.detach_component(&NodeId::new("inst1")).expect("detaches");
        let detached = find_node(s.active_children(), &new_id).expect("detached node");
        assert!(matches!(detached, PenNode::Frame(_)));
        assert_eq!(
            crate::fills::first_solid_fill_hex(detached),
            Some("#ff0000"),
            "TS detach applies descendants[ref] onto the materialized tree"
        );
    }
}
