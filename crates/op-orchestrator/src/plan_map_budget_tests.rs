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
        assert!(call.user_prompt.contains("~350px of content"));
    }
}
