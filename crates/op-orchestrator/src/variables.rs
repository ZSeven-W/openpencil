//! plan 派生变量 —— seed / 快照 / 回滚。
//!
//! S3b-1a:plan 已无 `palette`。三个公共函数入眠态(签名不变,
//! `run.rs` 调用方不动)。忠实的 styleGuideName → 解析 guide →
//! 播种变量是后续项。

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

/// 在 seed *之前* 调用。S3b-1a:plan 已无 `palette`,恒为空快照。
/// 忠实的 styleGuideName → 解析 guide → 播种变量是后续项。
pub fn snapshot_plan_vars(_sink: &dyn DocSink, _plan: &OrchestratorPlan) -> VarSnapshot {
    VarSnapshot::default()
}

/// plan 调色板 → seed 命令。S3b-1a:plan 无 `palette`,恒为空。
pub fn seed_commands(_plan: &OrchestratorPlan) -> Vec<EditorCommand> {
    Vec::new()
}

/// 回滚 seed 新建的变量。S3b-1a:无 seed,故 no-op(快照恒空)。
pub fn rollback(sink: &mut dyn DocSink, snap: &VarSnapshot) {
    for name in &snap.created {
        sink.apply(EditorCommand::DeleteVariable { name: name.clone() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::VecDocSink;

    #[test]
    fn seed_is_dormant() {
        let plan = crate::plan::build_fallback_plan(&crate::types::DesignRequest {
            prompt: "a page".into(),
            model: None,
            provider: None,
            design_md: None,
        });
        assert!(seed_commands(&plan).is_empty());
        let sink = VecDocSink::new();
        assert!(snapshot_plan_vars(&sink, &plan).created.is_empty());
    }

    #[test]
    fn rollback_of_empty_snapshot_is_noop() {
        let mut sink = VecDocSink::new();
        rollback(&mut sink, &VarSnapshot::default());
        assert!(sink.applied.is_empty());
    }
}
