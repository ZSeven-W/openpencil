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
    assert!(!call.system_prompt.contains("quiet group/page note"));
}

fn assert_no_contract(call: &CallRequest) {
    assert!(
        !call
            .system_prompt
            .contains("SOURCE FACTS — CONTENT AUTHORITY:"),
        "{}",
        call.system_prompt
    );
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
            assert!(call.system_prompt.contains(
                "use honest placeholders only where the SOURCE FACTS contract requires them"
            ));
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
            "按我提供的参考稿文案复刻：\n岸边咖啡\n评分 4.8\n08:00–21:00",
            "user-provided source/reference content the user asks to reproduce",
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
fn briefs_without_detectable_source_content_skip_the_contract() {
    // Supplied-looking values with no reusable detection (no brand label, no
    // reference cue, no money/clock/distance value, no no-invention wording)
    // no longer carry the contract; the original brief still reaches the
    // model unchanged and the SHARED CONTENT DECISIONS instruction still
    // demands verbatim sourceFacts excerpts.
    for brief in [
        "岸边咖啡地点详情，评分4.8，地址滨江路128号。",
        "做骑手资料示例，姓名李师傅；其余明确使用虚构示例数据。",
        "a pricing page",
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
        assert_no_contract(&call);
        assert_original_brief_is_unchanged(&call, brief);
        for mode in [
            PlanningMode::Rich,
            PlanningMode::Minimal,
            PlanningMode::Compact,
        ] {
            let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
            assert_no_contract(&planning.call_request);
            assert!(planning.call_request.user_prompt.contains(brief));
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
fn conceptual_ui_briefs_get_no_sample_data_contract_or_notes() {
    // Exact October regression briefs: conceptual screens with requested
    // populated structures but zero user source content. The unconditional
    // contract made these render "Sample data" / "Sample workout data for
    // preview" captions (ab/codex-regress-1019 app-03, app-18).
    for (width, brief) in [
        (1440.0, "Admin console (1440×900): sidebar with grouped nav, header with search and avatar, filter bar with three selects, a users table of 10 rows × 6 columns (status pills, row actions), pagination footer."),
        (375.0, "Mobile banking home (375×812): a balance card with gradient, four quick actions, a spending donut chart with legend, a transaction list of six items, bottom tab bar."),
        (375.0, "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom."),
        (375.0, "外卖 App 三个页面（各 375×812，每页一个独立屏幕）：首页（地址+搜索+分类九宫格+商家列表五个）、商家详情页（头图+评分行+菜品列表六个带价格与加购按钮+底部购物车栏）、订单确认页（地址卡+商品清单+费用明细+提交按钮）。三页必须齐全。"),
    ] {
        let (request, mut proposal, section) = delivery_fixture(brief);
        proposal.root_frame.width = width;
        let (call, _) = bsp(&section, &proposal, &request, AbortFlag::new(), false, false);
        assert_no_contract(&call);
        assert_original_brief_is_unchanged(&call, brief);
        for mode in [PlanningMode::Rich, PlanningMode::Minimal, PlanningMode::Compact] {
            let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
            assert_no_contract(&planning.call_request);
            assert!(planning.call_request.user_prompt.contains(brief));
        }
    }
}

#[test]
fn supplied_business_values_append_the_contract() {
    // Prices/clock times reuse the delivery value shapes: the brief itself
    // supplies data the design must preserve verbatim.
    for brief in [
        "晴日咖啡，静安店，到店自取；拿铁28元、美式22元、燕麦拿铁32元",
        "Coffee shop detail, open 08:00–21:00, 1.2km away",
        "预约课程：私教课￥199/节，场馆3.5公里内免费接送",
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
        assert!(call
            .system_prompt
            .contains("Preserve business names, values and claims supplied"));
        for mode in [
            PlanningMode::Rich,
            PlanningMode::Minimal,
            PlanningMode::Compact,
        ] {
            let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
            assert!(planning
                .call_request
                .system_prompt
                .contains("SOURCE FACTS — CONTENT AUTHORITY:"));
        }
    }
}

#[test]
fn source_facts_supplied_identifiers_do_not_authorize_extra_card_metadata() {
    // Bare identifier/name mentions carry no detectable source: no contract.
    let brief =
        "配送详情，订单号 OP-20261015-088，请在页头和订单卡中展示。骑手卡包含李师傅和联系按钮。";
    let (request, proposal, section) = delivery_fixture(brief);
    let (call, _) = bsp(
        &section,
        &proposal,
        &request,
        AbortFlag::new(),
        false,
        false,
    );
    assert_no_contract(&call);
    assert_original_brief_is_unchanged(&call, brief);
    // Once a brand makes the brief source-backed, the identifier clauses
    // still forbid borrowing the order number into unrelated cards.
    let branded =
        "配送详情，品牌“晚点食堂”，订单号 OP-20261015-088，请在页头和订单卡中展示。骑手卡包含李师傅和联系按钮。";
    let (request, proposal, section) = delivery_fixture(branded);
    let (call, _) = bsp(
        &section,
        &proposal,
        &request,
        AbortFlag::new(),
        false,
        false,
    );
    assert!(call
        .system_prompt
        .contains("copy its exact supplied value only to those locations"));
    assert!(call
        .system_prompt
        .contains("does not belong in the rider/contact card"));
    for mode in [
        PlanningMode::Rich,
        PlanningMode::Minimal,
        PlanningMode::Compact,
    ] {
        let planning = build_orchestrator_prompt(&request, mode, AbortFlag::new());
        assert_system_contract_is_last(&planning.call_request);
        assert!(planning
            .call_request
            .system_prompt
            .contains("does not authorize duplicating it into unrelated sections"));
    }
}

#[test]
fn source_facts_fill_requested_concept_fields_instead_of_placeholders() {
    // A concept brief that supplies a few display values still asks for a
    // designed screen: unsupplied requested fields (schedules, kiln times,
    // forecast rows) get demo content, not "待补充" placeholders.
    let brief = "陶艺工坊 App 两屏（各 375×812）：首页「本周开窑」卡片带倒计时，课程列表三门课（拉坯入门 2 小时 ¥180），我的作品横滑；详情页烧制记录四步时间线。";
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
    for required in [
        "screens to design, not real content to reproduce",
        "schedules, timelines, lists or detail fields",
        "consistent with any supplied values",
        "Only when the user forbids sample/demo data",
        "never renders as \"—\", \"待补充\", \"待填写\" or \"TBD\"",
        "prices, durations, dates, counts and remaining seats/stock",
    ] {
        assert!(call.system_prompt.contains(required), "{required}");
    }
}
