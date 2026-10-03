//! Keep page-sized decorative geometry out of a fixed board's content flow.
use super::*;
use serde_json::Value;

pub(super) fn repair(sink: &mut dyn DocSink, root_id: &str, plan: &OrchestratorPlan) {
    let (width, height) = (plan.root_frame.width, plan.root_frame.height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let Some(root) = find_root(sink.state(), root_id) else {
        return;
    };
    let Ok(mut value) = serde_json::to_value(root) else {
        return;
    };
    let before = value.clone();
    if anchor_layers(&mut value, width, height) == 0 {
        return;
    }
    super::fixed_board_spacing::apply_layout_changes(sink, &before, &value);
    // Child zero paints on top. The proven leading background layers belong
    // behind content, including small grid dots that otherwise punch holes in
    // heading glyphs. Keep the background layers' own relative order.
    if let Some(body) = value
        .get("children")
        .and_then(Value::as_array)
        .and_then(|c| c.first())
    {
        if let (Some(parent), Some(layers)) = (
            body.get("id").and_then(Value::as_str),
            body.get("children").and_then(Value::as_array),
        ) {
            for layer in layers.iter().take_while(|layer| pure_geometry(layer)) {
                if let Some(id) = layer.get("id").and_then(Value::as_str) {
                    sink.apply(EditorCommand::MoveNode {
                        node_id: NodeId::new(id),
                        target_parent: NodeId::new(parent),
                        page_id: None,
                        index: Some(layers.len().saturating_sub(1)),
                    });
                }
            }
        }
    }
}

fn pure_geometry(node: &Value) -> bool {
    if node.get("visible").and_then(Value::as_bool) == Some(false)
        || node.get("opacity").and_then(Value::as_f64) == Some(0.0)
    {
        return false;
    }
    match node.get("type").and_then(Value::as_str) {
        Some("frame" | "group") => node
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| !children.is_empty() && children.iter().all(pure_geometry)),
        Some("rectangle" | "ellipse" | "path" | "line" | "polygon") => {
            node.get("fill").is_some() || node.get("stroke").is_some()
        }
        _ => false,
    }
}

fn contains_text(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("text")
        || node
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| children.iter().any(contains_text))
}

fn anchor_layers(root: &mut Value, width: f64, height: f64) -> usize {
    let Some(children) = root.get_mut("children").and_then(Value::as_array_mut) else {
        return 0;
    };
    if children.len() != 1 {
        return 0;
    }
    let body = &mut children[0];
    if body.get("type").and_then(Value::as_str) != Some("frame")
        || !matches!(
            body.get("layout").and_then(Value::as_str),
            Some("vertical" | "none")
        )
        || body.get("height").and_then(Value::as_str) != Some("fit_content")
        || !contains_text(body)
    {
        return 0;
    }
    let Some(layers) = body.get_mut("children").and_then(Value::as_array_mut) else {
        return 0;
    };
    let leading = layers
        .iter()
        .take_while(|layer| pure_geometry(layer))
        .count();
    // Generated margin/grid wrappers often hug their contents. Prove the
    // page-sized background from the geometry inside, not the wrapper's
    // absent numeric size. A cluster of chart dots alone does not qualify.
    if !layers[..leading]
        .iter()
        .any(|layer| page_geometry(layer, width, height))
    {
        return 0;
    }
    let mut count = 0;
    for layer in &mut layers[..leading] {
        if layer.get("constraints").is_none() {
            layer["constraints"] = serde_json::json!({"h":"left", "v":"top"});
            // Constraints only opt out of flow when an authored coordinate
            // exists; hugging background wrappers often omitted both axes.
            if layer.get("x").is_none() {
                layer["x"] = serde_json::json!(0.0);
            }
            if layer.get("y").is_none() {
                layer["y"] = serde_json::json!(0.0);
            }
            count += 1;
        }
    }
    if count > 0 {
        body["height"] = Value::String("fill_container".into());
        root["height"] = serde_json::json!(height);
    }
    count
}

fn page_geometry(node: &Value, width: f64, height: f64) -> bool {
    let h = node.get("height").and_then(Value::as_f64).unwrap_or(0.0);
    let w = node.get("width").and_then(Value::as_f64).unwrap_or(0.0);
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let margin_rule = name.contains("margin") || name.contains("rule") || name.contains("边距");
    (h >= height * 0.95 && w >= width * 0.95)
        || (margin_rule && h >= height * 0.8 && w > 0.0 && w <= 8.0)
        || node
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| page_geometry(child, width, height))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::json!({"type":"frame","id":"root","width":1080,"height":4275,"layout":"vertical","children":[
          {"type":"frame","id":"body","width":"fill_container","height":"fit_content","layout":"vertical","children":[
            {"type":"frame","id":"grid","x":0,"y":0,"width":1080,"height":1440,"layout":"none","children":[{"type":"ellipse","id":"dot","width":6,"height":6,"fill":[{"type":"solid","color":"#cccccc"}]}]},
            {"type":"rectangle","id":"rule","x":62,"y":0,"width":2,"height":1440,"fill":[{"type":"solid","color":"#cc0000"}]},
            {"type":"text","id":"title","width":"fill_container","height":72,"content":"Save and export","fontSize":48}
          ]}
        ]})
    }
    #[test]
    fn full_page_geometry_stops_pushing_text_below_the_artboard() {
        let mut root = fixture();
        assert_eq!(anchor_layers(&mut root, 1080.0, 1440.0), 2);
        let doc = serde_json::from_value(serde_json::json!({"version":"1.0.0","children":[root]}))
            .unwrap();
        let scene = op_pen_loader::editor_state_to_layout_scene(&EditorState::from_document(doc));
        let page = &scene.pages[0];
        assert!(page.find("title").unwrap().bounds.origin.y < 200.0);
        assert_eq!(page.find("root").unwrap().bounds.size.y, 1440.0);
        assert_eq!(page.find("rule").unwrap().bounds.origin.x, 62.0);
    }
    #[test]
    fn background_repairs_preserve_every_existing_node_identity() {
        let doc =
            serde_json::from_value(serde_json::json!({"version":"1.0.0","children":[fixture()]}))
                .unwrap();
        let mut state = EditorState::from_document(doc);
        let root = state.active_children()[0].clone();
        let mut plan = crate::loop_finalize::synthesize_plan(std::slice::from_ref(&root), 1080.0);
        plan.root_frame.height = 1440.0;
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        repair(&mut sink, "root", &plan);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let page = scene.active_page().unwrap();
        for id in ["root", "body", "grid", "dot", "rule", "title"] {
            assert!(page.find(id).is_some(), "{id}");
        }
        assert!(page.find("title").unwrap().bounds.origin.y < 200.0);
        assert_eq!(page.find("root").unwrap().bounds.size.y, 1440.0);
        let body = find_root(sink.state(), "body").unwrap();
        assert_eq!(body.children().unwrap()[0].id_str(), "title");
    }
    #[test]
    fn hugging_margin_layer_stops_creating_a_page_of_blank_flow_space() {
        let mut root = fixture();
        let layers = root["children"][0]["children"].as_array_mut().unwrap();
        layers[0] = serde_json::json!({"type":"frame","id":"margin","width":"fill_container","height":"fit_content","layout":"none","children":[
            {"type":"rectangle","id":"margin-rule","name":"margin-rule","width":2,"height":1216,"fill":[{"type":"solid","color":"#cc0000"}]}
        ]});
        layers.remove(1);
        assert_eq!(anchor_layers(&mut root, 1080.0, 1440.0), 1);
        assert_eq!(
            root["children"][0]["children"][0]["constraints"]["v"],
            "top"
        );
        assert_eq!(root["children"][0]["height"], "fill_container");
        let doc = serde_json::from_value(serde_json::json!({"version":"1.0.0","children":[root]}))
            .unwrap();
        let scene = op_pen_loader::editor_state_to_layout_scene(&EditorState::from_document(doc));
        assert!(scene.pages[0].find("title").unwrap().bounds.origin.y < 200.0);
    }
    #[test]
    fn full_page_content_graphics_are_not_reinterpreted_as_backgrounds() {
        let mut root = fixture();
        root["children"][0]["children"][0]["children"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"type":"text","id":"chart-label","content":"Actual chart"}));
        let before = root.clone();
        assert_eq!(anchor_layers(&mut root, 1080.0, 1440.0), 0);
        assert_eq!(root, before);
    }
}
