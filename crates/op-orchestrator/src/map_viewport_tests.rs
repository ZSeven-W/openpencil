use super::*;
use serde_json::json;

fn plan() -> OrchestratorPlan {
    crate::plan::parse_plan(&json!({
        "rootFrame":{"id":"page","name":"Navigation","width":375,"height":812},
        "subtasks":[
            {"id":"map","label":"全屏地图与路线","elements":"ARCHETYPE: route-map — full screen","region":{"width":375,"height":812},"covers":["城市街区地图"]},
            {"id":"directions","label":"悬浮方向提示卡","elements":"前方200米右转","region":{"width":375,"height":110},"covers":["前方200米右转"]},
            {"id":"controls","label":"右侧地图控件","elements":"图层和定位","region":{"width":375,"height":0}},
            {"id":"summary","label":"底部行程摘要条","elements":"剩余3.2公里，预计12分钟，结束导航","region":{"width":375,"height":110}}
        ]
    }).to_string()).unwrap()
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "设计一页手机骑行导航。全屏地图，上方悬浮方向提示，右侧小控件，下方行程摘要。"
            .into(),
        ..Default::default()
    }
}

#[test]
fn real_navigation_plan_becomes_one_viewport_without_losing_content() {
    let mut plan = plan();
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    let task = &plan.subtasks[0];
    assert_eq!(task.region.height, 734.0);
    assert!(task.bleed_hero);
    assert_eq!(task.parent_frame_id.as_deref(), Some("page"));
    let elements = task.elements.as_deref().unwrap();
    for copy in [
        "前方200米右转",
        "剩余3.2公里",
        "预计12分钟",
        "结束导航",
        "图层和定位",
        "map-template:running",
    ] {
        assert!(elements.contains(copy), "{copy}");
    }
    assert_eq!(task.covers.as_ref().unwrap().len(), 2);
    let before = serde_json::to_value(&plan).unwrap();
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(before, serde_json::to_value(&plan).unwrap());
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &req(),
            crate::types::AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("height:734, layout:none"));
        assert!(call.user_prompt.contains("map image LAST"));
        assert!(!call.user_prompt.contains("section owns padding:[0,24]"));
        assert!(!call
            .system_prompt
            .contains("Keep every section root height="));
        assert!(!call
            .user_prompt
            .contains("NEVER set a fixed pixel height on the root"));
    }
}

#[test]
fn ordinary_tracking_and_unrelated_sections_do_not_become_overlays() {
    for prompt in [
        "配送追踪详情",
        "骑行导航但不要全屏地图",
        "location picker with full-screen map",
    ] {
        let mut plan = plan();
        fold(
            &mut plan,
            &DesignRequest {
                prompt: prompt.into(),
                ..Default::default()
            },
        );
        assert_eq!(plan.subtasks.len(), 4, "{prompt}");
    }
    let mut plan = plan();
    plan.subtasks[3].label = "订单和骑手列表".into();
    let outside = plan.subtasks[3].clone();
    fold(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[1], outside);
    assert!(!plan.subtasks[0]
        .elements
        .as_ref()
        .unwrap()
        .contains("订单和骑手列表"));
}

#[test]
fn live_planner_control_and_information_labels_share_the_map_viewport() {
    let mut plan = plan();
    plan.subtasks[1].label = "方向提示悬浮条".into();
    plan.subtasks[2].label = "右侧地图操作按钮".into();
    plan.subtasks[3].label = "底部导航信息栏".into();
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].region.height, 734.0);
    let mut plan = super::tests::plan();
    plan.subtasks[2].label = "右侧图层与定位按钮".into();
    plan.subtasks[3].label = "底部导航信息栏".into();
    fold(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    let mut plan = super::tests::plan();
    plan.subtasks[3].label = "底部导航栏".into();
    let footer = plan.subtasks[3].clone();
    fold(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(
        plan.subtasks[1], footer,
        "destination tabs stay outside the map viewport"
    );
}

#[test]
fn coverage_appended_trip_datum_folds_into_the_existing_summary_once() {
    let mut plan = plan();
    let mut datum = plan.subtasks[3].clone();
    datum.id = "brief-section-1".into();
    datum.label = "下方紧凑显示剩余3.2公里".into();
    datum.elements = Some("the brief explicitly requires this section: 下方紧凑显示剩余3.2公里 — build exactly what it names".into());
    plan.subtasks.insert(3, datum);
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    assert!(plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .contains("render each datum once"));
}

#[test]
fn live_six_part_replan_keeps_navigation_actions_inside_the_viewport() {
    for controls in ["右侧浮动控件", "右侧悬浮控件", "右侧地图操作按钮"] {
        let mut plan = plan();
        plan.subtasks[2].label = controls.into();
        // Some plans give overlays the whole map's available region.
        plan.subtasks[2].region.height = 812.0;
        plan.subtasks[3].label = "底部导航信息栏".into();
        let mut datum = plan.subtasks[3].clone();
        datum.id = "brief-section-1".into();
        datum.label = "下方紧凑显示剩余3.2公里".into();
        datum.elements = Some("剩余3.2公里".into());
        datum.region.height = 0.0;
        let mut action = datum.clone();
        action.id = "brief-section-2".into();
        action.label = "结束导航按钮".into();
        action.elements = Some("the brief explicitly requires: 结束导航按钮".into());
        action.covers = Some(vec!["结束导航按钮".into()]);
        plan.subtasks.extend([datum, action]);
        crate::plan_normalize::normalize(&mut plan, &req());
        assert_eq!(plan.subtasks.len(), 1, "{controls}");
        let owner = &plan.subtasks[0];
        assert!(owns(owner));
        assert_eq!(owner.region.height, 734.0);
        assert!(owner
            .covers
            .as_ref()
            .unwrap()
            .contains(&"结束导航按钮".into()));
        let elements = owner.elements.as_deref().unwrap();
        for copy in [controls, "结束导航按钮", "剩余3.2公里", "预计12分钟"] {
            assert!(elements.contains(copy), "{copy}");
        }
    }
}

#[test]
fn floating_ownership_uses_the_description_not_only_the_section_name() {
    let mut plan = plan();
    plan.subtasks[2].label = "右侧控制按钮组".into();
    plan.subtasks[2].elements =
        Some("右侧小型竖排按钮组：图层切换按钮与定位按钮，两个44px控件".into());
    let mut brand = plan.subtasks[1].clone();
    brand.id = "brand-header".into();
    brand.label = "顶部品牌栏".into();
    brand.elements = Some("悬浮于地图上方的紧凑品牌条：沿途".into());
    brand.region.height = 0.0;
    let mut distance = brand.clone();
    distance.id = "brief-section-1".into();
    distance.label = "下方紧凑显示剩余3.2公里".into();
    distance.elements = Some("剩余3.2公里".into());
    let mut action = brand.clone();
    action.id = "brief-section-2".into();
    action.label = "结束导航按钮".into();
    action.elements = Some("结束导航按钮".into());
    plan.subtasks.extend([brand, distance, action]);
    // Seven semantic layers are still one viewport, including brief reminders.
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    assert!(owns(&plan.subtasks[0]));
    assert_eq!(plan.subtasks[0].region.height, 734.0);
    assert!(plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .contains("紧凑品牌条：沿途"));
}

#[test]
fn floating_description_does_not_swallow_destination_tabs_or_order_content() {
    for label in ["底部Tab栏", "订单和骑手列表"] {
        let mut plan = plan();
        let mut outside = plan.subtasks[3].clone();
        outside.id = "unrelated".into();
        outside.label = label.into();
        outside.region.height = 64.0;
        outside.elements = Some("floating overlay, with map controls".into());
        plan.subtasks.push(outside.clone());
        fold(&mut plan, &req());
        assert_eq!(plan.subtasks.len(), 2, "{label}");
        assert_eq!(plan.subtasks[1], outside);
        assert!(!plan.subtasks[0]
            .elements
            .as_deref()
            .unwrap()
            .contains("floating overlay, with map controls"));
    }
}

#[test]
fn replanned_archetype_on_banner_and_brief_layer_reminder_do_not_split_the_map() {
    let mut plan = plan();
    plan.subtasks[0].elements = Some("城市街区全屏地图，暖橙路线".into());
    plan.subtasks[1].id = "nav-banner".into();
    plan.subtasks[1].label = "方向提示横幅".into();
    plan.subtasks[1].elements = Some("ARCHETYPE: route-map — 前方200米右转".into());
    plan.subtasks.swap(0, 1);
    let mut layer = plan.subtasks[2].clone();
    layer.id = "brief-section-1".into();
    layer.label = "右侧小型图层".into();
    plan.subtasks.insert(2, layer);
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 1);
    assert!(owns(&plan.subtasks[0]));
    let content = plan.subtasks[0].elements.as_deref().unwrap();
    for copy in ["前方200米右转", "剩余3.2公里", "图层和定位"] {
        assert!(content.contains(copy), "{copy}");
    }
}

#[test]
fn map_note_stays_outside_without_breaking_overlay_composition() {
    let mut plan = plan();
    let mut note = plan.subtasks[3].clone();
    note.id = "map-source-note".into();
    note.label = "示例数据说明".into();
    note.elements = Some("地图仅为示意".into());
    note.region.height = 32.0;
    note.covers = Some(vec!["地图仅为示意".into()]);
    plan.subtasks.push(note);
    crate::plan_normalize::normalize(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[0].region.height, 686.0);
    assert_eq!(plan.subtasks[1].label, "示例数据说明");
    assert!(!plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .contains("地图仅为示意"));
}

#[test]
fn viewport_ownership_never_folds_another_screen() {
    let mut plan = plan();
    for task in &mut plan.subtasks {
        task.screen = Some("导航".into());
    }
    let mut profile = plan.subtasks[3].clone();
    profile.id = "profile".into();
    profile.screen = Some("个人".into());
    profile.label = "个人资料".into();
    plan.subtasks.push(profile.clone());
    fold(&mut plan, &req());
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[1], profile);
}

#[test]
fn actual_subagent_keeps_viewport_height_after_section_normalization() {
    use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
    use crate::types::DocSink;
    let mut plan = plan();
    crate::plan_normalize::normalize(&mut plan, &req());
    plan.subtasks[0].parent_frame_id = None;
    let script = r#"const sec=I(null,{type:"frame",layout:"none",width:375,height:734});
I(sec,{type:"text",name:"Directions",x:16,y:16,width:280,height:28,content:"前方200米右转",fontSize:20});
I(sec,{type:"text",name:"Summary",x:16,y:650,width:280,height:28,content:"剩余3.2公里，预计12分钟",fontSize:16});
I(sec,{type:"image",name:"Map",x:0,y:0,width:375,height:734,imageSearchQuery:"map-template:running"});"#;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(script.into())]);
    let mut sink = VecDocSink::new();
    let outcome = futures::executor::block_on(crate::subagent::run_subtask(
        &plan.subtasks[0],
        &plan,
        &req(),
        &llm,
        &mut sink,
        &crate::types::AbortFlag::new(),
        false,
        false,
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    let nodes = serde_json::to_value(sink.state().active_children()).unwrap();
    assert_eq!(nodes[0]["height"], 734.0);
    assert_eq!(nodes[0]["layout"], "none");
    assert_eq!(nodes[0]["children"][1]["y"], 650.0);
}
