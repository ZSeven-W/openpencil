//! Guard the assembled model messages, including poisoned planner suggestions
//! and the fallback paths which discard almost every optional skill.

use super::*;
use crate::plan::RetryFeedback;
use prompt_source_facts::SOURCE_FACTS_CONTRACT;

const DELIVERY_BRIEF: &str = "请设计一套可编辑的高保真手机 App 界面（mobile app，375×812）。交付 1 页界面。\n\n用户需求：设计一页手机外卖配送追踪详情。品牌“晚点食堂”。\n预计12分钟送达、骑手李师傅及联系按钮；订单为双人晚餐，送到滨江路88号。";
const PLANNER_ADDITION: &str = "骑手李师傅 + 评分标签 4.9 + 已完成3200单";

fn delivery_fixture(brief: &str) -> (DesignRequest, OrchestratorPlan, Subtask) {
    let mut request = req();
    request.prompt = brief.into();
    request.model = Some("glm-5.3-flash".into());
    let mut proposal = plan();
    proposal.root_frame.width = 375.0;
    proposal.root_frame.height = 812.0;
    proposal.style_guide_name = Some("finance-clean-mobile-light".into());
    let mut section = subtask();
    section.label = "配送状态与骑手信息".into();
    section.region.width = 375.0;
    section.elements = Some(PLANNER_ADDITION.into());
    proposal.subtasks = vec![section.clone()];
    (request, proposal, section)
}

fn assert_system_contract_is_last(call: &CallRequest) {
    assert!(call.system_prompt.ends_with(SOURCE_FACTS_CONTRACT));
    assert_eq!(
        call.system_prompt
            .matches("SOURCE FACTS — CONTENT AUTHORITY:")
            .count(),
        1
    );
    assert!(!call
        .user_prompt
        .contains("SOURCE FACTS — CONTENT AUTHORITY:"));
    assert!(!call
        .system_prompt
        .contains("Generate EVERY row/card/item with realistic values"));
}

fn assert_original_brief_is_unchanged(call: &CallRequest, brief: &str) {
    let original = call
        .user_prompt
        .split_once("ORIGINAL USER BRIEF:\n")
        .unwrap()
        .1
        .split_once("\nEND ORIGINAL USER BRIEF")
        .unwrap()
        .0;
    assert_eq!(original, brief);
}

#[test]
fn source_facts_contract_is_system_tail_in_all_planning_modes() {
    let (request, _, _) = delivery_fixture(DELIVERY_BRIEF);
    for mode in [
        PlanningMode::Rich,
        PlanningMode::Minimal,
        PlanningMode::Compact,
    ] {
        let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
        assert_system_contract_is_last(&planning.call_request);
        assert!(planning.call_request.user_prompt.contains(DELIVERY_BRIEF));
        assert!(planning
            .call_request
            .system_prompt
            .contains("do not plan it"));
        assert!(planning
            .call_request
            .system_prompt
            .contains("omit the entire extra label/badge/value"));
    }
}

#[test]
fn source_facts_outrank_poisoned_elements_on_every_tier_and_retry_path() {
    let (mut request, proposal, mut section) = delivery_fixture(DELIVERY_BRIEF);
    for (model, tier) in [
        ("glm-4.6", ModelTier::Basic),
        ("gpt-4o", ModelTier::Standard),
        ("glm-5.3-flash", ModelTier::Full),
    ] {
        assert_eq!(resolve_model_profile(model).tier, tier);
        request.model = Some(model.into());
        for (reduced, minimal, feedback) in [
            (false, false, None),
            (
                false,
                false,
                Some(RetryFeedback::Completeness(
                    "Keep every planned metadata item".into(),
                )),
            ),
            (true, false, None),
            (
                true,
                true,
                Some(RetryFeedback::Geometry("Rider row overflows".into())),
            ),
        ] {
            section.retry_feedback = feedback;
            let (call, _) = bsp(
                &section,
                &proposal,
                &request,
                AbortFlag::new(),
                reduced,
                minimal,
            );
            assert_system_contract_is_last(&call);
            assert_original_brief_is_unchanged(&call, DELIVERY_BRIEF);
            assert!(call
                .user_prompt
                .contains("PLANNER PROPOSAL — Page sections:"));
            assert!(call.user_prompt.contains(&format!(
                "PROPOSED ELEMENTS (planner layout suggestions, not source facts): {PLANNER_ADDITION}"
            )));
            assert!(call
                .system_prompt
                .contains("prior model output are NOT factual sources or permission"));
            assert!(call.system_prompt.contains("complete promised items"));
            assert!(call
                .system_prompt
                .contains("omit the entire extra label/badge/value"));
            assert!(call
                .system_prompt
                .contains("or demo content authorized by the SOURCE FACTS contract"));
            if matches!(section.retry_feedback, Some(RetryFeedback::Completeness(_))) {
                assert!(call.user_prompt.contains("COMPLETENESS FIX REQUIRED"));
            }
        }
    }
}

#[test]
fn source_facts_preserve_supplied_samples_and_requested_missing_fields() {
    for (brief, required_contract) in [
        (
            "岸边咖啡地点详情，评分4.8，地址滨江路128号。",
            "Preserve business names, values and claims supplied",
        ),
        (
            "按我提供的参考稿文案复刻：\n岸边咖啡\n评分 4.8\n08:00–21:00",
            "user-provided source/reference content the user asks to reproduce",
        ),
        (
            "做骑手资料示例，姓名李师傅；其余明确使用虚构示例数据，并标注示例。",
            "Explicit requests for sample/fictional data allow it",
        ),
        (
            "骑手资料保留姓名和评分字段。姓名李师傅，评分还没填写；不要示例数据。",
            "preserve that field and its layout with an honest placeholder",
        ),
    ] {
        let (request, proposal, section) = delivery_fixture(brief);
        let (call, _) = bsp(
            &section,
            &proposal,
            &request,
            AbortFlag::new(),
            false,
            false,
        );
        assert_system_contract_is_last(&call);
        assert_original_brief_is_unchanged(&call, brief);
        assert!(call.system_prompt.contains(required_contract));
        assert!(call
            .system_prompt
            .contains("A prohibition on sample data is not permission"));
        assert!(call.system_prompt.contains(
            "Preserve supplied values and explicit no-invention, empty-state or unfilled-field instructions"
        ));
        assert!(call
            .system_prompt
            .contains("\"暂无评分\" that asserts a business fact"));
        assert!(call
            .system_prompt
            .contains("content-free reference skeletons"));
        assert!(call.system_prompt.contains("not visual invention"));
        for mode in [
            PlanningMode::Rich,
            PlanningMode::Minimal,
            PlanningMode::Compact,
        ] {
            let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
            assert_system_contract_is_last(&planning.call_request);
            assert!(planning.call_request.user_prompt.contains(brief));
            assert!(planning
                .call_request
                .system_prompt
                .contains(required_contract));
        }
    }
}

#[test]
fn source_facts_contract_also_covers_legacy_node_protocol() {
    let (request, proposal, section) = delivery_fixture(DELIVERY_BRIEF);
    let (call, _) = prompt_subagent::build_subagent_prompt_core(
        &section,
        &proposal,
        &request,
        AbortFlag::new(),
        true,
        true,
        false,
        false,
        &ComponentLibrary::default(),
        &[],
    );
    assert_system_contract_is_last(&call);
    assert_original_brief_is_unchanged(&call, DELIVERY_BRIEF);
    assert!(call
        .system_prompt
        .contains("Syntax example only, not factual source data"));
    assert!(!call
        .system_prompt
        .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"));
}

#[test]
fn source_facts_keep_requested_concept_tables_and_food_lists_populated() {
    // Original arena briefs request populated structures without the word
    // "sample". The contract must allow illustrative content within that scope.
    for (width, brief) in [
        (1440.0, "Admin console (1440×900): sidebar with grouped nav, header with search and avatar, filter bar with three selects, a users table of 10 rows × 6 columns (status pills, row actions), pagination footer."),
        (375.0, "外卖 App 三个页面（各 375×812，每页一个独立屏幕）：首页（地址+搜索+分类九宫格+商家列表五个）、商家详情页（头图+评分行+菜品列表六个带价格与加购按钮+底部购物车栏）、订单确认页（地址卡+商品清单+费用明细+提交按钮）。三页必须齐全。"),
    ] {
        let (request, mut proposal, mut section) = delivery_fixture(brief);
        proposal.root_frame.width = width;
        section.region.width = width;
        let (call, _) = bsp(&section, &proposal, &request, AbortFlag::new(), false, false);
        assert_system_contract_is_last(&call);
        assert_original_brief_is_unchanged(&call, brief);
        for required in [
            "authorizes representative demo content for those requested components",
            "even without the word \"sample\"",
            "one quiet group/page note, not a badge on every item",
            "This never authorizes extra unrequested metadata",
            "these override demo-content permission",
        ] {
            assert!(call.system_prompt.contains(required));
            for mode in [PlanningMode::Rich, PlanningMode::Minimal, PlanningMode::Compact] {
                let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
                assert_system_contract_is_last(&planning.call_request);
                assert!(planning.call_request.system_prompt.contains(required));
                assert!(planning.call_request.user_prompt.contains(brief));
            }
        }
    }
}
