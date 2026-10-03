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
    if anchor_layers(&mut value, width, height) == 0 {
        return;
    }
    if let Ok(node) = serde_json::from_value(value) {
        sink.apply(EditorCommand::ReplaceSubtree {
            node_id: NodeId::new(root_id),
            node,
            drop_children: true,
            page_id: None,
        });
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
        || body.get("layout").and_then(Value::as_str) != Some("vertical")
        || body.get("height").and_then(Value::as_str) != Some("fit_content")
        || !contains_text(body)
    {
        return 0;
    }
    let Some(layers) = body.get_mut("children").and_then(Value::as_array_mut) else {
        return 0;
    };
    let mut count = 0;
    for layer in layers {
        if !pure_geometry(layer) {
            break;
        }
        if layer.get("constraints").is_some() {
            continue;
        }
        let h = layer.get("height").and_then(Value::as_f64).unwrap_or(0.0);
        let w = layer.get("width").and_then(Value::as_f64).unwrap_or(0.0);
        if h < height * 0.95 || !(w >= width * 0.95 || (w > 0.0 && w <= 8.0)) {
            continue;
        }
        layer["constraints"] = serde_json::json!({"h":"left", "v":"top"});
        count += 1;
    }
    if count > 0 {
        body["height"] = Value::String("fill_container".into());
        root["height"] = serde_json::json!(height);
    }
    count
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
