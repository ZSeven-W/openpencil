use super::*;

fn overlay_root(map: bool, control_width: Value) -> Value {
    let mut root = evidence_root();
    let mut media = json!({"type":"image","id":"hero-image","name":"Cafe photo",
        "width":327,"height":280,"src":"cafe.jpg"});
    if map {
        media["name"] = json!("Map");
        media["src"] = json!("");
        media["imageSearchQuery"] = json!("map-template:delivery");
    }
    root["children"][1] = json!({"type":"frame","id":"hero-section","name":"Hero",
        "width":"fill_container","height":"fit_content","layout":"vertical","padding":[0,24],
        "children":[{"type":"frame","id":"viewport","name":"Media viewport",
            "width":327,"height":280,"layout":"none","clipContent":true,"children":[
                {"type":"frame","id":"layers","name":"Authored layer button","x":275,"y":12,
                    "width":control_width,"height":40,"layout":"horizontal",
                    "justifyContent":"center","alignItems":"center","cornerRadius":20,
                    "fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
                        {"type":"icon_font","id":"layer-icon","name":"Authored layers icon",
                            "iconFontName":"layers","iconFontFamily":"lucide","width":18,"height":18}]},
                {"type":"text","id":"eta","name":"Authored ETA","x":16,"y":204,
                    "content":"预计12分钟送达","width":200,"height":22,"fontSize":14,
                    "fill":[{"type":"solid","color":"#0F172A"}]},
                {"type":"text","id":"address","name":"Authored address","x":16,"y":240,
                    "content":"滨江路88号","width":200,"height":22,"fontSize":14,
                    "fill":[{"type":"solid","color":"#0F172A"}]},
                media]}]});
    root
}

fn descendants(value: &Value) -> Vec<&Value> {
    let mut nodes = vec![value];
    for child in value["children"].as_array().into_iter().flatten() {
        nodes.extend(descendants(child));
    }
    nodes
}

fn named<'a>(root: &'a Value, name: &str) -> &'a Value {
    descendants(root)
        .into_iter()
        .find(|n| n["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn scrim_count(root: &Value) -> usize {
    descendants(root)
        .into_iter()
        .filter(|node| {
            node["fill"]
                .as_array()
                .is_some_and(|fills| fills.iter().any(|fill| fill["explain"] == "hero scrim"))
        })
        .count()
}

#[test]
fn map_overlay_keeps_one_compact_authored_control_without_photo_scrim_after_cleanup() {
    for width in [json!(40), json!("fill_container")] {
        let mut sink = sink_with(overlay_root(true, width));
        crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);
        let root = root_value(&sink);
        let viewport = named(&root, "Media viewport");
        let map = named(&root, "Map (map placeholder)");
        let button = named(&root, "Authored layer button");
        let icon = named(&root, "Authored layers icon");
        let ground = named(&root, "map-ground");
        assert!(named(&root, "Hero (bleed)").is_object());
        assert_eq!(
            scrim_count(&root),
            0,
            "map must not inherit photographic tint"
        );
        assert_eq!(
            descendants(viewport)
                .into_iter()
                .filter(|n| n["iconFontName"] == "layers")
                .count(),
            1
        );
        assert_eq!(named(&root, "Authored ETA")["content"], "预计12分钟送达");
        assert_eq!(named(&root, "Authored address")["content"], "滨江路88号");
        assert_ne!(
            named(&root, "Authored ETA")["fill"][0]["color"],
            "#FFFFFFCC"
        );
        let rects = crate::image_fallback_policy::resolved_rects(&sink.state);
        let rect = |n: &Value| rects[n["id"].as_str().unwrap()];
        for n in [map, viewport, ground] {
            assert_eq!((rect(n).width, rect(n).height), (375.0, 280.0));
        }
        assert_eq!((rect(button).width, rect(button).height), (40.0, 40.0));
        assert_eq!((rect(icon).width, rect(icon).height), (18.0, 18.0));
        assert_eq!(rect(icon).x + 9.0, rect(button).x + 20.0);
        assert_eq!(rect(icon).y + 9.0, rect(button).y + 20.0);
        assert!(!map["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["name"] == "Map layer control"));
    }
}

#[test]
fn real_photo_overlay_still_gets_hero_scrim_after_cleanup() {
    let mut sink = sink_with(overlay_root(false, json!(40)));
    crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);
    let root = root_value(&sink);
    assert!(named(&root, "Hero (bleed)").is_object());
    assert_eq!(scrim_count(&root), 1);
    assert_eq!(named(&root, "Cafe photo")["type"], "image");
    assert_eq!(
        named(&root, "Authored ETA")["fill"][0]["color"],
        "#FFFFFFCC"
    );
}

#[test]
fn map_scrim_exemption_preserves_authored_wash_and_label_styles() {
    let mut root = overlay_root(true, json!(40));
    let wash = json!({"type":"rectangle","id":"authored-wash","name":"Authored wash",
        "x":0,"y":0,"width":327,"height":280,"fill":[{"type":"linear_gradient","angle":90,
            "stops":[{"offset":0,"color":"#FFFFFF00"},{"offset":1,"color":"#FFFFFF66"}]}]});
    root["children"][1]["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(3, wash.clone());
    let mut sink = sink_with(root);
    let original_wash = named(&root_value(&sink), "Authored wash").clone();
    assert_eq!(enforce(&mut sink, &plan(), "root"), 1);
    let root = root_value(&sink);
    assert_eq!(scrim_count(&root), 0);
    assert_eq!(named(&root, "Authored wash")["fill"], original_wash["fill"]);
    assert_eq!(named(&root, "Authored ETA")["fill"][0]["color"], "#0F172A");
    assert_eq!(
        named(&root, "Authored address")["fill"][0]["color"],
        "#0F172A"
    );
}

#[test]
fn direct_map_overlay_cards_keep_insets_through_complete_cleanup() {
    let mut root = evidence_root();
    root["children"][1] = json!({"type":"frame","id":"hero-section","name":"Hero",
    "explain":"op-navigation-viewport","width":375,"height":766,
    "layout":"none","padding":0,"clipContent":true,"children":[
        {"type":"frame","id":"direction","name":"Direction card","x":16,"y":16,
            "width":343,"height":80,"layout":"vertical","padding":16,"cornerRadius":14,
            "fill":[{"type":"solid","color":"#1A2A44"}],"children":[
                {"type":"frame","id":"direction-row","layout":"horizontal","children":[
                    {"type":"text","id":"instruction","content":"前方200米右转","fontSize":18}]}]},
        {"type":"frame","id":"trip","name":"Trip card","x":16,"y":678,
            "width":343,"height":64,"layout":"horizontal","padding":16,"cornerRadius":14,
            "fill":[{"type":"solid","color":"#1A2A44"}],"children":[
                {"type":"frame","id":"trip-row","layout":"horizontal","children":[
                    {"type":"text","id":"distance","content":"3.2公里 12分钟","fontSize":16}]}]},
        {"type":"image","id":"map","name":"Map","src":"","width":375,"height":766,
            "imageSearchQuery":"map-template:running"}
    ]});
    let mut sink = sink_with(root);
    crate::cleanup::run_cleanup_passes(&mut sink, &plan(), &["root"]);
    let rects = crate::image_fallback_policy::resolved_rects(&sink.state);
    for id in ["direction", "trip"] {
        assert_eq!(
            rects[id].width, 343.0,
            "{id} must not become background media"
        );
        assert_eq!(rects[id].x - rects["hero-section"].x, 16.0);
    }
    assert_eq!(rects["map"].width, 375.0);
    let value = root_value(&sink);
    assert_eq!(scrim_count(&value), 0);
    assert_eq!(named(&value, "map-ground")["width"], 375.0);
}

#[test]
fn painted_nested_copy_and_icon_rows_are_not_coloured_media() {
    for leaf in [
        json!({"type":"text","content":"Directions"}),
        json!({"type":"icon_font","iconFontName":"layers"}),
    ] {
        let card = json!({"type":"frame","fill":[{"type":"solid","color":"#123456"}],
            "children":[{"type":"frame","children":[leaf]}]});
        assert!(!is_coloured_media(&card));
    }
}
