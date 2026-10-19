use super::*;

#[test]
fn authored_vector_map_from_app07_is_not_replaced_without_text_or_controls() {
    // A/app-07/rep-3, map-canvas n13; retain only roads, route and markers.
    let node: PenNode =
        serde_json::from_str(include_str!("fixtures/regress_1019/authored_map.json")).unwrap();
    let original = node.clone();
    assert!(map_placeholder(&node, &HashMap::new(), Theme::Light).is_empty());
    assert_eq!(node, original);
}

#[test]
fn resolved_map_images_are_not_placeholder_slots() {
    for src in [
        "provided-map.png",
        "https://example.test/map.png",
        "data:image/png;base64,abc",
    ] {
        let node: PenNode = serde_json::from_value(json!({
            "type":"image", "id":"map", "name":"Map", "src":src,
            "width":327, "height":260, "imageSearchQuery":"city map"
        }))
        .unwrap();
        assert!(map_placeholder(&node, &HashMap::new(), Theme::Light).is_empty());
    }
}

#[test]
fn ride_screen_context_overrides_a_location_image_slot() {
    for context in [
        "打车 App 主页（375×812）",
        "Ride-hailing Home",
        "Taxi booking",
    ] {
        // B/app-07 uses a Map image with map-template:location on all 3 runs.
        let root: PenNode = serde_json::from_value(json!({
            "type":"frame", "id":"screen", "name":context, "children":[{
                "type":"frame", "id":"map-section", "name":"地图占位区", "children":[{
                    "type":"image", "id":"slot", "name":"Map", "src":"",
                    "imageSearchQuery":"map-template:location", "width":375, "height":260
                }]
            }]
        }))
        .unwrap();
        let patches = map_placeholder(&root, &HashMap::new(), Theme::Light);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].node_id, "slot");
        let patch: Value = serde_json::from_str(&patches[0].patch_json).unwrap();
        assert!(patch["explain"]
            .as_str()
            .unwrap()
            .starts_with("map-template:ride;"));
        let children = patch["children"].as_array().unwrap();
        for name in ["map-route", "map-origin", "map-pin", "map-vehicle"] {
            assert!(children.iter().any(|child| child["name"] == name), "{name}");
        }
        let car = children
            .iter()
            .find(|child| child["name"] == "map-vehicle")
            .unwrap();
        assert_eq!(car["children"][0]["iconFontName"], "car");
    }
}

#[test]
fn location_fallback_has_current_location_halo_and_major_roads() {
    let node: PenNode = serde_json::from_value(json!({
        "type":"rectangle", "id":"slot", "name":"Map", "width":327, "height":260
    }))
    .unwrap();
    let patches = map_placeholder(&node, &HashMap::new(), Theme::Light);
    let patch: Value = serde_json::from_str(&patches[0].patch_json).unwrap();
    let children = patch["children"].as_array().unwrap();
    assert!(children
        .iter()
        .any(|child| child["name"] == "map-avenue" && child["type"] == "path"));
    assert!(children
        .iter()
        .any(|child| child["name"] == "map-location-halo" && child["opacity"] == 0.16));
}
