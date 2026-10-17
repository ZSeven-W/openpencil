use super::*;
use crate::types::{AbortFlag, AppendContext, ContinuationContext};
use serde_json::json;

const BRIEF: &str = "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.";

fn plan() -> OrchestratorPlan {
    // Exact planner output from the 09 full-pipeline workout regression.
    let mut plan =
        crate::plan::parse_plan(include_str!("test_fixtures/workout_detail_plan_1017.json"))
            .unwrap();
    plan.root_frame.gap = Some(16.0);
    plan
}

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        ..Default::default()
    }
}

fn unchanged(plan: &OrchestratorPlan, req: &DesignRequest) {
    let mut result = plan.clone();
    reserve_supporting_content(&mut result, req);
    assert_eq!(&result, plan, "{}", req.prompt);
}

#[test]
fn real_workout_plan_reserves_three_readable_rows_and_keeps_all_six() {
    let mut plan = plan();
    let list = plan.subtasks[2].clone();
    let cta = plan.subtasks[3].clone();
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan.subtasks[0].region.height, 180.0);
    assert_eq!(plan.subtasks[1].region.height, 160.0);
    assert_eq!(plan.subtasks[2], list);
    assert_eq!(plan.subtasks[3], cta);
    assert!(plan.subtasks[2]
        .elements
        .as_deref()
        .unwrap()
        .contains("six exercise rows"));
    assert!(crate::first_viewport::owns(&plan.subtasks[0]));
    assert!(crate::first_viewport::owns(&plan.subtasks[1]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[2]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[3]));
    let before = plan.clone();
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan, before);
}

#[test]
fn real_taller_action_plan_keeps_usable_photo_budget_instead_of_skipping() {
    let mut plan = plan();
    for (task, height) in plan.subtasks.iter_mut().zip([240.0, 120.0, 344.0, 108.0]) {
        task.region.height = height;
    }
    let list = plan.subtasks[2].clone();
    let cta = plan.subtasks[3].clone();
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan.subtasks[0].region.height, 138.0);
    assert_eq!(plan.subtasks[1].region.height, 160.0);
    assert_eq!(plan.subtasks[2], list);
    assert_eq!(plan.subtasks[3], cta);
    assert!(crate::first_viewport::owns(&plan.subtasks[0]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[2]));
    let before = plan.clone();
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan, before);
}

#[test]
fn normalized_contract_reaches_all_retry_prompts_without_bounding_the_list() {
    let mut plan = plan();
    let req = request(BRIEF);
    crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan.subtasks[0].region.height, 180.0);
    assert_eq!(plan.subtasks[1].region.height, 160.0);
    assert_eq!(plan.subtasks[2].region.height, 286.0);
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        for index in [0, 1] {
            let (call, _) = crate::prompt::build_subagent_prompt(
                &plan.subtasks[index],
                &plan,
                &req,
                AbortFlag::new(),
                reduced,
                minimal,
                &op_editor_core::ComponentLibrary::default(),
            );
            assert!(call.user_prompt.contains("DETAIL-SECTION CONTRACT"));
            assert!(call.user_prompt.contains("first three normal-size rows"));
            assert!(call.user_prompt.contains("do not squeeze all rows"));
            assert!(!call
                .user_prompt
                .contains("Required sections must all be visible"));
        }
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[2],
            &plan,
            &req,
            AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("six exercise rows"));
        assert!(call.user_prompt.contains("Target content amount: ~286px"));
        assert!(!crate::first_viewport::owns(&plan.subtasks[2]));
    }
    let before = plan.clone();
    crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan, before);
}

#[test]
fn native_measurement_rejects_overgrown_hero_and_summary_but_not_full_list() {
    let mut plan = plan();
    reserve_supporting_content(&mut plan, &request(BRIEF));
    for (index, height, rejected) in [
        (0, 180, false),
        (0, 208, true),
        (1, 139, false),
        (1, 170, true),
        (2, 445, false),
    ] {
        let nodes: Vec<jian_ops_schema::node::PenNode> = serde_json::from_value(json!([{
            "type":"frame", "id":"section", "width":"fill_container", "height":"fit_content",
            "layout":"vertical", "children":[{"type":"rectangle", "id":"body", "width":"fill_container", "height":height}]
        }])).unwrap();
        let reason = crate::first_viewport::rejection(
            &plan.subtasks[index],
            &nodes,
            &op_editor_core::EditorState::new(),
        );
        assert_eq!(
            reason.is_some(),
            rejected,
            "section {index}, height {height}: {reason:?}"
        );
    }
}

#[test]
fn authored_photo_size_and_story_compositions_are_not_rewritten() {
    for exception in [
        "hero image height 330px",
        "hero image 40% of screen",
        "hero image 16:9 aspect ratio",
        "主图高度330像素",
        "主图占40%",
        "full-screen hero photo",
        "photo gallery",
        "photo album",
        "editorial narrative",
        "参考设计",
        "replicate the reference",
        "照片轮播",
        "large hero image",
        "large photo",
        "大图展示",
        "超大主图",
        "hero height 330",
        "hero image 375×330",
        "hero image uses half the screen",
        "a 330px-tall hero image",
        "330像素高的主图",
        "a dominant full-bleed photo",
        "keep a tall hero image",
        "主图要大",
    ] {
        unchanged(&plan(), &request(&format!("{BRIEF} {exception}")));
    }
    for prompt in [
        "Image-led product detail with a large photo and narrative story",
        "Workout detail with title and a start button pinned at the bottom",
        "Workout detail with hero image and six exercise list rows",
        "Workout detail with hero image, six exercise list rows and a button",
    ] {
        unchanged(&plan(), &request(prompt));
    }
}

#[test]
fn negated_pinning_in_brief_or_plan_preserves_the_original_detail_composition() {
    for action in [
        "a bottom button that is not pinned and scrolls with the list",
        "a bottom button that is not fixed and scrolls with the list",
        "a bottom button that is not sticky and scrolls with the list",
        "底部按钮，不固定，随正文滚动",
        "底部按钮，不要固定，随正文滚动",
        "底部按钮，非固定，随正文滚动",
    ] {
        let brief = BRIEF.replace("a start button pinned at the bottom", action);
        // Either source can override a stale positive pinning instruction.
        unchanged(&plan(), &request(&brief));
        let mut unpinned = plan();
        unpinned.subtasks[3].label = "Start Button".into();
        unpinned.subtasks[3].elements = Some(action.into());
        unchanged(&unpinned, &request(BRIEF));
        unchanged(&unpinned, &request(&brief));
    }
}

#[test]
fn existing_canvas_and_non_mobile_roots_remain_unchanged() {
    let mut req = request(BRIEF);
    req.append_context = Some(AppendContext {
        target_parent_id: "existing".into(),
        target_width: 375.0,
        existing_section_labels: vec![],
        is_mobile: true,
    });
    unchanged(&plan(), &req);
    req.append_context = None;
    req.continuation_context = Some(ContinuationContext {
        screen_width: 375.0,
        screen_height: 812.0,
        background_color: None,
        screen_names: vec!["detail".into()],
    });
    unchanged(&plan(), &req);
    req.continuation_context = None;
    req.reference_skeleton = Some(serde_json::from_value(json!({"source":"reference", "width":375, "sections":[], "navKind":"none", "heroKind":"imageLed", "columnRhythm":[]})).unwrap());
    unchanged(&plan(), &req);
    for (width, height) in [
        (1440.0, 900.0),
        (375.0, 1400.0),
        (375.0, 500.0),
        (300.0, 812.0),
    ] {
        let mut plan = plan();
        plan.root_frame.width = width;
        plan.root_frame.height = height;
        unchanged(&plan, &request(BRIEF));
    }
}

#[test]
fn feasibility_is_atomic_and_usable_existing_sizes_are_preserved() {
    let req = request(BRIEF);
    let mut too_small = plan();
    too_small.root_frame.height = 740.0;
    unchanged(&too_small, &req);
    let mut tall_summary = plan();
    tall_summary.subtasks[1].region.height = 260.0;
    unchanged(&tall_summary, &req);
    let mut tiny_hero = plan();
    tiny_hero.subtasks[0].region.height = 100.0;
    unchanged(&tiny_hero, &req);
    let mut roomy = plan();
    roomy.root_frame.height = 900.0;
    roomy.subtasks[1].region.height = 180.0;
    reserve_supporting_content(&mut roomy, &req);
    assert_eq!(roomy.subtasks[0].region.height, 208.0);
    assert_eq!(roomy.subtasks[1].region.height, 180.0);
    let mut compact = plan();
    compact.subtasks[0].region.height = 168.0;
    reserve_supporting_content(&mut compact, &req);
    assert_eq!(compact.subtasks[0].region.height, 168.0);
}

#[test]
fn ambiguous_or_combined_sections_do_not_trigger_an_automatic_budget() {
    let req = request(BRIEF);
    let mut combined = plan();
    combined.subtasks[1].label = "Hero title and meta".into();
    unchanged(&combined, &req);
    let mut reversed = plan();
    reversed.subtasks.swap(1, 2);
    unchanged(&reversed, &req);
    let mut no_count = plan();
    no_count.subtasks[2].elements = Some("A list of exercise rows".into());
    unchanged(&no_count, &req);
    let mut two_heroes = plan();
    let mut extra = two_heroes.subtasks[0].clone();
    extra.id = "secondHero".into();
    two_heroes.subtasks.insert(1, extra);
    unchanged(&two_heroes, &req);
    let mut extra_section = plan();
    let mut extra = extra_section.subtasks[1].clone();
    extra.id = "extra".into();
    extra.label = "Membership offer".into();
    extra_section.subtasks.insert(0, extra);
    unchanged(&extra_section, &req);
}

#[test]
fn mobile_screen_groups_receive_independent_budgets() {
    let mut plan = plan();
    for task in &mut plan.subtasks {
        task.screen = Some("workout".into());
    }
    let mut second = plan.subtasks.clone();
    for task in &mut second {
        task.id = format!("other-{}", task.id);
        task.screen = Some("session".into());
    }
    second[1].region.height = 176.0;
    plan.subtasks.extend(second);
    let mut unrelated = plan.subtasks[2].clone();
    unrelated.screen = Some("profile".into());
    unrelated.region.height = 700.0;
    plan.subtasks.push(unrelated.clone());
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan.subtasks[0].region.height, 180.0);
    assert_eq!(plan.subtasks[4].region.height, 164.0);
    assert_eq!(plan.subtasks[5].region.height, 176.0);
    assert_eq!(plan.subtasks[8], unrelated);
}

#[test]
fn a_compact_header_is_reserved_without_becoming_a_measured_section() {
    let mut plan = plan();
    plan.root_frame.height = 900.0;
    let mut header = plan.subtasks[1].clone();
    header.id = "header".into();
    header.label = "Navigation header".into();
    header.elements = None;
    header.region.height = 48.0;
    plan.subtasks.insert(0, header.clone());
    reserve_supporting_content(&mut plan, &request(BRIEF));
    assert_eq!(plan.subtasks[0], header);
    assert_eq!(plan.subtasks[1].region.height, 204.0);
    assert!(!crate::first_viewport::owns(&plan.subtasks[0]));
}

#[test]
fn explicit_all_content_contract_takes_precedence_over_partial_row_budget() {
    let mut plan = plan();
    let req = request(&format!(
        "{BRIEF} First screen must include all six exercise rows."
    ));
    crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan.subtasks[1].region.height, 130.0);
    assert!(crate::first_viewport::owns(&plan.subtasks[2]));
    assert!(
        crate::first_viewport::instruction(&plan.subtasks[0]).contains("FIRST-VIEWPORT CONTRACT")
    );
    assert!(
        !crate::first_viewport::instruction(&plan.subtasks[0]).contains("DETAIL-SECTION CONTRACT")
    );
    let before = plan.clone();
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan, before);
}
