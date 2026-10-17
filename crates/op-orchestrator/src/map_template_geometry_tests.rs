use super::*;
use crate::map_placeholder::build_map_placeholder;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::RepairSummary;
use crate::test_support::VecDocSink;

fn find<'a>(nodes: &'a [PenNode], name: &str) -> &'a PenNode {
    fn search<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a PenNode> {
        for node in nodes {
            if node.base().name.as_deref() == Some(name) {
                return Some(node);
            }
            if let Some(found) = search(node.children().map(Vec::as_slice).unwrap_or(&[]), name) {
                return Some(found);
            }
        }
        None
    }
    search(nodes, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn assert_geometry(map: &PenNode, width: f64, height: f64, kind: MapTemplate) {
    let expected = children(map.id_str(), width, height, Theme::Light, kind);
    for target in expected {
        let name = target["name"].as_str().unwrap();
        let actual = serde_json::to_value(find(map.children().unwrap(), name)).unwrap();
        for field in ["x", "y", "width", "height", "d"] {
            if let Some(value) = target.get(field) {
                if let Some(number) = value.as_f64() {
                    assert!(
                        (actual[field].as_f64().unwrap() - number).abs() < 0.001,
                        "{name}.{field}: {} != {value}",
                        actual[field]
                    );
                } else {
                    assert_eq!(actual[field], *value, "{name}.{field}");
                }
            }
        }
    }
}

fn assert_nested_bounds(state: &op_editor_core::EditorState, map: &PenNode) {
    fn check(node: &PenNode, viewport: ResolvedRect, rects: &HashMap<String, ResolvedRect>) {
        let rect = rects[node.id_str()];
        assert!(
            rect.x >= viewport.x - 0.01
                && rect.y >= viewport.y - 0.01
                && rect.x + rect.width <= viewport.x + viewport.width + 0.01
                && rect.y + rect.height <= viewport.y + viewport.height + 0.01,
            "{} is outside its viewport: {rect:?} vs {viewport:?}",
            node.id_str()
        );
        for child in node.children().into_iter().flatten() {
            check(child, rect, rects);
        }
    }
    let rects = crate::image_fallback_policy::resolved_rects(state);
    check(map, rects[map.id_str()], &rects);
}

#[test]
fn map_geometry_follows_mobile_rail_through_complete_cleanup() {
    let mut sink = VecDocSink::new();
    sink.state.doc.children = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Delivery detail","width":375,"height":812,
        "layout":"vertical","gap":16,"children":[
            {"type":"frame","id":"header","name":"Header","layout":"horizontal",
             "width":"fill_container","height":48,"children":[
                {"type":"text","id":"title","content":"配送追踪","fontSize":20}]},
            {"type":"frame","id":"section","name":"城市街区配送地图","layout":"vertical",
             "width":"fill_container","height":"fit_content","children":[
                {"type":"image","id":"map","name":"Map","width":"fill_container",
                 "height":260,"imageSearchQuery":"map-template:delivery","src":""}]},
            {"type":"frame","id":"summary","name":"Delivery summary","layout":"vertical",
             "width":"fill_container","height":140,"padding":[16,24],"children":[
                {"type":"text","id":"eta","content":"预计 15 分钟送达","fontSize":22}]}]
    }))
    .unwrap()];
    let plan = OrchestratorPlan {
        shared_context: None,
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Delivery detail".into(),
            width: 375.0,
            height: 812.0,
            layout: Some("vertical".into()),
            gap: Some(16.0),
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
        palette: None,
    };
    let mut summary = RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary(&mut sink, &plan, &["root"], &mut summary);
    let map = find(sink.state.active_children(), "Map (map placeholder)");
    let rects = crate::image_fallback_policy::resolved_rects(&sink.state);
    let rect = rects.get(map.id_str()).unwrap();
    assert_eq!(
        rect.width, 327.0,
        "fixture must exercise the late 24px rail"
    );
    assert_geometry(map, rect.width, rect.height, MapTemplate::Delivery);
    assert_nested_bounds(&sink.state, map);
    assert!(summary
        .records()
        .iter()
        .any(|record| record.pass == "map-template-geometry"));
    let previous = serde_json::to_value(map).unwrap();
    settle(&mut sink, &["root".to_string()]);
    assert_eq!(
        serde_json::to_value(find(sink.state.active_children(), "Map (map placeholder)")).unwrap(),
        previous
    );
}

#[test]
fn map_geometry_resize_preserves_authored_overlay_styles_ids_and_layer_button_size() {
    for kind in [
        MapTemplate::Running,
        MapTemplate::Delivery,
        MapTemplate::Location,
    ] {
        let slot: PenNode = serde_json::from_value(json!({"type":"image","id":"map","src":"",
            "name":"Map","width":375,"height":260,"imageSearchQuery":format!("map-template:{}",kind.key())})).unwrap();
        let mut map = build_map_placeholder(&slot, 375.0, 260.0, Theme::Dark);
        map["id"] = json!("map");
        map["width"] = json!(327);
        map["height"] = json!(300);
        let parts = map["children"].as_array_mut().unwrap();
        let block = parts
            .iter_mut()
            .find(|p| p["name"] == "map-block-4")
            .unwrap();
        block["fill"] = json!([{"type":"solid","color":"#ABCDEF"}]);
        block["cornerRadius"] = json!(11);
        let overlay = json!({"type":"rectangle","id":"overlay","name":"Authored overlay",
            "x":16,"y":20,"width":60,"height":24,"fill":[{"type":"solid","color":"#BCADFE"}]});
        parts.insert(0, overlay);
        let mut sink = VecDocSink::new();
        // Replacement/import can remap IDs. Ownership is independent of that.
        sink.apply(EditorCommand::InsertSubtree {
            nodes: vec![serde_json::from_value(map).unwrap()],
            parent_id: NodeId::NONE,
            page_id: None,
        });
        let root_id = sink.state.active_children()[0].id_str().to_string();
        let before_overlay =
            serde_json::to_value(find(sink.state.active_children(), "Authored overlay")).unwrap();
        let ids_before: Vec<_> = sink.state.active_children()[0]
            .children()
            .unwrap()
            .iter()
            .map(|p| p.id_str().to_string())
            .collect();
        settle(&mut sink, std::slice::from_ref(&root_id));
        let map = &sink.state.active_children()[0];
        assert_geometry(map, 327.0, 300.0, kind);
        assert_nested_bounds(&sink.state, map);
        let block = serde_json::to_value(find(map.children().unwrap(), "map-block-4")).unwrap();
        assert_eq!(block["fill"][0]["color"], "#ABCDEF");
        assert_eq!(block["cornerRadius"].as_f64(), Some(11.0));
        assert_eq!(
            serde_json::to_value(find(map.children().unwrap(), "Authored overlay")).unwrap(),
            before_overlay
        );
        let ids_after: Vec<_> = map
            .children()
            .unwrap()
            .iter()
            .map(|p| p.id_str().to_string())
            .collect();
        assert_eq!(ids_before, ids_after);
        let button = find(map.children().unwrap(), "Map layer control");
        assert_eq!(button.width_px(), Some(40.0));
        assert_eq!(button.height_px(), Some(40.0));
        let doc = serde_json::to_value(&sink.state.doc).unwrap();
        settle(&mut sink, &[root_id]);
        assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), doc);
    }
}

#[test]
fn map_geometry_leaves_unmarked_authored_maps_untouched() {
    let mut sink = VecDocSink::new();
    sink.state.doc.children = vec![serde_json::from_value(json!({
        "type":"frame","id":"map","role":"map","name":"Map","width":327,"height":260,
        "layout":"none","explain":"map-template:delivery; illustrative, not geographic data",
        "children":[{"type":"rectangle","id":"block","name":"map-block-4","x":273.75,
                     "y":10,"width":90,"height":50}]
    }))
    .unwrap()];
    let before = serde_json::to_value(&sink.state.doc).unwrap();
    settle(&mut sink, &["map".to_string()]);
    assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), before);
    assert!(sink.applied.is_empty());
}

#[test]
fn fixed_width_template_fits_a_narrower_clipping_card() {
    let slot: PenNode = serde_json::from_value(json!({"type":"image","id":"map","src":"",
        "name":"Map","width":375,"height":430,"imageSearchQuery":"map-template:delivery"}))
    .unwrap();
    let mut map = build_map_placeholder(&slot, 375.0, 430.0, Theme::Dark);
    map["id"] = json!("map");
    let overlay = json!({"type":"text","id":"overlay","name":"Authored label",
        "x":16,"y":20,"width":60,"height":24,"content":"餐厅"});
    map["children"].as_array_mut().unwrap().push(overlay);
    let mut sink = VecDocSink::new();
    sink.state.doc.children = vec![serde_json::from_value(json!({
        "type":"frame","id":"card","name":"Map Card","layout":"none",
        "width":343,"height":446,"padding":[8,8],"clipContent":true,
        "children":[map]
    }))
    .unwrap()];
    let before_overlay =
        serde_json::to_value(find(sink.state.active_children(), "Authored label")).unwrap();
    settle(&mut sink, &["card".into()]);
    let map = find(sink.state.active_children(), "Map (map placeholder)");
    assert_eq!(map.width_px(), Some(327.0));
    assert_geometry(map, 327.0, 430.0, MapTemplate::Delivery);
    assert_nested_bounds(&sink.state, map);
    assert_eq!(
        serde_json::to_value(find(sink.state.active_children(), "Authored label")).unwrap(),
        before_overlay
    );
    let rects = crate::image_fallback_policy::resolved_rects(&sink.state);
    let button = find(map.children().unwrap(), "Map layer control");
    let button_rect = rects[button.id_str()];
    let card = rects["card"];
    assert!(button_rect.x + button_rect.width <= card.x + card.width - 8.0);
    assert_eq!(button.width_px(), Some(40.0));
    let previous = serde_json::to_value(&sink.state.doc).unwrap();
    settle(&mut sink, &["card".into()]);
    assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), previous);
}

#[test]
fn viewport_fit_does_not_shrink_panned_or_unclipped_maps_or_horizontal_rails() {
    for (layout, clip, x) in [
        ("horizontal", true, 0),
        ("none", false, 0),
        ("none", true, -24),
    ] {
        let slot: PenNode = serde_json::from_value(json!({"type":"image","id":"map","src":"",
            "name":"Map","width":375,"height":260,"imageSearchQuery":"map-template:delivery"}))
        .unwrap();
        let mut map = build_map_placeholder(&slot, 375.0, 260.0, Theme::Dark);
        map["id"] = json!("map");
        map["x"] = json!(x);
        let mut sink = VecDocSink::new();
        sink.state.doc.children = vec![serde_json::from_value(json!({
            "type":"frame","id":"card","width":327,"height":260,
            "layout":layout,"clipContent":clip,"children":[map]
        }))
        .unwrap()];
        let before = serde_json::to_value(&sink.state.doc).unwrap();
        settle(&mut sink, &["card".into()]);
        assert_eq!(
            serde_json::to_value(&sink.state.doc).unwrap(),
            before,
            "{layout}, {clip}, {x}"
        );
    }
}
