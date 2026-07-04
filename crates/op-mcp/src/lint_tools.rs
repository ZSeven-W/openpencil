use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::PenNode;
use op_design_lint::{detect_all, Issue};
use op_editor_core::{EditorState, PenNodeExt};

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct LintDocument {
    issues: Vec<Issue>,
    subtree_ids: BTreeMap<String, BTreeSet<String>>,
}

impl McpTool for LintDocument {
    fn name(&self) -> &str {
        "lint_document"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let allowed = match args.get("nodeId").map(|value| value.trim()) {
            None | Some("") => None,
            Some(node_id) => match self.subtree_ids.get(node_id) {
                Some(ids) => Some(ids),
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!("nodeId not found: {node_id}"),
                    )
                }
            },
        };
        let issues: Vec<&Issue> = self
            .issues
            .iter()
            .filter(|issue| allowed.is_none_or(|ids| ids.contains(&issue.node_id)))
            .collect();
        ToolOutcome::OkJson(
            serde_json::json!({
                "count": issues.len(),
                "issues": issues,
            })
            .to_string(),
        )
    }
}

pub fn lint_document_snapshot(state: &EditorState) -> LintDocument {
    let mut issues = Vec::new();
    let mut subtree_ids = BTreeMap::new();
    if let Some(pages) = state.doc.pages.as_ref() {
        for page in pages {
            for root in &page.children {
                collect_subtree_ids(root, &mut subtree_ids);
                issues.extend(detect_all(root, &state.doc));
            }
        }
    }
    for root in &state.doc.children {
        collect_subtree_ids(root, &mut subtree_ids);
        issues.extend(detect_all(root, &state.doc));
    }
    LintDocument {
        issues,
        subtree_ids,
    }
}

fn collect_subtree_ids(
    node: &PenNode,
    subtree_ids: &mut BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    ids.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            ids.extend(collect_subtree_ids(child, subtree_ids));
        }
    }
    subtree_ids.insert(node.id_str().to_string(), ids.clone());
    ids
}
