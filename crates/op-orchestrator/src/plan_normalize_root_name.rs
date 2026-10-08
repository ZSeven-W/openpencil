//! Keep the board's name in the brief's language.
//!
//! The planning corpus shows its root in English (`"name":"Page"`), and a
//! planner answering a Chinese brief sometimes copies that register: a
//! 咖啡店小程序首页 brief landed a board named "Coffee Shop Home", which the
//! side-by-side directions view prints as the board's title. When the brief
//! is Chinese and the planned name carries no Han character at all, name the
//! board after the brief the same way a conversation is titled.

use crate::output_language::{brief_language, Lang};
use crate::plan::OrchestratorPlan;
use crate::plan_coverage::is_han;
use crate::types::DesignRequest;

pub(super) fn localize_root_name(plan: &mut OrchestratorPlan, req: &DesignRequest) {
    // A fallback title copied from Home's trusted wrapper is implementation
    // prose, not a work name. Its Chinese characters used to bypass the guard
    // below and leave names like "做一套图文卡片（card" on the canvas.
    if req.prompt.starts_with("请做一套图文卡片（card")
        && ["请做一套图文卡片", "做一套图文卡片"]
            .iter()
            .any(|prefix| plan.root_frame.name.starts_with(prefix))
        && (plan.root_frame.name.contains("（card") || plan.root_frame.name.contains("(card"))
    {
        plan.root_frame.name = "图文卡片".into();
        return;
    }
    if brief_language(&req.prompt) != Some(Lang::Cjk) || plan.root_frame.name.chars().any(is_han) {
        return;
    }
    if let Some(title) = op_editor_core::suggest_chat_title(&req.prompt) {
        // A brief that lists its sections after a colon names the screen
        // before it: `咖啡店小程序首页：顶部门店信息、…`.
        let title = title
            .split(['：', ':'])
            .next()
            .map(str::trim)
            .filter(|head| !head.is_empty())
            .unwrap_or(title.as_str())
            .to_string();
        if title.chars().any(is_han) {
            plan.root_frame.name = title;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::RootFrameSpec;

    fn plan(name: &str) -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: name.into(),
                width: 375.0,
                height: 812.0,
                layout: Some("vertical".into()),
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: Vec::new(),
            style_guide_name: None,
        }
    }

    fn req(prompt: &str) -> DesignRequest {
        DesignRequest {
            prompt: prompt.into(),
            ..DesignRequest::default()
        }
    }

    #[test]
    fn an_english_board_name_under_a_chinese_brief_takes_the_brief_title() {
        let mut p = plan("Coffee Shop Home");
        localize_root_name(
            &mut p,
            &req("设计一个咖啡店小程序首页：顶部门店信息、今日推荐饮品卡片、积分进度、底部导航"),
        );
        assert_eq!(p.root_frame.name, "咖啡店小程序首页");
    }

    #[test]
    fn a_chinese_name_or_an_english_brief_is_left_alone() {
        let mut p = plan("咖啡店首页");
        localize_root_name(&mut p, &req("设计一个咖啡店小程序首页"));
        assert_eq!(p.root_frame.name, "咖啡店首页");
        let mut p = plan("Coffee Shop Home");
        localize_root_name(&mut p, &req("Design a coffee shop home screen"));
        assert_eq!(p.root_frame.name, "Coffee Shop Home");
    }

    #[test]
    fn a_copied_home_preamble_is_a_work_name_without_the_schema_fragment() {
        let request =
            req("请做一套图文卡片（card，竖版3:4，多页轮播）。保持统一排版。\n用户需求：咖啡小聚");
        let mut p = plan("做一套图文卡片（card");
        localize_root_name(&mut p, &request);
        assert_eq!(p.root_frame.name, "图文卡片");
        let mut p = plan("咖啡小聚");
        localize_root_name(&mut p, &request);
        assert_eq!(p.root_frame.name, "咖啡小聚");
    }
}
