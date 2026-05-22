//! plan 规范化 —— 单屏路径。
//!
//! 行为忠实 TS,但**一次性算分类、干净派生**,不做 TS 那种
//! in-place strip-then-reclassify(`orchestrator.ts:838-845` 自标
//! fragile)。

use crate::plan::OrchestratorPlan;
use crate::types::DesignRequest;

/// 规范化产出的派生信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormInfo {
    /// 根 frame 窄到移动端宽度 —— scaffold 阶段据此注入固定状态栏。
    pub is_mobile: bool,
}

/// 移动端宽度上限(含)—— ≤ 此值视为移动端单屏。
const MOBILE_MAX_WIDTH: f64 = 480.0;

/// subtask 的 id / label 命中即视为"状态栏"区块 —— 移动端由
/// scaffold 注入固定状态栏,plan 里若带状态栏 subtask 则剔除。
fn is_status_bar_subtask(id: &str, label: &str) -> bool {
    let hay = format!("{} {}", id.to_lowercase(), label.to_lowercase());
    hay.contains("status bar") || hay.contains("status-bar") || hay.contains("statusbar")
}

/// 就地规范化 `plan`:
/// - 一次性判定 `is_mobile`(根 frame 宽度);
/// - 移动端剔除 plan 自带的状态栏 subtask(状态栏改由 scaffold 注入);
/// - 给每个 subtask 赋 `id_prefix = id`、`parent_frame_id = 根 id`。
pub fn normalize(plan: &mut OrchestratorPlan, _req: &DesignRequest) -> NormInfo {
    let is_mobile = plan.root_frame.width <= MOBILE_MAX_WIDTH;

    if is_mobile {
        plan.subtasks
            .retain(|st| !is_status_bar_subtask(&st.id, &st.label));
    }

    let root_id = plan.root_frame.id.clone();
    for st in &mut plan.subtasks {
        st.id_prefix = st.id.clone();
        st.parent_frame_id = Some(root_id.clone());
    }

    NormInfo { is_mobile }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "x".into(),
            model: None,
            provider: None,
        }
    }

    fn subtask(id: &str, label: &str) -> Subtask {
        Subtask {
            id: id.into(),
            label: label.into(),
            region: Region {
                width: 100.0,
                height: 100.0,
            },
            id_prefix: String::new(),
            parent_frame_id: None,
        }
    }

    fn plan(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "P".into(),
                width,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks,
            style_guide: None,
        }
    }

    #[test]
    fn normalize_assigns_id_prefix_and_parent() {
        let mut p = plan(1200.0, vec![subtask("hero", "Hero"), subtask("feat", "Features")]);
        let info = normalize(&mut p, &req());
        assert!(!info.is_mobile);
        for st in &p.subtasks {
            assert_eq!(st.id_prefix, st.id);
            assert_eq!(st.parent_frame_id.as_deref(), Some("root"));
        }
    }

    #[test]
    fn normalize_flags_mobile_by_width() {
        let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
        let info = normalize(&mut p, &req());
        assert!(info.is_mobile);
    }

    #[test]
    fn normalize_strips_status_bar_subtask_on_mobile() {
        let mut p = plan(
            390.0,
            vec![subtask("status-bar", "Status Bar"), subtask("hero", "Hero")],
        );
        normalize(&mut p, &req());
        assert_eq!(p.subtasks.len(), 1);
        assert_eq!(p.subtasks[0].id, "hero");
    }

    #[test]
    fn normalize_keeps_status_bar_subtask_on_desktop() {
        // 桌面端不剔除(只有移动端 scaffold 注入固定状态栏)。
        let mut p = plan(
            1200.0,
            vec![subtask("status-bar", "Status Bar"), subtask("hero", "Hero")],
        );
        normalize(&mut p, &req());
        assert_eq!(p.subtasks.len(), 2);
    }
}
