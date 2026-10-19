//! Readable route and marker colors for newly generated owned map parts.
//! Never change the document palette or an authored overlay.

use op_design_lint::color::{color_contrast, parse_hex_color};
use serde_json::Value;

pub(super) fn adapt(patch: &mut Value, resolve: impl Fn(&str) -> Option<String>) {
    let Some(hint) = patch["explain"].as_str() else {
        return;
    };
    if !hint.starts_with("map-template:") {
        return;
    }
    let kind = crate::map_templates::MapTemplate::from_hint(hint);
    let Some(children) = patch["children"].as_array() else {
        return;
    };
    let mut backgrounds = Vec::new();
    for child in children.iter().filter(|child| owned(child)) {
        let name = child["name"].as_str().unwrap_or_default();
        let color = if name == "map-ground"
            || name.starts_with("map-block-")
            || matches!(name, "map-park" | "map-pond" | "map-waterfront")
        {
            &child["fill"][0]["color"]
        } else {
            continue;
        };
        if let Some(color) = color.as_str().and_then(&resolve) {
            if parse_hex_color(&color).is_some() && !backgrounds.contains(&color) {
                backgrounds.push(color);
            }
        }
    }
    if backgrounds.is_empty() {
        return;
    }
    let readable = |color: &str| {
        parse_hex_color(color).is_some()
            && backgrounds
                .iter()
                .all(|background| color_contrast(color, background) >= 3.0)
    };
    let primary = resolve("$--primary");
    let accent = resolve("$--accent");
    // Prefer existing brand tokens. Scene defaults only rescue an unreadable
    // route; the original document tokens remain untouched.
    let (light, dark) = match kind {
        crate::map_templates::MapTemplate::Running => ("#86EFAC", "#166534"),
        crate::map_templates::MapTemplate::Delivery => ("#FDBA74", "#9A3412"),
        crate::map_templates::MapTemplate::Ride | crate::map_templates::MapTemplate::Location => {
            ("#93C5FD", "#1E40AF")
        }
    };
    let candidates = [
        ("$--primary", primary.as_deref()),
        ("$--accent", accent.as_deref()),
        (light, Some(light)),
        (dark, Some(dark)),
    ];
    let Some((route, route_color)) = candidates
        .into_iter()
        .find(|(_, color)| color.is_some_and(readable))
    else {
        return;
    };
    let route_color = route_color.unwrap();
    // The halo is owned decoration, not terrain. If only its surface clashes
    // with the brand route, change that backing instead of changing the brand.
    let backing = backgrounds
        .iter()
        .max_by(|a, b| color_contrast(route_color, a).total_cmp(&color_contrast(route_color, b)));
    let foreground = resolve("$--primary-foreground");
    let ink = if foreground.as_deref().is_some_and(|color| {
        parse_hex_color(color).is_some() && color_contrast(color, route_color) >= 4.5
    }) {
        "$--primary-foreground"
    } else if color_contrast("#FFFFFF", route_color) >= color_contrast("#111827", route_color) {
        "#FFFFFF"
    } else {
        "#111827"
    };
    for child in patch["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .filter(|child| owned(child))
    {
        let name = child["name"].as_str().unwrap_or_default();
        let surface = match name {
            "map-route-halo" => Some(&mut child["stroke"]["fill"][0]["color"]),
            "map-origin-halo" | "map-destination-halo" | "map-courier" | "map-vehicle" => {
                Some(&mut child["fill"][0]["color"])
            }
            _ => None,
        };
        if let (Some(surface), Some(backing)) = (surface, backing) {
            if surface
                .as_str()
                .and_then(&resolve)
                .is_some_and(|color| color_contrast(route_color, &color) < 3.0)
            {
                *surface = Value::String(backing.clone());
            }
        }
        replace(child, route, ink);
    }
}

fn owned(node: &Value) -> bool {
    node["explain"]
        .as_str()
        .is_some_and(|hint| hint.starts_with("map-template-part:"))
}

fn replace(value: &mut Value, route: &str, ink: &str) {
    match value {
        Value::String(text) if text == "$--primary" => *text = route.to_string(),
        Value::String(text) if text == "$--primary-foreground" => *text = ink.to_string(),
        Value::Array(values) => {
            for value in values {
                replace(value, route, ink);
            }
        }
        Value::Object(fields) => {
            for value in fields.values_mut() {
                replace(value, route, ink);
            }
        }
        _ => {}
    }
}
