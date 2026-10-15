use super::*;

use crate::image_fallback_policy::ResolvedRect;
use crate::role_defaults::Theme;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, PenNodeExt};
use op_image_enrich::collect_targets;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn map_node() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame",
        "id": "city-map",
        "name": "地图占位区",
        "imageSearchQuery": "city map",
        "width": 327,
        "height": 280,
        "x": 16,
        "y": 120,
        "cornerRadius": 12,
        "children": [{
            "type": "frame",
            "id": "locate",
            "name": "Locate button",
            "x": 267,
            "y": 220,
            "width": 44,
            "height": 44,
            "layout": "none"
        }]
    }))
    .expect("valid map fixture")
}

fn rects() -> HashMap<String, ResolvedRect> {
    HashMap::from([(
        "city-map".to_string(),
        ResolvedRect {
            x: 16.0,
            y: 120.0,
            width: 327.0,
            height: 280.0,
        },
    )])
}

fn patch_value(patch: &MapPlaceholderPatch) -> Value {
    serde_json::from_str(&patch.patch_json).expect("valid map patch")
}

fn evidence_section() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame",
        "id": "map-section",
        "name": "Map & Current Location",
        "width": 375,
        "height": 320,
        "layout": "vertical",
        "children": [{
            "type": "image",
            "id": "map-image",
            "src": "placeholder://image-search-failed",
            "imageSearchQuery": "city map",
            "width": 327,
            "height": 300
        }, {
            "type": "frame",
            "id": "current-location-card",
            "name": "Current location card",
            "layout": "vertical",
            "width": 327,
            "height": 88,
            "children": [{
                "type": "text",
                "id": "current-location-label",
                "content": "当前位置",
                "width": 100,
                "height": 20
            }, {
                "type": "text_input",
                "id": "current-location-input",
                "width": 300,
                "height": 44,
                "placeholder": "当前位置"
            }]
        }]
    }))
    .expect("valid section map fixture")
}

#[test]
fn city_map_becomes_a_vector_placeholder_and_clears_search_intent() {
    let source = map_node();
    let patches = map_placeholder(&source, &rects(), Theme::Light);
    assert_eq!(patches.len(), 1);
    let patch = patch_value(&patches[0]);
    assert_eq!(patch["type"], "frame");
    assert_eq!(patch["layout"], "none");
    assert_eq!(patch["clipContent"], true);
    assert_eq!(patch["fill"][0]["color"], "$--muted");
    assert!(patch["imageSearchQuery"].is_null());

    let children = patch["children"].as_array().unwrap();
    assert_eq!(
        patch["explain"],
        "map-template:location; illustrative, not geographic data; viewport:327x280"
    );
    assert!(children.iter().any(|child| child["name"] == "map-pin"));
    assert!(!children.iter().any(|child| child["name"] == "map-route"));
    let layers = children
        .iter()
        .find(|child| child["name"] == "Map layer control")
        .unwrap();
    assert_eq!(layers["width"], 40);
    assert_eq!(layers["height"], 40);
    // Authored controls must paint above the map (lower index paints on top).
    assert_eq!(children[0]["id"], "locate");
    assert!(patch.get("height").is_none());
    assert!(children.iter().any(|child| child["id"] == "locate"));
    for child in children {
        for field in ["x", "y", "width", "height"] {
            assert!(child[field].is_number(), "{field} missing on {child}");
        }
    }
}

#[test]
fn map_layout_is_stable_and_dark_pages_use_secondary() {
    let source = map_node();
    let light_a = map_placeholder(&source, &rects(), Theme::Light);
    let light_b = map_placeholder(&source, &rects(), Theme::Light);
    assert_eq!(light_a, light_b);

    let dark = patch_value(&map_placeholder(&source, &rects(), Theme::Dark)[0]);
    assert_eq!(dark["fill"][0]["color"], "$--secondary");
}

#[test]
fn section_map_name_does_not_replace_content_container() {
    let source = evidence_section();
    let original_card = source.children().unwrap()[1].clone();
    let rects = HashMap::from([
        (
            "map-section".to_string(),
            ResolvedRect {
                x: 0.0,
                y: 0.0,
                width: 375.0,
                height: 320.0,
            },
        ),
        (
            "map-image".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 80.0,
                width: 327.0,
                height: 300.0,
            },
        ),
    ]);
    let patches = map_placeholder(&source, &rects, Theme::Light);
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].node_id, "map-image");

    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![source],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    state.apply(EditorCommand::PatchNodeData {
        node_id: op_editor_core::NodeId::new("map-image"),
        patch_json: patches[0].patch_json.clone(),
        page_id: None,
    });

    let section = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("map-section"),
    )
    .expect("section remains");
    assert_eq!(
        serde_json::to_value(section).unwrap()["layout"],
        json!("vertical")
    );
    let children = section.children().expect("section children remain");
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].base().name.as_deref(),
        Some("Map (map placeholder)")
    );
    assert_eq!(
        serde_json::to_value(&children[1]).unwrap(),
        serde_json::to_value(original_card).unwrap()
    );
    assert!(matches!(
        children[1].children().unwrap()[0],
        PenNode::Text(_)
    ));
    assert!(matches!(
        children[1].children().unwrap()[1],
        PenNode::TextInput(_)
    ));
}

#[test]
fn fit_content_height_is_sized_without_overwriting_width_sizing() {
    let source: PenNode = serde_json::from_value(json!({
        "type": "image",
        "id": "fit-map",
        "name": "Map image",
        "src": "placeholder://image-search-failed",
        "imagePrompt": "city map",
        "width": "fill_container",
        "height": "fit_content"
    }))
    .expect("valid fit-content map fixture");
    let patch = patch_value(&map_placeholder(&source, &HashMap::new(), Theme::Light)[0]);
    assert_eq!(patch["height"], json!(300.0));
    assert!(patch.get("width").is_none());
    let block = patch["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["name"] == "map-block-1")
        .unwrap();
    assert_eq!(block["width"], json!(327.0 * 0.18));
}

#[test]
fn applying_map_patch_preserves_pinned_control_and_removes_targets() {
    let source = map_node();
    let patch = patch_value(&map_placeholder(&source, &rects(), Theme::Light)[0]);
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![source],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    state.apply(EditorCommand::PatchNodeData {
        node_id: op_editor_core::NodeId::new("city-map"),
        patch_json: patch.to_string(),
        page_id: None,
    });
    let rewritten = op_editor_core::walkers::find_node(
        state.active_children(),
        &op_editor_core::NodeId::new("city-map"),
    )
    .expect("rewritten map exists");
    assert!(matches!(rewritten, PenNode::Frame(_)));
    assert_eq!(rewritten.base().x, Some(16.0));
    assert_eq!(rewritten.base().y, Some(120.0));
    assert_eq!(rewritten.width_px(), Some(327.0));
    assert_eq!(rewritten.height_px(), Some(280.0));
    assert_eq!(
        serde_json::to_value(rewritten).unwrap()["cornerRadius"].as_f64(),
        Some(12.0)
    );
    assert!(rewritten
        .children()
        .unwrap()
        .iter()
        .any(|child| child.id_str() == "locate"));
    assert!(map_placeholder(rewritten, &rects(), Theme::Light).is_empty());
    assert!(collect_targets(&state, &HashSet::new()).is_empty());
}

#[test]
fn decorations_inside_a_hand_drawn_map_and_small_map_shapes_are_left_alone() {
    // GLM-5.3-Flash drew its own map and named the inner blocks map-park /
    // map-water; each was turned into a nested mini map. A map-named
    // container's children are decorations, and a lone map-named shape
    // narrower than 200 px is not a slot either. A wide leaf still is.
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Ride Home",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "map-canvas",
                "name": "map-canvas",
                "width": 327,
                "height": 320,
                "layout": "none",
                "children": [
                    {"type": "rectangle", "id": "park", "name": "map-park", "x": 20, "y": 30, "width": 110, "height": 90},
                    {"type": "text", "id": "road-label", "name": "road-label", "content": "科苑南路", "x": 60, "y": 10}
                ]
            },
            {"type": "rectangle", "id": "lone-water", "name": "map-water", "width": 110, "height": 90},
            {"type": "rectangle", "id": "wide-tile", "name": "map-tile", "width": 327, "height": 300}
        ]
    }))
    .expect("fixture");
    let rects = HashMap::from([
        (
            "map-canvas".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 62.0,
                width: 327.0,
                height: 320.0,
            },
        ),
        (
            "park".to_string(),
            ResolvedRect {
                x: 44.0,
                y: 92.0,
                width: 110.0,
                height: 90.0,
            },
        ),
        (
            "lone-water".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 400.0,
                width: 110.0,
                height: 90.0,
            },
        ),
        (
            "wide-tile".to_string(),
            ResolvedRect {
                x: 24.0,
                y: 500.0,
                width: 327.0,
                height: 300.0,
            },
        ),
    ]);
    let patches = map_placeholder(&root, &rects, Theme::Light);
    let ids: Vec<&str> = patches.iter().map(|p| p.node_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["wide-tile"],
        "only the wide leaf is a map slot; got {ids:?}"
    );
}

/// "Mapo tofu" and "maple" start with "map" but are food. Measured 0923-0924:
/// every 麻婆豆腐 dish photo and a Sichuan store hero became fake street maps.
#[test]
fn food_words_that_start_with_map_are_not_maps() {
    for (name, query, prompt) in [
        (
            "dish-photo-2",
            "mapo tofu",
            "overhead food photography of mapo tofu in a clay bowl",
        ),
        (
            "store-hero-image",
            "sichuan restaurant",
            "warm restaurant table with mapo tofu and chili oil",
        ),
        (
            "pancake-photo",
            "maple syrup pancakes",
            "stack of pancakes with maple syrup",
        ),
    ] {
        let node: PenNode = serde_json::from_value(json!({
            "type": "image", "id": name, "name": name, "src": "", "width": 88, "height": 88,
            "imageSearchQuery": query, "imagePrompt": prompt
        }))
        .expect("valid image");
        assert!(
            !is_map_placeholder_candidate(&node),
            "{name} is food, not a map"
        );
    }
    let real: PenNode = serde_json::from_value(json!({
        "type": "image", "id": "store-map", "name": "store-location-map", "src": "",
        "width": 327, "height": 160, "imageSearchQuery": "city map"
    }))
    .expect("valid image");
    assert!(is_map_placeholder_candidate(&real));
}

fn authored_layers_button(width: Value) -> Value {
    json!({"type":"frame","id":"authored-layers","name":"图层切换按钮",
        "x":275,"y":12,"width":width,"height":40,"layout":"horizontal",
        "fill":[{"type":"solid","color":"#FFFFFF"}],"cornerRadius":20,
        "children":[{"type":"icon_font","id":"authored-icon","iconFontName":"layers",
            "width":18,"height":18,"fill":[{"type":"solid","color":"#284339"}]}]})
}

#[test]
fn map_with_authored_sibling_layer_control_uses_one_compact_control() {
    for width in [json!(40), json!("fill_container")] {
        let button = authored_layers_button(width);
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
            "name":"地图视口","layout":"none","width":327,"height":280,
            "children":[button,{"type":"text","id":"eta","content":"预计12分钟送达"},
                {"type":"image","id":"slot","name":"Map","src":"","width":327,"height":280,
                    "imageSearchQuery":"map-template:delivery"}]}))
        .unwrap();
        let patches = map_placeholder(&source, &HashMap::new(), Theme::Light);
        let control = patch_value(
            patches
                .iter()
                .find(|p| p.node_id == "authored-layers")
                .unwrap(),
        );
        assert_eq!(control["width"], 40);
        assert_eq!(control["height"], 40);
        assert_eq!(control["padding"], 0);
        assert!(
            control.get("children").is_none(),
            "preserve authored icon and events"
        );
        assert!(control.get("fill").is_none(), "preserve authored styling");
        let map = patch_value(patches.iter().find(|p| p.node_id == "slot").unwrap());
        assert!(!map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
        assert!(map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "map-pin"));
    }
}

#[test]
fn map_preserves_and_compacts_its_own_authored_layer_control() {
    let button = authored_layers_button(json!("fill_container"));
    let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"map",
        "name":"Map","width":327,"height":280,"children":[button]}))
    .unwrap();
    let original_button = serde_json::to_value(&source.children().unwrap()[0]).unwrap();
    let map = build_map_placeholder(&source, 327.0, 280.0, Theme::Light);
    let children = map["children"].as_array().unwrap();
    assert_eq!(children[0]["id"], "authored-layers");
    assert_eq!(children[0]["width"], 40);
    assert_eq!(children[0]["height"], 40);
    assert_eq!(children[0]["children"], original_button["children"]);
    assert_eq!(children[0]["fill"], original_button["fill"]);
    assert!(!children.iter().any(|n| n["name"] == "Map layer control"));
}

#[test]
fn layer_button_outside_map_viewport_does_not_suppress_template_control() {
    let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"root",
        "layout":"vertical","width":375,"height":812,
        "children":[authored_layers_button(json!("fill_container")),
            {"type":"frame","id":"viewport","layout":"none","width":327,"height":280,
                "children":[{"type":"image","id":"slot","name":"Map","src":"",
                    "width":327,"height":280,"imageSearchQuery":"map-template:location"}]}]}))
    .unwrap();
    let patches = map_placeholder(&source, &HashMap::new(), Theme::Light);
    assert_eq!(
        patches.len(),
        1,
        "unrelated layers button must stay untouched"
    );
    assert_eq!(patches[0].node_id, "slot");
    assert!(patch_value(&patches[0])["children"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "Map layer control"));
}

#[test]
fn map_decoration_does_not_trigger_layer_control_repair() {
    let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
        "layout":"none","width":327,"height":280,"children":[
            authored_layers_button(json!("fill_container")),
            {"type":"rectangle","id":"park","name":"map-park","width":40,"height":40}]}))
    .unwrap();
    let rects = HashMap::from([(
        "park".into(),
        ResolvedRect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 40.0,
        },
    )]);
    assert!(map_placeholder(&source, &rects, Theme::Light).is_empty());
}

#[test]
fn hidden_authored_layers_does_not_remove_visible_template_control() {
    for (hide_icon, hidden) in [
        (false, json!({"visible":false})),
        (false, json!({"opacity":0})),
        (false, json!({"enabled":false})),
        (true, json!({"visible":false})),
        (true, json!({"opacity":0})),
        (true, json!({"enabled":false})),
    ] {
        let mut button = authored_layers_button(json!(40));
        let target = if hide_icon {
            &mut button["children"][0]
        } else {
            &mut button
        };
        target
            .as_object_mut()
            .unwrap()
            .extend(hidden.as_object().unwrap().clone());
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
            "layout":"none","width":327,"height":280,"children":[button,
                {"type":"image","id":"slot","name":"Map","src":"","width":327,"height":280,
                    "imageSearchQuery":"map-template:location"}]}))
        .unwrap();
        let patches = map_placeholder(&source, &HashMap::new(), Theme::Light);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].node_id, "slot");
        assert!(patch_value(&patches[0])["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
    }
}

#[test]
fn absolute_map_controls_with_default_zero_coordinates_do_not_get_a_duplicate() {
    for missing in [vec!["x"], vec!["y"], vec!["x", "y"]] {
        let mut button = authored_layers_button(json!(40));
        for key in &missing {
            button.as_object_mut().unwrap().remove(*key);
        }
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
            "layout":"none","width":327,"height":380,"children":[button.clone(),
                {"type":"image","id":"slot","name":"Map","src":"","width":327,"height":380,
                "imageSearchQuery":"map-template:delivery"}]}))
        .unwrap();
        let patches = map_placeholder(&source, &HashMap::new(), Theme::Dark);
        let map = patch_value(patches.iter().find(|p| p.node_id == "slot").unwrap());
        assert!(!map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
        // The same control must survive when it is directly inside the slot.
        let slot: PenNode = serde_json::from_value(json!({"type":"frame","id":"slot",
            "name":"Map","layout":"none","width":327,"height":380,"children":[button.clone()]}))
        .unwrap();
        let map = build_map_placeholder(&slot, 327.0, 380.0, Theme::Dark);
        let original = serde_json::to_value(&slot.children().unwrap()[0]).unwrap();
        let controls: Vec<_> = map["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["id"] == "authored-layers" || n["name"] == "Map layer control")
            .collect();
        assert_eq!(controls.len(), 1);
        assert_eq!(controls[0]["id"], "authored-layers");
        assert_eq!(controls[0]["children"], original["children"]);
        assert_eq!(controls[0]["fill"], original["fill"]);
    }
}

#[test]
fn icon_only_map_toolbar_never_becomes_a_second_map() {
    for name in [
        "右侧地图控件",
        "地图操作按钮",
        "Map controls",
        "map-toolbar",
    ] {
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"tools",
            "name":name,"layout":"vertical","width":375,"height":"fit_content",
            "children":[authored_layers_button(json!(40))]}))
        .unwrap();
        assert!(
            map_placeholder(&source, &HashMap::new(), Theme::Dark).is_empty(),
            "{name}"
        );
    }
}

#[test]
fn layer_toggle_with_map_glyph_or_nested_toolbar_suppresses_only_its_own_default() {
    for nested in [false, true] {
        let mut control = authored_layers_button(json!(44));
        control["name"] = json!("layer-toggle-button");
        control["children"][0]["iconFontName"] = json!("map");
        if nested {
            control = json!({"type":"frame","id":"toolbar","layout":"vertical",
                "width":44,"height":"fit_content","children":[control,
                    {"type":"frame","id":"locate","width":44,"height":44,
                     "children":[{"type":"icon_font","id":"locate-icon","iconFontName":"crosshair"}]}]});
        }
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
            "layout":"none","width":375,"height":734,"children":[control,
                {"type":"image","id":"map","src":"","width":375,"height":734,
                 "imageSearchQuery":"map-template:running"}]}))
        .unwrap();
        let patches = map_placeholder(&source, &HashMap::new(), Theme::Dark);
        assert!(patches.iter().any(|p| p.node_id == "authored-layers"));
        assert!(!patches
            .iter()
            .any(|p| matches!(p.node_id.as_str(), "toolbar" | "locate")));
        let map = patch_value(patches.iter().find(|p| p.node_id == "map").unwrap());
        assert!(!map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
    }
}

#[test]
fn nested_other_map_or_hidden_toolbar_never_steals_the_maps_default_control() {
    for other_map in [false, true] {
        let mut toolbar = json!({"type":"frame","id":"toolbar","layout":"none","width":327,"height":300,
            "children":[authored_layers_button(json!(44))]});
        if other_map {
            toolbar["children"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type":"image","id":"other-map",
                "src":"","width":327,"height":300,"imageSearchQuery":"map-template:location"}));
        } else {
            toolbar["visible"] = json!(false);
        }
        let source: PenNode = serde_json::from_value(json!({"type":"frame","id":"viewport",
            "layout":"none","width":375,"height":734,"children":[toolbar,
                {"type":"image","id":"map","src":"","width":375,"height":734,
                 "imageSearchQuery":"map-template:running"}]}))
        .unwrap();
        let patches = map_placeholder(&source, &HashMap::new(), Theme::Dark);
        let map = patch_value(patches.iter().find(|p| p.node_id == "map").unwrap());
        assert!(map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
    }
}
