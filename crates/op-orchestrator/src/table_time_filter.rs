//! Time filtering requires an explicit document snapshot and per-row timestamps.
use super::*;
const CLOCK: &str = "op-table-clock:v1 ";
const TIME: &str = "op-table-time:v1 ";
const UNAVAILABLE: &str = "op-table-time-unavailable:v1 ";
fn metadata(value: &Value, marker: &str) -> Option<Value> {
    serde_json::from_str(
        value["explain"]
            .as_str()?
            .lines()
            .find_map(|line| line.strip_prefix(marker))?,
    )
    .ok()
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
fn unavailable(sink: &mut dyn DocSink, control: &Value, chinese: bool) {
    if metadata(control, UNAVAILABLE).is_some() || control["enabled"] == false {
        return;
    }
    let original = json!({"value":control["value"],"enabled":control["enabled"],"opacity":control["opacity"],"placeholder":control["placeholder"]});
    let explain = format!(
        "{}\n{UNAVAILABLE}{original}",
        control["explain"].as_str().unwrap_or("")
    );
    patch(
        sink,
        control,
        json!({"value":null,"enabled":false,"opacity":control["opacity"].as_f64().unwrap_or(1.0)*0.55,"placeholder":if chinese{"日期未提供"}else{"Dates unavailable"},"explain":explain}),
    );
}
fn duration(value: &str) -> Option<u64> {
    if !value.is_ascii() {
        return None;
    }
    let (n, unit) = value.split_at(value.len().checked_sub(1)?);
    let n = n.parse::<u64>().ok()?;
    n.checked_mul(match unit {
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return None,
    })
}
fn controls<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    let name = value["name"].as_str().unwrap_or("").to_lowercase();
    if value["type"] == "select"
        && (name.contains("last active")
            || name.contains("last activity")
            || name.contains("最近活跃"))
    {
        out.push(value);
    }
    for child in children(value) {
        controls(child, out);
    }
}
pub(super) fn wire(sink: &mut dyn DocSink, root: &Value, table: &Value) {
    if !owned(table, TABLE_MARKER) {
        return;
    }
    let mut found = vec![];
    controls(root, &mut found);
    if found.len() != 1 || protected(found[0]) {
        return;
    }
    let control = found[0];
    let chinese = table["explain"]
        .as_str()
        .is_some_and(|s| s.contains("lang=zh"))
        || control["name"].as_str().is_some_and(|s| s.contains("活跃"));
    if control["enabled"] == false && metadata(control, UNAVAILABLE).is_none() {
        return;
    }
    let rows = &children(table)[1..];
    if rows.is_empty() || !rows.iter().all(|row| owned(row, ROW_MARKER)) {
        return;
    }
    let Some(reference) = metadata(table, CLOCK).and_then(|v| v["reference_ms"].as_u64()) else {
        unavailable(sink, control, chinese);
        return;
    };
    let Some(times): Option<Vec<Option<u64>>> = rows
        .iter()
        .map(|row| {
            let meta = metadata(row, TIME)?;
            let value = meta.get("last_active_ms")?;
            if value.is_null() {
                Some(None)
            } else {
                value.as_u64().filter(|n| *n <= reference).map(Some)
            }
        })
        .collect()
    else {
        unavailable(sink, control, chinese);
        return;
    };
    let Some(id) = control["id"].as_str() else {
        return;
    };
    let key = key(id);
    if sink
        .state()
        .doc
        .state
        .as_ref()
        .is_some_and(|s| s.contains_key(&key))
    {
        return;
    }
    let Some(mut options) = control["options"].as_array().cloned() else {
        return;
    };
    let values: Option<BTreeSet<_>> = options.iter().map(|o| o["value"].as_str()).collect();
    if values.as_ref().is_none_or(|v| v.len() != options.len()) {
        return;
    }
    let mut choices = vec![];
    let mut all = None;
    for option in &options {
        let value = option["value"].as_str().unwrap();
        if all_label(option["label"].as_str().unwrap_or("")) {
            if all.replace(value.to_owned()).is_some() {
                return;
            }
        } else {
            let Some(ms) = duration(value) else {
                return;
            };
            choices.push((value.to_owned(), ms));
        }
    }
    if choices.is_empty() {
        return;
    }
    let all = all.unwrap_or_else(|| "__op_all".to_owned());
    if !options
        .iter()
        .any(|o| all_label(o["label"].as_str().unwrap_or("")))
    {
        if options.iter().any(|o| o["value"] == all) {
            return;
        }
        options.insert(
            0,
            json!({"value":all,"label":if chinese{"全部时间"}else{"All time"}}),
        );
    }
    let Some(previous): Option<Vec<String>> = rows
        .iter()
        .map(|row| row["bindings"]["visible"].as_str().map(str::to_owned))
        .collect()
    else {
        return;
    };
    sink.apply(EditorCommand::MergeAppState {
        plan_idx: usize::MAX,
        state: BTreeMap::from([(
            key.clone(),
            serde_json::from_value(json!({"type":"string","default":all})).unwrap(),
        )]),
    });
    let mut conditions = vec![];
    for (index, row) in rows.iter().enumerate() {
        let mut matches = vec![format!("$app.{key} == {}", json!(all))];
        for (value, ms) in &choices {
            if times[index].is_some_and(|at| reference - at <= *ms) {
                matches.push(format!("$app.{key} == {}", json!(value)));
            }
        }
        let condition = format!("({}) && ({})", previous[index], matches.join(" || "));
        let mut bindings = row["bindings"].as_object().cloned().unwrap_or_default();
        bindings.insert("visible".into(), json!(condition));
        patch(sink, row, json!({"bindings":bindings}));
        conditions.push(condition);
    }
    let mut update = json!({"value":all,"options":options,"bindings":{"bind:value":format!("$state.{key}")},"explain":marker(control,CONTROL_MARKER)});
    // A previously unavailable control recovers when complete source data arrives.
    if let Some(original) = metadata(control, UNAVAILABLE) {
        for field in ["enabled", "opacity", "placeholder"] {
            update[field] = original[field].clone();
        }
    }
    if let Some(mut paging) = op_editor_core::table_pagination::Spec::from_node(table) {
        update["events"] = json!({"onChange":[{"set":{format!("$app.{}",paging.page):"1"}}]});
        paging.filter_keys.push(key.clone());
        let explain = table["explain"]
            .as_str()
            .unwrap_or("")
            .lines()
            .map(|line| {
                if line.starts_with(op_editor_core::table_pagination::MARKER) {
                    format!(
                        "{}{}",
                        op_editor_core::table_pagination::MARKER,
                        serde_json::to_string(&paging).unwrap()
                    )
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        patch(sink, table, json!({"explain":explain}));
    }
    patch(sink, control, update);
    let chinese = table["explain"]
        .as_str()
        .is_some_and(|s| s.contains("lang=zh"));
    wire_counter(
        sink,
        root,
        &counter_expression(&conditions, chinese),
        Some(&counter_expression(&previous, chinese)),
    );
}
