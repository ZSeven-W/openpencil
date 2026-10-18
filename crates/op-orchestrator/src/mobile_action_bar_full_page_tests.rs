use super::*;
use crate::types::{AppendContext, DesignRequest};
use serde_json::{json, Value};

fn plan(intent: &str) -> OrchestratorPlan {
    serde_json::from_value(json!({
        "rootFrame":{"id":"page","name":"Shop","width":375,"height":812,"layout":"vertical"},
        "subtasks":[{"id":"detail","label":"Product details","screen":"Product",
            "elements":intent,"region":{"width":375,"height":812}}]
    }))
    .unwrap()
}

fn rail() -> Value {
    json!({"type":"frame","id":"opaque-target","name":"opaque", "layout":"horizontal",
    "width":"fill_container","height":80,"gap":12,"padding":[16,0],
    "children":[
        {"type":"frame","id":"amount","width":"fill_container","height":48},
        {"type":"frame","id":"buy","role":"button","width":120,"height":48,
            "events":{"onTap":[{"toast":"\"Ready\""}]}}
    ]})
}

fn page() -> Value {
    json!({"type":"frame","id":"whole-page","width":"fill_container","height":"fit_content",
    "layout":"vertical","children":[
        {"type":"frame","id":"hero","width":"fill_container","height":400},
        {"type":"frame","id":"inset","width":"fill_container","height":"fit_content",
            "layout":"vertical","padding":[0,24,0,24],"children":[
                {"type":"frame","id":"body","width":"fill_container","height":440},
                rail()
            ]}
    ]})
}

fn apply(value: Value, plan: &OrchestratorPlan, req: &DesignRequest) -> Vec<PenNode> {
    let mut nodes = vec![serde_json::from_value(value).unwrap()];
    crate::mobile_action_bar_plan::preserve(
        &mut nodes,
        &plan.subtasks[0],
        plan,
        req,
        &EditorState::new(),
    );
    nodes
}

fn marked_ids(nodes: &[PenNode]) -> Vec<String> {
    let mut result = Vec::new();
    fn visit(node: &PenNode, result: &mut Vec<String>) {
        if is_marked(node) {
            result.push(node.id_str().to_owned());
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, result);
            }
        }
    }
    for node in nodes {
        visit(node, &mut result);
    }
    result
}

#[test]
fn whole_page_intent_marks_only_the_bounded_trailing_descendant() {
    for intent in [
        "商品页：主图、规格、评价三条、底部加购条（客服、加入购物车、立即购买按钮）",
        "购物车页：商品两条、金额汇总、底部结算条（全选、合计、去结算按钮）",
        "Product details with a bottom action bar",
        "A purchase button fixed at the bottom",
    ] {
        let plan = plan(intent);
        let value = page();
        let mut expected: Vec<PenNode> = vec![serde_json::from_value(value.clone()).unwrap()];
        expected[0].children_mut().unwrap()[1]
            .children_mut()
            .unwrap()[1]
            .base_mut()
            .explain = Some(MARKER.into());
        let mut nodes = apply(value, &plan, &DesignRequest::default());
        assert_eq!(
            nodes, expected,
            "only the descendant marker may change: {intent}"
        );
        assert_eq!(marked_ids(&nodes), ["opaque-target"]);
        crate::mobile_action_bar_plan::preserve(
            &mut nodes,
            &plan.subtasks[0],
            &plan,
            &DesignRequest::default(),
            &EditorState::new(),
        );
        assert_eq!(nodes, expected, "marking is idempotent");
        let loaded: Vec<PenNode> =
            serde_json::from_str(&serde_json::to_string(&nodes).unwrap()).unwrap();
        assert_eq!(loaded, expected);
    }
}

#[test]
fn whole_page_numeric_width_and_short_cart_with_top_action_are_supported() {
    let mut value = page();
    value["width"] = json!(375);
    value["height"] = json!(812);
    value["children"][0] = json!({"type":"frame","id":"navigation","width":375,
    "height":56,"layout":"horizontal","children":[
        {"type":"frame","id":"back","role":"button","width":44,"height":44}
    ]});
    value["children"][1]["width"] = json!(375);
    value["children"][1]["children"][0]["height"] = json!(120);
    value["children"][1]["children"][1]["width"] = json!(327);
    let nodes = apply(
        value,
        &plan("底部结算条（全选、结算按钮）"),
        &DesignRequest::default(),
    );
    assert_eq!(marked_ids(&nodes), ["opaque-target"]);
    assert!(
        !is_marked(&nodes[0]),
        "the 812px full-page root must never be a footer"
    );
}

#[test]
fn full_page_contract_rejects_unplanned_or_explicitly_unpinned_content() {
    for intent in [
        "商品页：主图、规格、评价和购买按钮",
        "bottom button after reviews",
        "bottom action bar, not fixed",
        "bottom action bar, not pinned",
        "bottom action bar scrolls with content",
        "bottom action bar in normal document flow",
        "bottom action bar; no footer",
        "底部加购条不固定",
        "底部结算条随内容滚动",
        "无底部操作栏",
        "不要底部加购条",
        "不要页脚，底部结算条",
    ] {
        let nodes = apply(page(), &plan(intent), &DesignRequest::default());
        assert!(marked_ids(&nodes).is_empty(), "{intent}");
    }
}

#[test]
fn full_page_contract_keeps_sections_and_existing_context_unchanged() {
    for mode in [
        "section",
        "wide",
        "append",
        "reference",
        "continuation",
        "nonterminal",
    ] {
        let mut plan = plan("底部加购条");
        let mut req = DesignRequest::default();
        match mode {
            "section" => plan.subtasks[0].region.height = 80.0,
            "wide" => plan.root_frame.width = 1440.0,
            "append" => {
                req.append_context = Some(AppendContext {
                    target_parent_id: "existing".into(),
                    target_width: 375.0,
                    existing_section_labels: vec![],
                    is_mobile: true,
                })
            }
            "reference" => {
                req.reference_skeleton = Some(
                    serde_json::from_value(json!({
                        "source":"reference","width":375,"sections":[],"navKind":"none",
                        "heroKind":"none","columnRhythm":[]
                    }))
                    .unwrap(),
                )
            }
            "continuation" => {
                req.continuation_context = Some(crate::types::ContinuationContext {
                    screen_width: 375.0,
                    screen_height: 812.0,
                    background_color: None,
                    screen_names: vec!["Product".into()],
                })
            }
            "nonterminal" => {
                let mut next = plan.subtasks[0].clone();
                next.id = "next".into();
                plan.subtasks.push(next);
            }
            _ => unreachable!(),
        }
        let original: Vec<PenNode> = vec![serde_json::from_value(page()).unwrap()];
        assert_eq!(apply(page(), &plan, &req), original, "{mode}");
    }
}

#[test]
fn unsafe_wrappers_and_ambiguous_or_unbounded_rails_do_not_gain_intent() {
    for case in [
        "card wrapper",
        "overlay wrapper",
        "hidden wrapper",
        "hidden rail",
        "transparent rail",
        "positioned wrapper",
        "positioned rail",
        "constraints",
        "wrapper expression padding",
        "rail expression padding",
        "rail negative padding",
        "wrapper expression gap",
        "rail expression width",
        "vertical padding",
        "asymmetric padding",
        "oversized gutter",
        "rounded wrapper",
        "wrapper stroke",
        "shadow wrapper",
        "narrow wrapper",
        "narrow rail",
        "horizontal wrapper",
        "geometry binding",
        "card rail",
        "plain button",
        "empty rail",
        "not trailing",
        "duplicate rails",
        "tall rail",
        "clipped overflowing child",
        "positioned child",
        "invisible action",
        "whole page only",
    ] {
        let mut value = page();
        match case {
            "card wrapper" => value["children"][1]["role"] = json!("card"),
            "overlay wrapper" => value["children"][1]["role"] = json!("overlay"),
            "hidden wrapper" => value["children"][1]["visible"] = json!(false),
            "hidden rail" => value["children"][1]["children"][1]["visible"] = json!(false),
            "transparent rail" => value["children"][1]["children"][1]["opacity"] = json!(0),
            "positioned wrapper" => value["children"][1]["y"] = json!(0),
            "positioned rail" => value["children"][1]["children"][1]["x"] = json!(0),
            "constraints" => value["children"][1]["constraints"] = json!({"h":"left","v":"bottom"}),
            "wrapper expression padding" => value["children"][1]["padding"] = json!("$spacing"),
            "rail expression padding" => {
                value["children"][1]["children"][1]["padding"] = json!("$spacing")
            }
            "rail negative padding" => {
                value["children"][1]["children"][1]["padding"] = json!([-1, 0])
            }
            "wrapper expression gap" => value["children"][1]["gap"] = json!("$spacing"),
            "rail expression width" => {
                value["children"][1]["children"][1]["width"] = json!("$width")
            }
            "vertical padding" => value["children"][1]["padding"] = json!([12, 24]),
            "asymmetric padding" => value["children"][1]["padding"] = json!([0, 24, 0, 16]),
            "oversized gutter" => value["children"][1]["padding"] = json!([0, 48]),
            "rounded wrapper" => value["children"][1]["cornerRadius"] = json!(16),
            "wrapper stroke" => value["children"][1]["stroke"] = json!({"thickness":1}),
            "shadow wrapper" => {
                value["children"][1]["effects"] = json!([{"type":"shadow","offsetX":0,"offsetY":2,"blur":8,"spread":0,"color":"#000000"}])
            }
            "narrow wrapper" => value["children"][1]["width"] = json!(300),
            "narrow rail" => value["children"][1]["children"][1]["width"] = json!(280),
            "horizontal wrapper" => value["children"][1]["layout"] = json!("horizontal"),
            "geometry binding" => {
                value["children"][1]["bindings"] = json!({"visible":"$state.open"})
            }
            "card rail" => value["children"][1]["children"][1]["role"] = json!("card"),
            "plain button" => value["children"][1]["children"][1]["role"] = json!("button"),
            "empty rail" => value["children"][1]["children"][1]["children"] = json!([]),
            "not trailing" => value["children"][1]["children"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                "type":"frame","id":"after","width":"fill_container","height":40})),
            "duplicate rails" => {
                let mut other = rail();
                other["id"] = json!("other-rail");
                other["children"][0]["id"] = json!("other-amount");
                other["children"][1]["id"] = json!("other-buy");
                value["children"][1]["children"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
            }
            "tall rail" => value["children"][1]["children"][1]["height"] = json!(160),
            "clipped overflowing child" => {
                value["children"][1]["children"][1]["clipContent"] = json!(true);
                value["children"][1]["children"][1]["children"][1]["height"] = json!(180);
            }
            "positioned child" => {
                value["children"][1]["children"][1]["children"][1]["y"] = json!(0)
            }
            "invisible action" => {
                value["children"][1]["children"][1]["children"][1]["visible"] = json!(false)
            }
            "whole page only" => {
                value = rail();
                value["height"] = json!(812);
            }
            _ => unreachable!(),
        }
        let before: Vec<PenNode> = vec![serde_json::from_value(value.clone()).unwrap()];
        let nodes = apply(
            value,
            &plan("底部结算条（合计与去结算按钮）"),
            &DesignRequest::default(),
        );
        assert_eq!(nodes, before, "{case}");
    }
}
