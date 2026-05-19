//! 阶段 2 —— 单屏画布搭建。
//!
//! 产出一条 `InsertSubtree`:把根 frame(移动端再带一个固定状态
//! 栏 child)插到活动页根。根 frame 用 JSON 构建后反序列化为
//! canonical `PenNode` —— 避免在 Rust 侧硬写富 schema 的每个字段,
//! 且与 `parse` 模块的解析路径一致。

use crate::plan::OrchestratorPlan;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

/// 移动端固定状态栏的高度(iOS 风格 chrome,通用 mockup 约定)。
const STATUS_BAR_HEIGHT: f64 = 44.0;

/// 构建阶段 2 的画布命令。`is_mobile` 为真时根 frame 带一个固定
/// 状态栏 child。返回 `Err` 表示根 frame JSON 模板有问题(实现
/// bug,非用户输入问题)。
pub fn build_scaffold(
    plan: &OrchestratorPlan,
    is_mobile: bool,
) -> Result<Vec<EditorCommand>, String> {
    let rf = &plan.root_frame;
    let layout = rf.layout.as_deref().unwrap_or("vertical");
    let fill_hex = rf.fill.as_deref().unwrap_or("#FFFFFF");

    let children = if is_mobile {
        json!([{
            "type": "frame",
            "id": format!("{}-status-bar", rf.id),
            "name": "Status Bar",
            "x": 0,
            "y": 0,
            "width": rf.width,
            "height": STATUS_BAR_HEIGHT,
            "fill": [{ "type": "solid", "color": fill_hex }],
            "children": [],
        }])
    } else {
        json!([])
    };

    let frame = json!({
        "type": "frame",
        "id": rf.id,
        "name": rf.name,
        "x": 0,
        "y": 0,
        "width": rf.width,
        "height": rf.height,
        "layout": layout,
        "gap": rf.gap.unwrap_or(0.0),
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": children,
    });

    let node: PenNode =
        serde_json::from_value(frame).map_err(|e| format!("scaffold root frame: {e}"))?;

    Ok(vec![EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, RootFrameSpec};
    use op_editor_core::PenNodeExt;

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Design".into(),
                width: 1200.0,
                height: 800.0,
                layout: Some("vertical".into()),
                gap: Some(0.0),
                padding: Some(0.0),
                fill: Some("#FFFFFF".into()),
            },
            subtasks: vec![],
            style_guide: None,
        }
    }

    #[test]
    fn build_scaffold_desktop_one_root_no_children() {
        let cmds = build_scaffold(&plan(), false).expect("scaffold");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            EditorCommand::InsertSubtree { nodes, parent_id } => {
                assert_eq!(nodes.len(), 1);
                assert!(!parent_id.is_real()); // NONE → page root
                assert_eq!(nodes[0].id_str(), "root");
                assert!(nodes[0].children().map(|c| c.is_empty()).unwrap_or(true));
            }
            other => panic!("expected InsertSubtree, got {other:?}"),
        }
    }

    #[test]
    fn build_scaffold_mobile_injects_status_bar() {
        let cmds = build_scaffold(&plan(), true).expect("scaffold");
        match &cmds[0] {
            EditorCommand::InsertSubtree { nodes, .. } => {
                let children = nodes[0].children().expect("frame children");
                assert_eq!(children.len(), 1);
            }
            other => panic!("expected InsertSubtree, got {other:?}"),
        }
    }
}
