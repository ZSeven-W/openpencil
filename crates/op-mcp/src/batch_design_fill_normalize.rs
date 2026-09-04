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
    // Normalize fill variant type names (hyphenated / camelCase → snake_case) BEFORE
    // any variant-specific code. GLM-5.3 occasionally emits `"linear-gradient"` (CSS
    // spelling) or `"linearGradient"` (camelCase) instead of the schema's
    // `"linear_gradient"`. Map all six variants uniformly; leave unrecognized
    // values untouched so the schema still rejects genuinely wrong types.
    normalize_fill_type_name(obj);
    // Bare "gradient" is ambiguous and unmappable — the model's ONLY way to
    // communicate a linear gradient without a type name is an angle + stops.
    // Map it ONLY when it has the linear gradient shape, leaving lone bare
    // "gradient" for the schema to reject.
    normalize_bare_gradient(obj);
    // Gradient stops may have `pos` instead of the schema's `offset` field name.
    // Rename them so the stop deserializes.
    normalize_gradient_stops(obj);
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

/// Map bare "gradient" to `linear_gradient` when the object has a `stops` array,
/// indicating a linear gradient's shape. A bare "gradient" without stops is left
/// untouched so the schema can reject it properly.
fn normalize_bare_gradient(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(type_value) = obj.get("type") else {
        return;
    };
    let Some(raw) = type_value.as_str() else {
        return;
    };
    if !raw.trim().eq_ignore_ascii_case("gradient") {
        return;
    }
    // Only rewrite to linear_gradient if the object has stops, proving it's
    // a linear gradient and not an ambiguous bare word.
    let has_stops = obj.get("stops").map(|v| v.is_array()).unwrap_or(false);
    if has_stops {
        obj.insert(
            "type".into(),
            serde_json::Value::String("linear_gradient".to_string()),
        );
    }
}

/// Rename `pos` to `offset` in gradient stops where `offset` is absent. The schema
/// requires `offset: f32` in each stop, but models occasionally write `pos` instead.
fn normalize_gradient_stops(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(serde_json::Value::Array(stops)) = obj.get_mut("stops") else {
        return;
    };
    for stop in stops.iter_mut() {
        if let serde_json::Value::Object(stop_obj) = stop {
            if !stop_obj.contains_key("offset") {
                if let Some(pos) = stop_obj.remove("pos") {
                    stop_obj.insert("offset".into(), pos);
                }
            }
        }
    }
}

/// Normalize fill variant type name from hyphenated or camelCase to snake_case.
/// Maps `linear-gradient` / `linearGradient` → `linear_gradient` and siblings
/// for all six variants: solid, linear_gradient, radial_gradient, mesh_gradient,
/// shader, image. Unrecognized values are left untouched for the schema to reject.
fn normalize_fill_type_name(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let Some(type_value) = obj.get("type") else {
        return;
    };
    let Some(raw) = type_value.as_str() else {
        return;
    };
    let canon = raw.trim().to_ascii_lowercase().replace('-', "_");
    let normalized = match canon.as_str() {
        "solid" => Some("solid"),
        "linear_gradient" | "lineargradient" => Some("linear_gradient"),
        "radial_gradient" | "radialgradient" => Some("radial_gradient"),
        "mesh_gradient" | "meshgradient" => Some("mesh_gradient"),
        "shader" => Some("shader"),
        "image" => Some("image"),
        _ => None,
    };
    if let Some(valid) = normalized {
        obj.insert("type".into(), serde_json::Value::String(valid.to_string()));
    }
}

#[cfg(test)]
#[path = "batch_design_fill_normalize_tests.rs"]
mod tests;
