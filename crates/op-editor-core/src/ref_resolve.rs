//! Render-time component-instance expansion — the canonical port of
//! `pen-renderer/src/document-flattener.ts::resolveRefs` /
//! `remapIds`.
//!
//! A `PenNode::Ref` renders as its target component's subtree:
//! instance-level props override the component's (position, size,
//! name, …), every descendant id is remapped to the virtual
//! `refId__childId` space, and `descendants` overrides are spread
//! onto the matching children. The scene builder runs this pass
//! BEFORE `$variable` resolution so expanded subtrees resolve their
//! tokens too.
//!
//! The instance/component prop merge uses `serde_json::Value` object
//! spreads — byte-for-byte the TS semantics (`{ ...component,
//! ...definedRefProps }` and `{ ...child, ...override }`), with no
//! per-field porting drift.
//!
//! DELIBERATE DIVERGENCE from TS: nested instances inside an
//! expanded component expand recursively here (TS stops after one
//! `remapIds` pass, leaving inner Ref children unrendered). A
//! 2-level instance therefore yields `outer__inner__leaf` virtual
//! ids that have no TS counterpart — virtual ids are render-only
//! and never persisted, so the extra depth only ever ADDS rendered
//! content.

use std::collections::BTreeSet;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde_json::{Map, Value};

/// Expand every `Ref` node in `doc` into its resolved component
/// subtree. Unresolvable refs (unknown target, cycle) are dropped —
/// TS `resolveRefs` returns `[]` for them.
pub fn resolve_refs_for_canvas(doc: &PenDocument) -> PenDocument {
    let mut resolved = doc.clone();
    // Component lookup runs against the WHOLE document (any page),
    // so cross-page instances resolve like `instantiate_component`
    // allows authoring them. `resolve_nodes` only ever READS through
    // `lookup` (find_node / expand_ref never mutate it), so the
    // original `doc` borrow serves as the lookup source directly —
    // it is a distinct object from `resolved`, which is built fresh
    // into `out` vecs rather than mutated in place, so there is no
    // borrow conflict and no need for a second whole-document clone.
    let mut visited = BTreeSet::new();
    if let Some(pages) = resolved.pages.as_mut() {
        for page in pages {
            page.children = resolve_nodes(&page.children, doc, &mut visited);
        }
    }
    resolved.children = resolve_nodes(&resolved.children, doc, &mut visited);
    resolved
}

/// True when any `Ref` node exists anywhere in the document — the
/// scene builder's early-out so ref-free documents (the common case)
/// skip the expansion pass's full-tree clone.
pub fn document_has_refs(doc: &PenDocument) -> bool {
    fn walk(nodes: &[PenNode]) -> bool {
        nodes.iter().any(|node| {
            matches!(node, PenNode::Ref(_))
                || crate::pen_node_ext::PenNodeExt::children(node).is_some_and(|c| walk(c))
        })
    }
    doc.pages
        .as_ref()
        .is_some_and(|pages| pages.iter().any(|p| walk(&p.children)))
        || walk(&doc.children)
}

fn resolve_nodes(
    nodes: &[PenNode],
    lookup: &PenDocument,
    visited: &mut BTreeSet<String>,
) -> Vec<PenNode> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            PenNode::Ref(reference) => {
                if visited.contains(&reference.target) {
                    continue; // Cycle guard — drop like TS.
                }
                let Some(component) = find_node(lookup, &reference.target) else {
                    continue; // Unknown target — drop like TS.
                };
                visited.insert(reference.target.clone());
                if let Some(resolved) = expand_ref(node, &component, lookup, visited) {
                    out.push(resolved);
                }
                visited.remove(&reference.target);
            }
            other => {
                let mut clone = other.clone();
                if let Some(children) = crate::pen_node_ext::PenNodeExt::children_mut(&mut clone) {
                    let resolved = resolve_nodes(children, lookup, visited);
                    *children = resolved;
                }
                out.push(clone);
            }
        }
    }
    out
}

/// Merge `component` under the instance's defined props and remap the
/// component's children into the instance's virtual id space.
fn expand_ref(
    ref_node: &PenNode,
    component: &PenNode,
    lookup: &PenDocument,
    visited: &mut BTreeSet<String>,
) -> Option<PenNode> {
    let component_value = serde_json::to_value(component).ok()?;
    let ref_value = serde_json::to_value(ref_node).ok()?;
    let (Value::Object(component_map), Value::Object(ref_map)) = (component_value, ref_value)
    else {
        return None;
    };

    let mut merged = component_map.clone();
    // `descendants[ref.target]` carries the instance's TOP-LEVEL
    // overrides (fill / stroke / effects / size, written by the
    // property panel's override router). TS's render flattener never
    // applies them (panel + detach do) — applying them here is a
    // DELIBERATE DIVERGENCE so an override-edited instance renders
    // exactly what the inspector shows.
    apply_top_level_overrides(&mut merged, &ref_map);
    for (key, value) in &ref_map {
        // `type`/`ref` identify the instance, `descendants` routes
        // child overrides, `children` (slot content) never replaces
        // the component subtree (TS skips it too).
        if matches!(key.as_str(), "type" | "ref" | "descendants" | "children") {
            continue;
        }
        if !value.is_null() {
            merged.insert(key.clone(), value.clone());
        }
    }
    // Keep the component's type; the instance cannot change it.
    if let Some(t) = component_map.get("type") {
        merged.insert("type".into(), t.clone());
    }
    // Name falls back to the component's when the instance has none.
    if merged.get("name").is_none_or(Value::is_null) {
        if let Some(name) = component_map.get("name") {
            merged.insert("name".into(), name.clone());
        }
    }
    // An expanded instance is not itself a component definition.
    merged.remove("reusable");

    // Children: clone the component's subtree into the instance's
    // virtual id space with `descendants` overrides applied.
    let ref_id = ref_map
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let overrides = ref_map.get("descendants").and_then(Value::as_object);
    // Pencil's slot-component pattern: the master is an empty styled
    // shell carrying a `slot:[…]` id list, and each instance supplies
    // its real content as its OWN inline `children`. When the master
    // has no children to render, fall back to the instance's inline
    // slot content so the shell isn't rendered empty (an empty
    // `fit_content` frame collapses to its padding box → a white
    // circle for a high-`cornerRadius` card). The master-has-children
    // branch is unchanged, so the 115 normal refs and the master-with-
    // default-content (ambiguous) refs keep TS-parity behaviour.
    let children_source = match component_map.get("children") {
        Some(Value::Array(children)) if !children.is_empty() => Some(children),
        _ => ref_map.get("children").and_then(Value::as_array),
    };
    if let Some(children) = children_source {
        let remapped: Vec<Value> = children
            .iter()
            .map(|child| remap_ids(child, ref_id, overrides))
            .collect();
        merged.insert("children".into(), Value::Array(remapped));
    }

    // A failed re-parse must not silently erase the instance from the
    // render output — fall back to the component subtree positioned at
    // the instance (no prop overlay), which still shows the content.
    let mut expanded: PenNode = match serde_json::from_value(Value::Object(merged)) {
        Ok(node) => node,
        Err(_) => {
            let mut fallback = component.clone();
            let ref_base = crate::pen_node_ext::PenNodeExt::base(ref_node);
            let ref_id = ref_base.id.clone();
            let (x, y) = (ref_base.x, ref_base.y);
            let base = crate::pen_node_ext::PenNodeExt::base_mut(&mut fallback);
            base.id = ref_id.clone();
            base.x = x;
            base.y = y;
            // The fallback bypassed `remap_ids`, so push its children
            // into the instance's virtual id space by hand — component
            // ids must never appear twice in one scene.
            remap_ids_typed(&mut fallback, &ref_id);
            fallback
        }
    };
    // Nested instances inside the component expand recursively (the
    // visited set still holds this target, so self-cycles drop).
    if let Some(children) = crate::pen_node_ext::PenNodeExt::children_mut(&mut expanded) {
        let resolved = resolve_nodes(children, lookup, visited);
        *children = resolved;
    }
    Some(expanded)
}

/// Typed mirror of [`remap_ids`] for the serde-failure fallback:
/// every DESCENDANT id becomes `refId__originalId` (flat prefix at
/// all depths, like TS). The root keeps the instance id the caller
/// already wrote.
fn remap_ids_typed(node: &mut PenNode, ref_id: &str) {
    if let Some(children) = crate::pen_node_ext::PenNodeExt::children_mut(node) {
        for child in children {
            let original = crate::pen_node_ext::PenNodeExt::base(child).id.clone();
            crate::pen_node_ext::PenNodeExt::base_mut(child).id = format!("{ref_id}__{original}");
            remap_ids_typed(child, ref_id);
        }
    }
}

/// TS `remapIds`: virtual id `refId__childId`, `descendants`
/// overrides spread onto the matching ORIGINAL child id, recursing
/// with the same override map.
fn remap_ids(child: &Value, ref_id: &str, overrides: Option<&Map<String, Value>>) -> Value {
    let Value::Object(child_map) = child else {
        return child.clone();
    };
    let original_id = child_map
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut mapped = child_map.clone();
    if let Some(ov) = overrides
        .and_then(|o| o.get(original_id))
        .and_then(Value::as_object)
    {
        for (key, value) in ov {
            mapped.insert(key.clone(), value.clone());
        }
    }
    mapped.insert(
        "id".into(),
        Value::String(format!("{ref_id}__{original_id}")),
    );
    if let Some(Value::Array(children)) = mapped.clone().get("children") {
        let remapped: Vec<Value> = children
            .iter()
            .map(|c| remap_ids(c, ref_id, overrides))
            .collect();
        mapped.insert("children".into(), Value::Array(remapped));
    }
    Value::Object(mapped)
}

/// Materialize an instance into an independent subtree — the TS
/// `detachComponent` case-2 merge: component props under the
/// instance's defined props, `descendants` overrides applied to the
/// matching children (recursively, by original id), `reusable`
/// dropped, ids left UNTOUCHED (the caller re-mints them via
/// `deep_clone_with_new_ids` so the detached tree gets real ids,
/// not virtual `refId__childId` ones).
pub(crate) fn materialize_instance(ref_node: &PenNode, component: &PenNode) -> Option<PenNode> {
    let component_value = serde_json::to_value(component).ok()?;
    let ref_value = serde_json::to_value(ref_node).ok()?;
    let (Value::Object(component_map), Value::Object(ref_map)) = (component_value, ref_value)
    else {
        return None;
    };
    let mut merged = component_map.clone();
    // TS detach (case 2) applies `descendants[node.ref]` onto the
    // source BEFORE the instance's direct props: top-level overrides
    // materialize into the detached tree.
    apply_top_level_overrides(&mut merged, &ref_map);
    for (key, value) in &ref_map {
        if matches!(
            key.as_str(),
            "type" | "ref" | "descendants" | "children" | "id"
        ) {
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
    merged.remove("reusable");
    let overrides = ref_map.get("descendants").and_then(Value::as_object);
    if let Some(Value::Array(children)) = component_map.get("children") {
        let with_overrides: Vec<Value> = children
            .iter()
            .map(|child| apply_overrides(child, overrides))
            .collect();
        merged.insert("children".into(), Value::Array(with_overrides));
    }
    serde_json::from_value(Value::Object(merged)).ok()
}

/// Spread the instance's `descendants[ref.target]` object onto a
/// merged top-level map — the TS `{ ...component, ...topOverrides }`
/// panel/detach semantics. Identity / structural keys never move.
fn apply_top_level_overrides(merged: &mut Map<String, Value>, ref_map: &Map<String, Value>) {
    let Some(target) = ref_map.get("ref").and_then(Value::as_str) else {
        return;
    };
    let Some(top) = ref_map
        .get("descendants")
        .and_then(Value::as_object)
        .and_then(|d| d.get(target))
        .and_then(Value::as_object)
    else {
        return;
    };
    for (key, value) in top {
        if !matches!(key.as_str(), "id" | "type" | "children") {
            merged.insert(key.clone(), value.clone());
        }
    }
}

/// Spread `descendants` overrides onto matching children by their
/// ORIGINAL ids, recursively — `remap_ids` minus the id rewrite.
/// `pub(crate)` so the inspector's merged-display resolver
/// (`instance_override.rs`) reuses the exact same child-merge.
pub(crate) fn apply_overrides(child: &Value, overrides: Option<&Map<String, Value>>) -> Value {
    let Value::Object(child_map) = child else {
        return child.clone();
    };
    let original_id = child_map
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut mapped = child_map.clone();
    if let Some(ov) = overrides
        .and_then(|o| o.get(original_id))
        .and_then(Value::as_object)
    {
        for (key, value) in ov {
            mapped.insert(key.clone(), value.clone());
        }
    }
    if let Some(Value::Array(children)) = mapped.clone().get("children") {
        let walked: Vec<Value> = children
            .iter()
            .map(|c| apply_overrides(c, overrides))
            .collect();
        mapped.insert("children".into(), Value::Array(walked));
    }
    Value::Object(mapped)
}

/// Document-wide component lookup for instance flows (detach,
/// inspector merge).
pub(crate) fn find_component_node(doc: &PenDocument, id: &str) -> Option<PenNode> {
    find_node(doc, id)
}

/// Find a node by id anywhere in the document (pages + fallback
/// children) — TS `findNodeInTree` against the full root set.
fn find_node(doc: &PenDocument, id: &str) -> Option<PenNode> {
    fn walk(nodes: &[PenNode], id: &str) -> Option<PenNode> {
        for node in nodes {
            if crate::pen_node_ext::PenNodeExt::base(node).id == id {
                return Some(node.clone());
            }
            if let Some(children) = crate::pen_node_ext::PenNodeExt::children(node) {
                if let Some(hit) = walk(children, id) {
                    return Some(hit);
                }
            }
        }
        None
    }
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(hit) = walk(&page.children, id) {
                return Some(hit);
            }
        }
    }
    walk(&doc.children, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_from(json: &str) -> PenDocument {
        jian_ops_schema::load_str(json)
            .expect("fixture parses")
            .value
    }

    const DOC_WITH_INSTANCE: &str = r##"{
      "version":"0.8.0",
      "children":[
        {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#222222"}],
         "children":[
           {"type":"text","id":"title","name":"Title","content":"Hello"},
           {"type":"rectangle","id":"badge","width":20,"height":20,
            "fill":[{"type":"solid","color":"#ff0000"}]}
         ]},
        {"type":"ref","id":"inst1","ref":"card","x":300,"y":50,
         "descendants":{"title":{"content":"World"}}}
      ]
    }"##;

    #[test]
    fn ref_expands_to_component_subtree_with_instance_position() {
        let doc = doc_from(DOC_WITH_INSTANCE);
        let resolved = resolve_refs_for_canvas(&doc);
        assert_eq!(resolved.children.len(), 2);
        let PenNode::Frame(inst) = &resolved.children[1] else {
            panic!("instance expands to the component's Frame type");
        };
        assert_eq!(inst.base.id, "inst1");
        assert_eq!(inst.base.x, Some(300.0));
        assert_eq!(inst.base.y, Some(50.0));
        assert_eq!(
            inst.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(200.0)),
            "size inherits the component"
        );
        assert_eq!(inst.reusable, None, "expanded instance is not reusable");
        let children = inst.children.as_ref().expect("subtree expands");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn descendant_ids_are_remapped_and_overrides_applied() {
        let doc = doc_from(DOC_WITH_INSTANCE);
        let resolved = resolve_refs_for_canvas(&doc);
        let PenNode::Frame(inst) = &resolved.children[1] else {
            panic!("frame");
        };
        let children = inst.children.as_ref().unwrap();
        let PenNode::Text(title) = &children[0] else {
            panic!("text child survives");
        };
        assert_eq!(title.base.id, "inst1__title");
        assert_eq!(
            title.content,
            jian_ops_schema::node::text::TextContent::Plain("World".into()),
            "descendants override lands on the matching child"
        );
        let PenNode::Rectangle(badge) = &children[1] else {
            panic!("rect child survives");
        };
        assert_eq!(badge.base.id, "inst1__badge");
    }

    #[test]
    fn unknown_target_and_cycles_drop_instead_of_panicking() {
        let doc = doc_from(
            r##"{
              "version":"0.8.0",
              "children":[
                {"type":"ref","id":"dangling","ref":"missing"},
                {"type":"frame","id":"a","name":"A","reusable":true,
                 "children":[{"type":"ref","id":"loop","ref":"a"}]}
              ]
            }"##,
        );
        let resolved = resolve_refs_for_canvas(&doc);
        // The dangling ref is dropped. The self-recursive component
        // expands ONE level (its inner ref to itself is the cycle the
        // visited set breaks): `a` keeps the expanded instance, and
        // that instance's own cyclic ref child is dropped.
        assert_eq!(resolved.children.len(), 1);
        let PenNode::Frame(a) = &resolved.children[0] else {
            panic!("frame");
        };
        let a_children = a.children.as_ref().expect("a keeps its children");
        assert_eq!(a_children.len(), 1, "the loop instance expanded once");
        let PenNode::Frame(expanded) = &a_children[0] else {
            panic!("expanded instance is a frame");
        };
        assert_eq!(expanded.base.id, "loop");
        assert_eq!(
            expanded.children.as_ref().map(Vec::len),
            Some(0),
            "the cyclic inner ref is dropped"
        );
    }

    // Pencil slot-component pattern: the master is an empty styled
    // shell (carrying a `slot` id list, no children of its own) and
    // the instance supplies its real content as its OWN inline
    // `children`. Without the fallback the expanded instance renders
    // empty (a high-`cornerRadius` `fit_content` card collapses to a
    // white circle).
    const DOC_WITH_SLOT_INSTANCE: &str = r##"{
      "version":"0.8.0",
      "children":[
        {"type":"frame","id":"cardMaster","name":"Card/Default","reusable":true,
         "width":"fit_content","height":"fit_content","cornerRadius":24,"padding":16,
         "slot":["regionA","regionB"],
         "fill":[{"type":"solid","color":"#f5f5f5"}]},
        {"type":"ref","id":"card1","ref":"cardMaster","x":40,"y":40,
         "children":[
           {"type":"text","id":"label","name":"Label","content":"Revenue"},
           {"type":"text","id":"value","name":"Value","content":"$12.4k"}
         ]}
      ]
    }"##;

    #[test]
    fn empty_master_falls_back_to_instance_slot_children() {
        let doc = doc_from(DOC_WITH_SLOT_INSTANCE);
        let resolved = resolve_refs_for_canvas(&doc);
        assert_eq!(resolved.children.len(), 2);
        let PenNode::Frame(card) = &resolved.children[1] else {
            panic!("slot instance expands to the master's Frame type");
        };
        assert_eq!(card.base.id, "card1");
        // The master shell's style is preserved (still the white card).
        assert_eq!(card.base.x, Some(40.0));
        let children = card
            .children
            .as_ref()
            .expect("instance slot content fills the empty master shell");
        assert_eq!(
            children.len(),
            2,
            "instance's own inline children render instead of an empty shell"
        );
        let PenNode::Text(label) = &children[0] else {
            panic!("first slot child survives");
        };
        // Slot children are remapped into the instance's virtual id
        // space exactly like master children would be.
        assert_eq!(label.base.id, "card1__label");
        assert_eq!(
            label.content,
            jian_ops_schema::node::text::TextContent::Plain("Revenue".into())
        );
    }

    #[test]
    fn master_children_win_over_instance_children_no_regression() {
        // A master WITH its own children keeps TS-parity behaviour:
        // the instance's inline children never replace the master
        // subtree (this guards the 115 normal + ambiguous refs against
        // the slot-fallback path).
        let doc = doc_from(
            r##"{
              "version":"0.8.0",
              "children":[
                {"type":"frame","id":"master","name":"Master","reusable":true,
                 "width":200,"height":100,
                 "children":[{"type":"text","id":"masterText","name":"M","content":"FromMaster"}]},
                {"type":"ref","id":"inst","ref":"master","x":10,"y":10,
                 "children":[{"type":"text","id":"instText","name":"I","content":"FromInstance"}]}
              ]
            }"##,
        );
        let resolved = resolve_refs_for_canvas(&doc);
        let PenNode::Frame(inst) = &resolved.children[1] else {
            panic!("frame");
        };
        let children = inst.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        let PenNode::Text(text) = &children[0] else {
            panic!("master text wins");
        };
        assert_eq!(text.base.id, "inst__masterText");
        assert_eq!(
            text.content,
            jian_ops_schema::node::text::TextContent::Plain("FromMaster".into()),
            "instance inline children do not override a master that has children"
        );
    }
}
