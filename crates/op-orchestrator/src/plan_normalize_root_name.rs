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
}
