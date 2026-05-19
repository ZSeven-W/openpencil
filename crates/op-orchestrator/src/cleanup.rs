//! 阶段 4 —— 清理 pass。
//!
//! [`run_cleanup_passes`] 在所有 subtask 插入完成后运行,是独立
//! 函数 —— S3a 顺序路径与 S3b 并发路径都复用它(spec §9)。
//!
//! [`descendant_count`] 给 `run()` 的"零内容"判定提供基线:
//! scaffold 之后数一次,subtask 全跑完再数一次,没涨即零内容。

use crate::plan::OrchestratorPlan;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt};

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

/// 阶段 4 清理 pass —— 在全部 subtask 插入完成后运行。
///
/// S3a 骨架版本目前不改文档:三个 TS 清理 pass(移动端重复状态栏
/// 去重 `removeDuplicateStatusBars`、单组件 section root unwrap
/// `unwrapSingleComponentSectionRoot`、高度自适应
/// `adjustRootFrameHeightToContent`)需对照 TS 源码逐条移植并配合
/// 画布验证 —— 列为 S3a 的后续细化项。函数签名与调用点在此固定,
/// 使 S3b 并发路径可直接复用(spec §9)。
pub fn run_cleanup_passes(_sink: &mut dyn DocSink, _plan: &OrchestratorPlan) {
    // 后续细化:① removeDuplicateStatusBars ② unwrapSingleComponentSectionRoot
    // ③ adjustRootFrameHeightToContent —— 逐条对照 TS orchestrator.ts。
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, RootFrameSpec};
    use crate::test_support::VecDocSink;
    use op_editor_core::{EditorCommand, NodeId};
    use serde_json::json;

    fn frame_json(id: &str, children: serde_json::Value) -> PenNode {
        serde_json::from_value(json!({
            "type": "frame", "id": id, "name": id,
            "x": 0, "y": 0, "width": 100, "height": 100,
            "children": children,
        }))
        .expect("frame json")
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
            style_guide: None,
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

    /// 同 `frame_json` 但返回 `serde_json::Value`(供嵌套构造)。
    fn frame_json_value(id: &str, children: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "frame", "id": id, "name": id,
            "x": 0, "y": 0, "width": 100, "height": 100,
            "children": children,
        })
    }

    #[test]
    fn run_cleanup_passes_is_callable() {
        let mut sink = VecDocSink::new();
        run_cleanup_passes(&mut sink, &plan());
        // 骨架版不改文档。
        assert!(sink.applied.is_empty());
    }
}
