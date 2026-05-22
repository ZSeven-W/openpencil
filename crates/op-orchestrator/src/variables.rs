//! plan 派生变量 —— seed / 快照 / 回滚。
//!
//! 规划产出的 `style_guide.palette` 在阶段 2 被 seed 成文档里的
//! 颜色变量;sub-agent 生成的节点会带 `$color-*` 引用。若本轮零
//! 内容,这些 seed 出来的变量必须回滚,否则留下 dangling 引用。
//!
//! 设计:`seed` 只用 `CreateVariable`(只创建不存在的变量,不
//! 覆盖用户既有变量);快照记下"哪些 plan 变量名在 seed 前不
//! 存在",回滚就 `DeleteVariable` 这些 —— 正好删掉 seed 真正新建
//! 的那批,既有变量不受影响。

use crate::plan::OrchestratorPlan;
use crate::types::DocSink;
use op_editor_core::EditorCommand;

/// 回滚快照 —— seed 前"不存在"的 plan 变量名集合(即 seed 会
/// 真正新建的那批)。
#[derive(Debug, Clone, Default)]
pub struct VarSnapshot {
    /// seed 前不存在的变量名 —— 回滚时删除这些。
    pub created: Vec<String>,
}

/// 在 seed *之前* 调用 —— 记下 plan 调色板里哪些变量名当前
/// 不存在(seed 会新建它们)。
pub fn snapshot_plan_vars(sink: &dyn DocSink, plan: &OrchestratorPlan) -> VarSnapshot {
    let state = sink.state();
    let created = palette_iter(plan)
        .filter(|(name, _)| state.find_variable(name).is_none())
        .map(|(name, _)| name.clone())
        .collect();
    VarSnapshot { created }
}

/// plan 调色板 → `CreateVariable` 命令(每个颜色一条)。
/// `CreateVariable` 对已存在的同名变量会被 applier 拒(返回
/// `false`),故不会覆盖用户既有变量。
pub fn seed_commands(plan: &OrchestratorPlan) -> Vec<EditorCommand> {
    palette_iter(plan)
        .map(|(name, hex)| EditorCommand::CreateVariable {
            name: name.clone(),
            kind: "color".into(),
            default_value: hex.clone(),
        })
        .collect()
}

/// 回滚 —— 删除 seed 真正新建的那批变量(快照里记下的)。
/// 既有变量不在快照里,不受影响。
pub fn rollback(sink: &mut dyn DocSink, snap: &VarSnapshot) {
    for name in &snap.created {
        sink.apply(EditorCommand::DeleteVariable { name: name.clone() });
    }
}

/// plan 调色板的 `(名称, hex)` 迭代器 —— `style_guide` 缺省时为空。
fn palette_iter(plan: &OrchestratorPlan) -> impl Iterator<Item = (&String, &String)> {
    plan.style_guide
        .as_ref()
        .into_iter()
        .flat_map(|sg| sg.palette.iter())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, RootFrameSpec, StyleGuide};
    use crate::test_support::VecDocSink;
    use std::collections::BTreeMap;

    fn plan_with_palette(pairs: &[(&str, &str)]) -> OrchestratorPlan {
        let mut palette = BTreeMap::new();
        for (k, v) in pairs {
            palette.insert((*k).to_string(), (*v).to_string());
        }
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
            style_guide: Some(StyleGuide { palette }),
        }
    }

    #[test]
    fn seed_commands_one_per_palette_color() {
        let plan = plan_with_palette(&[("color-1", "#2563EB"), ("color-2", "#0EA5E9")]);
        let cmds = seed_commands(&plan);
        assert_eq!(cmds.len(), 2);
        assert!(matches!(
            &cmds[0],
            EditorCommand::CreateVariable { kind, .. } if kind == "color"
        ));
    }

    #[test]
    fn seed_then_rollback_round_trips() {
        let plan = plan_with_palette(&[("color-1", "#2563EB")]);
        let mut sink = VecDocSink::new();

        // seed 前快照:color-1 不存在 → 进 created 集
        let snap = snapshot_plan_vars(&sink, &plan);
        assert_eq!(snap.created, vec!["color-1".to_string()]);

        // seed
        for cmd in seed_commands(&plan) {
            sink.apply(cmd);
        }
        assert!(sink.state().find_variable("color-1").is_some());

        // 回滚 → 变量被删
        rollback(&mut sink, &snap);
        assert!(sink.state().find_variable("color-1").is_none());
    }

    #[test]
    fn no_style_guide_yields_no_commands() {
        let mut plan = plan_with_palette(&[]);
        plan.style_guide = None;
        assert!(seed_commands(&plan).is_empty());
    }
}
