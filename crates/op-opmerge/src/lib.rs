//! Node-level three-way merge for OpenPencil `.op` documents.
//!
//! A `.op` file is JSON; a textual git merge of it produces broken
//! JSON the moment two branches touch nearby lines. This crate
//! merges `.op` documents *structurally* instead: it keys every
//! `PenNode` by its `id`, runs a three-way merge per node, and
//! reports the residue as a list of [`NodeConflict`]s the editor
//! can resolve one node at a time.
//!
//! ## Scope
//!
//! This is the foundation cut. It auto-merges **per-node property
//! changes** — the common design edit (a moved rect, a recoloured
//! fill) — when only one branch touched a node and left its child
//! order intact. Genuine divergence (both branches changed the same
//! node, a delete vs a modify, a one-sided child reorder / add /
//! delete, a node added only on the other branch) is surfaced as a
//! [`NodeConflict`] rather than guessed at — it is never silently
//! dropped. Structural *auto-merge* (applying a one-sided reorder,
//! grafting a remotely-added node) is left to the resolution step.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{Map, Value};

/// Object keys whose values are child-node arrays.
const CHILD_KEYS: [&str; 2] = ["children", "pages"];

/// An error from a structured merge.
#[derive(Debug, thiserror::Error)]
pub enum OpMergeError {
    /// A document was not valid JSON.
    #[error("the {0} document is not valid JSON: {1}")]
    Parse(&'static str, String),
    /// A document used the same node `id` twice — a structural merge
    /// keys on `id`, so a duplicate would silently merge the wrong
    /// node. Reject it rather than guess.
    #[error("the {0} document has a duplicate node id: {1}")]
    DuplicateId(&'static str, String),
}

/// How a single node diverged between the two branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeConflictKind {
    /// Both branches changed the node's properties, differently.
    BothModified,
    /// One branch reorganised the node's children (a reorder, add or
    /// delete of a child) — not safe to auto-apply.
    StructuralChange,
    /// One branch deleted the node while the other kept / changed it.
    DeleteModify,
    /// Both branches added a node with the same `id` but different
    /// content.
    BothAdded,
    /// The node exists only on the other (remote) branch.
    AddedOnRemote,
}

impl NodeConflictKind {
    /// The `op-i18n` key for this conflict kind's short label. The
    /// crate stays locale-free — the host translates the key against
    /// the active UI locale.
    pub fn i18n_key(self) -> &'static str {
        match self {
            NodeConflictKind::BothModified => "git.conflict.bothModified",
            NodeConflictKind::StructuralChange => "git.conflict.structuralChange",
            NodeConflictKind::DeleteModify => "git.conflict.deleteModify",
            NodeConflictKind::BothAdded => "git.conflict.bothAdded",
            NodeConflictKind::AddedOnRemote => "git.conflict.addedOnRemote",
        }
    }
}

/// One unresolved node-level conflict from a structured `.op` merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConflict {
    /// The conflicting node's `id`.
    pub id: String,
    /// A display label — the node's `name`, else its `type`, else
    /// the `id`.
    pub label: String,
    /// How the node diverged.
    pub kind: NodeConflictKind,
    /// Whether resolving to *theirs* can be applied exactly — true
    /// only when both sides keep the node under the same parent with
    /// the same child set (the resolver swaps properties and reorders
    /// existing children, but never moves / grafts / drops a node).
    /// A `false` conflict is resolvable only to *ours*.
    pub theirs_applicable: bool,
}

/// The outcome of a structured `.op` merge.
#[derive(Debug, Clone)]
pub struct OpMergeResult {
    /// The merged document — the local (`ours`) tree with every
    /// cleanly auto-mergeable remote change applied.
    pub merged: Value,
    /// Node-level conflicts left for the editor to resolve.
    pub conflicts: Vec<NodeConflict>,
}

impl OpMergeResult {
    /// Whether the merge was fully automatic (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// The merged document serialized as pretty-printed JSON — the
    /// form written back to disk to complete an auto-merge.
    pub fn merged_json(&self) -> String {
        serde_json::to_string_pretty(&self.merged).unwrap_or_default()
    }
}

/// One node's mergeable state — its own properties, the ordered ids
/// of its direct children, and its parent's id. Two records compare
/// equal only when properties, child order *and* parent all match,
/// so a reorder or a reparent is a detectable change rather than an
/// invisible one.
#[derive(Debug, Clone, PartialEq)]
struct NodeRecord {
    /// The node object with its child-node arrays removed.
    props: Value,
    /// Ordered ids of the node's direct children.
    child_ids: Vec<String>,
    /// The id of the node's parent — `None` for a top-level node.
    parent_id: Option<String>,
}

/// Three-way merge the `.op` documents `base` → (`ours`, `theirs`).
///
/// `base` is the merge-base revision, `ours` the local branch,
/// `theirs` the branch being merged in. Returns the merged tree plus
/// the node conflicts; an `Err` only when an input is not JSON or
/// reuses a node id.
pub fn merge_op_documents(
    base: &str,
    ours: &str,
    theirs: &str,
) -> Result<OpMergeResult, OpMergeError> {
    merge_core(base, ours, theirs, &HashMap::new())
}

/// Three-way merge with per-node conflict resolutions applied.
///
/// `choices` maps a conflicting node `id` to the side the user
/// picked — `true` = take theirs, `false` = keep ours. A conflict
/// with a choice is resolved (and dropped from the returned
/// `conflicts`); one without a choice still surfaces. When every
/// conflict has a choice the result is clean and ready to write.
///
/// A `true` (take-theirs) choice only applies to a conflict whose
/// [`NodeConflict::theirs_applicable`] is set — one the resolver can
/// reproduce exactly; for any other conflict a `true` choice falls
/// back to keeping *ours*.
pub fn resolve_op_merge(
    base: &str,
    ours: &str,
    theirs: &str,
    choices: &HashMap<String, bool>,
) -> Result<OpMergeResult, OpMergeError> {
    merge_core(base, ours, theirs, choices)
}

/// Shared three-way merge core — see [`merge_op_documents`] and
/// [`resolve_op_merge`].
fn merge_core(
    base: &str,
    ours: &str,
    theirs: &str,
    choices: &HashMap<String, bool>,
) -> Result<OpMergeResult, OpMergeError> {
    let base_doc = parse(base, "base")?;
    let ours_doc = parse(ours, "ours")?;
    let theirs_doc = parse(theirs, "theirs")?;

    let base_nodes = flatten(&base_doc, "base")?;
    let ours_nodes = flatten(&ours_doc, "ours")?;
    let theirs_nodes = flatten(&theirs_doc, "theirs")?;

    // Conflict candidates discovered by the 3-way walk — split into
    // resolved / unresolved against `choices` after the loop.
    let mut candidates: Vec<NodeConflict> = Vec::new();
    // Nodes whose properties should be overwritten with the remote
    // version — a clean "only theirs changed it" auto-merge, plus
    // any conflict the caller resolved to *theirs*.
    let mut take_theirs: HashSet<String> = HashSet::new();

    let all_ids: BTreeSet<&String> = base_nodes
        .keys()
        .chain(ours_nodes.keys())
        .chain(theirs_nodes.keys())
        .collect();

    for id in all_ids {
        let base_node = base_nodes.get(id);
        let ours_node = ours_nodes.get(id);
        let theirs_node = theirs_nodes.get(id);
        match (base_node, ours_node, theirs_node) {
            // Present on both branches.
            (base_node, Some(o), Some(t)) => {
                if o == t {
                    continue; // Identical on both sides.
                }
                match base_node {
                    Some(b) => {
                        let ours_changed = o != b;
                        let theirs_changed = t != b;
                        if ours_changed && theirs_changed {
                            candidates.push(conflict(
                                id,
                                Some(o),
                                Some(t),
                                NodeConflictKind::BothModified,
                            ));
                        } else if theirs_changed {
                            // Only the remote branch changed it.
                            let props_differ = b.props != t.props;
                            let order_differ = b.child_ids != t.child_ids;
                            let parent_differ = b.parent_id != t.parent_id;
                            if parent_differ {
                                // The node was moved to a different
                                // parent — a structural move that a
                                // property swap cannot apply.
                                candidates.push(conflict(
                                    id,
                                    Some(o),
                                    Some(t),
                                    NodeConflictKind::StructuralChange,
                                ));
                            } else if props_differ && !order_differ {
                                // A pure property change — auto-merge.
                                take_theirs.insert(id.clone());
                            } else if order_differ
                                && same_child_set(&b.child_ids, &t.child_ids)
                            {
                                // A pure reorder — not captured by
                                // any child conflict, so surface it.
                                candidates.push(conflict(
                                    id,
                                    Some(o),
                                    Some(t),
                                    NodeConflictKind::StructuralChange,
                                ));
                            } else if props_differ {
                                // Properties changed alongside a
                                // child add / delete — cannot be
                                // applied by a plain property swap.
                                candidates.push(conflict(
                                    id,
                                    Some(o),
                                    Some(t),
                                    NodeConflictKind::StructuralChange,
                                ));
                            }
                            // else: a pure child add / delete — the
                            // affected child nodes self-report.
                        }
                        // Only `ours` changed → keep ours (default).
                    }
                    // Added on both branches with differing content.
                    None => candidates.push(conflict(
                        id,
                        Some(o),
                        Some(t),
                        NodeConflictKind::BothAdded,
                    )),
                }
            }
            // Present locally, gone on the remote branch.
            (Some(_), Some(o), None) => {
                candidates.push(conflict(id, Some(o), None, NodeConflictKind::DeleteModify));
            }
            // Gone locally, present on the remote branch.
            (Some(_), None, Some(t)) => {
                candidates.push(conflict(id, None, Some(t), NodeConflictKind::DeleteModify));
            }
            // Added only on the remote branch.
            (None, None, Some(t)) => {
                candidates.push(conflict(id, None, Some(t), NodeConflictKind::AddedOnRemote));
            }
            // Added only locally, or deleted on both — no conflict.
            _ => {}
        }
    }

    // Split candidates against the caller's choices: a choice
    // resolves the conflict (and `true` on a property-style kind
    // takes theirs); a candidate with no choice still surfaces.
    let mut conflicts = Vec::new();
    for candidate in candidates {
        match choices.get(&candidate.id) {
            Some(&take) => {
                // A `theirs` choice only applies when the resolver
                // can reproduce theirs exactly; otherwise it falls
                // back to keeping ours.
                if take && candidate.theirs_applicable {
                    take_theirs.insert(candidate.id.clone());
                }
            }
            None => conflicts.push(candidate),
        }
    }

    // Apply every taken node — the clean remote-only changes plus
    // the conflicts the caller resolved to theirs — onto an `ours`
    // clone, which is the merged tree.
    let mut merged = ours_doc;
    if !take_theirs.is_empty() {
        apply_overrides(&mut merged, &theirs_nodes, &take_theirs);
    }
    conflicts.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(OpMergeResult { merged, conflicts })
}

/// Parse one input document, tagging the error with its role.
fn parse(text: &str, role: &'static str) -> Result<Value, OpMergeError> {
    serde_json::from_str(text).map_err(|e| OpMergeError::Parse(role, e.to_string()))
}

/// Build a map of `id` → [`NodeRecord`] for every id-bearing node in
/// the document. Errors on a duplicate id.
fn flatten(doc: &Value, role: &'static str) -> Result<HashMap<String, NodeRecord>, OpMergeError> {
    let mut out = HashMap::new();
    collect(doc, role, None, &mut out)?;
    Ok(out)
}

/// Recursively collect every id-bearing node into `out`. `parent` is
/// the id of the nearest id-bearing ancestor.
fn collect(
    value: &Value,
    role: &'static str,
    parent: Option<&str>,
    out: &mut HashMap<String, NodeRecord>,
) -> Result<(), OpMergeError> {
    match value {
        Value::Object(map) => {
            let this_id = match map.get("id") {
                Some(Value::String(id)) => Some(id.as_str()),
                _ => None,
            };
            if let Some(id) = this_id {
                let record = NodeRecord {
                    props: own_props(map),
                    child_ids: child_ids(map),
                    parent_id: parent.map(String::from),
                };
                if out.insert(id.to_string(), record).is_some() {
                    return Err(OpMergeError::DuplicateId(role, id.to_string()));
                }
            }
            // A node's children carry its id as their parent; a
            // root object with no id passes the inherited parent on.
            let child_parent = this_id.or(parent);
            for key in CHILD_KEYS {
                if let Some(Value::Array(children)) = map.get(key) {
                    for child in children {
                        collect(child, role, child_parent, out)?;
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect(item, role, parent, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// A node object with its child-node arrays stripped — its "own
/// properties" for change detection.
fn own_props(map: &Map<String, Value>) -> Value {
    let props: Map<String, Value> = map
        .iter()
        .filter(|(k, _)| !CHILD_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Value::Object(props)
}

/// Whether two child-id lists hold the same set of ids (ids are
/// unique per document, so set equality is a length + membership
/// check) — distinguishes a pure reorder from an add / delete.
fn same_child_set(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().all(|id| b.contains(id))
}

/// Reorder a child-node array to match the id order in `order`.
/// Children whose id is not in `order` keep their relative position
/// after the known ones (a stable sort).
fn reorder_children(children: &mut [Value], order: &[String]) {
    children.sort_by_key(|child| {
        match child.get("id").and_then(Value::as_str) {
            Some(id) => order.iter().position(|o| o == id).unwrap_or(order.len()),
            None => order.len(),
        }
    });
}

/// The ordered ids of a node's direct children.
fn child_ids(map: &Map<String, Value>) -> Vec<String> {
    let mut ids = Vec::new();
    for key in CHILD_KEYS {
        if let Some(Value::Array(children)) = map.get(key) {
            for child in children {
                if let Some(Value::String(id)) = child.get("id") {
                    ids.push(id.clone());
                }
            }
        }
    }
    ids
}

/// Build a [`NodeConflict`] for `id`. `theirs_applicable` is `true`
/// only when both sides are present with the same parent and child
/// set — the precondition for [`apply_overrides`] to reproduce
/// theirs exactly.
fn conflict(
    id: &str,
    ours: Option<&NodeRecord>,
    theirs: Option<&NodeRecord>,
    kind: NodeConflictKind,
) -> NodeConflict {
    let label = theirs
        .or(ours)
        .map(|r| label_of(&r.props, id))
        .unwrap_or_else(|| id.to_string());
    let theirs_applicable = match (ours, theirs) {
        (Some(o), Some(t)) => {
            same_child_set(&o.child_ids, &t.child_ids) && o.parent_id == t.parent_id
        }
        _ => false,
    };
    NodeConflict {
        id: id.to_string(),
        label,
        kind,
        theirs_applicable,
    }
}

/// A display label for a node — its `name`, else `type`, else `id`.
fn label_of(props: &Value, id: &str) -> String {
    for key in ["name", "type"] {
        if let Some(Value::String(s)) = props.get(key) {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    id.to_string()
}

/// Overwrite every node in `take` with the remote version, walking
/// the merged tree in place: the node's own properties are swapped
/// for theirs, and its preserved child arrays are reordered to
/// theirs' child order (a no-op for a pure property change, the
/// actual fix for a resolved structural-reorder conflict).
fn apply_overrides(
    value: &mut Value,
    theirs_nodes: &HashMap<String, NodeRecord>,
    take: &HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            let id = match map.get("id") {
                Some(Value::String(id)) => Some(id.clone()),
                _ => None,
            };
            if let Some(id) = id {
                if take.contains(&id) {
                    if let Some(record) = theirs_nodes.get(&id) {
                        if let Value::Object(props) = &record.props {
                            let kept: Vec<(String, Value)> = CHILD_KEYS
                                .iter()
                                .filter_map(|k| map.get(*k).map(|v| (k.to_string(), v.clone())))
                                .collect();
                            *map = props.clone();
                            for (k, mut v) in kept {
                                if let Value::Array(children) = &mut v {
                                    reorder_children(children, &record.child_ids);
                                }
                                map.insert(k, v);
                            }
                        }
                    }
                }
            }
            for key in CHILD_KEYS {
                if let Some(Value::Array(children)) = map.get_mut(key) {
                    for child in children {
                        apply_overrides(child, theirs_nodes, take);
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                apply_overrides(item, theirs_nodes, take);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
