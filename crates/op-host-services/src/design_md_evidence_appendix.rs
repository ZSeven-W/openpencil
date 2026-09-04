//! Deterministic exact-line appendix tokens derived from sanitized evidence.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub(crate) struct DesignMdAppendixProvenance {
    pub(crate) spacing: BTreeSet<String>,
    pub(crate) shadows: BTreeSet<String>,
    pub(crate) gradients: BTreeSet<String>,
    pub(crate) css_variables: BTreeSet<String>,
    pub(crate) components: BTreeSet<String>,
    pub(crate) treatments: BTreeSet<String>,
    pub(crate) media_queries: BTreeSet<String>,
}

pub(crate) fn from_sanitized_json(json: &str) -> DesignMdAppendixProvenance {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return DesignMdAppendixProvenance::default();
    };
    let mut out = DesignMdAppendixProvenance::default();
    collect_spacing(&root, &mut out.spacing);
    collect_values(&root, "shadows", &mut out.shadows);
    collect_values(&root, "gradients", &mut out.gradients);
    collect_variables(&root, &mut out.css_variables);
    collect_components(&root, &mut out.components, &mut out.treatments);
    if let Some(values) = root
        .get("mediaQueries")
        .and_then(serde_json::Value::as_array)
    {
        for value in values.iter().filter_map(serde_json::Value::as_str) {
            out.media_queries.insert(value.to_string());
        }
    }
    out
}

fn collect_spacing(root: &serde_json::Value, out: &mut BTreeSet<String>) {
    let Some(values) = root.get("spacing").and_then(serde_json::Value::as_array) else {
        return;
    };
    for value in values {
        let Some(property) = value.get("property").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(number) = value.get("value").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        let property = match property {
            "margin" => "Margin",
            "padding" => "Padding",
            "gap" => "Gap",
            _ => continue,
        };
        out.insert(format!("{property}: {}px", format_number(number)));
    }
}

fn collect_values(root: &serde_json::Value, field: &str, out: &mut BTreeSet<String>) {
    let Some(values) = root.get(field).and_then(serde_json::Value::as_array) else {
        return;
    };
    for value in values
        .iter()
        .filter_map(|value| value.get("value"))
        .filter_map(serde_json::Value::as_str)
    {
        out.insert(value.to_string());
    }
}

fn collect_variables(root: &serde_json::Value, out: &mut BTreeSet<String>) {
    let Some(values) = root
        .get("cssVariables")
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };
    for value in values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let canonical: BTreeMap<String, serde_json::Value> = object
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if let Ok(json) = serde_json::to_string(&canonical) {
            out.insert(json);
        }
    }
}

fn collect_components(
    root: &serde_json::Value,
    components: &mut BTreeSet<String>,
    treatments: &mut BTreeSet<String>,
) {
    let Some(values) = root.get("components").and_then(serde_json::Value::as_array) else {
        return;
    };
    for value in values {
        let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) else {
            continue;
        };
        components.insert(kind.to_string());
        let Some(samples) = value.get("samples").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for sample in samples {
            let Some(sample) = sample.as_object() else {
                continue;
            };
            let mut canonical: BTreeMap<String, serde_json::Value> = sample
                .iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            canonical.insert(
                "kind".to_string(),
                serde_json::Value::String(kind.to_string()),
            );
            if let Ok(json) = serde_json::to_string(&canonical) {
                treatments.insert(json);
            }
        }
    }
}

fn format_number(value: f64) -> String {
    let mut value = value.to_string();
    if value.ends_with(".0") {
        value.truncate(value.len() - 2);
    }
    value
}
