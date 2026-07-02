//! `batch_design` MCP tool — emits TS's `{ results:[{binding,nodeId}],
//! nodeCount }` for the operations path.
//!
//! Split out of `batch_design.rs` (at the 800-line cap) because this needs a
//! document snapshot. TS's `runBatchDesignDsl` returns each binding's inserted
//! node id in-process; Rust defers id allocation to the host apply
//! (`cmd_insert_subtree` remaps every id). For a single-user localhost MCP the
//! tool's snapshot == the live doc at apply, and the apply runs the EXACT same
//! allocation (`remap_subtree_ids_mapping`), so the tool can PREDICT the
//! host-assigned ids off a CLONE of the forest and report them — while the
//! emitted `InsertSubtree` command is unchanged. Non-operations / Direct /
//! parse-error paths fall back to the flat `dispatch_batch_design`.

use std::collections::{BTreeMap, HashSet};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::promote::PromoteNote;
use op_editor_core::command_node::remap_subtree_ids_mapping;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

use super::batch_design::{dispatch_batch_design, parse_operations, ParsedOperations};
use super::batch_page::optional_page_id;
use super::read_nodes::{page_nodes_snapshots, PageNodes};
use super::{EditorCommand, McpTool, ToolOutcome};

/// `batch_design` — insert a node forest in one shot. Holds a document snapshot
/// (page node sets + the doc-wide id state) so the operations path can report
/// the TS result; the mutation still flows through `EditorCommand::InsertSubtree`.
pub struct BatchDesign {
    /// Full editor-state snapshot — the multi-op DSL program executor
    /// (`batch_program.rs`) simulates each program line against a clone
    /// of this to mirror TS's per-line best-effort semantics.
    state: EditorState,
    pages: Vec<PageNodes>,
    active_page_id: String,
    existing_ids: HashSet<NodeId>,
    id_seed: Option<u64>,
}

impl McpTool for BatchDesign {
    fn name(&self) -> &str {
        "batch_design"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(operations) = args.get("operations") else {
            // nodes_json (a Rust convenience) / missing → flat dispatch.
            return dispatch_batch_design(args, None);
        };
        match parse_operations(operations) {
            Ok(ParsedOperations::Insert {
                parent_id,
                nodes,
                count: _,
                bindings,
                promoted,
            }) => self.insert_with_result(args, parent_id, nodes, bindings, promoted),
            // A single direct op keeps the flat dispatch (which re-parses
            // and returns the existing command-per-op shape).
            Ok(ParsedOperations::Direct(_)) => dispatch_batch_design(args, None),
            // Everything else — multi-line MIXED programs, per-line parse
            // failures — runs the TS DSL program executor, which collects
            // per-line errors and applies the surviving lines (TS
            // `runBatchDesignDsl` best-effort semantics).
            Err(_) => super::batch_program::run_batch_design_program(&self.state, operations, args),
        }
    }
}

pub fn batch_design_snapshot(state: &EditorState) -> BatchDesign {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    BatchDesign {
        state: state.clone(),
        pages,
        active_page_id,
        existing_ids: state.collect_node_ids(),
        id_seed: state.next_node_id_seed(),
    }
}

impl BatchDesign {
    fn insert_with_result(
        &self,
        args: &BTreeMap<String, String>,
        parent_id: NodeId,
        mut nodes: Vec<PenNode>,
        bindings: Vec<String>,
        promoted: Vec<PromoteNote>,
    ) -> ToolOutcome {
        let Some(seed0) = self.id_seed else {
            return dispatch_batch_design(args, None); // id-space exhausted → flat path
        };
        let mut seed = seed0;
        let mut taken = self.existing_ids.clone();
        // Assign unique ids to the forest IN PLACE (from the snapshot's
        // max-id+1, so free vs the snapshot), then emit them as AUTHORED ids
        // the host PRESERVES (`InsertAuthoredSubtree`) rather than remaps. So
        // the reported ids are EXACTLY what lands in the doc — or, if the doc
        // changed between this snapshot and the apply (e.g. a concurrent edit
        // on the live desktop canvas) so an id now collides, the apply REJECTS
        // and the response is demoted to an error. Never silently-wrong ids.
        // (For the no-collision case the ids equal what `InsertSubtree`'s remap
        // would have produced, so the on-canvas result is unchanged.)
        let Some(map) = remap_subtree_ids_mapping(&mut nodes, &mut seed, &mut taken) else {
            return dispatch_batch_design(args, None);
        };

        // Each binding-node carries its binding as its pre-assignment authored
        // id, so map entries whose old id is a binding give binding -> new id.
        let binding_set: HashSet<&String> = bindings.iter().collect();
        let results: Vec<Value> = map
            .iter()
            .filter(|(old, _)| binding_set.contains(old))
            .map(|(old, new)| json!({ "binding": old, "nodeId": new }))
            .collect();

        // TS nodeCount = countNodes(getDocChildren(doc, pageId)) AFTER insert =
        // target page's pre-insert node count + every inserted node (map.len()).
        let base = self
            .resolve_page(args)
            .map(|page| count_forest(&page.roots))
            .unwrap_or(0);
        let node_count = base + map.len();

        // Phase E3 — report the legacy role frames normalized into widget
        // nodes. Omitted entirely (so the result is byte-identical to before)
        // when nothing was promoted, which is the common case.
        let mut result = json!({ "results": results, "nodeCount": node_count });
        if !promoted.is_empty() {
            let notes: Vec<Value> = promoted
                .iter()
                .map(|n| json!({ "nodeId": n.node_id, "fromRole": n.from_role, "to": n.to }))
                .collect();
            result["promoted"] = json!(notes);
        }
        ToolOutcome::OkJsonWithCommand(
            result.to_string(),
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id: optional_page_id(args),
            },
        )
    }

    /// Resolve the target page for the node-count base, mirroring the host's
    /// `command_apply::command_page_index` (id-match → numeric index → active).
    fn resolve_page(&self, args: &BTreeMap<String, String>) -> Option<&PageNodes> {
        match optional_page_id(args)
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            None => self
                .pages
                .iter()
                .find(|p| p.id == self.active_page_id)
                .or_else(|| self.pages.first()),
            Some(raw) => self.pages.iter().find(|p| p.id == raw).or_else(|| {
                raw.parse::<usize>()
                    .ok()
                    .and_then(|idx| self.pages.get(idx))
            }),
        }
    }
}

/// Count every node in a forest (TS `countNodes`).
fn count_forest(nodes: &[PenNode]) -> usize {
    nodes.iter().map(count_subtree).sum()
}

fn count_subtree(node: &PenNode) -> usize {
    1 + node
        .children()
        .map(|children| children.iter().map(count_subtree).sum::<usize>())
        .unwrap_or(0)
}
