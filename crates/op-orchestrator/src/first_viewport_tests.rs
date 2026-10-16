use super::*;
use crate::types::AbortFlag;
use serde_json::json;

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "咖啡点单App，375×812，首屏包含品牌、推荐饮品、三款饮品价格以及自取说明。".into(),
        ..Default::default()
    }
}

fn plan() -> OrchestratorPlan {
    // Heights and hero direction from the rejected GLM coffee plan.
    crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"晴日咖啡首页","width":375,"height":812,"layout":"vertical","gap":0},
        "subtasks":[
            {"id":"hero","label":"品牌与首图","elements":"ARCHETYPE: image-led — photo hero40%, display48","region":{"width":375,"height":330}},
            {"id":"recommend","label":"推荐饮品价格卡","region":{"width":375,"height":330}},
            {"id":"pickup","label":"自取说明","region":{"width":375,"height":80}},
            {"id":"tabbar","label":"Bottom Navigation Bar","region":{"width":375,"height":78}}
        ]
    }).to_string()).unwrap()
}

#[test]
fn real_coffee_plan_reserves_required_copy_before_photo() {
    let mut plan = plan();
    reserve(&mut plan, &request());
    assert_eq!(plan.subtasks[0].region.height, 238.0);
    assert_eq!(plan.subtasks[1].region.height, 330.0);
    assert_eq!(plan.subtasks[2].region.height, 80.0);
    assert_eq!(plan.subtasks[3].region.height, 78.0);
    let before = serde_json::to_value(&plan).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &request(),
            AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("fit within 238px"));
        assert!(call.user_prompt.contains("never a lone final glyph"));
        assert!(call.user_prompt.contains("MAXIMUM, not a quota"));
        assert!(call.user_prompt.contains("Maximum section height: 238px"));
        assert!(call.user_prompt.contains("at most 238px of content"));
        assert!(!call
            .user_prompt
            .contains("Generate enough elements to fill this area"));
        assert!(call
            .user_prompt
            .contains("supporting sections must not become a second photo hero"));
    }
}

#[test]
fn unconstrained_long_and_multiscreen_pages_remain_unchanged() {
    for prompt in [
        "咖啡点单App首页",
        "咖啡菜单长页，可滚动",
        "image-led product detail",
    ] {
        let mut plan = plan();
        let before = serde_json::to_value(&plan).unwrap();
        let req = DesignRequest {
            prompt: prompt.into(),
            ..Default::default()
        };
        reserve(&mut plan, &req);
        assert_eq!(before, serde_json::to_value(&plan).unwrap());
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &req,
            AbortFlag::new(),
            false,
            false,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call
            .user_prompt
            .contains("Target content amount: ~330px tall"));
        assert!(call
            .user_prompt
            .contains("Generate enough elements to fill this area"));
    }
    let mut plan = plan();
    plan.subtasks[0].screen = Some("home".into());
    plan.subtasks[1].screen = Some("menu".into());
    let before = serde_json::to_value(&plan).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn excess_section_spacing_yields_before_required_copy() {
    let mut plan = plan();
    plan.root_frame.gap = Some(16.0);
    // Actual second coffee plan: brand, recommendation, prices, note, tabs.
    let mut nav = plan.subtasks[3].clone();
    nav.id = "bottom-nav".into();
    plan.subtasks.push(nav);
    for (task, height) in plan
        .subtasks
        .iter_mut()
        .zip([300.0, 180.0, 190.0, 60.0, 66.0])
    {
        task.region.height = height;
    }
    reserve(&mut plan, &request());
    assert_eq!(plan.root_frame.gap, Some(8.0));
    assert_eq!(plan.subtasks[0].region.height, 190.0);
    assert_eq!(
        plan.subtasks
            .iter()
            .skip(1)
            .map(|t| t.region.height)
            .collect::<Vec<_>>(),
        vec![180.0, 190.0, 60.0, 66.0]
    );
    let before = serde_json::to_value(&plan).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn compact_brand_repeating_the_screen_archetype_keeps_its_copy_budget() {
    let mut plan = plan();
    plan.subtasks[0].region.height = 120.0;
    plan.subtasks[1].elements = Some("ARCHETYPE: image-led — coffee photo 40%".into());
    plan.subtasks[1].region.height = 330.0;
    plan.subtasks[2].region.height = 240.0;
    let mut note = plan.subtasks[3].clone();
    note.id = "pickup".into();
    note.region.height = 72.0;
    plan.subtasks.insert(3, note);
    reserve(&mut plan, &request());
    assert_eq!(plan.subtasks[0].region.height, 120.0);
    assert_eq!(plan.subtasks[1].region.height, 216.0);
    assert!(plan.subtasks.iter().all(owns));
}

#[test]
fn infeasible_or_ambiguous_plan_does_not_squeeze_required_content() {
    let mut plan = plan();
    plan.subtasks[1].region.height = 700.0;
    let before = serde_json::to_value(&plan).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn dense_coffee_plan_uses_compact_media_instead_of_skipping_the_contract() {
    // Exact section heights from the next full-pipeline coffee rerun. The
    // old 160px hard minimum missed this feasible composition by just 8px.
    let mut plan = plan();
    plan.subtasks[0].elements = None;
    plan.subtasks[1].elements = Some("ARCHETYPE: image-led — latte photo".into());
    let mut note = plan.subtasks[2].clone();
    note.id = "pickup-note".into();
    plan.subtasks.insert(3, note);
    for (task, height) in plan
        .subtasks
        .iter_mut()
        .zip([96.0, 240.0, 264.0, 96.0, 78.0])
    {
        task.region.height = height;
    }
    plan.root_frame.gap = Some(16.0);
    reserve(&mut plan, &request());
    assert_eq!(plan.root_frame.gap, Some(8.0));
    assert_eq!(plan.subtasks[1].region.height, 152.0);
    assert!(plan.subtasks.iter().all(owns));
    assert_eq!(plan.subtasks[3].region.height, 96.0);
    let before = serde_json::to_value(&plan).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn screen_archetype_on_short_header_still_budgets_the_actual_photo_sibling() {
    // Exact next full-pipeline plan: the image-led marker is on a 92px brand
    // header; the 330px hero has only its media label and description.
    let mut plan = crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"晴日咖啡首页","width":375,"height":812,"layout":"vertical","gap":0},
        "subtasks":[
            {"id":"brand-header","label":"品牌头部","elements":"品牌名标题；ARCHETYPE: image-led — 图片主角在下方","region":{"width":375,"height":92}},
            {"id":"hero-drink","label":"推荐饮品主图","elements":"大幅真实拿铁咖啡照片（约40%首屏高度）","region":{"width":375,"height":330}},
            {"id":"menu-list","label":"三款饮品价格列表","elements":"三张横向卡片各含真实咖啡小图与名称价格","region":{"width":375,"height":230}},
            {"id":"pickup-note","label":"自取说明与下单按钮","region":{"width":375,"height":160}},
            {"id":"tabs","label":"Bottom Navigation","region":{"width":375,"height":78}}
        ]
    }).to_string()).unwrap();
    reserve(&mut plan, &request());
    assert_eq!(plan.subtasks[0].region.height, 92.0);
    assert_eq!(plan.subtasks[1].region.height, 166.0);
    assert_eq!(plan.subtasks[2].region.height, 230.0);
    assert!(plan.subtasks.iter().all(owns));
    let settled = plan.clone();
    reserve(&mut plan, &request());
    assert_eq!(settled, plan);

    let mut plan = settled;
    plan.subtasks[0].elements = Some("Brand title only".into());
    plan.subtasks[1].elements = Some("quiet section; ARCHETYPE: image-led — coffee photo".into());
    assert!(is_image_section(&plan.subtasks[1]));
    plan.subtasks[1].elements = Some("A large account balance figure".into());
    assert!(!is_image_section(&plan.subtasks[1]));
    assert!(!is_image_section(&plan.subtasks[2]));
}

#[test]
fn real_layout_rejects_overflow_without_clipping_or_deleting_copy() {
    let mut plan = plan();
    reserve(&mut plan, &request());
    for (image_height, rejected) in [(330.0, true), (140.0, false)] {
        let nodes:Vec<PenNode>=serde_json::from_value(json!([{
            "type":"frame","id":"hero","width":"fill_container","height":"fit_content","layout":"vertical","gap":12,
            "children":[
                {"type":"rectangle","id":"photo","width":"fill_container","height":image_height},
                {"type":"text","id":"title","content":"晴日咖啡 · 静安店","width":"fill_container","fontSize":28,"height":40}
            ]
        }])).unwrap();
        let original = nodes.clone();
        let reason = rejection(&plan.subtasks[0], &nodes, &EditorState::new());
        assert_eq!(reason.is_some(), rejected, "{reason:?}");
        assert_eq!(nodes, original);
        if let Some(reason) = reason {
            assert!(crate::retry::is_self_check_rejection(&reason));
        }
    }
}

#[test]
fn price_list_retry_identifies_the_space_to_recover_without_losing_rows() {
    let mut plan = plan();
    reserve(&mut plan, &request());
    let task = &mut plan.subtasks[1];
    task.region.height = 196.0;
    let nodes_for = |row_height| {
        serde_json::from_value::<Vec<PenNode>>(json!([{
            "type":"frame","id":"section","width":"fill_container","height":"fit_content","layout":"vertical","gap":12,
            "children":[
                {"type":"text","id":"title","content":"饮品单","height":24},
                {"type":"frame","id":"prices","layout":"vertical","width":"fill_container","height":"fit_content","gap":8,
                 "children":[
                    {"type":"text","id":"latte","content":"拿铁 28元","height":row_height},
                    {"type":"text","id":"americano","content":"美式 22元","height":row_height},
                    {"type":"text","id":"oat","content":"燕麦拿铁 32元","height":row_height}
                 ]}
            ]
        }])).unwrap()
    };
    let oversized = nodes_for(56);
    let original = oversized.clone();
    let reason = rejection(task, &oversized, &EditorState::new()).unwrap();
    assert!(reason.contains("recover at least 24px"), "{reason}");
    assert!(reason.contains("title=24px"), "{reason}");
    assert!(reason.contains("prices=184px"), "{reason}");
    assert_eq!(
        original, oversized,
        "Diagnostics must not rewrite the design"
    );
    let compact = nodes_for(48);
    assert_eq!(rejection(task, &compact, &EditorState::new()), None);
    let texts = serde_json::to_string(&compact).unwrap();
    for required in ["拿铁 28元", "美式 22元", "燕麦拿铁 32元"] {
        assert!(texts.contains(required));
    }
}
