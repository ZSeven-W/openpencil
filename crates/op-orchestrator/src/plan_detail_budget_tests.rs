use super::*;
use crate::types::{AbortFlag, AppendContext, ContinuationContext};
use serde_json::json;

const BRIEF: &str = "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.";
// The row-budget rewrite is an explicit-compact contract: the brief itself
// must ask for a compact/small photo before hero area is traded for list
// rows. A plain "hero image" detail keeps the archetype's authored hero.
const COMPACT_BRIEF: &str = "Workout detail screen (375×812): compact hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.";

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
fn plain_hero_detail_keeps_the_planner_composition() {
    // No compact/small-photo cue: the authored 330px hero (~40% of 812) and
    // the 130px summary survive both the reserve pass and full normalization.
    let req = request(BRIEF);
    let mut plan = plan();
    let before = plan.clone();
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan, before);
    crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan.subtasks[0].region.height, 330.0);
    assert_eq!(plan.subtasks[1].region.height, 130.0);
    assert_eq!(plan.subtasks[2].region.height, 286.0);
    assert_eq!(plan.subtasks[3].region.height, 66.0);
    for task in &plan.subtasks {
        assert!(!crate::first_viewport::owns(task));
    }
    let (call, _) = crate::prompt::build_subagent_prompt(
        &plan.subtasks[0],
        &plan,
        &req,
        AbortFlag::new(),
        false,
        false,
        &op_editor_core::ComponentLibrary::default(),
    );
    assert!(!call.user_prompt.contains("DETAIL-SECTION CONTRACT"));
    let mut merged = merged_plan();
    let merged_before = merged.clone();
    reserve_supporting_content(&mut merged, &req);
    assert_eq!(merged, merged_before);
}

#[test]
fn only_explicit_compact_or_small_photo_cues_arm_the_row_budget() {
    for cue in [
        "compact",
        "compact hero image",
        "small photo at the top",
        "thumbnail header",
        "紧凑",
        "小图",
    ] {
        let brief = format!("{BRIEF} {cue}");
        let mut plan = plan();
        reserve_supporting_content(&mut plan, &request(&brief));
        assert!(
            plan.subtasks[0].region.height < 330.0,
            "cue {cue:?} must arm the budget"
        );
    }
    for prompt in [
        "Workout detail screen with hero image, six exercise list rows and a pinned start button",
        "健身详情页，主图头图加返回按钮，六个动作列表，底部固定开始按钮",
    ] {
        unchanged(&plan(), &request(prompt));
    }
}

#[test]
fn explicit_compact_brief_reserves_three_readable_rows_and_keeps_all_six() {
    let mut plan = plan();
    let list = plan.subtasks[2].clone();
    let cta = plan.subtasks[3].clone();
    let req = request(COMPACT_BRIEF);
    reserve_supporting_content(&mut plan, &req);
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
    reserve_supporting_content(&mut plan, &req);
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
    let req = request(COMPACT_BRIEF);
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan.subtasks[0].region.height, 138.0);
    assert_eq!(plan.subtasks[1].region.height, 160.0);
    assert_eq!(plan.subtasks[2], list);
    assert_eq!(plan.subtasks[3], cta);
    assert!(crate::first_viewport::owns(&plan.subtasks[0]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[2]));
    let before = plan.clone();
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan, before);
}

#[test]
fn normalized_contract_reaches_all_retry_prompts_without_bounding_the_list() {
    let mut plan = plan();
    let req = request(COMPACT_BRIEF);
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
    reserve_supporting_content(&mut plan, &request(COMPACT_BRIEF));
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
        unchanged(&merged_plan(), &request(&format!("{BRIEF} {exception}")));
        unchanged(&plan(), &request(&format!("{COMPACT_BRIEF} {exception}")));
        unchanged(
            &merged_plan(),
            &request(&format!("{COMPACT_BRIEF} {exception}")),
        );
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
        let brief = COMPACT_BRIEF.replace("a start button pinned at the bottom", action);
        // Either source can override a stale positive pinning instruction.
        unchanged(&plan(), &request(&brief));
        let mut unpinned = plan();
        unpinned.subtasks[3].label = "Start Button".into();
        unpinned.subtasks[3].elements = Some(action.into());
        unchanged(&unpinned, &request(COMPACT_BRIEF));
        unchanged(&unpinned, &request(&brief));
    }
}

#[test]
fn existing_canvas_and_non_mobile_roots_remain_unchanged() {
    let mut req = request(COMPACT_BRIEF);
    req.append_context = Some(AppendContext {
        target_parent_id: "existing".into(),
        target_width: 375.0,
        existing_section_labels: vec![],
        is_mobile: true,
    });
    unchanged(&plan(), &req);
    unchanged(&merged_plan(), &req);
    req.append_context = None;
    req.continuation_context = Some(ContinuationContext {
        screen_width: 375.0,
        screen_height: 812.0,
        background_color: None,
        screen_names: vec!["detail".into()],
    });
    unchanged(&plan(), &req);
    unchanged(&merged_plan(), &req);
    req.continuation_context = None;
    req.reference_skeleton = Some(serde_json::from_value(json!({"source":"reference", "width":375, "sections":[], "navKind":"none", "heroKind":"imageLed", "columnRhythm":[]})).unwrap());
    unchanged(&plan(), &req);
    unchanged(&merged_plan(), &req);
    for (width, height) in [
        (1440.0, 900.0),
        (375.0, 1400.0),
        (375.0, 500.0),
        (300.0, 812.0),
    ] {
        let mut plan = plan();
        plan.root_frame.width = width;
        plan.root_frame.height = height;
        unchanged(&plan, &request(COMPACT_BRIEF));
    }
}

#[test]
fn feasibility_is_atomic_and_usable_existing_sizes_are_preserved() {
    let req = request(COMPACT_BRIEF);
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
    // Without the compact cue the same roomy plan is not rewritten.
    let mut plain_roomy = plan();
    plain_roomy.root_frame.height = 900.0;
    plain_roomy.subtasks[1].region.height = 180.0;
    unchanged(&plain_roomy, &request(BRIEF));
    let mut compact = plan();
    compact.subtasks[0].region.height = 168.0;
    reserve_supporting_content(&mut compact, &req);
    assert_eq!(compact.subtasks[0].region.height, 168.0);
}

#[test]
fn ambiguous_or_combined_sections_do_not_trigger_an_automatic_budget() {
    let req = request(COMPACT_BRIEF);
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
    reserve_supporting_content(&mut plan, &request(COMPACT_BRIEF));
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
    reserve_supporting_content(&mut plan, &request(COMPACT_BRIEF));
    assert_eq!(plan.subtasks[0], header);
    assert_eq!(plan.subtasks[1].region.height, 204.0);
    assert!(!crate::first_viewport::owns(&plan.subtasks[0]));
}

#[test]
fn explicit_all_content_contract_takes_precedence_over_partial_row_budget() {
    let mut plan = plan();
    let req = request(&format!(
        "{COMPACT_BRIEF} First screen must include all six exercise rows."
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

fn merged_plan() -> OrchestratorPlan {
    // Exact run-4 planner output: the copy and badges share the hero owner.
    crate::plan::parse_plan(include_str!("test_fixtures/workout_detail_1018_plan.json")).unwrap()
}

#[test]
fn real_merged_hero_reserves_complete_copy_and_three_rows_without_rewriting_items() {
    let mut plan = merged_plan();
    let before = plan.clone();
    // These words describe separate parts of the hero, not a second CTA.
    assert!(has_pinned_action(&task_text(&plan.subtasks[0])));
    assert!(!is_standalone_pinned_action(&plan.subtasks[0]));
    let req = request(COMPACT_BRIEF);
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan.subtasks[0].region.height, 280.0);
    assert_eq!(plan.subtasks[1..], before.subtasks[1..]);
    assert_eq!(plan.root_frame, before.root_frame);
    assert!(plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .starts_with(before.subtasks[0].elements.as_deref().unwrap()));
    assert!(crate::first_viewport::owns(&plan.subtasks[0]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[1]));
    assert!(!crate::first_viewport::owns(&plan.subtasks[2]));
    // 62 system + complete merged hero + list heading + 3 normal rows +
    // original CTA + breathing inset. The remaining rows stay scrollable.
    assert!(
        62.0 + plan.subtasks[0].region.height
            + 40.0
            + 3.0 * 72.0
            + plan.subtasks[2].region.height
            + 24.0
            <= plan.root_frame.height
    );
    let normalized = plan.clone();
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan, normalized);
}

#[test]
fn merged_contract_reaches_every_retry_without_bounding_the_full_list() {
    let mut plan = merged_plan();
    let req = request(COMPACT_BRIEF);
    crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan.subtasks[0].region.height, 280.0);
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &req,
            AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        for instruction in [
            "DETAIL-SECTION CONTRACT",
            "MERGED DETAIL COMPOSITION",
            "complete title, metadata and every difficulty badge",
            "Recompose the photo and copy",
            "shrinking text, deleting badges, clipping overlays or forcing the parent height",
        ] {
            assert!(call.user_prompt.contains(instruction), "{instruction}");
        }
        assert!(!crate::first_viewport::owns(&plan.subtasks[1]));
        assert!(plan.subtasks[1]
            .elements
            .as_deref()
            .unwrap()
            .contains("six exercise rows"));
    }
}

#[test]
fn native_measurement_rejects_a_tall_merged_composition_without_clipping_its_nodes() {
    let mut plan = merged_plan();
    reserve_supporting_content(&mut plan, &request(COMPACT_BRIEF));
    for (photo_height, rejected) in [(120, false), (180, true)] {
        let nodes: Vec<jian_ops_schema::node::PenNode> = serde_json::from_value(json!([{
            "type":"frame", "id":"merged", "layout":"vertical", "gap":12,
            "width":"fill_container", "height":"fit_content", "children":[
                {"type":"frame", "id":"photo", "width":"fill_container", "height":photo_height,
                 "layout":"none", "children":[
                    {"type":"frame", "id":"back", "name":"Back button", "x":16, "y":16,
                     "width":40, "height":40}
                ]},
                {"type":"frame", "id":"summary", "layout":"vertical", "gap":8,
                 "padding":16, "width":"fill_container", "height":"fit_content", "children":[
                    {"type":"text", "id":"title", "content":"Full Body Strength", "fontSize":24,
                     "width":"fill_container", "height":32},
                    {"type":"text", "id":"meta", "content":"45 min · 320 kcal", "fontSize":14,
                     "width":"fill_container", "height":20},
                    {"type":"frame", "id":"badges", "name":"Difficulty Badges", "layout":"horizontal",
                     "gap":8, "width":"fill_container", "height":32, "children":[
                        {"type":"text", "id":"beginner", "content":"Beginner", "fontSize":14,
                         "width":100, "height":32},
                        {"type":"text", "id":"intermediate", "content":"Intermediate", "fontSize":14,
                         "width":120, "height":32}
                     ]}
                 ]}
            ]
        }])).unwrap();
        let original = serde_json::to_value(&nodes).unwrap();
        let reason = crate::first_viewport::rejection(
            &plan.subtasks[0],
            &nodes,
            &op_editor_core::EditorState::new(),
        );
        assert_eq!(reason.is_some(), rejected, "{reason:?}");
        assert_eq!(serde_json::to_value(&nodes).unwrap(), original);
        if let Some(reason) = reason {
            assert!(reason.contains("Do not clip content or force the root height"));
        }
        assert!(crate::first_viewport::rejection(
            &plan.subtasks[1],
            &nodes,
            &op_editor_core::EditorState::new(),
        )
        .is_none());
    }
}

#[test]
fn merged_budget_accounts_for_header_gaps_padding_and_content_floor_atomically() {
    let req = request(COMPACT_BRIEF);
    let mut compact = merged_plan();
    compact.subtasks[0].region.height = 264.0;
    reserve_supporting_content(&mut compact, &req);
    assert_eq!(compact.subtasks[0].region.height, 264.0);
    let mut minimum = merged_plan();
    minimum.root_frame.height = 662.0;
    reserve_supporting_content(&mut minimum, &req);
    assert_eq!(minimum.subtasks[0].region.height, 240.0);
    assert_eq!(62.0 + 240.0 + 40.0 + 3.0 * 72.0 + 80.0 + 24.0, 662.0);
    let mut infeasible = merged_plan();
    infeasible.root_frame.height = 661.0;
    unchanged(&infeasible, &req);
    let mut already_short = merged_plan();
    already_short.subtasks[0].region.height = 220.0;
    unchanged(&already_short, &req);

    let mut with_header = merged_plan();
    let mut header = plan().subtasks[1].clone();
    header.id = "navigation".into();
    header.label = "Navigation Header".into();
    header.elements = Some("compact back navigation".into());
    header.region.height = 48.0;
    with_header.subtasks.insert(0, header.clone());
    with_header.root_frame.gap = Some(16.0);
    with_header.root_frame.padding = Some(12.0);
    reserve_supporting_content(&mut with_header, &req);
    assert_eq!(with_header.subtasks[0], header);
    assert!(!crate::first_viewport::owns(&with_header.subtasks[0]));
    assert_eq!(with_header.subtasks[1].region.height, 254.0);
    assert_eq!(
        62.0 + 48.0 + 4.0 * 16.0 + 24.0 + 254.0 + 40.0 + 216.0 + 80.0 + 24.0,
        812.0
    );
}

#[test]
fn real_four_part_plan_does_not_treat_pinned_back_and_bottom_scrim_as_an_action() {
    // Exact run-5 planner response, including its zero root gap.
    let mut plan = crate::plan::parse_plan(r##"{"rootFrame":{"id":"page","name":"Workout Detail","width":375,"height":812,"layout":"vertical","gap":0,"fill":[{"type":"solid","color":"#0A0A0A"}]},"styleGuideName":"dark-bold-mobile","subtasks":[{"id":"hero","label":"Hero Image with Back Button","covers":["hero image","back button"],"elements":"ARCHETYPE: image-led — protagonist full-bleed hero photo, first screen ~38%, pinned back control top-left, no repeated titles or invented stats, imagery one relevant workout photo with bottom gradient scrim blending into canvas","region":{"width":375,"height":310}},{"id":"title","label":"Workout Title, Meta Row and Difficulty Badges","covers":["workout title and meta row","difficulty badges"],"elements":"workout title (2 lines max, 28-32px), meta row with duration + equipment, difficulty badge group (Beginner/Intermediate/Advanced, one active state)","region":{"width":375,"height":110}},{"id":"exercises","label":"Exercise List","covers":["exercise list"],"elements":"quiet section: secondary visual weight; retain legible active states, list heading with exercise count, 6 exercise rows each with rounded thumbnail, exercise name, sets x reps label, scrollable list","region":{"width":375,"height":328}},{"id":"start","label":"Pinned Start Button","covers":["start button"],"elements":"pinned full-width primary Start Workout button (48-52px) on subtle top gradient, safe-area padding","region":{"width":375,"height":64}}]}"##).unwrap();
    let before = plan.clone();
    assert!(has_pinned_action(&task_text(&plan.subtasks[0])));
    assert!(!has_pinned_action(&task_text(&plan.subtasks[3])));
    // A plain hero brief preserves the authored composition end to end.
    crate::plan_normalize::normalize(&mut plan, &request(BRIEF));
    assert_eq!(plan.root_frame.gap, Some(16.0));
    assert_eq!(plan.root_frame.padding, Some(0.0));
    assert_eq!(plan.subtasks[0].region.height, 310.0);
    assert_eq!(plan.subtasks[1].region.height, 110.0);
    assert_eq!(plan.subtasks[2].region.height, 328.0);
    assert_eq!(plan.subtasks[3].region.height, 64.0);
    for task in &plan.subtasks {
        assert!(!crate::first_viewport::owns(task));
    }
    // Every other raw-plan field is preserved apart from standard ownership
    // metadata and the derived hero-bleed flag.
    let mut expected = before;
    expected.root_frame.gap = Some(16.0);
    expected.root_frame.padding = Some(0.0);
    for task in &mut expected.subtasks {
        task.id_prefix = task.id.clone();
        task.parent_frame_id = Some("page".into());
    }
    expected.subtasks[0].bleed_hero = true;
    assert_eq!(plan, expected);

    // Under an explicit compact cue the same plan gets the row budget: the
    // hero's pinned back control and scrim still do not count as the action.
    let mut compact = crate::plan::parse_plan(r##"{"rootFrame":{"id":"page","name":"Workout Detail","width":375,"height":812,"layout":"vertical","gap":0,"fill":[{"type":"solid","color":"#0A0A0A"}]},"styleGuideName":"dark-bold-mobile","subtasks":[{"id":"hero","label":"Hero Image with Back Button","covers":["hero image","back button"],"elements":"ARCHETYPE: image-led — protagonist full-bleed hero photo, first screen ~38%, pinned back control top-left, no repeated titles or invented stats, imagery one relevant workout photo with bottom gradient scrim blending into canvas","region":{"width":375,"height":310}},{"id":"title","label":"Workout Title, Meta Row and Difficulty Badges","covers":["workout title and meta row","difficulty badges"],"elements":"workout title (2 lines max, 28-32px), meta row with duration + equipment, difficulty badge group (Beginner/Intermediate/Advanced, one active state)","region":{"width":375,"height":110}},{"id":"exercises","label":"Exercise List","covers":["exercise list"],"elements":"quiet section: secondary visual weight; retain legible active states, list heading with exercise count, 6 exercise rows each with rounded thumbnail, exercise name, sets x reps label, scrollable list","region":{"width":375,"height":328}},{"id":"start","label":"Pinned Start Button","covers":["start button"],"elements":"pinned full-width primary Start Workout button (48-52px) on subtle top gradient, safe-area padding","region":{"width":375,"height":64}}]}"##).unwrap();
    crate::plan_normalize::normalize(&mut compact, &request(COMPACT_BRIEF));
    // Native normalization restores the mobile root's default 16px rhythm.
    // The four gaps must remain reserved rather than granting them to media.
    assert_eq!(compact.root_frame.gap, Some(16.0));
    assert_eq!(compact.root_frame.padding, Some(0.0));
    assert_eq!(compact.subtasks[0].region.height, 182.0);
    assert_eq!(
        62.0 + 4.0 * 16.0 + 182.0 + 160.0 + 40.0 + 216.0 + 64.0 + 24.0,
        812.0
    );
    assert_eq!(compact.subtasks[1].region.height, 160.0);
    assert_eq!(compact.subtasks[2].region.height, 328.0);
    assert_eq!(compact.subtasks[3].region.height, 64.0);
    assert!(crate::first_viewport::owns(&compact.subtasks[0]));
    assert!(crate::first_viewport::owns(&compact.subtasks[1]));
}

#[test]
fn merged_budget_requires_unambiguous_ownership_and_an_independent_terminal_action() {
    let req = request(COMPACT_BRIEF);
    for elements in [
        "Hero image with overlaid title and meta; no badges",
        "Hero image with title and metadata; badges belong to another section",
        "Hero photo; separate title and meta section with difficulty badges",
        "Hero photo with title, metadata and badges",
        "Hero photo with title and meta overlaid; difficulty badges belong to exercise list",
        "Hero photo with title and meta overlaid; difficulty chips are owned by the list",
        "Hero photo with title and meta overlaid; difficulty badges inside exercise list",
        "主图照片叠层包含标题与摘要；难度标签属于训练列表",
    ] {
        let mut missing_owner = merged_plan();
        missing_owner.subtasks[0].elements = Some(elements.into());
        unchanged(&missing_owner, &req);
    }
    let mut no_action = merged_plan();
    no_action.subtasks.pop();
    unchanged(&no_action, &req);
    for (id, label) in [
        ("terminal", "Back Button"),
        ("terminal", "Hero Action"),
        ("terminal", "Photo Overlay Button"),
        ("terminal", "Navigation Button"),
        ("terminal", "BackButton"),
        ("terminal", "backButton"),
        ("terminal", "PhotoOverlayButton"),
        ("backButton", "Button"),
        ("photoOverlayButton", "Button"),
    ] {
        let mut fake_action = merged_plan();
        fake_action.subtasks[2].id = id.into();
        fake_action.subtasks[2].label = label.into();
        fake_action.subtasks[2].elements = Some("pinned top-left back button".into());
        unchanged(&fake_action, &req);
    }
    let mut duplicate_action = merged_plan();
    let mut extra = duplicate_action.subtasks[2].clone();
    extra.id = "other-action".into();
    duplicate_action.subtasks.insert(2, extra);
    unchanged(&duplicate_action, &req);
    let mut nonterminal = merged_plan();
    nonterminal.subtasks.swap(1, 2);
    unchanged(&nonterminal, &req);
    for action in [
        "bottom button is not pinned",
        "bottom button is not fixed",
        "bottom button is not sticky",
        "底部按钮不固定",
        "底部按钮不要固定",
        "底部按钮非固定",
    ] {
        let mut unpinned = merged_plan();
        unpinned.subtasks[2].label = "Start Button".into();
        unpinned.subtasks[2].elements = Some(action.into());
        unchanged(&unpinned, &req);
    }
    for (label, height) in [
        ("Navigation Header", 120.0),
        ("Photo Header", 48.0),
        ("Promo Header", 48.0),
        ("Exercise List Header", 48.0),
    ] {
        let mut ambiguous_header = merged_plan();
        let mut header = plan().subtasks[1].clone();
        header.id = "leading".into();
        header.label = label.into();
        header.elements = None;
        header.region.height = height;
        ambiguous_header.subtasks.insert(0, header);
        unchanged(&ambiguous_header, &req);
    }
}
