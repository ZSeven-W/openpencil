//! Recover only blank height around a centered row of fixed-size controls.
use jian_scene::layout_scene::SceneNode;
use serde_json::Value;

pub(super) fn spend_blank_row_height(node: &mut Value, scene: &SceneNode, deficit: f64) -> bool {
    let mut remaining = deficit;
    visit(node, scene, &mut remaining, true);
    remaining < deficit
}

fn visit(node: &mut Value, scene: &SceneNode, remaining: &mut f64, root: bool) {
    if *remaining <= 0.0 || node.get("constraints").is_some() || super::is_scroll_viewport(node) {
        return;
    }
    if let Some(slack) = row_slack(node, scene) {
        let spent = slack.min(*remaining);
        node["height"] = serde_json::json!(node["height"].as_f64().unwrap() - spent);
        *remaining -= spent;
        return;
    }
    // Only hug-height vertical ancestors propagate reclaimed space to the
    // board. Shrinking a row inside a fixed card would only alter that card.
    if node["layout"] != "vertical"
        || (!root
            && (node.get("clipContent").and_then(Value::as_bool) == Some(true)
                || node
                    .get("height")
                    .is_some_and(|height| height.as_str() != Some("fit_content"))))
        || (!root
            && ["x", "y", "position", "minHeight", "maxHeight"]
                .iter()
                .any(|key| node.get(key).is_some()))
    {
        return;
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            visit(child, scene, remaining, false);
        }
    }
}

fn row_slack(node: &Value, scene: &SceneNode) -> Option<f64> {
    if node["layout"] != "horizontal"
        || node["alignItems"] != "center"
        || node.get("clipContent").and_then(Value::as_bool) == Some(true)
        || [
            "role",
            "x",
            "y",
            "position",
            "minHeight",
            "maxHeight",
            "events",
            "bindings",
            "stroke",
            "effects",
        ]
        .iter()
        .any(|key| node.get(key).is_some())
        || node.get("rotation").and_then(Value::as_f64).unwrap_or(0.0) != 0.0
        || node.get("reusable").and_then(Value::as_bool) == Some(true)
        || node.get("padding").is_some_and(Value::is_string)
        || node
            .get("fill")
            .is_some_and(|fill| !fill.as_array().is_some_and(Vec::is_empty))
    {
        return None;
    }
    let height = node["height"].as_f64()?;
    if !(44.0..=160.0).contains(&height) {
        return None;
    }
    let children = node["children"].as_array()?;
    if children.len() < 2 {
        return None;
    }
    let mut tallest: f64 = 0.0;
    for child in children {
        if !matches!(child["role"].as_str(), Some("button" | "icon-button"))
            || ["x", "y", "constraints"]
                .iter()
                .any(|key| child.get(key).is_some())
            || child["height"].as_f64().is_none()
        {
            return None;
        }
        let rect = scene.find(child["id"].as_str()?)?.bounds;
        tallest = tallest.max(f64::from(rect.size.y));
    }
    let padding = super::padding(node);
    // Controls retain their measured size and at least 8px above and below.
    let minimum = tallest + (padding[0] + padding[2]).max((padding[0] - padding[2]).abs() + 16.0);
    (height > minimum && tallest > 0.0).then_some(height - minimum)
}
