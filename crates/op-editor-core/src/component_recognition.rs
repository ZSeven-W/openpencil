//! Repeated-structure → component recognition for imported designs.
//!
//! An imported web page is a flat copy of its DOM: three feature cards are
//! three unrelated frame trees, so changing the card's radius means editing
//! it three times. This pass finds frames whose subtrees are *structurally
//! identical* — same node types, same layout, same styles, same geometry —
//! and differ only in their content (text, image source, layer names). The
//! first becomes a reusable component master (`reusable: true`, in place);
//! every other one becomes a `ref` instance of it carrying its own content
//! as `descendants` overrides.
//!
//! Conservative by construction:
//!
//! - Only fields that are *content* may differ between members: a
//!   descendant's `content`, `src`, `alt`, `href` and `name`. Every other
//!   field — including which content fields are present at all — must be
//!   equal, or the two frames are not grouped.
//! - Every instance is **proven** before it replaces the original: the ref
//!   is expanded through the same resolver the canvas renders with
//!   ([`crate::ref_resolve`]) and the result must equal the original subtree
//!   (ids aside). An instance that does not round-trip is left untouched.
//! - Candidates are frames below a page root with some content (text or
//!   image), no ref or reusable frame inside, and either a visible box of
//!   their own (fill, stroke, radius, effects) or two or more children —
//!   a bare wrapper around one child is plumbing, not a part.
//!   Nested matches are not stacked: the largest structures claim their
//!   whole subtree first, so a card's own button is never a second master
//!   inside the card master.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde_json::{Map, Value};

use crate::pen_node_ext::PenNodeExt;

/// Descendant fields that carry an instance's own content. A member may
/// differ from the master only in these (its value is then an override).
pub const CONTENT_FIELDS: [&str; 5] = ["content", "src", "alt", "href", "name"];

/// Root fields ignored when grouping: identity and placement.
const ROOT_IGNORED: [&str; 4] = ["id", "name", "x", "y"];

/// Subtrees larger than this are never componentized — a whole repeated
/// page section is a layout, not a reusable part.
pub const MAX_COMPONENT_NODES: usize = 400;

/// What kind of part a recognized component is, from its structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    /// A painted, rounded or outlined box with one short label.
    Button,
    /// A painted / outlined container holding two or more pieces of content.
    Card,
    /// A list item (`li`).
    ListItem,
    /// Anything else that repeats.
    Item,
}

impl ComponentKind {
    /// Default layer / component name.
    pub fn default_name(self) -> &'static str {
        match self {
            ComponentKind::Button => "Button",
            ComponentKind::Card => "Card",
            ComponentKind::ListItem => "List item",
            ComponentKind::Item => "Item",
        }
    }
}

/// One component the pass created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognizedComponent {
    /// The master frame (kept in place, now `reusable`).
    pub master_id: String,
    pub name: String,
    pub kind: ComponentKind,
    /// Former copies, now `ref` instances of the master (same ids).
    pub instance_ids: Vec<String>,
}

/// Find repeated structures under `roots` (a page's top-level nodes) and
/// turn them into one master + ref instances each. Returns what was
/// created; an empty list means the tree is unchanged.
pub fn componentize_repeated_structures(roots: &mut [PenNode]) -> Vec<RecognizedComponent> {
    let mut candidates = Vec::new();
    let mut preorder = 0usize;
    for root in roots.iter() {
        let Ok(value) = serde_json::to_value(root) else {
            continue;
        };
        visit(&value, 0, &mut preorder, &mut candidates);
    }
    let groups = select_groups(candidates);
    let mut created = Vec::new();
    let mut used_names: HashMap<&'static str, usize> = HashMap::new();
    for group in groups {
        if let Some(component) = convert_group(roots, &group, &mut used_names) {
            created.push(component);
        }
    }
    created
}

/// A frame that may become part of a component group.
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    signature: u64,
    /// Preorder index of the node; its subtree spans `[pre, pre + size)`.
    pre: usize,
    size: usize,
}

/// What a visited subtree reports up to its parent.
struct Visit {
    /// Hash of the subtree as a descendant (content fields keyed only).
    hash: u64,
    size: usize,
    /// A ref or a reusable frame inside (or at) this node.
    blocked: bool,
    /// A text or image inside (or at) this node.
    has_content: bool,
}

fn visit(value: &Value, depth: usize, preorder: &mut usize, out: &mut Vec<Candidate>) -> Visit {
    let pre = *preorder;
    *preorder += 1;
    let Some(object) = value.as_object() else {
        return Visit {
            hash: 0,
            size: 1,
            blocked: true,
            has_content: false,
        };
    };
    let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
    let mut blocked =
        kind == "ref" || object.get("reusable").and_then(Value::as_bool) == Some(true);
    let mut has_content = matches!(kind, "text" | "image");
    let mut size = 1usize;
    let mut child_hashes = Vec::new();
    let painted = ["fill", "stroke", "cornerRadius", "effects"]
        .iter()
        .any(|key| object.get(*key).is_some_and(|v| !v.is_null()));
    if let Some(children) = object.get("children").and_then(Value::as_array) {
        for child in children {
            let report = visit(child, depth + 1, preorder, out);
            size += report.size;
            blocked |= report.blocked;
            has_content |= report.has_content;
            child_hashes.push(report.hash);
        }
    }
    let own_hash = |ignored: &[&str], keyed_only: &[&str]| {
        let mut hasher = DefaultHasher::new();
        for (key, field) in sorted(object) {
            if key == "children" || ignored.contains(&key) {
                continue;
            }
            key.hash(&mut hasher);
            if !keyed_only.contains(&key) {
                hash_value(field, &mut hasher);
            }
        }
        child_hashes.len().hash(&mut hasher);
        child_hashes.hash(&mut hasher);
        hasher.finish()
    };
    let descendant_hash = own_hash(&["id"], &CONTENT_FIELDS);
    let eligible = kind == "frame"
        && depth >= 1
        && !blocked
        && has_content
        && (2..=MAX_COMPONENT_NODES).contains(&size)
        // A bare wrapper around one child (the importer's `h3` / margin
        // frames) is plumbing, not a part.
        && (painted || child_hashes.len() >= 2);
    if eligible {
        if let Some(id) = object.get("id").and_then(Value::as_str) {
            out.push(Candidate {
                id: id.to_string(),
                signature: own_hash(&ROOT_IGNORED, &[]),
                pre,
                size,
            });
        }
    }
    Visit {
        hash: descendant_hash,
        size,
        blocked,
        has_content,
    }
}

fn sorted(object: &Map<String, Value>) -> Vec<(&str, &Value)> {
    let mut entries: Vec<_> = object.iter().map(|(k, v)| (k.as_str(), v)).collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// Order-independent-of-map-implementation hash of a JSON value.
fn hash_value(value: &Value, hasher: &mut DefaultHasher) {
    match value {
        Value::Null => 0u8.hash(hasher),
        Value::Bool(b) => (1u8, b).hash(hasher),
        Value::Number(n) => (2u8, n.to_string()).hash(hasher),
        Value::String(s) => (3u8, s).hash(hasher),
        Value::Array(items) => {
            (4u8, items.len()).hash(hasher);
            for item in items {
                hash_value(item, hasher);
            }
        }
        Value::Object(object) => {
            (5u8, object.len()).hash(hasher);
            for (key, field) in sorted(object) {
                key.hash(hasher);
                hash_value(field, hasher);
            }
        }
    }
}

/// Group candidates by signature and pick non-overlapping groups, largest
/// structures first. Each returned group lists member ids in document
/// order; the first is the master.
fn select_groups(candidates: Vec<Candidate>) -> Vec<Vec<String>> {
    let mut by_signature: HashMap<u64, Vec<Candidate>> = HashMap::new();
    for candidate in candidates {
        by_signature
            .entry(candidate.signature)
            .or_default()
            .push(candidate);
    }
    let mut groups: Vec<Vec<Candidate>> = by_signature
        .into_values()
        .filter(|members| members.len() >= 2)
        .collect();
    for members in &mut groups {
        members.sort_by_key(|member| member.pre);
    }
    groups.sort_by(|a, b| b[0].size.cmp(&a[0].size).then(a[0].pre.cmp(&b[0].pre)));

    // Accepted subtree ranges, keyed by start: tree ranges are nested or
    // disjoint, so a candidate is taken iff some accepted range contains it.
    let mut claimed: BTreeMap<usize, usize> = BTreeMap::new();
    let taken = |claimed: &BTreeMap<usize, usize>, pre: usize| {
        claimed
            .range(..=pre)
            .next_back()
            .is_some_and(|(_, end)| pre < *end)
    };
    let mut accepted = Vec::new();
    for members in groups {
        let free: Vec<Candidate> = members
            .into_iter()
            .filter(|member| !taken(&claimed, member.pre))
            .collect();
        if free.len() < 2 {
            continue;
        }
        for member in &free {
            claimed.insert(member.pre, member.pre + member.size);
        }
        accepted.push(free.into_iter().map(|member| member.id).collect());
    }
    accepted
}

/// Make the group's first member the master and every other member a
/// proven ref instance. `None` when no instance survived the proof.
fn convert_group(
    roots: &mut [PenNode],
    ids: &[String],
    used_names: &mut HashMap<&'static str, usize>,
) -> Option<RecognizedComponent> {
    let (master_id, instance_ids) = ids.split_first()?;
    let master = crate::walkers::find_node(roots, &crate::NodeId::new(master_id.clone()))?;
    let kind = classify(master);
    let base_name = kind.default_name();
    let count = used_names.entry(base_name).or_insert(0);
    *count += 1;
    let name = if *count == 1 {
        base_name.to_string()
    } else {
        format!("{base_name} {count}")
    };

    let mut master_node = master.clone();
    if let PenNode::Frame(frame) = &mut master_node {
        frame.reusable = Some(true);
        frame.base.name = Some(name.clone());
    } else {
        return None;
    }
    let master_value = serde_json::to_value(&master_node).ok()?;
    // A one-node document the resolver looks the master up in.
    let lookup: PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [master_value.clone()],
    }))
    .ok()?;

    let mut converted = Vec::new();
    for instance_id in instance_ids {
        let Some(original) =
            crate::walkers::find_node(roots, &crate::NodeId::new(instance_id.clone()))
        else {
            continue;
        };
        let Some(reference) = instance_ref(&master_value, original, &name) else {
            continue;
        };
        if !round_trips(&reference, original, &lookup) {
            continue;
        }
        if replace_node(roots, instance_id, reference) {
            converted.push(instance_id.clone());
        }
    }
    if converted.is_empty() {
        *used_names.entry(base_name).or_insert(1) -= 1;
        return None;
    }
    replace_node(roots, master_id, master_node);
    Some(RecognizedComponent {
        master_id: master_id.clone(),
        name,
        kind,
        instance_ids: converted,
    })
}

/// The `ref` node that renders as `original`: the instance's own identity
/// and placement plus every content field that differs from the master.
fn instance_ref(master: &Value, original: &PenNode, name: &str) -> Option<PenNode> {
    let original_value = serde_json::to_value(original).ok()?;
    let mut overrides = Map::new();
    collect_overrides(master, &original_value, true, &mut overrides)?;
    let mut reference = Map::new();
    reference.insert("type".into(), Value::String("ref".into()));
    reference.insert("id".into(), Value::String(original.id_str().to_string()));
    reference.insert("ref".into(), master.get("id")?.clone());
    reference.insert("name".into(), Value::String(name.to_string()));
    for key in ["x", "y"] {
        if let Some(value) = original_value.get(key).filter(|v| !v.is_null()) {
            reference.insert(key.into(), value.clone());
        }
    }
    if !overrides.is_empty() {
        reference.insert("descendants".into(), Value::Object(overrides));
    }
    serde_json::from_value(Value::Object(reference)).ok()
}

/// Walk master and instance in parallel, recording each descendant's
/// differing content fields under the master descendant's id. `None` when
/// the two shapes disagree (never expected after grouping).
fn collect_overrides(
    master: &Value,
    instance: &Value,
    is_root: bool,
    out: &mut Map<String, Value>,
) -> Option<()> {
    if !is_root {
        let mut changed = Map::new();
        for field in CONTENT_FIELDS {
            let (m, i) = (master.get(field), instance.get(field));
            if m.is_some() != i.is_some() {
                return None;
            }
            if let (Some(m), Some(i)) = (m, i) {
                if m != i {
                    changed.insert(field.to_string(), i.clone());
                }
            }
        }
        if !changed.is_empty() {
            let id = master.get("id")?.as_str()?.to_string();
            out.insert(id, Value::Object(changed));
        }
    }
    let empty = Vec::new();
    let mc = master
        .get("children")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let ic = instance
        .get("children")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    if mc.len() != ic.len() {
        return None;
    }
    for (m, i) in mc.iter().zip(ic) {
        collect_overrides(m, i, false, out)?;
    }
    Some(())
}

/// Expand `reference` exactly as the canvas does and compare with the
/// original subtree, ignoring ids (instances live in a virtual id space)
/// and the root's layer name.
fn round_trips(reference: &PenNode, original: &PenNode, lookup: &PenDocument) -> bool {
    let expanded =
        crate::ref_resolve::resolve_refs_for_canvas_roots(std::slice::from_ref(reference), lookup);
    let [expanded] = expanded.as_slice() else {
        return false;
    };
    let (Ok(mut a), Ok(mut b)) = (
        serde_json::to_value(expanded),
        serde_json::to_value(original),
    ) else {
        return false;
    };
    for value in [&mut a, &mut b] {
        strip_ids(value);
        if let Some(object) = value.as_object_mut() {
            object.remove("name");
            object.remove("reusable");
        }
    }
    a == b
}

fn strip_ids(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("id");
        if let Some(children) = object.get_mut("children").and_then(Value::as_array_mut) {
            for child in children {
                strip_ids(child);
            }
        }
    }
}

fn replace_node(nodes: &mut [PenNode], id: &str, replacement: PenNode) -> bool {
    let mut slot = Some(replacement);
    replace_in(nodes, id, &mut slot)
}

fn replace_in(nodes: &mut [PenNode], id: &str, slot: &mut Option<PenNode>) -> bool {
    for node in nodes.iter_mut() {
        if node.id_str() == id {
            if let Some(replacement) = slot.take() {
                *node = replacement;
                return true;
            }
            return false;
        }
        if let Some(children) = node.children_mut() {
            if replace_in(children, id, slot) {
                return true;
            }
        }
    }
    false
}

/// Name the part from its structure (and, for list items, its tag).
fn classify(node: &PenNode) -> ComponentKind {
    let PenNode::Frame(frame) = node else {
        return ComponentKind::Item;
    };
    let mut texts = 0usize;
    let mut images = 0usize;
    let mut total = 0usize;
    count_content(node, &mut texts, &mut images, &mut total);
    let painted = frame.container.fill.as_ref().is_some_and(|f| !f.is_empty())
        || frame.container.stroke.is_some()
        || frame.container.corner_radius.is_some();
    let tag = frame
        .base
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if painted && texts == 1 && images == 0 && total <= 6 {
        return ComponentKind::Button;
    }
    if matches!(tag.as_str(), "a" | "button") && texts == 1 && images == 0 {
        return ComponentKind::Button;
    }
    if tag == "li" {
        return ComponentKind::ListItem;
    }
    if painted && texts + images >= 2 {
        return ComponentKind::Card;
    }
    ComponentKind::Item
}

fn count_content(node: &PenNode, texts: &mut usize, images: &mut usize, total: &mut usize) {
    *total += 1;
    match node {
        PenNode::Text(_) => *texts += 1,
        PenNode::Image(_) => *images += 1,
        _ => {}
    }
    if let Some(children) = node.children() {
        for child in children {
            count_content(child, texts, images, total);
        }
    }
}

#[cfg(test)]
#[path = "component_recognition_tests.rs"]
mod tests;
