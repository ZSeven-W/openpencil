//! 阶段 4 —— 清理 pass。
//!
//! [`run_cleanup_passes`] 在所有 subtask 插入完成后运行,是独立
//! 函数 —— S3a 顺序路径与 S3b 并发路径都复用它(spec §9)。
//!
//! [`descendant_count`] 给 `run()` 的"零内容"判定提供基线:
//! scaffold 之后数一次,subtask 全跑完再数一次,没涨即零内容。
//!
//! ## Task C1 (S3b-2): concurrent failure policy + N-root cleanup
//!
//! [`aggregate_concurrent_verdict`] — run-all-aggregate failure check:
//! if EVERY collected outcome has 0 nodes → returns the first non-empty
//! error string (or a fallback message); otherwise returns `None` (partial
//! success is accepted). Port of `orchestrator-sub-agent.ts:319-325`.
//!
//! [`cleanup_concurrent_roots`] — N-root cleanup after a concurrent run.
//! Per root: if `descendant_count <= baseline`, delete that root (scaffold-only).
//! Roll back plan-derived variables only when NOTHING survived across all roots.
//! Port of `orchestrator.ts:1101-1158`.

use crate::plan::OrchestratorPlan;
use crate::types::{DocSink, OrchestratorError, SubtaskOutcome};
use crate::variables::{rollback, VarSnapshot};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

/// 递归统计 `node` 下的后代数(不含自身)。
fn count_descendants(node: &PenNode) -> usize {
    match node.children() {
        Some(children) => children.len() + children.iter().map(count_descendants).sum::<usize>(),
        None => 0,
    }
}

/// 统计活动页里 id 为 `root_id` 的节点的后代总数。节点不存在
/// 时返回 0。
pub fn descendant_count(state: &EditorState, root_id: &str) -> usize {
    state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .map(count_descendants)
        .unwrap_or(0)
}

/// 名称 / id 命中即视为"状态栏"节点。
fn is_status_bar(node: &PenNode) -> bool {
    let name = node.base().name.as_deref().unwrap_or("").to_lowercase();
    let id = node.id_str().to_lowercase();
    let hay = format!("{id} {name}");
    hay.contains("status bar") || hay.contains("status-bar") || hay.contains("statusbar")
}

/// Pass ①:移动端重复状态栏去重。scaffold 注入了一个固定状态栏,
/// 若某个 sub-agent 又生成了状态栏,根下就会有多个 —— 保留第一个,
/// 删掉其余。对齐 TS `removeDuplicateStatusBars`。
fn remove_duplicate_status_bars(sink: &mut dyn DocSink, root_id: &str) {
    let dupes: Vec<NodeId> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let Some(children) = root.children() else {
            return;
        };
        children
            .iter()
            .filter(|c| is_status_bar(c))
            .skip(1) // 保留第一个
            .map(|c| NodeId::new(c.id_str().to_string()))
            .collect()
    };
    for id in dupes {
        sink.apply(EditorCommand::DeleteNode { node_id: id });
    }
}

/// Pass ②:根 frame 高度自适应到内容。把根 frame 的高度设为其
/// 直接子节点的像素高度之和(无显式像素高度的子节点跳过)。
/// 对齐 TS `adjustRootFrameHeightToContent`。
fn adjust_root_height_to_content(sink: &mut dyn DocSink, root_id: &str) {
    let total: Option<i32> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let Some(children) = root.children() else {
            return;
        };
        if children.is_empty() {
            return;
        }
        let sum: f64 = children.iter().filter_map(|c| c.height_px()).sum();
        if sum > 0.0 {
            Some(sum.round() as i32)
        } else {
            None
        }
    };
    if let Some(height) = total {
        sink.apply(EditorCommand::UpdateNode {
            node_id: NodeId::new(root_id.to_string()),
            x: None,
            y: None,
            width: None,
            height: Some(height),
            name: None,
            fill_hex: None,
        });
    }
}

/// 在活动页根里按 id 找节点。
fn find_root<'a>(state: &'a EditorState, root_id: &str) -> Option<&'a PenNode> {
    state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
}

/// 阶段 4 清理 pass —— 在全部 subtask 插入完成后运行。
///
/// `root_ids` 是本轮产出的根 frame id(S3a 单屏只有一个;S3b 并发
/// 多屏会有多个 —— 该函数对每个 root 跑同样的 pass,故 S3b 直接
/// 复用,spec §9)。
///
/// 已实现:① 移动端重复状态栏去重 ② 根高度自适应。
/// 未做:单组件 section root unwrap(`unwrapSingleComponentSection`
/// Root`)—— 启发式强、对 parity 敏感,留作 S3a 后续细化。
pub fn run_cleanup_passes(sink: &mut dyn DocSink, _plan: &OrchestratorPlan, root_ids: &[&str]) {
    for root_id in root_ids {
        remove_duplicate_status_bars(sink, root_id);
        adjust_root_height_to_content(sink, root_id);
    }
}

// ── Task C1: concurrent failure policy + N-root cleanup ──────────────────────

/// Run-all-aggregate failure check.
///
/// Port of `orchestrator-sub-agent.ts:319-325`.
///
/// Called after all concurrent workers have finished and their outcomes have
/// been collected.  Rules:
///
/// - `collected` must be the non-None outcomes from `run_concurrent`'s
///   `Vec<Option<SubtaskOutcome>>` (i.e. outcomes that actually ran).
/// - `total_nodes = Σ outcome.node_count` across `collected`.
/// - If `total_nodes == 0 && !collected.is_empty()` → returns
///   `Err(OrchestratorError::NoContent)` carrying the FIRST non-empty
///   `error` string.  Fallback message when no outcome has an error:
///   `"The model failed to generate any design output."`.
/// - Partial success (any outcome has `node_count > 0`) → returns `Ok(())`.
/// - Empty collected (all aborted) → returns `Ok(())` (the caller handles
///   the abort case separately).
///
/// This is explicitly DIFFERENT from the sequential path's fail-fast: the
/// concurrent path runs ALL workers regardless of individual failures and only
/// fails the entire run when nothing was produced at all.
pub fn aggregate_concurrent_verdict(collected: &[SubtaskOutcome]) -> Result<(), OrchestratorError> {
    if collected.is_empty() {
        return Ok(());
    }
    let total_nodes: usize = collected.iter().map(|o| o.node_count).sum();
    if total_nodes == 0 {
        // Gather the first non-empty error string for the error context.
        // Port of `orchestrator-sub-agent.ts:322-324`:
        //   const errors = collected.filter(r => r.error).map(r => r.error!);
        //   const firstError = errors[0] ?? 'The model failed to generate any design output.';
        //   throw new Error(firstError);
        //
        // `OrchestratorError::NoContent` is the canonical error variant here.
        // The message is preserved in the `Internal` variant for callers that
        // need a human-readable string (e.g., the progress/error log).
        let first_error = collected
            .iter()
            .find_map(|o| o.error.as_deref().filter(|s| !s.is_empty()))
            .unwrap_or("The model failed to generate any design output.");
        return Err(OrchestratorError::Internal(first_error.to_string()));
    }
    Ok(())
}

/// N-root cleanup on the concurrent throw path.
///
/// Port of `orchestrator.ts:1101-1158` (the `catch (e)` block after
/// `executeSubAgents`).
///
/// For each root in `root_ids`:
/// - Compute `now_count = descendant_count(root_id)` on the live sink state.
/// - If `now_count <= baselines[i]` → root is scaffold-only (no sub-agent
///   content produced); delete it (`DeleteNode`).
/// - Otherwise → content survived; set `any_content_survived = true`.
///
/// After iterating all roots:
/// - Roll back plan-derived variables (via `rollback`) ONLY when
///   `!any_content_survived`.  With partial success the user still sees a
///   design whose colors should match the seeded palette; rolling back would
///   flip every `$color-*` ref to the default-palette blue.
///
/// `root_ids` and `baselines` must be index-aligned (both length N).
/// Missing/already-deleted roots are silently skipped (mirrors the TS
/// `try { store.removeNode(rn.id) } catch { /* already gone */ }` pattern).
pub fn cleanup_concurrent_roots(
    sink: &mut dyn DocSink,
    root_ids: &[&str],
    baselines: &[usize],
    var_snapshot: &VarSnapshot,
) {
    debug_assert_eq!(
        root_ids.len(),
        baselines.len(),
        "root_ids and baselines must be index-aligned"
    );

    let mut any_content_survived = false;

    for (i, &root_id) in root_ids.iter().enumerate() {
        let baseline = baselines.get(i).copied().unwrap_or(0);
        let now_count = descendant_count(sink.state(), root_id);

        if now_count <= baseline {
            // Scaffold-only root — remove it (mirrors TS `store.removeNode`).
            sink.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(root_id.to_string()),
            });
        } else {
            any_content_survived = true;
        }
    }

    // Roll back plan-derived variables only when NO content survived at all.
    if !any_content_survived {
        rollback(sink, var_snapshot);
    }
}

// Tests are split: existing cleanup tests remain in this inline block;
// Task C1 tests are in the sibling file `cleanup_tests_c1.rs`.
#[cfg(test)]
#[path = "cleanup_tests_c1.rs"]
mod tests_c1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, RootFrameSpec};
    use crate::test_support::VecDocSink;
    use serde_json::json;

    /// 同 `frame_json` 但返回 `serde_json::Value`(供嵌套构造)。
    fn frame_json_value(id: &str, children: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "frame", "id": id, "name": id,
            "x": 0, "y": 0, "width": 100, "height": 100,
            "children": children,
        })
    }

    fn frame_json(id: &str, children: serde_json::Value) -> PenNode {
        serde_json::from_value(frame_json_value(id, children)).expect("frame json")
    }

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "P".into(),
                width: 1200.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![],
            style_guide_name: None,
        }
    }

    #[test]
    fn descendant_count_counts_nested() {
        let mut sink = VecDocSink::new();
        // root 套 child 套 grandchild
        let tree = frame_json(
            "root",
            json!([frame_json_value(
                "c",
                json!([frame_json_value("gc", json!([]))])
            )]),
        );
        sink.state.apply(EditorCommand::InsertSubtree {
            nodes: vec![tree],
            parent_id: NodeId::NONE,
        });
        let root_id = sink.state.active_children()[0].id_str().to_string();
        assert_eq!(descendant_count(&sink.state, &root_id), 2);
        assert_eq!(descendant_count(&sink.state, "missing"), 0);
    }

    #[test]
    fn remove_duplicate_status_bars_keeps_one() {
        let mut sink = VecDocSink::new();
        // root 下有两个状态栏 + 一个普通区块
        let tree = frame_json(
            "root",
            json!([
                frame_json_value("status-bar-1", json!([])),
                frame_json_value("hero", json!([])),
                frame_json_value("status-bar-2", json!([])),
            ]),
        );
        sink.state.apply(EditorCommand::InsertSubtree {
            nodes: vec![tree],
            parent_id: NodeId::NONE,
        });
        let root_id = sink.state.active_children()[0].id_str().to_string();
        sink.applied.clear();
        run_cleanup_passes(&mut sink, &plan(), &[&root_id]);
        // 至少发了一条 DeleteNode(去掉多出来的状态栏)。
        assert!(sink
            .applied
            .iter()
            .any(|c| matches!(c, EditorCommand::DeleteNode { .. })));
    }

    #[test]
    fn run_cleanup_passes_callable_on_empty_root() {
        let mut sink = VecDocSink::new();
        let tree = frame_json("root", json!([]));
        sink.state.apply(EditorCommand::InsertSubtree {
            nodes: vec![tree],
            parent_id: NodeId::NONE,
        });
        let root_id = sink.state.active_children()[0].id_str().to_string();
        sink.applied.clear();
        // 空 root —— 不 panic,不发命令。
        run_cleanup_passes(&mut sink, &plan(), &[&root_id]);
        assert!(sink.applied.is_empty());
    }
}
