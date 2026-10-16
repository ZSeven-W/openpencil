use super::*;
use serde_json::json;

fn plan() -> OrchestratorPlan {
    crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"Delivery","width":375,"height":812,"gap":16},
        "subtasks":[
            {"id":"header","label":"顶部导航标题","region":{"width":375,"height":96}},
            {"id":"map","label":"城市街区配送地图","elements":"ARCHETYPE: route-map — first screen 55%","region":{"width":375,"height":460}},
            {"id":"info","label":"骑手与订单信息","region":{"width":375,"height":256}}
        ]
    }).to_string()).unwrap()
}

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        ..Default::default()
    }
}

#[test]
fn real_tracking_plan_reserves_chrome_and_gaps_without_squeezing_support() {
    let mut plan = plan();
    let req = request("设计手机配送追踪详情");
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(plan.subtasks[1].region.height, 350.0);
    assert_eq!(plan.subtasks[0].region.height, 96.0);
    assert_eq!(plan.subtasks[2].region.height, 256.0);
    assert!(plan.subtasks[1]
        .elements
        .as_ref()
        .unwrap()
        .contains("at most 350px"));
    let before = serde_json::to_value(&plan).unwrap();
    reserve_supporting_content(&mut plan, &req);
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn intentional_large_maps_and_ambiguous_compositions_are_preserved() {
    for brief in [
        "骑行导航",
        "Full-screen delivery map",
        "配送地图高度460px",
        "复刻参考图",
        "可滚动配送详情",
    ] {
        let mut plan = plan();
        let before = serde_json::to_value(&plan).unwrap();
        reserve_supporting_content(&mut plan, &request(brief));
        assert_eq!(before, serde_json::to_value(&plan).unwrap(), "{brief}");
    }
    for map_height in [200.0, 100.0] {
        let mut plan = plan();
        plan.subtasks[1].region.height = map_height;
        let before = serde_json::to_value(&plan).unwrap();
        reserve_supporting_content(&mut plan, &request("配送详情"));
        assert_eq!(before, serde_json::to_value(&plan).unwrap());
    }
    let mut plan = plan();
    plan.subtasks[2].region.height = 600.0;
    reserve_supporting_content(&mut plan, &request("配送详情"));
    assert_eq!(plan.subtasks[1].region.height, 460.0);
}

#[test]
fn budgets_are_per_screen_and_multiple_map_owners_are_not_guessed() {
    let mut plan = plan();
    for task in &mut plan.subtasks {
        task.screen = Some("delivery".into());
    }
    let mut other = plan.subtasks[2].clone();
    other.id = "unrelated".into();
    other.screen = Some("profile".into());
    other.region.height = 700.0;
    plan.subtasks.push(other);
    reserve_supporting_content(&mut plan, &request("配送App"));
    assert_eq!(plan.subtasks[1].region.height, 350.0);
    assert_eq!(plan.subtasks[3].region.height, 700.0);
    let mut other_map = plan.subtasks[1].clone();
    other_map.id = "map2".into();
    plan.subtasks.push(other_map);
    let before = serde_json::to_value(&plan).unwrap();
    reserve_supporting_content(&mut plan, &request("配送App"));
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
}

#[test]
fn normalized_budget_reaches_all_generation_retry_prompts() {
    let mut plan = plan();
    let req = request("设计手机配送追踪详情");
    crate::plan_normalize::normalize(&mut plan, &req);
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[1],
            &plan,
            &req,
            crate::types::AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("at most 350px"));
        assert!(call.user_prompt.contains("at most 350px of content"));
        assert!(call.user_prompt.contains("Maximum section height: 350px"));
        assert!(!call
            .user_prompt
            .contains("Generate enough elements to fill this area"));
    }
}

fn taxi_plan() -> OrchestratorPlan {
    crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"打车主页","width":375,"height":812,"layout":"vertical","gap":0},
        "subtasks":[
            {"id":"map-area","label":"地图占位区","elements":"ARCHETYPE: route-map — protagonist map 45% full-bleed top","region":{"width":375,"height":360}},
            {"id":"pickup-card","label":"当前位置卡","region":{"width":375,"height":72}},
            {"id":"destination-input","label":"目的地输入","region":{"width":375,"height":56}},
            {"id":"ride-types","label":"车型选择卡","region":{"width":375,"height":120}},
            {"id":"coupon-bar","label":"优惠券条","region":{"width":375,"height":44}},
            {"id":"confirm-cta","label":"确认叫车按钮","region":{"width":375,"height":64}},
            {"id":"bottom-nav","label":"底部导航栏","region":{"width":375,"height":80}}
        ]
    }).to_string()).unwrap()
}

#[test]
fn taxi_placeholder_and_tab_navigation_do_not_disable_the_map_budget() {
    // Original historical prompt and planner output from the clipped rerun.
    for prompt in [
        "打车 App 主页（375×812）：地图占位区、当前位置卡、目的地输入、车型选择三档横向卡（图标+名称+预估价）、优惠券条、确认叫车按钮、底部导航。",
        "Ride home, map placeholder, pickup, destination, vehicles, coupon, confirm ride and bottom navigation bar",
    ] {
        let mut plan = taxi_plan();
        let req = request(prompt);
        crate::plan_normalize::normalize(&mut plan, &req);
        assert_eq!(plan.subtasks[0].region.height, 204.0, "{prompt}");
        assert_eq!(plan.subtasks[5].region.height, 64.0);
        assert_eq!(plan.subtasks[6].region.height, 78.0);
        let before = serde_json::to_value(&plan).unwrap();
        reserve_supporting_content(&mut plan, &req);
        assert_eq!(before, serde_json::to_value(&plan).unwrap());
    }
}

#[test]
fn a_reserved_map_also_checks_the_supporting_sections_actual_height() {
    let mut plan = taxi_plan();
    crate::plan_normalize::normalize(&mut plan, &request("打车首页，地图占位区和底部导航"));
    for height in [132, 112] {
        let nodes: Vec<jian_ops_schema::node::PenNode> = serde_json::from_value(json!([{
            "type":"frame","id":"vehicles","width":"fill_container","height":"fit_content","layout":"vertical","children":[
                {"type":"rectangle","id":"cards","width":"fill_container","height":height}
            ]
        }])).unwrap();
        let reason = crate::first_viewport::rejection(
            &plan.subtasks[3],
            &nodes,
            &op_editor_core::EditorState::new(),
        );
        assert_eq!(reason.is_some(), height > 120, "{reason:?}");
    }
}

#[test]
fn actual_route_navigation_and_authored_map_sizes_still_opt_out() {
    for prompt in [
        "打车地图占45%，有底部导航",
        "打车地图占位区，但地图高度460px",
        "骑行导航，底部导航栏",
        "Turn-by-turn navigation with bottom navigation bar",
    ] {
        let mut plan = plan();
        let before = serde_json::to_value(&plan).unwrap();
        reserve_supporting_content(&mut plan, &request(prompt));
        assert_eq!(before, serde_json::to_value(&plan).unwrap(), "{prompt}");
    }
}
