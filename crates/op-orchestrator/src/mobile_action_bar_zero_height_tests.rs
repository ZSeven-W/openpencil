use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::AbortFlag;
use futures::executor::block_on;
use op_editor_core::EditorState;
use serde_json::{json, Value};

fn fixture() -> (OrchestratorPlan, Vec<PenNode>) {
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"page","name":"商品详情","width":375,"height":812,
            "layout":"vertical","gap":16,"fill":[{"type":"solid","color":"#FFFFFF"}]},
        "styleGuideName":"warm-food-mobile-light",
        "subtasks":[
            {"id":"hero","label":"商品大图","screen":"detail",
                "elements":"商品主图360px，保留大图展示","region":{"width":375,"height":360}},
            {"id":"info","label":"商品信息","screen":"detail",
                "region":{"width":375,"height":379}},
            {"id":"reviews","label":"评价列表","screen":"detail",
                "elements":"完整保留三条评价","region":{"width":375,"height":328}},
            {"id":"bottom-bar","label":"bottom-bar","screen":"detail",
                "elements":"固定底部操作条，客服、店铺及加入购物车按钮，底部保留安全区",
                "region":{"width":375,"height":0},"parentFrameId":"page"}
        ]
    }))
    .unwrap();
    // Keep the saved control geometry, including its normalized [12,16]
    // padding. Explicit transitions also let the full replay assert complete
    // control preservation without unrelated default-motion additions.
    let rail = serde_json::from_value(json!({
        "type":"frame","id":"action-rail","name":"底部加购条",
        "width":"fill_container","height":"fit_content","layout":"vertical",
        "padding":[0,24,0,24],"fill":[{"type":"solid","color":"#FFFFFF"}],
        "stroke":{"thickness":{"top":1,"right":0,"bottom":0,"left":0},
            "fill":[{"type":"solid","color":"#E8E3DB"}]},
        "children":[{
            "type":"frame","id":"action-row","name":"加购操作条",
            "width":"fill_container","height":"fit_content","layout":"horizontal",
            "gap":12,"padding":[12,0,24,0],"alignItems":"center","children":[
                {"type":"frame","id":"side-actions","name":"辅助操作",
                    "width":"fit_content","height":"fit_content","layout":"horizontal",
                    "gap":8,"children":[
                        {"type":"frame","id":"service","name":"客服","role":"button",
                            "transition":{"durationMs":150,"easing":"standard","property":"scale"},
                            "width":48,"height":52,"layout":"vertical","gap":2,
                            "justifyContent":"center","alignItems":"center","children":[
                                {"type":"icon_font","id":"service-icon","iconFontFamily":"lucide",
                                    "iconFontName":"message-circle","width":20,"height":20,
                                    "fill":[{"type":"solid","color":"#414141"}]},
                                {"type":"text","id":"service-label","content":"客服","fontSize":11,
                                    "fill":[{"type":"solid","color":"#414141"}]}
                            ]},
                        {"type":"frame","id":"shop","name":"店铺","role":"button",
                            "transition":{"durationMs":150,"easing":"standard","property":"scale"},
                            "width":48,"height":52,"layout":"vertical","gap":2,
                            "justifyContent":"center","alignItems":"center","children":[
                                {"type":"icon_font","id":"shop-icon","iconFontFamily":"lucide",
                                    "iconFontName":"store","width":20,"height":20,
                                    "fill":[{"type":"solid","color":"#414141"}]},
                                {"type":"text","id":"shop-label","content":"店铺","fontSize":11,
                                    "fill":[{"type":"solid","color":"#414141"}]}
                            ]}
                    ]},
                {"type":"frame","id":"add-to-cart","name":"加入购物车","role":"button",
                    "transition":{"durationMs":150,"easing":"standard","property":"scale"},
                    "width":"fill_container","height":52,"layout":"horizontal","gap":8,
                    "padding":[12,16],
                    "justifyContent":"center","alignItems":"center","cornerRadius":16,
                    "fill":[{"type":"solid","color":"#E86B3A"}],
                    "events":{"onTap":[{"push":"\"/screen-1\""}]},"children":[
                        {"type":"icon_font","id":"add-icon","iconFontFamily":"lucide",
                            "iconFontName":"shopping-cart","width":20,"height":20,
                            "fill":[{"type":"solid","color":"#FFFFFF"}]},
                        {"type":"text","id":"add-label","content":"加入购物车",
                            "fontSize":16,"fontWeight":"600",
                            "fill":[{"type":"solid","color":"#FFFFFF"}]}
                    ]}
            ]
        }]
    }))
    .unwrap();
    (plan, vec![rail])
}

fn mark(nodes: &mut [PenNode], plan: &OrchestratorPlan, req: &DesignRequest) {
    preserve(
        nodes,
        plan.subtasks.last().unwrap(),
        plan,
        req,
        &EditorState::new(),
    );
}

#[test]
fn zero_height_explicit_action_uses_intrinsic_geometry_without_changing_the_tree() {
    let (plan, mut nodes) = fixture();
    assert!(!crate::mobile_action_bar::is_action_bar(&nodes[0]));
    let mut expected = serde_json::to_value(&nodes).unwrap();

    mark(&mut nodes, &plan, &DesignRequest::default());

    assert!(is_marked(&nodes[0]));
    assert!(crate::mobile_action_bar::is_action_bar(&nodes[0]));
    expected[0]["explain"] = json!(MARKER);
    assert_eq!(serde_json::to_value(&nodes).unwrap(), expected);
    let once = nodes.clone();
    mark(&mut nodes, &plan, &DesignRequest::default());
    assert_eq!(nodes, once);
}

#[test]
fn zero_height_does_not_pin_unfixed_body_or_reference_content() {
    for case in ["unfixed", "ordinary body", "nonterminal", "reference"] {
        let (mut plan, mut nodes) = fixture();
        let mut req = DesignRequest::default();
        match case {
            "unfixed" => {
                plan.subtasks.last_mut().unwrap().elements =
                    Some("底部操作条不固定，随正文滚动，包含加入购物车按钮".into());
            }
            "ordinary body" => {
                let task = plan.subtasks.last_mut().unwrap();
                task.label = "Product information".into();
                task.elements = Some("正文末尾的加入购物车按钮".into());
            }
            "nonterminal" => {
                let mut later = plan.subtasks[1].clone();
                later.id = "later-content".into();
                plan.subtasks.push(later);
            }
            "reference" => {
                req.reference_skeleton = Some(
                    serde_json::from_value(json!({
                        "source":"reference","width":375,"sections":[],"navKind":"none",
                        "heroKind":"none","columnRhythm":[]
                    }))
                    .unwrap(),
                );
            }
            _ => unreachable!(),
        }
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == "bottom-bar")
            .unwrap();
        let before = nodes.clone();
        preserve(&mut nodes, task, &plan, &req, &EditorState::new());
        assert_eq!(nodes, before, "{case}");
    }
}

#[test]
fn zero_height_fallback_rejects_oversized_generated_content_even_with_a_short_root() {
    for fixed_root in [false, true] {
        let (plan, mut nodes) = fixture();
        let mut rail = serde_json::to_value(&nodes[0]).unwrap();
        rail["children"][0]["children"][1]["height"] = json!(300);
        if fixed_root {
            rail["height"] = json!(88);
            rail["clipContent"] = json!(true);
        }
        nodes[0] = serde_json::from_value(rail).unwrap();
        let before = nodes.clone();

        mark(&mut nodes, &plan, &DesignRequest::default());

        assert_eq!(nodes, before, "fixed root: {fixed_root}");
    }
}

#[test]
fn zero_height_measurement_resolves_padding_variables_in_the_current_theme() {
    for (theme, expected) in [("Light", true), ("Dark", false)] {
        let (plan, mut nodes) = fixture();
        let mut rail = serde_json::to_value(&nodes[0]).unwrap();
        rail["children"][0]["padding"] = json!("$--action-padding");
        nodes[0] = serde_json::from_value(rail).unwrap();
        let before = nodes.clone();
        let mut context = EditorState::from_document(
            serde_json::from_value(json!({
                "version":"1.0.0","themes":{"Mode":["Light","Dark"]},"variables":{
                    "--action-padding":{"type":"number","value":[
                        {"value":12,"theme":{"Mode":"Light"}},
                        {"value":100,"theme":{"Mode":"Dark"}}
                    ]}
                },"children":[]
            }))
            .unwrap(),
        );
        context
            .ui
            .variables
            .active_theme
            .insert("Mode".into(), theme.into());

        preserve(
            &mut nodes,
            plan.subtasks.last().unwrap(),
            &plan,
            &DesignRequest::default(),
            &context,
        );

        assert_eq!(is_marked(&nodes[0]), expected, "active theme: {theme}");
        if !expected {
            assert_eq!(nodes, before);
        }
    }
}

#[test]
fn zero_height_fallback_rejects_sizing_expressions_even_with_defined_variables() {
    for defined in [false, true] {
        let (plan, mut nodes) = fixture();
        let mut rail = serde_json::to_value(&nodes[0]).unwrap();
        rail["children"][0]["children"][1]["height"] = json!("$--action-height");
        nodes[0] = serde_json::from_value(rail).unwrap();
        let before = nodes.clone();
        let variables = if defined {
            json!({"--action-height":{"type":"number","value":52}})
        } else {
            json!({})
        };
        let context = EditorState::from_document(
            serde_json::from_value(json!({
                "version":"1.0.0","variables":variables,"children":[]
            }))
            .unwrap(),
        );

        preserve(
            &mut nodes,
            plan.subtasks.last().unwrap(),
            &plan,
            &DesignRequest::default(),
            &context,
        );

        assert_eq!(nodes, before, "sizing variable defined: {defined}");
    }
}

#[test]
fn zero_height_fallback_does_not_accept_invalid_nonzero_planned_heights() {
    for height in [
        -1.0,
        1.0,
        43.0,
        161.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        let (mut plan, mut nodes) = fixture();
        plan.subtasks.last_mut().unwrap().region.height = height;
        let before = nodes.clone();

        mark(&mut nodes, &plan, &DesignRequest::default());

        assert_eq!(nodes, before, "planned height: {height}");
    }
}

fn find_named<'a>(node: &'a PenNode, name: &str) -> Option<&'a PenNode> {
    if node.base().name.as_deref() == Some(name) {
        Some(node)
    } else {
        node.children()
            .and_then(|children| children.iter().find_map(|child| find_named(child, name)))
    }
}

fn without_node_ids(nodes: &[PenNode]) -> Vec<PenNode> {
    fn clear_ids(nodes: &mut [PenNode]) {
        for node in nodes {
            node.base_mut().id.clear();
            if let Some(children) = node.children_mut() {
                clear_ids(children);
            }
        }
    }
    let mut nodes = nodes.to_vec();
    clear_ids(&mut nodes);
    nodes
}

fn finalize(sink: &mut VecDocSink, plan: &OrchestratorPlan) {
    let root_id = sink.state.active_children()[0].id_str().to_owned();
    crate::cleanup::finalize_design_with_summary_and_policy(
        sink,
        plan,
        &[root_id.as_str()],
        &mut crate::repair_summary::RepairSummary::default(),
        crate::cleanup::CleanupPolicy {
            preserve_requested_root_height: true,
            roots_are_run_output: true,
            is_deck: false,
        },
    );
}

#[test]
fn zero_height_action_survives_insertion_cleanup_save_and_replay_with_one_safe_inset() {
    let (plan, nodes) = fixture();
    let task = plan.subtasks.last().unwrap().clone();
    let req = DesignRequest {
        prompt: "商品详情页375×812，360px大幅主图、商品信息、三条完整评价，固定底部操作条，含客服、店铺和加入购物车按钮。".into(),
        ..DesignRequest::default()
    };
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(format!(
        "I(null, {});",
        serde_json::to_string(&nodes[0]).unwrap()
    ))]);
    let reviews: Vec<Value> = (1..=3)
        .map(|index| {
            json!({"type":"frame","id":format!("review-{index}"),
            "name":format!("Review {index}"),"width":"fill_container","height":104,
            "layout":"vertical","padding":16,"gap":8,"children":[
                {"type":"text","id":format!("review-author-{index}"),
                    "content":format!("顾客 {index}"),"fontSize":16,"height":16},
                {"type":"text","id":format!("review-copy-{index}"),
                    "content":"商品完整评价，保留全部内容。","fontSize":16,"height":48}
            ]})
        })
        .collect();
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0.0","children":[{
            "type":"frame","id":"page","name":"商品详情","x":40,"y":40,
            "width":375,"height":812,"layout":"vertical","gap":16,
            "fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
                {"type":"frame","id":"status","role":"status-bar","name":"Status Bar",
                    "width":"fill_container","height":62},
                {"type":"frame","id":"hero","name":"商品主图360px",
                    "width":"fill_container","height":360,"layout":"vertical","children":[
                        {"type":"rectangle","id":"hero-image","width":"fill_container",
                            "height":360,"fill":[{"type":"solid","color":"#D8C8B8"}]}
                    ]},
                {"type":"frame","id":"info","name":"商品信息","width":"fill_container",
                    "height":379,"layout":"vertical","children":[
                        {"type":"text","id":"info-copy","content":"商品名称与完整说明",
                            "fontSize":20,"height":379}
                    ]},
                {"type":"frame","id":"reviews","name":"评价列表","width":"fill_container",
                    "height":328,"layout":"vertical","gap":8,"children":reviews}
            ]
        }]}))
        .unwrap(),
    );
    let result = block_on(crate::subagent::run_subtask(
        &task,
        &plan,
        &req,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(result.error.is_none(), "{:?}", result.error);
    let inserted = sink.state.active_children()[0]
        .children()
        .unwrap()
        .last()
        .unwrap();
    assert!(is_marked(inserted), "insertion must retain explicit intent");
    let inserted_children = inserted.children().unwrap().to_vec();
    let button = find_named(inserted, "加入购物车").unwrap();
    let button_events = serde_json::to_value(button).unwrap()["events"].clone();

    finalize(&mut sink, &plan);
    let stored = serde_json::to_string(&sink.state.doc).unwrap();
    sink.state = EditorState::from_document(serde_json::from_str(&stored).unwrap());

    let root = &sink.state.active_children()[0];
    let children = root.children().unwrap();
    assert_eq!(children.len(), 3, "status, scrolling body, fixed action");
    let viewport = &children[1];
    let rail = &children[2];
    assert_eq!(rail.base().name.as_deref(), Some("底部加购条"));
    assert!(is_marked(rail));
    // Whole-root cleanup replacement remaps IDs, including untouched controls.
    assert_eq!(
        without_node_ids(rail.children().unwrap()),
        without_node_ids(&inserted_children),
        "keep the complete authored action subtree apart from remapped IDs"
    );
    assert_eq!(
        serde_json::to_value(find_named(rail, "加入购物车").unwrap()).unwrap()["events"],
        button_events,
        "keep the authored navigation action"
    );
    let viewport_value = serde_json::to_value(viewport).unwrap();
    assert_eq!(viewport_value["role"], "scroll-area");
    assert_eq!(viewport_value["height"], "fill_container");
    assert_eq!(viewport_value["clipContent"], true);
    assert!(!viewport_value["events"]["onScroll"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(find_named(viewport, "Review 3").is_some());
    assert!(find_named(viewport, "底部加购条").is_none());

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().unwrap();
    let board = page.find(root.id_str()).unwrap().bounds;
    let scroll = page.find(viewport.id_str()).unwrap().bounds;
    let footer = page.find(rail.id_str()).unwrap().bounds;
    let button = page
        .find(find_named(rail, "加入购物车").unwrap().id_str())
        .unwrap()
        .bounds;
    let last_review = page
        .find(find_named(viewport, "Review 3").unwrap().id_str())
        .unwrap()
        .bounds;
    let hero = page
        .find(find_named(viewport, "商品主图360px").unwrap().id_str())
        .unwrap()
        .bounds;
    assert_eq!(board.size.y, 812.0);
    assert_eq!(hero.size.y, 360.0, "preserve the requested large hero");
    assert_eq!(
        footer.size.y, 88.0,
        "count the horizontal row's safe inset once"
    );
    assert_eq!(button.size.y, 52.0);
    let bottom = board.origin.y + board.size.y;
    assert!((footer.origin.y + footer.size.y - bottom).abs() < 1.0);
    assert!((bottom - button.origin.y - button.size.y - 24.0).abs() < 1.0);
    assert!((scroll.origin.y + scroll.size.y - footer.origin.y).abs() < 1.0);
    assert!(
        last_review.origin.y + last_review.size.y > scroll.origin.y + scroll.size.y,
        "the complete review list remains reachable through scrolling"
    );

    let settled = sink.state.doc.clone();
    finalize(&mut sink, &plan);
    assert_eq!(
        sink.state.doc, settled,
        "replayed cleanup must be idempotent"
    );
}
