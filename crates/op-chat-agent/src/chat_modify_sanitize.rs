pub fn sanitize_modify_replacement(incoming: &mut serde_json::Value, existing: &serde_json::Value) {
    strip_unrequested_avatar_image_border(incoming, existing);
}

fn strip_unrequested_avatar_image_border(
    incoming: &mut serde_json::Value,
    existing: &serde_json::Value,
) {
    if has_visible_stroke(existing) {
        return;
    }
    if !is_avatar_like(incoming) && !is_avatar_like(existing) {
        return;
    }
    if !contains_image_visual(incoming) {
        return;
    }
    if let serde_json::Value::Object(obj) = incoming {
        obj.remove("stroke");
    }
}

fn has_visible_stroke(node: &serde_json::Value) -> bool {
    node.get("stroke")
        .and_then(|stroke| stroke.get("thickness"))
        .is_some_and(has_positive_thickness)
}

fn has_positive_thickness(value: &serde_json::Value) -> bool {
    if let Some(width) = value.as_f64() {
        return width > 0.0;
    }
    if let Some(widths) = value.as_array() {
        return widths.iter().any(has_positive_thickness);
    }
    value
        .as_object()
        .is_some_and(|widths| widths.values().any(has_positive_thickness))
}

fn is_avatar_like(node: &serde_json::Value) -> bool {
    node.get("role")
        .and_then(|v| v.as_str())
        .is_some_and(|role| role == "avatar")
        || node
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|name| {
                let lower = name.to_ascii_lowercase();
                lower.contains("avatar") || name.contains("头像")
            })
}

fn contains_image_visual(node: &serde_json::Value) -> bool {
    if node.get("type").and_then(|v| v.as_str()) == Some("image") {
        return true;
    }
    if fill_is_image(node.get("fill")) {
        return true;
    }
    node.get("children")
        .and_then(|v| v.as_array())
        .is_some_and(|children| children.iter().any(contains_image_visual))
}

fn fill_is_image(fill: Option<&serde_json::Value>) -> bool {
    match fill {
        Some(serde_json::Value::Object(obj)) => {
            obj.get("type").and_then(|v| v.as_str()) == Some("image")
        }
        Some(serde_json::Value::Array(items)) => items.iter().any(|item| fill_is_image(Some(item))),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn zero_width_existing_avatar_stroke_does_not_preserve_new_border() {
        let existing = json!({
            "type": "frame",
            "name": "Avatar",
            "role": "avatar",
            "stroke": {
                "thickness": 0,
                "fill": [{"type": "solid", "color": "#000000"}]
            }
        });
        let mut incoming = json!({
            "type": "frame",
            "name": "Avatar",
            "role": "avatar",
            "stroke": {
                "thickness": 1,
                "fill": [{"type": "solid", "color": "#E5E7EB"}]
            },
            "fill": [{"type": "image", "url": ""}]
        });

        sanitize_modify_replacement(&mut incoming, &existing);

        assert!(incoming.get("stroke").is_none());
    }
}
