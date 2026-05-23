//! plan 规范化 —— 单屏路径。
//!
//! 行为忠实 TS,但**一次性算分类、干净派生**,不做 TS 那种
//! in-place strip-then-reclassify(`orchestrator.ts:838-845` 自标
//! fragile)。

use crate::dashboard_columns::{
    infer_dashboard_section_height, infer_dashboard_section_width, is_dashboard_like_prompt,
};
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
/// - 给每个 subtask 赋 `id_prefix = id`、`parent_frame_id = 根 id`;
/// - dashboard-like plan:每个 subtask 的 region 用推断宽高覆写
///   (宽度无条件覆写;高度在 [inferred*0.6, inferred*1.6] 窗口内保留
///   LLM 值,超出则取推断值 —— 忠实 TS `normalizeOrchestratorPlan`
///   `orchestrator.ts:259-272`)。
pub fn normalize(plan: &mut OrchestratorPlan, req: &DesignRequest) -> NormInfo {
    let is_mobile = plan.root_frame.width <= MOBILE_MAX_WIDTH;

    if is_mobile {
        plan.subtasks
            .retain(|st| !is_status_bar_subtask(&st.id, &st.label));
    }

    let root_width = plan.root_frame.width;
    let dashboard_like = is_dashboard_like_prompt(&req.prompt, plan);

    let root_id = plan.root_frame.id.clone();
    for st in &mut plan.subtasks {
        st.id_prefix = st.id.clone();
        st.parent_frame_id = Some(root_id.clone());

        if dashboard_like {
            let inferred_width = infer_dashboard_section_width(st, root_width);
            let inferred_height = infer_dashboard_section_height(st);

            st.region.width = inferred_width;

            if st.region.height <= 0.0 {
                st.region.height = inferred_height;
            } else {
                let min_height = (inferred_height * 0.6).round();
                let max_height = (inferred_height * 1.6).round();
                st.region.height = f64::max(min_height, f64::min(st.region.height, max_height));
            }
        }
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
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
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
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
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
            style_guide_name: None,
        }
    }

    #[test]
    fn normalize_assigns_id_prefix_and_parent() {
        let mut p = plan(
            1200.0,
            vec![subtask("hero", "Hero"), subtask("feat", "Features")],
        );
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

    // -----------------------------------------------------------------------
    // Task C1 — dashboard branch
    // -----------------------------------------------------------------------

    fn dash_req(prompt: &str) -> DesignRequest {
        DesignRequest {
            prompt: prompt.into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
        }
    }

    /// Build a dashboard-like plan with a sidebar + two data subtasks.
    fn dashboard_plan(root_width: f64) -> OrchestratorPlan {
        let st_sidebar = Subtask {
            id: "sidebar".into(),
            label: "Sidebar Navigation".into(),
            region: Region {
                width: 100.0,
                height: 500.0,
            },
            id_prefix: String::new(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        };
        // chart subtask — LLM-provided height 300, within [inferred*0.6, inferred*1.6]
        // inferred for "chart" = 320  →  min=192, max=512  →  300 in range → keep 300
        let st_chart = Subtask {
            id: "revenue-chart".into(),
            label: "Revenue Chart".into(),
            region: Region {
                width: 800.0,
                height: 300.0,
            },
            id_prefix: String::new(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        };
        // metric subtask — LLM-provided height 0 (invalid) → use inferred = 160
        let st_metric = Subtask {
            id: "kpi-metrics".into(),
            label: "KPI Metrics".into(),
            region: Region {
                width: 800.0,
                height: 0.0,
            },
            id_prefix: String::new(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
        };
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Dashboard".into(),
                width: root_width,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![st_sidebar, st_chart, st_metric],
            style_guide_name: None,
        }
    }

    #[test]
    fn normalize_dashboard_rewrites_width_per_infer() {
        // root_width = 1200  →  main_width = max(320, 1200-260) = 940
        // sidebar → 260
        // chart/revenue → round(940 * 0.62) = round(582.8) = 583
        // metric (default) → 940
        let mut p = dashboard_plan(1200.0);
        let req = dash_req("an analytics admin dashboard");
        normalize(&mut p, &req);

        let sidebar = p.subtasks.iter().find(|s| s.id == "sidebar").unwrap();
        assert_eq!(sidebar.region.width, 260.0, "sidebar width must be 260");

        let chart = p.subtasks.iter().find(|s| s.id == "revenue-chart").unwrap();
        let expected_chart_w = (940.0_f64 * 0.62).round();
        assert_eq!(
            chart.region.width, expected_chart_w,
            "chart width = main*0.62"
        );

        let metric = p.subtasks.iter().find(|s| s.id == "kpi-metrics").unwrap();
        assert_eq!(metric.region.width, 940.0, "metric width = full main");
    }

    #[test]
    fn normalize_dashboard_height_kept_when_in_range() {
        // chart: inferred=320, LLM=300
        // min=round(320*0.6)=192, max=round(320*1.6)=512
        // 300 ∈ [192,512] → keep 300
        let mut p = dashboard_plan(1200.0);
        let req = dash_req("analytics admin dashboard");
        normalize(&mut p, &req);

        let chart = p.subtasks.iter().find(|s| s.id == "revenue-chart").unwrap();
        assert_eq!(
            chart.region.height, 300.0,
            "LLM height kept when in clamp range"
        );
    }

    #[test]
    fn normalize_dashboard_height_replaced_when_zero() {
        // metric: LLM height=0 (invalid) → use inferred=160
        let mut p = dashboard_plan(1200.0);
        let req = dash_req("data analytics admin dashboard");
        normalize(&mut p, &req);

        let metric = p.subtasks.iter().find(|s| s.id == "kpi-metrics").unwrap();
        assert_eq!(
            metric.region.height, 160.0,
            "zero LLM height replaced by inferred"
        );
    }

    #[test]
    fn normalize_dashboard_height_clamped_to_max() {
        // "customer-table" subtask: id+label match "table" (no "transaction"),
        // so inferred = 340  →  max=round(340*1.6)=544
        // max(round(340*0.6), min(2000, 544)) = max(204, 544) = 544
        let mut p = OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Dashboard".into(),
                width: 1200.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![Subtask {
                id: "customer-table".into(),
                label: "Customer Table".into(),
                region: Region {
                    width: 800.0,
                    height: 2000.0,
                },
                id_prefix: String::new(),
                parent_frame_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
            }],
            style_guide_name: None,
        };
        normalize(&mut p, &dash_req("data analytics admin dashboard"));
        assert_eq!(
            p.subtasks[0].region.height,
            (340.0_f64 * 1.6).round(),
            "over-tall LLM height clamped to max"
        );
    }

    #[test]
    fn normalize_dashboard_height_clamped_to_min() {
        // chart subtask with too-short LLM height (50)
        // inferred=320  →  min=round(320*0.6)=192, max=round(320*1.6)=512
        // max(192, min(50, 512)) = max(192, 50) = 192
        let mut p = OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "Dashboard".into(),
                width: 1200.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![Subtask {
                id: "rev-chart".into(),
                label: "Revenue Chart".into(),
                region: Region {
                    width: 800.0,
                    height: 50.0,
                },
                id_prefix: String::new(),
                parent_frame_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
            }],
            style_guide_name: None,
        };
        normalize(&mut p, &dash_req("data analytics admin dashboard"));
        assert_eq!(
            p.subtasks[0].region.height,
            (320.0_f64 * 0.6).round(),
            "too-short LLM height clamped to min"
        );
    }

    #[test]
    fn normalize_non_dashboard_plan_is_unaffected() {
        // A landing page prompt — NOT dashboard-like — must not rewrite regions.
        let mut p = plan(
            1200.0,
            vec![
                subtask("hero", "Hero Section"),
                subtask("features", "Feature Cards"),
            ],
        );
        // Give them non-zero regions so default-fill doesn't fire.
        p.subtasks[0].region = Region {
            width: 1200.0,
            height: 560.0,
        };
        p.subtasks[1].region = Region {
            width: 1200.0,
            height: 400.0,
        };

        normalize(
            &mut p,
            &dash_req("a beautiful landing page for a SaaS product"),
        );

        assert_eq!(p.subtasks[0].region.width, 1200.0);
        assert_eq!(p.subtasks[0].region.height, 560.0);
        assert_eq!(p.subtasks[1].region.width, 1200.0);
        assert_eq!(p.subtasks[1].region.height, 400.0);
    }
}
