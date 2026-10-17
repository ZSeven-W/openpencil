//! Regressions from the GLM food delivery draft: wrapped nav and checkout bars.
use super::*;
use serde_json::json;

fn phone(footer: Value) -> EditorState {
    state(
        json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical","gap":16,"children":[
            {"type":"frame","id":"status","name":"Status Bar","height":62,"width":"fill_container"},
            {"type":"frame","id":"content","width":"fill_container","height":"fit_content","layout":"vertical","gap":24,"padding":[24,24],"children":[
                {"type":"text","id":"first","content":"First merchant","height":460,"fontSize":20},
                {"type":"text","id":"last","content":"Fifth merchant","height":360,"fontSize":20}
            ]},footer
        ]}),
    )
}

fn assert_preserved_and_pinned(draft: &mut EditorState) {
    let original = serde_json::to_value(&draft.doc).unwrap();
    let mut sink = crate::loop_finalize::StateDocSink { state: draft };
    repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let footer = page.find("footer").unwrap().bounds;
    assert!(
        footer.origin.y >= 680.0 && footer.origin.y + footer.size.y <= 813.0,
        "{footer:?}"
    );
    assert_eq!(page.find("status").unwrap().bounds.origin.y, 0.0);
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    let children = value["children"][0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0], original["children"][0]["children"][0]);
    assert_eq!(children[1]["role"], "scroll-area");
    assert_eq!(
        children[1]["children"][0], original["children"][0]["children"][1],
        "preserve spacing, type and all content"
    );
    assert_eq!(
        children[2], original["children"][0]["children"][2],
        "preserve footer visuals and events"
    );
    assert!(crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty());
    let before = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(
        sink.state().doc,
        before,
        "intentional scroll content must not be compressed on replay"
    );
}

#[test]
fn wrapped_nav_preserves_status_and_content_spacing() {
    let mut draft = phone(
        json!({"type":"frame","id":"footer","name":"首页 · 底部导航","width":"fill_container","height":"fit_content","layout":"vertical","children":[
            {"type":"rectangle","id":"line","height":1,"width":"fill_container"},
            {"type":"frame","id":"tabs","role":"bottom-tab-bar","width":"fill_container","height":72,"layout":"horizontal","children":[
                {"type":"text","id":"home","content":"首页","height":20},
                {"type":"text","id":"orders","content":"订单","height":20},
                {"type":"text","id":"profile","content":"我的","height":20}
            ]}
        ]}),
    );
    assert_preserved_and_pinned(&mut draft);
}

#[test]
fn cart_and_checkout_bars_are_preserved_outside_scroll_content() {
    for name in ["商家详情 · 底部购物车栏", "cart-bar", "底部提交区"] {
        let mut draft = phone(
            json!({"type":"frame","id":"footer","name":name,"width":"fill_container","height":"fit_content","layout":"vertical","padding":[0,16,28,16],"children":[
                {"type":"frame","id":"checkout","width":"fill_container","height":60,"layout":"horizontal","children":[
                    {"type":"text","id":"price","content":"¥28","fontSize":18},
                    {"type":"frame","id":"buy","role":"button","width":100,"height":44,"events":{"onTap":[{"toast":"Checkout"}]},"children":[
                        {"type":"text","id":"label","content":"去结算","fontSize":16}
                    ]}
                ]}
            ]}),
        );
        assert_preserved_and_pinned(&mut draft);
    }
}

#[test]
fn explicitly_pinned_cta_keeps_the_complete_action_area_outside_the_scroll() {
    for name in [
        "Pinned Start Button",
        "fixed-bottom-cta",
        "底部固定开始训练按钮",
    ] {
        let mut draft = phone(json!({"type":"frame","id":"footer","name":name,
        "role":"button","width":"fill_container","height":"fit_content",
        "layout":"vertical","padding":[12,24,28,24],"children":[
            {"type":"frame","id":"start","role":"button","width":"fill_container",
             "height":48,"events":{"onTap":[{"toast":"Start workout"}]},"children":[
                {"type":"text","id":"start-label","content":"开始训练","fontSize":16}
            ]}
        ]}));
        assert_preserved_and_pinned(&mut draft);
    }
}

#[test]
fn flush_intrinsic_pinned_action_gains_safe_room_without_changing_content() {
    let mut draft = phone(
        json!({"type":"frame","id":"footer","name":"Pinned Start Button",
        "role":"button","width":"fill_container","height":"fit_content",
        "layout":"vertical","padding":[0,24,0,24],"children":[
            {"type":"frame","id":"start","role":"button","width":"fill_container",
             "height":48,"events":{"onTap":[{"toast":"Start workout"}]},"children":[
                {"type":"text","id":"start-label","content":"Start Workout","fontSize":16}
            ]}
        ]}),
    );
    let original = serde_json::to_value(&draft.doc).unwrap();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let footer = page.find("footer").unwrap().bounds;
    let button = page.find("start").unwrap().bounds;
    assert!((footer.origin.y + footer.size.y - 812.0).abs() < 1.0);
    assert_eq!(button.size.y, 48.0);
    assert!((812.0 - button.origin.y - button.size.y - 28.0).abs() < 1.0);
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    let root = &after["children"][0];
    assert_eq!(root["height"], 812.0);
    assert_eq!(root["children"][1]["role"], "scroll-area");
    assert_eq!(
        root["children"][1]["children"][0],
        original["children"][0]["children"][1]
    );
    assert_eq!(
        root["children"][2]["children"],
        original["children"][0]["children"][2]["children"]
    );
    assert!(
        !crate::geometry_validation::geometry_diagnostics(sink.state())
            .iter()
            .any(|message| message.contains("mobile-bottom-flush"))
    );
    let settled = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(settled, sink.state().doc);
}

#[test]
fn action_inset_preserves_existing_outer_room_and_authored_layout_constraints() {
    for case in [
        "outer-room",
        "fixed-height",
        "token-padding",
        "positioned-child",
    ] {
        let mut footer = json!({"type":"frame","id":"footer","name":"Pinned Start Button",
        "role":"button","width":"fill_container","height":"fit_content",
        "layout":"vertical","padding":[0,24,0,24],"children":[
            {"type":"frame","id":"start","role":"button","height":48,
             "events":{"onTap":[{"toast":"Start workout"}]}}
        ]});
        match case {
            "fixed-height" => footer["height"] = json!(88),
            "token-padding" => footer["padding"] = json!("$action-padding"),
            "positioned-child" => footer["children"][0]["x"] = json!(0),
            _ => {}
        }
        let mut draft = phone(footer);
        if case == "outer-room" {
            draft.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new("root"),
                patch_json: json!({"padding":[0,0,28,0]}).to_string(),
                page_id: None,
            });
        }
        let original =
            op_editor_core::walkers::find_node(draft.active_children(), &NodeId::new("footer"))
                .unwrap()
                .clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        super::super::mobile_scroll::repair(&mut sink, "root");
        let after = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new("footer"),
        )
        .unwrap();
        assert_eq!(&original, after, "{case}");
    }
}

#[test]
fn explicit_pinned_cta_can_recover_from_a_previously_saved_scroll_wrapper() {
    let mut draft = phone(
        json!({"type":"frame","id":"footer","name":"Pinned Start Button",
        "role":"button","width":"fill_container","height":88,"children":[
            {"type":"text","id":"start-label","content":"开始训练","height":24}
        ]}),
    );
    let original = serde_json::to_value(&draft.doc).unwrap();
    draft.apply(EditorCommand::MoveNode {
        node_id: NodeId::new("footer"),
        target_parent: NodeId::new("content"),
        page_id: None,
        index: None,
    });
    draft.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new("content"),
        patch_json: json!({"role":"scroll-area","height":"fill_container",
            "clipContent":true,"padding":[0,0,28,0]})
        .to_string(),
        page_id: None,
    });
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    assert!(super::super::mobile_scroll::repair(&mut sink, "root"));
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let footer = page.find("footer").unwrap().bounds;
    assert!(
        (footer.origin.y + footer.size.y - 812.0).abs() < 1.0,
        "{footer:?}"
    );
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    let children = after["children"][0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 3, "Reuse the existing scroll area");
    assert_eq!(children[1]["id"], "content");
    assert_eq!(
        children[1]["children"],
        original["children"][0]["children"][1]["children"]
    );
    assert_eq!(children[2], original["children"][0]["children"][2]);
    let settled = sink.state().doc.clone();
    assert!(!super::super::mobile_scroll::repair(&mut sink, "root"));
    assert_eq!(settled, sink.state().doc);
}

#[test]
fn ordinary_trailing_content_is_not_misidentified_as_fixed_chrome() {
    for (name, action) in [
        ("Card", true),
        ("cart-bar", false),
        ("Start Button", true),
        ("Unpinned Start Button", true),
        ("Pinned article", true),
        ("Pinned Start Button", false),
    ] {
        let mut footer = json!({"type":"frame","id":"footer","name":name,"height":60,"width":"fill_container","children":[
            {"type":"text","id":"tail-copy","content":"More content","fontSize":16}
        ]});
        if action {
            footer["events"] = json!({"onTap":[{"toast":"Read more"}]});
        }
        let mut draft = phone(footer);
        let before = serde_json::to_value(&draft.doc).unwrap();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        assert!(super::super::mobile_scroll::repair(&mut sink, "root"));
        let after = serde_json::to_value(&sink.state().doc).unwrap();
        let children = after["children"][0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[1]["role"], "scroll-area");
        assert_eq!(
            children[1]["children"][1], before["children"][0]["children"][2],
            "ordinary trailing content scrolls with the body, never pinned as chrome"
        );
    }
}

#[test]
fn nested_whole_screen_cart_is_promoted_without_losing_content() {
    let mut draft = state(
        json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical","children":[
            {"type":"frame","id":"status","name":"Status Bar","width":"fill_container","height":62},
            {"type":"frame","id":"shell","width":"fill_container","height":"fit_content","layout":"vertical","children":[
                {"type":"frame","id":"menu","width":"fill_container","height":1000,"layout":"vertical","gap":24,"children":[{"type":"text","id":"dish","content":"All six dishes stay available"}]},
                {"type":"frame","id":"cart","name":"购物车栏","width":"fill_container","height":64,"layout":"horizontal","children":[
                    {"type":"frame","id":"checkout","role":"button","height":44,"events":{"onTap":[{"push":"\"/checkout\""}]}}
                ]}
            ]}
        ]}),
    );
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    assert!(page.find("cart").unwrap().bounds.origin.y + 64.0 <= 813.0);
    assert_eq!(page.find("menu").unwrap().bounds.size.y, 1000.0);
    assert!(page.find("dish").is_some());
    let root = find_root(sink.state(), "root").unwrap();
    assert_eq!(root.children().unwrap().last().unwrap().id_str(), "cart");
    assert!(crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty());
    let before = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(before, sink.state().doc);
}

#[test]
fn nested_card_action_is_not_promoted_as_screen_chrome() {
    let mut draft = state(
        json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical","children":[
            {"type":"frame","id":"card","role":"card","width":"fill_container","layout":"vertical","children":[
                {"type":"text","id":"body","height":1000,"content":"Product details"},
                {"type":"frame","id":"footer","role":"cart-bar","width":"fill_container","height":64,"children":[
                    {"type":"frame","id":"button","role":"button","height":44}
                ]}
            ]}
        ]}),
    );
    let before = draft.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    assert!(!super::super::mobile_scroll::repair(&mut sink, "root"));
    assert_eq!(sink.state().doc, before);
}

fn scroll_text(id: &str, content: &str) -> Value {
    json!({"type":"text","id":id,"content":content,"width":"fit_content","height":"fit_content"})
}

#[test]
fn scroll_creation_replay_keeps_photo_card_and_add_button_inside_one_rail() {
    let mut draft = state(json!({"type":"frame","id":"root","width":390,"height":844,
    "layout":"vertical","clipContent":true,"children":[
        {"type":"frame","id":"status","role":"status-bar","width":"fill_container","height":44},
        {"type":"frame","id":"section","layout":"vertical","width":"fill_container",
         "padding":[0,24],"children":[
            {"type":"frame","id":"card","width":"fill_container","height":236,"layout":"none",
             "clipContent":true,"fill":[{"type":"solid","color":"#F6EDE1"}],"children":[
                {"type":"frame","id":"add","role":"button","x":282,"y":176,"width":44,"height":44,
                 "fill":[{"type":"solid","color":"#713E2B"}],"children":[scroll_text("add-label","+")]},
                {"type":"image","id":"photo","width":342,"height":236,"src":"photo.jpg"}
             ]},
            {"type":"rectangle","id":"more","width":"fill_container","height":900,"fill":[{"type":"solid","color":"#F6EDE1"}]}
         ]},
        {"type":"frame","id":"nav","role":"bottom-tab-bar","width":"fill_container","height":72,
         "layout":"horizontal","children":[scroll_text("home","首页"),scroll_text("menu","菜单"),scroll_text("orders","订单"),scroll_text("profile","我的")]}
    ]}));
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    assert!(super::super::mobile_scroll::repair(&mut sink, "root"));
    for _ in 0..2 {
        crate::mobile_content_rail::repair_mobile_content_rails_for_all_roots(&mut sink);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let page = scene.active_page().unwrap();
        let card = page.find("card").unwrap().bounds;
        let button = page.find("add").unwrap().bounds;
        assert!((card.size.x - 342.0).abs() < 0.5, "{card:?}");
        assert!(
            button.origin.x + button.size.x <= card.origin.x + card.size.x + 0.5,
            "{button:?} {card:?}"
        );
        assert!(
            !crate::geometry_validation::geometry_diagnostics(sink.state())
                .iter()
                .any(|s| s.contains("clipped-control"))
        );
    }
    assert!(serde_json::to_value(&sink.state().doc)
        .unwrap()
        .to_string()
        .contains("scroll-area"));
}

#[test]
fn detail_without_footer_scrolls_without_compressing_content() {
    let mut draft = phone(json!({"type":"text","id":"last-row","content":"Split 5",
        "width":"fill_container","height":44,"fontSize":18}));
    let before = serde_json::to_value(&draft.doc).unwrap();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    repair(&mut sink, "root");
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    let root = &after["children"][0];
    assert_eq!(root["height"], 812.0);
    assert_eq!(root["children"][0], before["children"][0]["children"][0]);
    let scroll = &root["children"][1];
    assert_eq!(scroll["role"], "scroll-area");
    assert_eq!(scroll["clipContent"], true);
    assert_eq!(scroll["padding"], json!([0.0, 0.0, 28.0, 0.0]));
    assert!(scroll["events"]["onScroll"].is_array());
    assert_eq!(
        scroll["children"],
        json!(before["children"][0]["children"].as_array().unwrap()[1..])
    );
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let viewport = page.find(scroll["id"].as_str().unwrap()).unwrap().bounds;
    let last = page.find("last-row").unwrap().bounds;
    assert_eq!(viewport.origin.y, 62.0);
    assert_eq!(viewport.size.y, 750.0);
    assert!(
        last.origin.y + last.size.y > 812.0,
        "overflow retained for scrolling"
    );
    assert!(crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty());
    let settled = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(settled, sink.state().doc);
}

#[test]
fn fitting_detail_and_narrow_poster_are_not_given_scroll() {
    for has_status in [true, false] {
        let mut draft = state(json!({"type":"frame","id":"root","width":375,"height":812,
        "layout":"vertical","children":[
            {"type":"frame","id":"top","height":62,"width":"fill_container"},
            {"type":"text","id":"body","content":"Keep my layout","height":if has_status {400} else {1000}}
        ]}));
        if has_status {
            draft.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new("top"),
                patch_json: json!({"role":"status-bar"}).to_string(),
                page_id: None,
            });
        }
        let before = draft.doc.clone();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        assert!(!super::super::mobile_scroll::repair(&mut sink, "root"));
        assert_eq!(before, sink.state().doc);
    }
}

#[test]
fn short_phone_content_keeps_only_the_navigation_at_the_bottom() {
    let mut draft = phone(json!({"type":"frame","id":"footer","role":"bottom-tab-bar",
    "width":"fill_container","height":78,"layout":"horizontal","children":[
        {"type":"text","id":"home","content":"首页","height":20},
        {"type":"text","id":"orders","content":"订单","height":20},
        {"type":"text","id":"profile","content":"我的","height":20}
    ]}));
    for (id, height) in [("first", 100), ("last", 120)] {
        draft.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: json!({"height":height}).to_string(),
            page_id: None,
        });
    }
    let original = serde_json::to_value(&draft.doc).unwrap();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
    super::super::repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let footer = page.find("footer").unwrap().bounds;
    assert!(
        (footer.origin.y + footer.size.y - 812.0).abs() < 1.0,
        "{footer:?}"
    );
    let body = page.find("content").unwrap().bounds;
    assert_eq!(body.size.y, 292.0, "Do not stretch or distribute the body");
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    let children = after["children"][0]["children"].as_array().unwrap();
    assert_eq!(
        children[1]["children"][0],
        original["children"][0]["children"][1]
    );
    assert_eq!(children[2], original["children"][0]["children"][2]);
    let settled = sink.state().doc.clone();
    super::super::repair(&mut sink, "root");
    assert_eq!(settled, sink.state().doc);
}
