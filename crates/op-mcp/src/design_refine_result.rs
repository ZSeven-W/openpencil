//! `design_refine` MCP tool — phase 3 of the layered design workflow.
//!
//! Split out of `batch_design.rs` (which sits at the 800-line cap) because
//! this tool builds TS design-refine's rich result — `{ rootId,
//! totalNodeCount, fixes[], layoutSnapshot }` — which needs a document
//! snapshot plus the layout-snapshot helper. The refine TRANSFORMS are shared
//! with the host apply path via `op_editor_core::command_refine::refine_subtree`,
//! so the reported `fixes[]` always match what `EditorCommand::RefineDesign`
//! actually applies.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::command_refine::refine_subtree;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::{json, Map, Value};

use super::batch_design::dispatch_phase;
use super::batch_layered::dispatch_design_refine;
use super::batch_page::optional_page_id;
use super::read_nodes::{page_nodes_snapshots, PageNodes};
use super::{McpTool, ToolErrorCode, ToolOutcome};

/// `design_refine` — phase 3 of the layered design workflow. Holds per-page
/// snapshots so it can report the post-refine result with TS parity; the
/// actual mutation still flows through `EditorCommand::RefineDesign`.
pub struct DesignRefine {
    pages: Vec<PageNodes>,
    active_page_id: String,
}

impl McpTool for DesignRefine {
    fn name(&self) -> &str {
        "design_refine"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.contains_key("rootId") || args.contains_key("root_id") {
            return self.refine_with_result(args);
        }
        // Legacy batch-like payloads still route through the phase dispatch.
        dispatch_phase(args, "refine")
    }
}

pub fn design_refine_snapshot(state: &EditorState) -> DesignRefine {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    DesignRefine {
        pages,
        active_page_id,
    }
}

impl DesignRefine {
    /// Resolve the target page the same way the host's
    /// `command_apply::command_page_index` does: empty/absent `pageId` → the
    /// active page; otherwise match by page id, falling back to a numeric index
    /// into the page list. `page_nodes_snapshots` preserves `doc.pages` order +
    /// ids, so the numeric index agrees with the host.
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

    /// Build TS design-refine's `{ rootId, totalNodeCount, fixes[],
    /// layoutSnapshot }` by SIMULATING the deterministic refine on a clone of
    /// the root (the same `refine_subtree` the host apply runs), then return it
    /// alongside the real `RefineDesign` command for the host to apply.
    fn refine_with_result(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if let Some(err) = super::batch_layered::reject_script_with_structured_payload(args) {
            return err;
        }
        // Reuse the validated command construction (rootId + canvasWidth parse).
        let command = match dispatch_design_refine(args) {
            ToolOutcome::OkWithCommand(_flat, command) => command,
            other => return other, // propagate validation errors verbatim
        };
        let root_id = args
            .get("rootId")
            .or_else(|| args.get("root_id"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Resolve the SAME page the host will refine. The command carries
        // `optional_page_id(args)` and the apply (command_apply.rs RefineDesign)
        // switches the active page to `command_page_index(...)` before refining
        // — id-match first, then numeric-index fallback, else the active page.
        let Some(page) = self.resolve_page(args) else {
            return ToolOutcome::Err(ToolErrorCode::ToolFailed, "page not found".into());
        };
        let children = &page.roots;

        // TS errors when the root is absent instead of silently no-op refining.
        let Some(root) = find_node(children, &NodeId::new(root_id.clone())) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("Root node not found: {root_id}"),
            );
        };

        let mut refined = root.clone();
        let fixes = refine_subtree(&mut refined);
        let fixes_json: Vec<Value> = fixes
            .iter()
            .map(|f| {
                let mut obj = Map::new();
                obj.insert("nodeId".into(), json!(f.node_id));
                // TS RefineFix.nodeName is optional — omit when absent.
                if let Some(name) = &f.node_name {
                    obj.insert("nodeName".into(), json!(name));
                }
                obj.insert("fix".into(), json!(f.fix));
                Value::Object(obj)
            })
            .collect();

        // Lay out against the POST-refine page (root swapped in) so ref-node
        // dimension lookups read the refined dimensions, matching the applied
        // document. TS: computeLayoutTree([root], allChildren, 3).
        let all_after: Vec<PenNode> = children
            .iter()
            .map(|n| {
                if n.id_str() == root_id {
                    refined.clone()
                } else {
                    n.clone()
                }
            })
            .collect();
        let layout_snapshot =
            crate::read_tools::subtree_layout_value(std::slice::from_ref(&refined), &all_after, 3);

        let result = json!({
            "rootId": root_id,
            "totalNodeCount": count_subtree(&refined),
            "fixes": fixes_json,
            "layoutSnapshot": layout_snapshot,
        });
        ToolOutcome::OkJsonWithCommand(result.to_string(), command)
    }
}

/// Count a node plus all its descendants (TS `flatten(root).length`).
fn count_subtree(node: &PenNode) -> usize {
    1 + node
        .children()
        .map(|children| children.iter().map(count_subtree).sum::<usize>())
        .unwrap_or(0)
}
