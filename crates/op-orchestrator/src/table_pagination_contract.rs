//! Bind a generated local dataset to its unbound pagination controls.
use super::*;
use op_editor_core::table_pagination::{buttons, Spec, MARKER};
fn named<'a>(value: &'a Value, needle: &str, out: &mut Vec<&'a Value>) {
    if value["name"]
        .as_str()
        .is_some_and(|s| s.to_lowercase().contains(needle))
    {
        out.push(value);
    }
    for child in children(value) {
        named(child, needle, out);
    }
}
fn deep_protected(value: &Value) -> bool {
    protected(value)
        || value["opacity"].is_string()
        || value["enabled"].is_string()
        || value.get("x").is_some()
        || value.get("y").is_some()
        || children(value).iter().any(deep_protected)
}
fn number(value: &Value) -> Option<u32> {
    single_text(value)?.parse().ok()
}
fn set_event(mut value: Value, key: &str, expression: &str) -> Value {
    value["events"] = json!({"onTap":[{"set":{format!("$app.{key}"):expression}}]});
    value
}
pub(super) fn wire(sink: &mut dyn DocSink, root: &Value, table: &Value) {
    if !owned(table, TABLE_MARKER) || Spec::from_node(table).is_some() {
        return;
    }
    let rows = &children(table)[1..];
    if rows.is_empty() || !rows.iter().all(|row| owned(row, ROW_MARKER)) {
        return;
    }
    let mut footers = vec![];
    named(root, "pagination footer", &mut footers);
    if footers.len() != 1 {
        return;
    }
    let footer = footers[0];
    let mut groups = vec![];
    named(footer, "page buttons", &mut groups);
    if groups.len() != 1 {
        return;
    }
    let group = groups[0];
    if group["layout"] != "horizontal" || deep_protected(group) {
        return;
    }
    let prev: Vec<_> = children(group)
        .iter()
        .filter(|n| {
            n["name"]
                .as_str()
                .is_some_and(|s| s.to_lowercase().contains("prev"))
        })
        .collect();
    let next: Vec<_> = children(group)
        .iter()
        .filter(|n| {
            n["name"]
                .as_str()
                .is_some_and(|s| s.to_lowercase().contains("next"))
        })
        .collect();
    let active: Vec<_> = children(group)
        .iter()
        .filter(|n| number(n) == Some(1))
        .collect();
    if prev.len() != 1 || next.len() != 1 || active.len() != 1 {
        return;
    }
    let mut controls = vec![];
    selects(footer, &mut controls);
    let sizes: Vec<_> = controls
        .iter()
        .filter(|c| {
            c["name"]
                .as_str()
                .is_some_and(|s| s.to_lowercase().contains("per page"))
        })
        .collect();
    if sizes.len() != 1 || protected(sizes[0]) {
        return;
    }
    let control = sizes[0];
    let Some(options) = control["options"].as_array() else {
        return;
    };
    let Some(size_values): Option<Vec<u32>> = options
        .iter()
        .map(|v| v["value"].as_str()?.parse().ok())
        .collect()
    else {
        return;
    };
    if size_values.is_empty() || size_values.iter().any(|n| !(5..=100).contains(n)) {
        return;
    }
    let Some(default_size) = control["value"]
        .as_str()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| size_values.contains(n))
    else {
        return;
    };
    let Some(table_id) = table["id"].as_str() else {
        return;
    };
    let Some(items_id) = group["id"].as_str() else {
        return;
    };
    let prefix = format!(
        "op_paging_{}",
        table_id
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let fields: [String; 6] =
        ["page", "size", "total", "pages", "start", "end"].map(|field| format!("{prefix}_{field}"));
    if sink
        .state()
        .doc
        .state
        .as_ref()
        .is_some_and(|state| fields.iter().any(|key| state.contains_key(key)))
    {
        return;
    }
    let mut counters = vec![];
    find_counter(footer, &mut counters);
    if counters.len() != 1 {
        return;
    }
    let counter = counters[0];
    if counter["events"].is_object() || counter.get("state").is_some() {
        return;
    }
    let old = counter["bindings"]["content"].as_str().unwrap_or("");
    if !old.starts_with("\"Rows on this page: \"") && !old.starts_with("\"当前页数据：\"") {
        return;
    }
    let mut controls = vec![];
    owned_controls(root, &mut controls);
    let keys: Vec<String> = controls
        .iter()
        .filter_map(|c| {
            c["bindings"]["bind:value"]
                .as_str()?
                .strip_prefix("$state.")
                .map(str::to_owned)
        })
        .collect();
    let mut inactive = children(group)
        .iter()
        .find(|n| number(n).is_some_and(|n| n != 1))
        .cloned()
        .unwrap_or_else(|| active[0].clone());
    if number(&inactive) == Some(1) {
        for field in ["fill", "stroke"] {
            inactive[field] = prev[0][field].clone();
        }
    }
    // The templates retain the authored typography, border, radius and colours.
    let spec = Spec {
        page: fields[0].clone(),
        size: fields[1].clone(),
        total: fields[2].clone(),
        pages: fields[3].clone(),
        start: fields[4].clone(),
        end: fields[5].clone(),
        filter_keys: keys,
        sizes: size_values,
        default_size,
        items_id: items_id.to_owned(),
        previous: set_event(
            prev[0].clone(),
            &fields[0],
            &format!("max(1, $app.{} - 1)", fields[0]),
        ),
        next: set_event(
            next[0].clone(),
            &fields[0],
            &format!("min($app.{}, $app.{} + 1)", fields[3], fields[0]),
        ),
        active: active[0].clone(),
        inactive,
    };
    let total = rows.len() as u32;
    let pages = total.div_ceil(default_size).max(1);
    let mut state = BTreeMap::new();
    for (key, default) in [
        (&fields[0], json!(1)),
        (&fields[1], json!(default_size.to_string())),
        (&fields[2], json!(total)),
        (&fields[3], json!(pages)),
        (&fields[4], json!(1)),
        (&fields[5], json!(total.min(default_size))),
    ] {
        state.insert(
            key.clone(),
            serde_json::from_value(
                json!({"type":if key==&fields[1]{"string"}else{"int"},"default":default}),
            )
            .expect("valid pager state"),
        );
    }
    sink.apply(EditorCommand::MergeAppState {
        plan_idx: usize::MAX,
        state,
    });
    patch(sink, group, json!({"children":buttons(&spec,1,pages)}));
    let explain = format!(
        "{}\n{MARKER}{}",
        table["explain"].as_str().unwrap_or(""),
        serde_json::to_string(&spec).unwrap()
    );
    patch(sink, table, json!({"explain":explain}));
    let expression = format!(
        "{} + $app.{} + '–' + $app.{} + {} + $app.{}",
        json!(if table["explain"]
            .as_str()
            .is_some_and(|s| s.contains("lang=zh"))
        {
            "当前记录 "
        } else {
            "Local records "
        }),
        fields[4],
        fields[5],
        json!(" / "),
        fields[2]
    );
    let mut bindings = counter["bindings"].as_object().cloned().unwrap_or_default();
    bindings.insert("content".into(), json!(expression));
    patch(sink, counter, json!({"bindings":bindings}));
    patch(
        sink,
        control,
        json!({"bindings":{"bind:value":format!("$state.{}",fields[1])},"events":{"onChange":[{"set":{format!("$app.{}",fields[0]):"1"}}]},"explain":marker(control,CONTROL_MARKER)}),
    );
    for control in controls {
        // Controls with custom handlers are preserved rather than spliced.
        if control["events"]
            .as_object()
            .is_none_or(|events| events.is_empty())
        {
            patch(
                sink,
                control,
                json!({"events":{"onChange":[{"set":{format!("$app.{}",fields[0]):"1"}}]}}),
            );
        }
    }
}
fn patch(sink: &mut dyn DocSink, node: &Value, value: Value) {
    if let Some(id) = node["id"].as_str() {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: value.to_string(),
            page_id: None,
        });
    }
}
fn owned_controls<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    if owned(value, CONTROL_MARKER) {
        out.push(value);
    }
    for child in children(value) {
        owned_controls(child, out);
    }
}
fn find_counter<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    if value["type"] == "text"
        && value["content"]
            .as_str()
            .is_some_and(|s| s.starts_with("Showing ") || s.starts_with("显示"))
    {
        out.push(value);
    }
    for child in children(value) {
        find_counter(child, out);
    }
}
