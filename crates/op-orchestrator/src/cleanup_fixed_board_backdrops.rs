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
    let body_width = inherited_width(body, Some(width));
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
        .any(|layer| page_geometry(layer, width, height, body_width))
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

fn inherited_width(node: &Value, parent_width: Option<f64>) -> Option<f64> {
    match node.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) if s == "fill_container" => parent_width,
        _ => None,
    }
}

fn page_geometry(node: &Value, width: f64, height: f64, parent_width: Option<f64>) -> bool {
    let h = node.get("height").and_then(Value::as_f64).unwrap_or(0.0);
    // A full-height grid commonly inherits the body's proven page width.
    // Do not assume every fill_container is page-wide: narrow/unknown parents
    // retain their own width proof through nested geometry wrappers.
    let inherited = inherited_width(node, parent_width);
    let w = inherited.unwrap_or(0.0);
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
                    .any(|child| page_geometry(child, width, height, inherited))
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
    fn inherited_full_width_grid_cannot_push_poster_text_outside_the_board() {
        let mut root = fixture();
        let layers = root["children"][0]["children"].as_array_mut().unwrap();
        layers[0]["width"] = serde_json::json!("fill_container");
        layers.remove(1);
        assert_eq!(anchor_layers(&mut root, 1080.0, 1440.0), 1);
        let doc = serde_json::from_value(serde_json::json!({"version":"1.0.0","children":[root]}))
            .unwrap();
        let scene = op_pen_loader::editor_state_to_layout_scene(&EditorState::from_document(doc));
        assert!(scene.pages[0].find("title").unwrap().bounds.origin.y < 200.0);
    }

    #[test]
    fn inherited_width_in_a_narrow_body_is_not_a_page_background() {
        let mut root = fixture();
        root["children"][0]["width"] = serde_json::json!(540.0);
        let layers = root["children"][0]["children"].as_array_mut().unwrap();
        layers[0]["width"] = serde_json::json!("fill_container");
        layers.remove(1);
        assert_eq!(anchor_layers(&mut root, 1080.0, 1440.0), 0);
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

    /// Replay the production repair on a captured design without regenerating
    /// its content. Ignored in CI because artifact paths are supplied locally.
    #[test]
    #[ignore = "requires OPENPENCIL_REPLAY_INPUT and OPENPENCIL_REPLAY_OUTPUT"]
    fn replay_captured_fixed_board_background_without_changing_copy() {
        fn copy(node: &Value, values: &mut Vec<(String, Value)>) {
            if node.get("type").and_then(Value::as_str) == Some("text") {
                values.push((
                    node["id"].as_str().unwrap().to_owned(),
                    node["content"].clone(),
                ));
            }
            for child in node
                .get("children")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                copy(child, values);
            }
        }
        let input = std::env::var("OPENPENCIL_REPLAY_INPUT").unwrap();
        let output = std::env::var("OPENPENCIL_REPLAY_OUTPUT").unwrap();
        let doc = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
        let mut state = EditorState::from_document(doc);
        let root = state.active_children()[0].clone();
        let before = serde_json::to_value(&root).unwrap();
        let width = before["width"].as_f64().unwrap();
        let height = before["height"].as_f64().unwrap();
        let mut original = Vec::new();
        copy(&before, &mut original);
        original.sort_by(|a, b| a.0.cmp(&b.0));
        let mut plan = crate::loop_finalize::synthesize_plan(std::slice::from_ref(&root), width);
        plan.root_frame.height = height;
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        repair(&mut sink, root.id_str(), &plan);
        let after = serde_json::to_value(find_root(sink.state(), root.id_str()).unwrap()).unwrap();
        let mut repaired = Vec::new();
        copy(&after, &mut repaired);
        repaired.sort_by(|a, b| a.0.cmp(&b.0));
        assert!(!original.is_empty());
        assert_eq!(original, repaired);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let page = scene.active_page().unwrap();
        let board = page.find(root.id_str()).unwrap().bounds;
        for (id, _) in &repaired {
            let text = page.find(id).unwrap().bounds;
            assert!(text.origin.y >= board.origin.y - 1.0, "{id}");
            assert!(
                text.origin.y + text.size.y <= board.origin.y + board.size.y + 1.0,
                "{id} remains outside the board"
            );
        }
        std::fs::write(
            output,
            serde_json::to_vec_pretty(&sink.state().doc).unwrap(),
        )
        .unwrap();
    }
}
