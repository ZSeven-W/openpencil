use super::{LintPreValidator, TestSink};
use op_orchestrator::{map_templates::MapTemplate, role_defaults::Theme, DocSink, PreValidator};
use serde_json::{json, Value};

fn map_value(kind: MapTemplate, theme: Theme, id: &str, w: f64, h: f64) -> Value {
    let image = serde_json::from_value(json!({"type":"image","id":id,"name":"Map",
        "src":"","width":w,"height":h,"imageSearchQuery":format!("map-template:{}", kind.key())}))
    .unwrap();
    let mut value = op_orchestrator::map_placeholder::build_map_placeholder(&image, w, h, theme);
    value["id"] = json!(id);
    value["width"] = json!(w);
    value["height"] = json!(h);
    value
}

#[test]
fn pre_validation_preserves_map_streets_routes_and_distinct_radii() {
    for kind in [
        MapTemplate::Running,
        MapTemplate::Delivery,
        MapTemplate::Location,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            let doc = serde_json::from_value(
                json!({"version":"1.0","children":[map_value(kind,theme,"map",375.0,330.0)]}),
            )
            .unwrap();
            let mut sink = TestSink::from_doc(doc);
            let before = serde_json::to_value(sink.state().doc.children[0].clone()).unwrap();
            LintPreValidator.run_pre_validation_fixes(&mut sink);
            let after = serde_json::to_value(sink.state().doc.children[0].clone()).unwrap();
            assert_eq!(
                before, after,
                "map geometry/style must survive the real pre-validator: {kind:?}"
            );
        }
    }
}

/// Replay a real generated page through the production template and validator.
/// This is a deterministic repair replay, not a second model generation.
#[test]
fn map_template_real_page_replay() {
    let (Ok(source), Ok(output)) = (
        std::env::var("OPENPENCIL_MAP_REPLAY_SOURCE"),
        std::env::var("OPENPENCIL_MAP_REPLAY_OUT"),
    ) else {
        return;
    };
    let doc = serde_json::from_str(&std::fs::read_to_string(source).unwrap()).unwrap();
    let mut sink = TestSink::from_doc(doc);
    let rects = op_orchestrator::image_fallback_policy::resolved_rects(sink.state());
    let mut maps = Vec::new();
    fn collect(node: &jian_ops_schema::node::PenNode, maps: &mut Vec<(String, String)>) {
        use op_editor_core::PenNodeExt;
        let v = serde_json::to_value(node).unwrap();
        if let Some(hint) = v["explain"]
            .as_str()
            .filter(|s| s.starts_with("map-template:"))
        {
            maps.push((node.id_str().to_string(), hint.to_string()));
            return;
        }
        for c in node.children().into_iter().flatten() {
            collect(c, maps);
        }
    }
    for root in sink.state().active_children() {
        collect(root, &mut maps);
    }
    assert!(!maps.is_empty());
    for (id, hint) in &maps {
        let rect = rects.get(id).unwrap();
        let mut patch = map_value(
            MapTemplate::from_hint(hint),
            Theme::Light,
            id,
            rect.width,
            rect.height,
        );
        // Production map conversion preserves the slot's sizing contract.
        patch.as_object_mut().unwrap().remove("width");
        patch.as_object_mut().unwrap().remove("height");
        sink.apply(op_editor_core::EditorCommand::PatchNodeData {
            node_id: op_editor_core::NodeId::new(id),
            page_id: None,
            patch_json: patch.to_string(),
        });
    }
    let before = serde_json::to_value(&sink.state().doc).unwrap();
    LintPreValidator.run_pre_validation_fixes(&mut sink);
    let after = serde_json::to_value(&sink.state().doc).unwrap();
    for (id, _) in maps {
        fn find<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
            if value["id"] == id {
                return Some(value);
            }
            value["children"]
                .as_array()?
                .iter()
                .find_map(|v| find(v, id))
        }
        assert_eq!(find(&before, &id), find(&after, &id));
    }
    std::fs::write(
        output,
        serde_json::to_string_pretty(&sink.state().doc).unwrap(),
    )
    .unwrap();
}
