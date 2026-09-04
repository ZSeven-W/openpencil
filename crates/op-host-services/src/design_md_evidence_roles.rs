//! Role-specific color provenance and sparse-palette fallbacks.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub(crate) struct RoleColorProvenance {
    pub(crate) all: BTreeSet<String>,
    pub(crate) by_role: BTreeMap<String, BTreeSet<String>>,
}

pub(crate) fn from_sanitized_json(json: &str) -> RoleColorProvenance {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return RoleColorProvenance::default();
    };
    let background = root
        .get("pageBackground")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("#FFFFFF")
        .to_string();
    let mut by_usage = BTreeMap::<&str, BTreeSet<String>>::new();
    if let Some(colors) = root.get("colors").and_then(serde_json::Value::as_array) {
        for color in colors {
            let Some(value) = color.get("value").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(usage) = color.get("usage").and_then(serde_json::Value::as_str) else {
                continue;
            };
            by_usage.entry(usage).or_default().insert(value.to_string());
        }
    }
    let mut component_backgrounds = BTreeSet::new();
    let mut component_text = BTreeSet::new();
    if let Some(components) = root.get("components").and_then(serde_json::Value::as_array) {
        for sample in components
            .iter()
            .filter_map(|component| component.get("samples"))
            .filter_map(serde_json::Value::as_array)
            .flatten()
        {
            collect_color(sample, "background", &mut component_backgrounds);
            collect_color(sample, "color", &mut component_text);
        }
    }
    let mut surfaces = by_usage.remove("background").unwrap_or_default();
    surfaces.extend(component_backgrounds);
    let mut texts = by_usage.remove("text").unwrap_or_default();
    texts.extend(component_text);
    let borders = by_usage.remove("border").unwrap_or_default();
    let gradients = by_usage.remove("gradient").unwrap_or_default();
    let mut all_observed = BTreeSet::from([background.clone()]);
    all_observed.extend(surfaces.iter().cloned());
    all_observed.extend(texts.iter().cloned());
    all_observed.extend(borders.iter().cloned());
    all_observed.extend(gradients.iter().cloned());
    for values in by_usage.into_values() {
        all_observed.extend(values);
    }

    let primary_fallback = if relative_luminance(&background) < 0.35 {
        "#FFFFFF".to_string()
    } else {
        "#111111".to_string()
    };
    let primary_text = if texts.is_empty() {
        BTreeSet::from([primary_fallback.clone()])
    } else {
        texts.clone()
    };
    let mut secondary_text = texts.clone();
    let mut muted_text = texts.clone();
    let mut border = borders.clone();
    for primary in &primary_text {
        secondary_text.insert(mix_hex(primary, &background, 184));
        muted_text.insert(mix_hex(primary, &background, 128));
        border.insert(mix_hex(primary, &background, 36));
    }
    let card_surface = if surfaces.is_empty() {
        BTreeSet::from([background.clone()])
    } else {
        surfaces.clone()
    };
    let mut accent = gradients;
    if accent.is_empty() {
        accent.extend(
            all_observed
                .iter()
                .filter(|color| **color != background && !card_surface.contains(*color))
                .cloned(),
        );
    }
    if accent.is_empty() {
        accent.extend(primary_text.iter().cloned());
    }
    let mut by_role = BTreeMap::new();
    by_role.insert("Page Background".to_string(), BTreeSet::from([background]));
    by_role.insert("Card Surface".to_string(), card_surface);
    by_role.insert("Primary Accent".to_string(), accent);
    by_role.insert("Primary Text".to_string(), primary_text);
    by_role.insert("Secondary Text".to_string(), secondary_text);
    by_role.insert("Muted Text".to_string(), muted_text);
    by_role.insert("Default Border".to_string(), border);
    let all = by_role
        .values()
        .flat_map(|values| values.iter().cloned())
        .collect();
    RoleColorProvenance { all, by_role }
}

fn collect_color(value: &serde_json::Value, field: &str, out: &mut BTreeSet<String>) {
    if let Some(color) = value.get(field).and_then(serde_json::Value::as_str) {
        out.insert(color.to_string());
    }
}

fn mix_hex(front: &str, back: &str, opacity: u8) -> String {
    let (front_red, front_green, front_blue) = rgb(front).unwrap_or((0, 0, 0));
    let (back_red, back_green, back_blue) = rgb(back).unwrap_or((255, 255, 255));
    let mix = |front: u8, back: u8| {
        let alpha = u32::from(opacity);
        (((u32::from(front) * alpha) + (u32::from(back) * (255 - alpha)) + 127) / 255) as u8
    };
    format!(
        "#{:02X}{:02X}{:02X}",
        mix(front_red, back_red),
        mix(front_green, back_green),
        mix(front_blue, back_blue)
    )
}

fn rgb(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }
    let channel = |start| u8::from_str_radix(&value[start..start + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

fn relative_luminance(value: &str) -> f64 {
    let Some((red, green, blue)) = rgb(value) else {
        return 1.0;
    };
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    (0.2126 * channel(red)) + (0.7152 * channel(green)) + (0.0722 * channel(blue))
}
