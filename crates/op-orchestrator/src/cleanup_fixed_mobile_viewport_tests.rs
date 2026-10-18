//! Retained 2026-10-18 full-page mobile layouts, not simplified repair-shaped trees.
//! Fixture manifests distinguish semantically preserved JSON from reconstruction.
use crate::cleanup::{finalize_design_with_summary_and_policy, CleanupPolicy};
use crate::plan::OrchestratorPlan;
use crate::repair_summary::RepairSummary;
use crate::test_support::VecDocSink;
use crate::types::DesignRequest;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const PRODUCT: &str = include_str!("test_fixtures/full_page_mobile_product_1018.json");
const RESTAURANT: &str = include_str!("test_fixtures/full_page_mobile_restaurant_1018.json");
const PLANS: &str = include_str!("test_fixtures/full_page_mobile_plans_1018.json");
const MANIFEST: &str = include_str!("test_fixtures/full_page_mobile_manifest_1018.json");

fn retained(source: &str) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(serde_json::from_str(source).unwrap());
    sink
}

fn node<'a>(sink: &'a VecDocSink, id: &str) -> &'a PenNode {
    op_editor_core::walkers::find_node(sink.state.active_children(), &NodeId::new(id))
        .unwrap_or_else(|| panic!("retained node {id} must survive"))
}

fn value(sink: &VecDocSink, id: &str) -> Value {
    serde_json::to_value(node(sink, id)).unwrap()
}

fn bounds(sink: &VecDocSink, id: &str) -> [f64; 4] {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let b = scene.active_page().unwrap().find(id).unwrap().bounds;
    [
        f64::from(b.origin.x),
        f64::from(b.origin.y),
        f64::from(b.size.x),
        f64::from(b.size.y),
    ]
}

fn request_and_plan(kind: &str) -> (DesignRequest, OrchestratorPlan) {
    let evidence: Value = serde_json::from_str(PLANS).unwrap();
    let req = DesignRequest {
        prompt: evidence[kind]["prompt"].as_str().unwrap().into(),
        ..DesignRequest::default()
    };
    // Both restaurant planner responses were rejected. The actual run used
    // the production fallback, not either of those rejected JSON plans.
    let mut plan = if kind == "product" {
        serde_json::from_value(evidence[kind]["plannerAttempts"][0]["plan"].clone()).unwrap()
    } else {
        crate::plan::build_fallback_plan(&req)
    };
    crate::plan_normalize::normalize(&mut plan, &req);
    for task in &mut plan.subtasks {
        task.parent_frame_id = match task.screen.as_deref() {
            Some("商品页") => Some("n180".into()),
            Some("购物车页") => Some("n270".into()),
            Some("首页") => Some("n11".into()),
            Some("商家详情") => Some("n241".into()),
            _ => None,
        };
    }
    (req, plan)
}

fn replay_product_intent(sink: &mut VecDocSink, plan: &OrchestratorPlan, req: &DesignRequest) {
    let context = sink.state.clone();
    for (body_id, task_id, bar_id) in [
        ("n181", "product-page", "n252"),
        ("n281", "cart-page", "n339"),
    ] {
        let Some(body) = op_editor_core::walkers::find_node_mut(
            &mut sink.state.doc.children,
            &NodeId::new(body_id),
        ) else {
            continue;
        };
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == task_id)
            .unwrap();
        crate::mobile_action_bar_plan::preserve(
            std::slice::from_mut(body),
            task,
            plan,
            req,
            &context,
        );
        assert!(
            !crate::mobile_action_bar_plan::is_marked(body),
            "a full page must never become its own fixed action bar"
        );
        assert!(crate::mobile_action_bar_plan::is_marked(node(sink, bar_id)));
    }
}

fn cleanup(sink: &mut VecDocSink, plan: &OrchestratorPlan, roots: &[&str], fixed: bool) {
    finalize_design_with_summary_and_policy(
        sink,
        plan,
        roots,
        &mut RepairSummary::default(),
        CleanupPolicy {
            preserve_requested_root_height: fixed,
            roots_are_run_output: true,
            is_deck: false,
        },
    );
}

fn is_scroll(node: &PenNode) -> bool {
    matches!(
        node.base().role.as_deref(),
        Some("scroll" | "scroll-area" | "scroll-viewport" | "viewport")
    )
}

fn contains(node: &PenNode, id: &str) -> bool {
    node.id_str() == id
        || node
            .children()
            .is_some_and(|children| children.iter().any(|child| contains(child, id)))
}

fn scroll_contains(node: &PenNode, id: &str) -> bool {
    (is_scroll(node) && contains(node, id))
        || node
            .children()
            .is_some_and(|children| children.iter().any(|child| scroll_contains(child, id)))
}

fn assert_viewport(sink: &VecDocSink, root_id: &str, content_id: &str, footer_id: &str) {
    let root = node(sink, root_id);
    assert_eq!(value(sink, root_id)["height"], 812.0);
    assert_eq!(
        root.children().unwrap()[0].base().role.as_deref(),
        Some("status-bar")
    );
    assert!(
        scroll_contains(root, content_id),
        "{content_id} needs a scroll ancestor"
    );
    assert!(
        !scroll_contains(root, footer_id),
        "{footer_id} must stay outside the scroll"
    );
    let board = bounds(sink, root_id);
    let footer = bounds(sink, footer_id);
    let bottom_gap = board[1] + board[3] - footer[1] - footer[3];
    assert!(
        footer[1] > board[1] + 600.0,
        "footer is floating: {footer:?}"
    );
    assert!(
        (-1.0..=29.0).contains(&bottom_gap),
        "footer bottom gap {bottom_gap}"
    );
}

fn business_contract(sink: &VecDocSink) -> BTreeMap<String, Value> {
    fn visit(node: &PenNode, result: &mut BTreeMap<String, Value>) {
        let value = serde_json::to_value(node).unwrap();
        let mut contract = serde_json::Map::new();
        for key in ["content", "events", "bindings", "state"] {
            if let Some(field) = value.get(key) {
                contract.insert(key.into(), field.clone());
            }
        }
        if matches!(
            node.base().role.as_deref(),
            Some("button" | "icon-button" | "input")
        ) {
            contract.insert("role".into(), value["role"].clone());
        }
        if !contract.is_empty() {
            result.insert(node.id_str().into(), Value::Object(contract));
        }
        for child in node.children().into_iter().flatten() {
            visit(child, result);
        }
    }
    let mut result = BTreeMap::new();
    for root in sink.state.active_children() {
        visit(root, &mut result);
    }
    result
}

fn assert_business_preserved(before: &BTreeMap<String, Value>, after: &VecDocSink) {
    for (id, fields) in before {
        let actual = value(after, id);
        for (key, expected) in fields.as_object().unwrap() {
            assert_eq!(&actual[key], expected, "{id}.{key} must remain authored");
        }
    }
}

fn assert_replay_stable(sink: &mut VecDocSink, plan: &OrchestratorPlan, roots: &[&str]) {
    let first = sink.state.doc.clone();
    cleanup(sink, plan, roots, true);
    assert_eq!(
        sink.state.doc, first,
        "a second complete cleanup must be idempotent"
    );
}

#[test]
fn retained_mobile_fixtures_keep_the_original_defects_and_provenance() {
    let product = retained(PRODUCT);
    let restaurant = retained(RESTAURANT);
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    assert_eq!(
        manifest["documents"][0]["sourceSha256"],
        "1d6698628642f269573a29ae5c80c359df435d6ca5caa1ee245501a6345f85e1"
    );
    assert_eq!(
        manifest["documents"][1]["sourceSha256"],
        "6142095bc8b4d00794d202b93d96a253d5e01e256d4b740d3a37ba92068c578a"
    );
    assert_eq!(value(&product, "n180")["height"], 812.0);
    assert_eq!(value(&restaurant, "n11")["height"], 1263.0);
    assert!(!scroll_contains(node(&product, "n180"), "n251"));
    assert!(!crate::mobile_action_bar_plan::is_marked(node(
        &product, "n252"
    )));
    assert_eq!(
        value(&product, "n339")["padding"],
        json!([16.0, 24.0, 32.0, 24.0])
    );
    assert!(bounds(&product, "n252")[1] > bounds(&product, "n180")[1] + 812.0);
    let plans: Value = serde_json::from_str(PLANS).unwrap();
    assert_eq!(plans["product"]["acceptedPlannerLine"], 7);
    assert_eq!(plans["restaurant"]["allPlannerAttemptsRejected"], true);
}

#[test]
fn retained_product_and_short_cart_keep_812_with_scroll_and_gutter_owned_footers() {
    let mut sink = retained(PRODUCT);
    let (req, plan) = request_and_plan("product");
    let before = business_contract(&sink);
    let original_state = serde_json::to_value(&sink.state.doc).unwrap()["state"].clone();
    let original_cart = bounds(&sink, "n339");
    let original_padding = value(&sink, "n339")["padding"].clone();
    replay_product_intent(&mut sink, &plan, &req);
    cleanup(&mut sink, &plan, &["n180", "n270"], true);
    assert_viewport(&sink, "n180", "n251", "n252");
    assert_viewport(&sink, "n270", "n289", "n339");
    let cart = bounds(&sink, "n339");
    assert!(
        (cart[0] - original_cart[0]).abs() < 1.0,
        "preserve the outer 24px rail"
    );
    assert!(
        (cart[2] - original_cart[2]).abs() < 1.0,
        "preserve the 327px bar width"
    );
    assert_eq!(value(&sink, "n339")["padding"], original_padding);
    let board = bounds(&sink, "n270");
    assert!((board[1] + board[3] - cart[1] - cart[3]).abs() < 1.0);
    for (root, control) in [("n180", "n256"), ("n180", "n258"), ("n270", "n348")] {
        let board = bounds(&sink, root);
        let button = bounds(&sink, control);
        assert!(
            button[1] >= board[1] && button[1] + button[3] <= board[1] + board[3] + 1.0,
            "{control} must be visible in the initial viewport: {button:?}"
        );
    }
    assert_business_preserved(&before, &sink);
    assert_eq!(
        serde_json::to_value(&sink.state.doc).unwrap()["state"],
        original_state
    );
    assert_replay_stable(&mut sink, &plan, &["n180", "n270"]);
}

#[test]
fn retained_product_late_status_is_anchored_before_spacing_is_spent() {
    let mut document: Value = serde_json::from_str(PRODUCT).unwrap();
    document["children"]
        .as_array_mut()
        .unwrap()
        .retain(|root| root["id"] == "n180");
    let children = document["children"][0]["children"].as_array_mut().unwrap();
    let status = children.remove(0);
    assert_eq!(status["id"], "n260");
    children.push(status);
    let mut sink = retained(&document.to_string());
    let (req, plan) = request_and_plan("product");
    let before = business_contract(&sink);
    let rhythm: Vec<_> = ["n199", "n214", "n220", "n231", "n242"]
        .into_iter()
        .map(|id| {
            (
                id,
                value(&sink, id)["gap"].clone(),
                value(&sink, id)["padding"].clone(),
            )
        })
        .collect();
    replay_product_intent(&mut sink, &plan, &req);
    cleanup(&mut sink, &plan, &["n180"], true);
    assert_viewport(&sink, "n180", "n251", "n252");
    assert_eq!(node(&sink, "n180").children().unwrap()[0].id_str(), "n260");
    for (id, gap, padding) in rhythm {
        assert_eq!(
            value(&sink, id)["gap"],
            gap,
            "{id} must not be compressed again"
        );
        assert_eq!(
            value(&sink, id)["padding"],
            padding,
            "{id} must keep its padding"
        );
    }
    assert_business_preserved(&before, &sink);
    assert_replay_stable(&mut sink, &plan, &["n180"]);
}

fn restaurant_at_requested_height(pre_cleanup_reconstruction: bool) -> VecDocSink {
    let mut document: Value = serde_json::from_str(RESTAURANT).unwrap();
    let home = &mut document["children"][0];
    assert_eq!(home["id"], "n11");
    assert_eq!(home["height"], 1263.0);
    home["height"] = json!(812.0);
    if pre_cleanup_reconstruction {
        // generation.log:703 records root 812 -> 1263 BEFORE inserting n240.
        // Unwrap only that later-added scroll, preserving its existing body.
        let children = home["children"].as_array_mut().unwrap();
        assert_eq!(children[1]["id"], "n240");
        let scroll = children.remove(1);
        let bodies = scroll["children"].as_array().unwrap().clone();
        children.splice(1..1, bodies);
        assert_eq!(children[0]["id"], "n12");
        assert_eq!(children.last().unwrap()["id"], "n229");
    }
    retained(&document.to_string())
}

#[test]
fn retained_restaurant_requested_812_survives_save_replay_and_pre_cleanup_reconstruction() {
    let (_, plan) = request_and_plan("restaurant");
    for reconstructed in [false, true] {
        let mut sink = restaurant_at_requested_height(reconstructed);
        let before = business_contract(&sink);
        // The detail's missing cart bar is retained as a known delivery defect.
        // A layout repair must not fabricate controls to satisfy this test.
        let original_detail_buttons: Vec<_> = before
            .iter()
            .filter(|(id, fields)| {
                fields.get("role").is_some() && contains(node(&sink, "n241"), id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        cleanup(&mut sink, &plan, &["n11", "n241"], true);
        assert_viewport(&sink, "n11", "n33", "n229");
        assert_eq!(value(&sink, "n241")["height"], 812.0);
        assert!(scroll_contains(node(&sink, "n241"), "n252"));
        assert_business_preserved(&before, &sink);
        let actual_detail_buttons: Vec<_> = business_contract(&sink)
            .into_iter()
            .filter(|(id, fields)| {
                fields.get("role").is_some() && contains(node(&sink, "n241"), id)
            })
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            actual_detail_buttons, original_detail_buttons,
            "viewport repair must not fabricate the missing restaurant cart controls"
        );
        assert_replay_stable(&mut sink, &plan, &["n11", "n241"]);
    }
}

#[test]
fn adaptive_retained_home_still_grows_when_no_fixed_height_contract_exists() {
    let (_, plan) = request_and_plan("restaurant");
    let mut sink = restaurant_at_requested_height(true);
    cleanup(&mut sink, &plan, &["n11"], false);
    assert!(value(&sink, "n11")["height"].as_f64().unwrap() > 812.0);
    assert!(contains(node(&sink, "n11"), "n229"));
}

#[test]
fn an_unmarked_full_page_does_not_promote_its_ordinary_trailing_action_row() {
    let mut sink = retained(PRODUCT);
    let (_, plan) = request_and_plan("product");
    // Intentionally skip the request-derived insertion marker. Names and the
    // presence of action controls alone cannot establish pinning ownership.
    cleanup(&mut sink, &plan, &["n180"], true);
    assert!(!crate::mobile_action_bar_plan::is_marked(node(
        &sink, "n252"
    )));
    assert!(scroll_contains(node(&sink, "n180"), "n252"));
}

#[test]
fn fixed_viewport_recovers_numeric_construction_wrapper_without_clipping_the_tail() {
    // The retained public reflow regression uses this exact 62 + 782 = 844
    // construction shell. Fixed mode must keep its hug repair, not its growth.
    let document = json!({"version":"1.0.0","children":[{
        "type":"frame","id":"root","name":"Viewport regression","width":390,"height":844,
        "layout":"vertical","clipContent":true,"children":[
            {"type":"frame","id":"status","role":"status-bar","name":"Status Bar",
             "width":"fill_container","height":62,"children":[
                {"type":"frame","id":"levels","name":"Levels","width":60,"height":14}
            ]},
            {"type":"frame","id":"content","name":"Temporary content wrapper","width":"fill_container","height":782,
             "layout":"vertical","gap":20,"clipContent":true,"children":[
                {"type":"frame","id":"a","name":"First content section","width":"fill_container","height":600,
                 "children":[{"type":"text","id":"a-label","content":"First content","height":24}]},
                {"type":"frame","id":"b","name":"Last content section","width":"fill_container","height":500,
                 "children":[{"type":"text","id":"b-label","content":"Last content","height":24}]}
            ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","name":"Bottom Tab Bar",
             "width":"fill_container","height":72,"layout":"horizontal","children":[
                {"type":"text","id":"tab-home","content":"Home","width":"fill_container","height":24},
                {"type":"text","id":"tab-orders","content":"Orders","width":"fill_container","height":24},
                {"type":"text","id":"tab-me","content":"Me","width":"fill_container","height":24}
            ]}
        ]
    }]});
    let mut sink = retained(&document.to_string());
    let plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"Phone","width":390,"height":844,"layout":"vertical"},
        "subtasks":[]
    }))
    .unwrap();
    assert_eq!(node(&sink, "root").id_str(), "root", "loading keeps IDs");
    cleanup(&mut sink, &plan, &["root"], true);
    // Complete cleanup may remint a root through a structural ReplaceSubtree
    // repair. Resolve unique authored identities rather than relying on IDs
    // from before cleanup.
    fn named_id(sink: &VecDocSink, name: &str) -> String {
        fn collect(node: &PenNode, name: &str, ids: &mut Vec<String>) {
            if node.base().name.as_deref() == Some(name) {
                ids.push(node.id_str().to_owned());
            }
            for child in node.children().into_iter().flatten() {
                collect(child, name, ids);
            }
        }
        let mut ids = Vec::new();
        for root in sink.state.active_children() {
            collect(root, name, &mut ids);
        }
        assert_eq!(ids.len(), 1, "one surviving authored {name}: {ids:?}");
        ids.remove(0)
    }
    assert_eq!(
        sink.state.active_children().len(),
        1,
        "no missing or extra screen"
    );
    let root_id = named_id(&sink, "Viewport regression");
    let content_id = named_id(&sink, "Temporary content wrapper");
    let first_id = named_id(&sink, "First content section");
    let last_id = named_id(&sink, "Last content section");
    let nav_id = named_id(&sink, "Bottom Tab Bar");
    assert_eq!(value(&sink, &root_id)["height"], 844.0);
    assert_eq!(value(&sink, &content_id)["height"], "fit_content");
    assert!(scroll_contains(node(&sink, &root_id), &last_id));
    assert_eq!(node(&sink, &content_id).children().unwrap().len(), 2);
    assert_eq!(bounds(&sink, &first_id)[3], 600.0);
    assert_eq!(
        value(&sink, &first_id)["children"][0]["content"],
        "First content"
    );
    assert_eq!(
        value(&sink, &last_id)["children"][0]["content"],
        "Last content"
    );
    let content = bounds(&sink, &content_id);
    let last = bounds(&sink, &last_id);
    assert_eq!(last[3], 500.0);
    assert!(
        last[1] + last[3] <= content[1] + content[3] + 1.0,
        "the inner clip must not hide the last item: content={content:?}, last={last:?}"
    );
    let nav = bounds(&sink, &nav_id);
    assert!((nav[1] + nav[3] - 844.0).abs() < 1.0);
    assert_replay_stable(&mut sink, &plan, &[&root_id]);
}
