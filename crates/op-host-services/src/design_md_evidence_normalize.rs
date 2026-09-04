//! Canonical color normalization for extension design evidence.
//!
//! Page background alpha is composited over white first. Every other
//! foreground token is then composited over that opaque page background,
//! matching `op-chrome-extension-core::design_md_palette::opaque_hex`.

pub(crate) fn normalize_design_color_evidence(root: &mut serde_json::Value) {
    let Some(object) = root.as_object_mut() else {
        return;
    };
    let normalized_background = object
        .get("pageBackground")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| opaque_hex(value, "#FFFFFF"));
    let background = normalized_background
        .clone()
        .unwrap_or_else(|| "#FFFFFF".to_string());
    if let Some(normalized_background) = normalized_background {
        object.insert(
            "pageBackground".to_string(),
            serde_json::Value::String(normalized_background),
        );
    }
    normalize_array_field(object.get_mut("colors"), "value", &background);
    if let Some(components) = object
        .get_mut("components")
        .and_then(serde_json::Value::as_array_mut)
    {
        for component in components {
            let Some(samples) = component
                .get_mut("samples")
                .and_then(serde_json::Value::as_array_mut)
            else {
                continue;
            };
            for sample in samples {
                normalize_object_field(sample, "background", &background);
                normalize_object_field(sample, "color", &background);
            }
        }
    }
    if let Some(variables) = object
        .get_mut("cssVariables")
        .and_then(serde_json::Value::as_array_mut)
    {
        for variable in variables {
            if variable.get("kind").and_then(serde_json::Value::as_str) == Some("color") {
                normalize_object_field(variable, "value", &background);
            }
        }
    }
}

fn normalize_array_field(value: Option<&mut serde_json::Value>, field: &str, background: &str) {
    let Some(values) = value.and_then(serde_json::Value::as_array_mut) else {
        return;
    };
    for value in values {
        normalize_object_field(value, field, background);
    }
}

fn normalize_object_field(value: &mut serde_json::Value, field: &str, background: &str) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let Some(color) = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .and_then(|value| opaque_hex(value, background))
    else {
        return;
    };
    object.insert(field.to_string(), serde_json::Value::String(color));
}

fn opaque_hex(value: &str, background: &str) -> Option<String> {
    let (red, green, blue, alpha) = rgba(value)?;
    if alpha == 255 {
        return Some(format!("#{red:02X}{green:02X}{blue:02X}"));
    }
    let (back_red, back_green, back_blue, _) = rgba(background)?;
    let blend = |front: u8, back: u8| {
        let alpha = u32::from(alpha);
        (((u32::from(front) * alpha) + (u32::from(back) * (255 - alpha)) + 127) / 255) as u8
    };
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        blend(red, back_red),
        blend(green, back_green),
        blend(blue, back_blue)
    ))
}

fn rgba(value: &str) -> Option<(u8, u8, u8, u8)> {
    let value = value.strip_prefix('#')?;
    if !matches!(value.len(), 6 | 8) || !value.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let channel = |start| u8::from_str_radix(&value[start..start + 2], 16).ok();
    Some((
        channel(0)?,
        channel(2)?,
        channel(4)?,
        if value.len() == 8 { channel(6)? } else { 255 },
    ))
}
