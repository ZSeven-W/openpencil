//! Regressions for images arriving after an interactive preview has started.

use jian_core::action::services::Router;
use jian_core::widget_state::WidgetState;
use jian_ops_schema::PenDocument;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

use crate::tests_binding_overlay::find;
use crate::{test_measure, PreviewSession};

fn fixture() -> Value {
    json!({"version":"1.1", "children":[{
        "type":"frame", "id":"screen", "width":375, "height":812,
        "children":[
            {"type":"frame","id":"photo","name":"Photo (image fallback)",
             "x":20,"y":40,"width":120,"height":120,"fill":[{"type":"solid","color":"#eeeeee"}],
             "explain":"image fallback: media {}","children":[
                 {"type":"icon_font","id":"photo-image-fallback-icon","name":"Image fallback icon",
                  "iconFontFamily":"lucide","iconFontName":"image","width":24,"height":24},
                 {"type":"text","id":"photo-image-fallback-caption","content":"Workout photo"}
             ]},
            {"type":"image","id":"image","src":"placeholder://image-search-failed",
             "x":160,"y":40,"width":80,"height":80},
            {"type":"switch","id":"switch","x":20,"y":180,"width":44,"height":24},
            {"type":"text","id":"title","x":20,"y":220,"content":"Original title"}
        ]
    }]})
}

fn enter(value: &Value, page: usize) -> PreviewSession {
    let doc: PenDocument = serde_json::from_value(value.clone()).unwrap();
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 900.0),
        &Default::default(),
        page,
        false,
        false,
        test_measure(),
        10,
    )
    .unwrap();
    session.bind_asset_source(7, 3);
    session
}

fn filled(mut value: Value) -> PenDocument {
    value["children"][0]["children"][0]["fill"] =
        json!([{"type":"image","url":"op-image:accepted-photo","mode":"crop"}]);
    value["children"][0]["children"][0]["children"] = json!([]);
    value["children"][0]["children"][1]["src"] = json!("op-image:accepted-thumb");
    serde_json::from_value(value).unwrap()
}

#[test]
fn accepted_image_sync_refreshes_paint_and_reset_without_resetting_input_or_geometry() {
    let value = fixture();
    let mut session = enter(&value, 0);
    let (x, y, w, h) = session.node_rect("switch").unwrap();
    assert!(session.dispatch_tap(x + w / 2.0, y + h / 2.0));
    let before = session.preview_scene_for_test();
    let rect = find(&before, "photo").bounds;
    let old_icon = find(&before, "photo-image-fallback-icon").bounds;
    let route_generation = session.route_generation;
    let mut latest = filled(value);
    // Even when a background completion shares a revision with user edits,
    // only images cross into the running preview.
    let root = latest.children[0].children_mut().unwrap();
    if let jian_ops_schema::node::PenNode::Text(title) = &mut root[3] {
        title.content = serde_json::from_value(json!("Unrelated edit")).unwrap();
    }
    root[0].base_mut().x = Some(200.0);
    assert!(session.sync_image_assets(&latest, 7, 3));
    let scene = session.preview_scene_for_test();
    assert_eq!(
        find(&scene, "photo").image_src.as_deref(),
        Some("op-image:accepted-photo")
    );
    assert_eq!(
        find(&scene, "image").image_src.as_deref(),
        Some("op-image:accepted-thumb")
    );
    assert!(scene
        .active_page()
        .unwrap()
        .find("photo-image-fallback-icon")
        .is_none());
    assert!(scene
        .active_page()
        .unwrap()
        .find("photo-image-fallback-caption")
        .is_none());
    assert_eq!(find(&scene, "photo").bounds, rect);
    assert_eq!(
        find(&scene, "title").text.as_deref(),
        Some("Original title")
    );
    assert_eq!(session.route_generation, route_generation);
    assert!(matches!(
        session.runtime.widget_states.get("switch"),
        Some(WidgetState::Toggle { on: true })
    ));
    assert!(
        !session.sync_image_assets(&latest, 7, 3),
        "same asset revision is a no-op"
    );
    let _ = session.dispatch_tap(old_icon.origin.x + 2.0, old_icon.origin.y + 2.0);
    let _ = session.dispatch_wheel(old_icon.origin.x + 2.0, old_icon.origin.y + 2.0, 0.0, -20.0);
    session.reset().unwrap();
    assert_eq!(
        find(&session.preview_scene_for_test(), "photo")
            .image_src
            .as_deref(),
        Some("op-image:accepted-photo")
    );
    assert!(
        !session.sync_image_assets(&latest, 8, 3),
        "reset retains document lifetime guard"
    );
}

#[test]
fn image_sync_rejects_new_document_lifetimes_and_failed_sources() {
    let value = fixture();
    let mut session = enter(&value, 0);
    let latest = filled(value.clone());
    assert!(!session.sync_image_assets(&latest, 8, 3));
    assert!(!session.sync_image_assets(&latest, 7, 4));
    assert!(session.sync_image_assets(&latest, 7, 3));
    assert!(!session.sync_image_assets(&serde_json::from_value(value).unwrap(), 7, 3));
    assert_eq!(
        find(&session.preview_scene_for_test(), "image")
            .image_src
            .as_deref(),
        Some("op-image:accepted-thumb")
    );
}

#[test]
fn photo_card_content_and_non_asset_fill_edits_are_not_synchronized() {
    let mut value = fixture();
    let card = &mut value["children"][0]["children"][0];
    card["name"] = json!("Authored photo card");
    card.as_object_mut().unwrap().remove("explain");
    card["fill"] = json!([{"type":"image","url":"op-image:old","mode":"fit"},{"type":"solid","color":"#ffffff"}]);
    card["children"] = json!([{"type":"text","id":"caption","content":"Keep my caption"}]);
    let mut session = enter(&value, 0);
    let mut latest = value.clone();
    latest["children"][0]["children"][0]["fill"][1]["color"] = json!("#ff0000");
    assert!(!session.sync_image_assets(&serde_json::from_value(latest.clone()).unwrap(), 7, 3));
    latest["children"][0]["children"][0]["fill"][0]["url"] = json!("op-image:new");
    latest["children"][0]["children"][0]["fill"][0]["mode"] = json!("crop");
    latest["children"][0]["children"][0]["children"] = json!([]);
    assert!(session.sync_image_assets(&serde_json::from_value(latest).unwrap(), 7, 3));
    let scene = session.preview_scene_for_test();
    assert_eq!(
        find(&scene, "caption").text.as_deref(),
        Some("Keep my caption")
    );
    let saved = serde_json::to_value(&session.reset_seed.document).unwrap();
    assert_eq!(
        saved["children"][0]["children"][0]["fill"][0]["mode"],
        "fit"
    );
    assert_eq!(
        saved["children"][0]["children"][0]["fill"][1]["color"],
        "#ffffff"
    );
}

#[test]
fn assets_update_offscreen_routes_and_keep_current_navigation_transition() {
    let mut value: Value =
        serde_json::from_str(crate::tests_app_mode::TWO_SCREEN_DOC_JSON).unwrap();
    value["pages"][0]["children"][0]["children"].as_array_mut().unwrap().push(json!({
        "type":"image","id":"home-image","src":"placeholder://image-search-failed","x":100,"y":20,"width":50,"height":50
    }));
    value["pages"][0]["children"][1]["children"].as_array_mut().unwrap().push(json!({
        "type":"image","id":"detail-image","src":"placeholder://image-search-failed","x":100,"y":20,"width":50,"height":50
    }));
    let mut session = enter(&value, 0);
    session.set_state("email", json!("retained@example.com"));
    session.router_for_test().push("/detail");
    assert!(session.reconcile(20).switched);
    let generation = session.route_generation;
    assert!(session.transition.is_some());
    value["pages"][0]["children"][0]["children"][3]["src"] = json!("op-image:home");
    value["pages"][0]["children"][1]["children"][1]["src"] = json!("op-image:detail");
    assert!(session.sync_image_assets(&serde_json::from_value(value).unwrap(), 7, 3));
    assert_eq!(session.current_path_for_test(), "/detail");
    assert_eq!(session.route_generation, generation);
    assert!(
        session.transition.is_some(),
        "asset arrival must not cancel a transition"
    );
    assert_eq!(
        find(&session.preview_scene_for_test(), "detail-image")
            .image_src
            .as_deref(),
        Some("op-image:detail")
    );
    session.router_for_test().pop();
    assert!(session.reconcile(500).switched);
    assert_eq!(
        find(&session.preview_scene_for_test(), "home-image")
            .image_src
            .as_deref(),
        Some("op-image:home")
    );
    assert_eq!(
        session.runtime.state.app_get("email").unwrap().0,
        json!("retained@example.com")
    );
    session.router_for_test().push("/detail");
    assert!(session.reconcile(1000).switched);
    assert_eq!(
        find(&session.preview_scene_for_test(), "detail-image")
            .image_src
            .as_deref(),
        Some("op-image:detail")
    );
}

#[test]
fn asset_arrival_keeps_scroll_and_binding_materialized_source() {
    let mut value = fixture();
    value["state"] = json!({"wide":{"type":"number","default":120}});
    value["children"][0]["children"][0]["bindings"] = json!({"width":"$app.wide"});
    let body = value["children"][0]["children"].clone();
    value["children"][0]["children"] = json!([{
        "type":"frame","id":"scroller","width":375,"height":400,"clipContent":true,
        "events":{"onScroll":[{"set":{"$app.scrolled":"true"}}]},
        "children":[{"type":"frame","id":"body","width":375,"height":1200,"children":body}]
    }]);
    let mut session = enter(&value, 0);
    assert!(session.dispatch_wheel(100.0, 300.0, 0.0, -80.0));
    let y = find(&session.preview_scene_for_test(), "body")
        .bounds
        .origin
        .y;
    assert!(y < 0.0);
    value["children"][0]["children"][0]["children"][0]["children"][0]["fill"] =
        json!([{"type":"image","url":"op-image:scrolled"}]);
    value["children"][0]["children"][0]["children"][0]["children"][0]["children"] = json!([]);
    assert!(session.sync_image_assets(&serde_json::from_value(value).unwrap(), 7, 3));
    let after = session.preview_scene_for_test();
    assert_eq!(find(&after, "body").bounds.origin.y, y);
    assert_eq!(
        find(&after, "photo").image_src.as_deref(),
        Some("op-image:scrolled")
    );
    session.set_state("wide", json!(150));
    assert_eq!(
        find(&session.preview_scene_for_test(), "photo")
            .image_src
            .as_deref(),
        Some("op-image:scrolled")
    );
}

#[test]
fn active_page_projection_receives_only_its_matching_assets() {
    let root = fixture()["children"][0].clone();
    let second = json!({"type":"frame","id":"screen-two","width":375,"height":812,
        "children":[{"type":"frame","id":"page-two-photo","width":120,"height":120,
        "fill":[{"type":"solid","color":"#eeeeee"}]}]});
    let mut value = json!({"version":"1.1","pages":[{"id":"one","name":"One","children":[root]},
        {"id":"two","name":"Two","children":[second]}]});
    let mut session = enter(&value, 1);
    value["pages"][1]["children"][0]["children"][0]["fill"] =
        json!([{"type":"image","url":"op-image:page-two"}]);
    assert!(session.sync_image_assets(&serde_json::from_value(value).unwrap(), 7, 3));
    let scene = session.preview_scene_for_test();
    assert_eq!(
        find(&scene, "page-two-photo").image_src.as_deref(),
        Some("op-image:page-two")
    );
    assert!(scene.active_page().unwrap().find("photo").is_none());
}

#[test]
fn component_root_fallback_is_removed_in_resolved_instance() {
    let mut value = fixture();
    let mut component = value["children"][0]["children"][0].clone();
    component["reusable"] = json!(true);
    value["children"][0]["children"] = json!([
        component,
        {"type":"ref","id":"instance","ref":"photo","x":180,"y":30}
    ]);
    let mut session = enter(&value, 0);
    let before = session.preview_scene_for_test();
    assert!(before
        .active_page()
        .unwrap()
        .find("instance__photo-image-fallback-icon")
        .is_some());
    value["children"][0]["children"][0]["fill"] =
        json!([{"type":"image","url":"op-image:component"}]);
    value["children"][0]["children"][0]["children"] = json!([]);
    assert!(session.sync_image_assets(&serde_json::from_value(value).unwrap(), 7, 3));
    let after = session.preview_scene_for_test();
    assert_eq!(
        find(&after, "instance").image_src.as_deref(),
        Some("op-image:component")
    );
    assert!(after
        .active_page()
        .unwrap()
        .find("instance__photo-image-fallback-icon")
        .is_none());
    assert!(session
        .runtime
        .document
        .as_ref()
        .unwrap()
        .tree
        .get("instance__photo-image-fallback-icon")
        .is_none());
}

#[test]
fn accepted_image_arrival_does_not_restart_an_inflight_animation() {
    let mut value = fixture();
    value["children"][0]["children"][0]["animations"] = json!([{
        "trigger":"mount","keyframes":[{"offset":0,"values":{"opacity":0}},
        {"offset":1,"values":{"opacity":1}}],"durationMs":1000,"easing":"linear"
    }]);
    let mut session = enter(&value, 0);
    session.begin_lifecycle(10);
    session.set_now_ms(410);
    let before = session.preview_scene_for_test();
    let opacity = find(&before, "photo").opacity;
    assert!((opacity - 0.4).abs() < 0.01);
    assert_eq!(session.active_animation_track_count(), 1);
    assert!(session.sync_image_assets(&filled(value), 7, 3));
    assert_eq!(
        find(&session.preview_scene_for_test(), "photo").opacity,
        opacity
    );
    assert_eq!(session.active_animation_track_count(), 1);
    session.set_now_ms(610);
    assert!((find(&session.preview_scene_for_test(), "photo").opacity - 0.6).abs() < 0.01);
}

#[test]
fn completion_during_a_pointer_press_keeps_captured_keys_valid() {
    use jian_core::gesture::pointer::PointerPhase;
    let mut value = fixture();
    value["state"] = json!({"clicked":{"type":"bool","default":false}});
    value["children"][0]["children"][0]["events"] =
        json!({"onTap":[{"set":{"$app.clicked":"true"}}]});
    let mut session = enter(&value, 0);
    let icon = find(
        &session.preview_scene_for_test(),
        "photo-image-fallback-icon",
    )
    .bounds;
    let x = icon.origin.x + 2.0;
    let y = icon.origin.y + 2.0;
    let _ = session.dispatch_pointer_phase_at(x, y, PointerPhase::Down, 20);
    assert!(session.sync_image_assets(&filled(value), 7, 3));
    assert!(session
        .runtime
        .document
        .as_ref()
        .unwrap()
        .tree
        .get("photo-image-fallback-icon")
        .is_none());
    let _ = session.dispatch_pointer_phase_at(x, y, PointerPhase::Up, 30);
    assert_eq!(
        session
            .runtime
            .state
            .app_get("clicked")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    let _ = session.dispatch_tap(x, y);
    let _ = session.dispatch_wheel(x, y, 0.0, -10.0);
}
