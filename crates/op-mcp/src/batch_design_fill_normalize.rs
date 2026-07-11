pub(super) fn normalize_fill(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(color) => {
            *value = serde_json::json!([{ "type": "solid", "color": color }]);
        }
        serde_json::Value::Object(_) => {
            let single = std::mem::take(value);
            *value = serde_json::Value::Array(vec![single]);
        }
        _ => {}
    }
    if let serde_json::Value::Array(items) = value {
        for item in items {
            normalize_fill_item(item);
        }
    }
}

fn normalize_fill_item(value: &mut serde_json::Value) {
    let serde_json::Value::Object(obj) = value else {
        return;
    };
    if obj.get("type").and_then(serde_json::Value::as_str) != Some("image") {
        return;
    }
    let has_url = obj.get("url").and_then(serde_json::Value::as_str).is_some();
    if !has_url {
        for alias in ["src", "source", "imageUrl", "image_url", "uri", "href"] {
            if let Some(value) = obj.get(alias).cloned() {
                if value.as_str().is_some_and(|text| !text.trim().is_empty()) {
                    obj.insert("url".into(), value);
                    break;
                }
            }
        }
        obj.entry("url")
            .or_insert_with(|| serde_json::Value::String(String::new()));
    }
    if !obj.contains_key("mode") {
        let mode = obj
            .remove("fit")
            .or_else(|| obj.remove("objectFit"))
            .and_then(|value| value.as_str().map(str::to_string))
            .and_then(|raw| match raw.trim().to_ascii_lowercase().as_str() {
                "cover" | "crop" => Some("crop"),
                "contain" | "fit" => Some("fit"),
                "fill" => Some("fill"),
                "stretch" => Some("stretch"),
                "tile" => Some("tile"),
                _ => None,
            });
        if let Some(mode) = mode {
            obj.insert("mode".into(), serde_json::Value::String(mode.to_string()));
        }
    }
}
