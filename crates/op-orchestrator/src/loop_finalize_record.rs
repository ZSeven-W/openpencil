//! Recording counterpart to the whole-document loop finalizer — split
//! from `loop_finalize.rs` (800-line file cap).
//! [`record_loop_finalize_counted`] runs the exact App finalizer against
//! a snapshot while recording one ordered, atomic host-replay command
//! sequence; the sink, shallow-patch, and snapshot machinery below exists
//! only to serve it.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// A successfully recorded whole-document loop finalization.
///
/// The commands are ordered exactly like [`apply_loop_finalize_counted`] and
/// have already been atomically replayed against the input snapshot.  The
/// replayed canonical document is byte-for-byte JSON-equivalent to `state`.
#[derive(Debug)]
pub struct RecordedLoopFinalize {
    pub state: EditorState,
    pub summary: RepairSummary,
    pub commands: Vec<EditorCommand>,
}

/// Fail-closed error from [`record_loop_finalize_counted`].  No command is
/// returned to the caller when a direct in-place pass performs a topology
/// change other than the explicit frame-to-widget promotion contract or a
/// semantic-phase same-id leaf rewrite, or when the final atomic replay does
/// not reproduce the App path's document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordLoopFinalizeError {
    message: String,
}

impl RecordLoopFinalizeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RecordLoopFinalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RecordLoopFinalizeError {}

/// Recording counterpart to [`StateDocSink`].  Sink-driven phases apply to
/// the working clone and retain only accepted commands for later host replay.
struct RecordingStateDocSink<'a> {
    state: &'a mut EditorState,
    commands: &'a mut Vec<EditorCommand>,
}

impl crate::types::DocSink for RecordingStateDocSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        if self.state.apply(cmd.clone()) {
            // `EditorCommand::Batch` is a useful pass-local atomic boundary
            // (for example the duplicate mobile page-shell merge). The final
            // MCP replay is itself one Batch, and op-editor-core deliberately
            // rejects nested batches. Once the local batch has applied
            // successfully to this exact working snapshot, retain its inner
            // commands in order: the eventual outer replay batch supplies the
            // same (strictly stronger, whole-finalize) atomic boundary.
            match cmd {
                EditorCommand::Batch { commands } => self.commands.extend(commands),
                command => self.commands.push(command),
            }
            true
        } else {
            false
        }
    }

    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        let ids = self
            .state
            .insert_subtree_returning_root_ids(nodes.clone(), parent_id)?;
        self.commands.push(EditorCommand::InsertSubtree {
            nodes,
            parent_id: parent_id.clone(),
            page_id: None,
        });
        Some(ids)
    }

    fn begin_undo_batch(&mut self) {}

    fn end_undo_batch(&mut self) {}
}

/// Run the exact App whole-document finalizer against a snapshot while
/// recording one ordered, atomic host-replay command sequence.
///
/// Sink-based phases retain their already accepted commands.  The three
/// in-place phases (semantic passes, app-state stripping, and final text fill)
/// are converted to same-id shallow patches.  Any unexpected topology edit
/// fails closed.  Before success, the command sequence is atomically replayed
/// on a fresh clone and its canonical document is compared with the direct App
/// result, so an MCP host never receives an unproven approximation.
pub fn record_loop_finalize_counted(
    input: &EditorState,
) -> Result<RecordedLoopFinalize, RecordLoopFinalizeError> {
    let mut state = input.clone();
    let mut summary = RepairSummary::default();
    let mut commands = Vec::new();
    if state.active_children().is_empty() {
        return Ok(RecordedLoopFinalize {
            state,
            summary,
            commands,
        });
    }

    {
        let mut sink = RecordingStateDocSink {
            state: &mut state,
            commands: &mut commands,
        };
        run_loop_finalize_prelude(&mut sink, &mut summary);
    }
    if !state.active_children().is_empty() {
        let before_direct = state.clone();
        let canvas_width = run_loop_finalize_direct_passes(&mut state, &mut summary);
        commands.extend(record_same_id_shallow_patches(
            &before_direct,
            &state,
            "semantic passes",
            true,
        )?);

        let before_hoist = state.clone();
        if let Some(command) = apply_loop_finalize_app_state_hoist(&mut state) {
            commands.extend(record_same_id_shallow_patches(
                &before_hoist,
                &state,
                "app-state hoist",
                false,
            )?);
            commands.push(command);
        }

        {
            let mut sink = RecordingStateDocSink {
                state: &mut state,
                commands: &mut commands,
            };
            run_loop_finalize_cleanup(&mut sink, canvas_width, &mut summary);
        }

        let before_text_fill = state.clone();
        run_loop_finalize_text_fill(&mut state);
        commands.extend(record_same_id_shallow_patches(
            &before_text_fill,
            &state,
            "text fill",
            false,
        )?);
    }

    let mut replay = input.clone();
    if !commands.is_empty()
        && !replay.apply(EditorCommand::Batch {
            commands: commands.clone(),
        })
    {
        return Err(RecordLoopFinalizeError::new(
            "whole-document finalize command replay was rejected atomically",
        ));
    }
    let expected = serde_json::to_value(&state.doc).map_err(|error| {
        RecordLoopFinalizeError::new(format!("serialize finalized document: {error}"))
    })?;
    let actual = serde_json::to_value(&replay.doc).map_err(|error| {
        RecordLoopFinalizeError::new(format!("serialize replayed document: {error}"))
    })?;
    if actual != expected {
        return Err(RecordLoopFinalizeError::new(
            "whole-document finalize replay diverged from the direct App finalizer",
        ));
    }

    Ok(RecordedLoopFinalize {
        state,
        summary,
        commands,
    })
}

#[derive(Debug, Clone)]
struct DirectNodeSnapshot {
    parent: Option<String>,
    index: usize,
    is_container: bool,
    value: serde_json::Map<String, Value>,
    children_wire: Option<Option<Vec<String>>>,
}

/// Convert one direct in-place phase into shallow, same-id node patches.
///
/// The direct finalizer passes are contractually field transforms.  The one
/// deliberate topology exception is `promote_forest`: a frame keeps its id and
/// slot while becoming a leaf widget, consuming its former descendants. The
/// semantic post-pass may also rewrite one leaf primitive into another in
/// place (for example a `text` "View all >" action into an `icon_font`
/// chevron). `PatchNodeData` safely represents that rewrite because the patch
/// carries the new tag and required fields while nulling fields absent from the
/// canonical replacement. Added ids, moves/reorders, or any other deletion
/// fail closed instead of being guessed into a broad subtree replacement.
fn record_same_id_shallow_patches(
    before: &EditorState,
    after: &EditorState,
    phase: &str,
    allow_leaf_type_rewrite: bool,
) -> Result<Vec<EditorCommand>, RecordLoopFinalizeError> {
    let (before_nodes, before_order) = snapshot_direct_nodes(before, phase)?;
    let (after_nodes, after_order) = snapshot_direct_nodes(after, phase)?;

    for id in after_nodes.keys() {
        if !before_nodes.contains_key(id) {
            return Err(RecordLoopFinalizeError::new(format!(
                "{phase} unexpectedly added node {id}"
            )));
        }
    }

    let mut promotions = BTreeSet::new();
    for id in before_nodes.keys() {
        let Some(after_node) = after_nodes.get(id) else {
            continue;
        };
        let before_node = &before_nodes[id];
        let before_type = before_node.value.get("type").and_then(Value::as_str);
        let after_type = after_node.value.get("type").and_then(Value::as_str);
        if before_type != after_type {
            let same_id_leaf_rewrite =
                allow_leaf_type_rewrite && !before_node.is_container && !after_node.is_container;
            let explicit_promotion = before_type == Some("frame")
                && after_type.is_some_and(|kind| kind != "frame")
                && after_node
                    .children_wire
                    .as_ref()
                    .is_none_or(|children| children.as_ref().is_none_or(Vec::is_empty));
            if !explicit_promotion && !same_id_leaf_rewrite {
                return Err(RecordLoopFinalizeError::new(format!(
                    "{phase} unexpectedly changed node {id} type from {before_type:?} to {after_type:?}"
                )));
            }
            if explicit_promotion {
                promotions.insert(id.clone());
            }
        }
    }

    for id in before_nodes.keys() {
        if after_nodes.contains_key(id) {
            continue;
        }
        let mut ancestor = before_nodes[id].parent.as_deref();
        let mut consumed_by_promotion = false;
        while let Some(parent) = ancestor {
            if promotions.contains(parent) {
                consumed_by_promotion = true;
                break;
            }
            ancestor = before_nodes
                .get(parent)
                .and_then(|node| node.parent.as_deref());
        }
        if !consumed_by_promotion {
            return Err(RecordLoopFinalizeError::new(format!(
                "{phase} unexpectedly removed node {id}"
            )));
        }
    }

    for id in after_nodes.keys() {
        let before_node = &before_nodes[id];
        let after_node = &after_nodes[id];
        if before_node.parent != after_node.parent || before_node.index != after_node.index {
            return Err(RecordLoopFinalizeError::new(format!(
                "{phase} unexpectedly moved or reordered node {id}"
            )));
        }
        if !promotions.contains(id)
            && direct_child_ids(&before_node.children_wire)
                != direct_child_ids(&after_node.children_wire)
        {
            return Err(RecordLoopFinalizeError::new(format!(
                "{phase} unexpectedly changed child topology for node {id}"
            )));
        }
    }

    let mut commands = Vec::new();
    for id in after_order {
        let before_node = &before_nodes[&id];
        let after_node = &after_nodes[&id];
        let mut patch = serde_json::Map::new();
        let keys: BTreeSet<&String> = before_node
            .value
            .keys()
            .chain(after_node.value.keys())
            .collect();
        for key in keys {
            if key == "id" || key == "children" {
                continue;
            }
            let before_value = before_node.value.get(key);
            let after_value = after_node.value.get(key);
            if before_value != after_value {
                patch.insert(key.clone(), after_value.cloned().unwrap_or(Value::Null));
            }
        }
        if promotions.contains(&id) {
            // A tagged-enum promotion must explicitly discard the old
            // container payload before serde constructs the new leaf variant.
            patch.insert("children".to_string(), Value::Null);
        } else if before_node.children_wire != after_node.children_wire {
            // Canonical transforms sometimes normalize an empty container's
            // `children: []` to an omitted/None field (or the reverse). The
            // child-id topology is unchanged, so preserve that wire-level
            // normalization with a shallow empty/null patch.
            patch.insert(
                "children".to_string(),
                after_node
                    .value
                    .get("children")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        if !patch.is_empty() {
            commands.push(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id),
                patch_json: Value::Object(patch).to_string(),
                page_id: None,
            });
        }
    }
    // A phase that only consumed descendants through promotion has its parent
    // promotion in `after_order`; no command may target a vanished id.
    debug_assert!(commands.iter().all(|command| match command {
        EditorCommand::PatchNodeData { node_id, .. } => after_nodes.contains_key(node_id.as_str()),
        _ => true,
    }));
    let _ = before_order; // retained by the snapshot helper for diagnostics/tests.
    Ok(commands)
}

fn direct_child_ids(children: &Option<Option<Vec<String>>>) -> &[String] {
    children
        .as_ref()
        .and_then(Option::as_ref)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn snapshot_direct_nodes(
    state: &EditorState,
    phase: &str,
) -> Result<(BTreeMap<String, DirectNodeSnapshot>, Vec<String>), RecordLoopFinalizeError> {
    fn walk(
        nodes: &[PenNode],
        parent: Option<&str>,
        out: &mut BTreeMap<String, DirectNodeSnapshot>,
        order: &mut Vec<String>,
        phase: &str,
    ) -> Result<(), RecordLoopFinalizeError> {
        for (index, node) in nodes.iter().enumerate() {
            let id = node.id_str().to_string();
            let value = serde_json::to_value(node).map_err(|error| {
                RecordLoopFinalizeError::new(format!("serialize node {id} during {phase}: {error}"))
            })?;
            let Value::Object(object) = value else {
                return Err(RecordLoopFinalizeError::new(format!(
                    "node {id} did not serialize as an object during {phase}"
                )));
            };
            let children_wire = match object.get("children") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(Value::Array(children)) => {
                    let mut ids = Vec::with_capacity(children.len());
                    for child in children {
                        let Some(child_id) = child.get("id").and_then(Value::as_str) else {
                            return Err(RecordLoopFinalizeError::new(format!(
                                "child without id below {id} during {phase}"
                            )));
                        };
                        ids.push(child_id.to_string());
                    }
                    Some(Some(ids))
                }
                Some(_) => {
                    return Err(RecordLoopFinalizeError::new(format!(
                        "non-array children on node {id} during {phase}"
                    )));
                }
            };
            if out
                .insert(
                    id.clone(),
                    DirectNodeSnapshot {
                        parent: parent.map(str::to_string),
                        index,
                        is_container: node.is_container(),
                        value: object,
                        children_wire,
                    },
                )
                .is_some()
            {
                return Err(RecordLoopFinalizeError::new(format!(
                    "duplicate node id {id} during {phase}"
                )));
            }
            order.push(id.clone());
            if let Some(children) = node.children() {
                walk(children, Some(&id), out, order, phase)?;
            }
        }
        Ok(())
    }

    let mut out = BTreeMap::new();
    let mut order = Vec::new();
    walk(state.active_children(), None, &mut out, &mut order, phase)?;
    Ok((out, order))
}
