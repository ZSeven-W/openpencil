//! Connect unbound, closed-enum draft controls to an unambiguous authored table.

use crate::types::DocSink;
use op_editor_core::table_filter_contract::{CONTROL_MARKER, ROW_MARKER, TABLE_MARKER};
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn children(value: &Value) -> &[Value] {
    value["children"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect::<String>()
        .to_lowercase()
}
fn texts(value: &Value, out: &mut Vec<String>) {
    if value["type"] == "text" {
        if let Some(text) = value["content"].as_str() {
            let text = text.trim();
            if !text.is_empty() && !matches!(text, "•" | "·" | "●" | "✓") {
                out.push(text.to_owned());
            }
        }
    }
    for child in children(value) {
        texts(child, out);
    }
}
fn single_text(value: &Value) -> Option<String> {
    let mut out = vec![];
    texts(value, &mut out);
    (out.len() == 1).then(|| out[0].clone())
}
fn owned(value: &Value, marker: &str) -> bool {
    value["explain"]
        .as_str()
        .is_some_and(|s| s.starts_with(marker))
}
fn protected(value: &Value) -> bool {
    value["bindings"].as_object().is_some_and(|v| !v.is_empty())
        || value["events"].as_object().is_some_and(|v| !v.is_empty())
        || value.get("state").is_some()
}
fn find_tables<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    let name = format!(
        "{} {}",
        value["name"].as_str().unwrap_or(""),
        value["role"].as_str().unwrap_or("")
    )
    .to_lowercase();
    let kids = children(value);
    // This contract covers a bounded static page, not a database-sized dataset.
    if (name.contains("table") || name.contains("表格"))
        && value["layout"] == "vertical"
        && kids.len() >= 3
        && kids.len() <= 101
        && kids
            .iter()
            .all(|row| row["type"] == "frame" && row["layout"] == "horizontal")
    {
        let count = children(&kids[0]).len();
        if count >= 2 && kids.iter().all(|row| children(row).len() == count) {
            out.push(value);
            return;
        }
    }
    for child in kids {
        find_tables(child, out);
    }
}
fn selects<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    if value["type"] == "select" {
        out.push(value);
    }
    for child in children(value) {
        selects(child, out);
    }
}
fn kind(value: &str) -> Option<&'static str> {
    match normalized(value).as_str() {
        "status" | "状态" => Some("status"),
        "role" | "角色" => Some("role"),
        _ => None,
    }
}
fn select_kind(value: &Value) -> Option<&'static str> {
    let name = value["name"].as_str().unwrap_or("").to_lowercase();
    if name.contains("status") || name.contains("状态") {
        Some("status")
    } else if name.contains("role") || name.contains("角色") {
        Some("role")
    } else {
        None
    }
}
fn all_label(label: &str) -> bool {
    let label = label.trim().to_lowercase();
    label == "all"
        || label.starts_with("all ")
        || label.starts_with("全部")
        || label.starts_with("所有")
        || label == "不限"
}
fn marker(value: &Value, prefix: &str) -> String {
    format!("{prefix}\n{}", value["explain"].as_str().unwrap_or(""))
}
fn key(id: &str) -> String {
    format!(
        "op_filter_{}",
        id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

struct Filter<'a> {
    control: &'a Value,
    key: String,
    all: String,
    initial: String,
    options: Vec<Value>,
    row_values: Vec<String>,
}

fn candidate<'a>(
    control: &'a Value,
    column: usize,
    rows: &[Value],
    column_kind: &str,
    state_keys: &BTreeSet<String>,
) -> Option<Filter<'a>> {
    if protected(control) || owned(control, CONTROL_MARKER) {
        return None;
    }
    let id = control["id"].as_str()?;
    let key = key(id);
    if state_keys.contains(&key) {
        return None;
    }
    let mut options = control["options"].as_array()?.clone();
    if options.is_empty() {
        return None;
    }
    let option_values: Option<BTreeSet<_>> = options
        .iter()
        .map(|option| option["value"].as_str())
        .collect();
    if option_values
        .as_ref()
        .is_none_or(|values| values.len() != options.len())
    {
        return None;
    }
    if options
        .iter()
        .filter(|option| all_label(option["label"].as_str().unwrap_or("")))
        .count()
        > 1
    {
        return None;
    }
    let mut values = vec![];
    for row in rows {
        let text = single_text(&children(row)[column])?;
        let text = normalized(&text);
        let matches: Vec<_> = options
            .iter()
            .filter(|option| {
                !all_label(option["label"].as_str().unwrap_or(""))
                    && ["label", "value"].iter().any(|field| {
                        option[*field]
                            .as_str()
                            .is_some_and(|v| normalized(v) == text)
                    })
            })
            .collect();
        if matches.len() != 1 {
            return None;
        }
        values.push(matches[0]["value"].as_str()?.to_owned());
    }
    let all = if let Some(option) = options
        .iter()
        .find(|option| all_label(option["label"].as_str().unwrap_or("")))
    {
        option["value"].as_str()?.to_owned()
    } else {
        let all = "__op_all".to_owned();
        if options.iter().any(|option| option["value"] == all) {
            return None;
        }
        let chinese = control["name"]
            .as_str()
            .is_some_and(|s| s.contains("状态") || s.contains("角色"));
        options.insert(0,json!({"value":all,"label":if chinese{if column_kind=="status"{"全部状态"}else{"全部角色"}}else if column_kind=="status"{"All statuses"}else{"All roles"}}));
        all
    };
    let initial = control["value"]
        .as_str()
        .filter(|selected| *selected == all || values.iter().all(|v| v == selected))
        .unwrap_or(&all)
        .to_owned();
    Some(Filter {
        control,
        key,
        all,
        initial,
        options,
        row_values: values,
    })
}

pub(crate) fn wire(sink: &mut dyn DocSink) {
    let state_keys: BTreeSet<_> = sink
        .state()
        .doc
        .state
        .as_ref()
        .map(|state| state.keys().cloned().collect())
        .unwrap_or_default();
    let roots: Vec<Value> = sink
        .state()
        .active_children()
        .iter()
        .filter_map(|root| serde_json::to_value(root).ok())
        .collect();
    for root in roots {
        let mut tables = vec![];
        find_tables(&root, &mut tables);
        if tables.len() != 1 {
            continue;
        }
        let table = tables[0];
        if owned(table, TABLE_MARKER) {
            continue;
        }
        let Some(table_id) = table["id"].as_str() else {
            continue;
        };
        let rows = &children(table)[1..];
        if rows.iter().any(|row| {
            row["id"].as_str().is_none()
                || row["bindings"]["visible"].is_string()
                || row["visible"] == false
                || row.get("x").is_some()
                || row.get("y").is_some()
                || row.get("constraints").is_some()
        }) {
            continue;
        }
        let mut controls = vec![];
        selects(&root, &mut controls);
        let mut filters = vec![];
        for (column, header) in children(&children(table)[0]).iter().enumerate() {
            let Some(label) = single_text(header) else {
                continue;
            };
            let Some(column_kind) = kind(&label) else {
                continue;
            };
            let found: Vec<_> = controls
                .iter()
                .filter(|control| select_kind(control) == Some(column_kind))
                .collect();
            if found.len() == 1 {
                if let Some(filter) = candidate(found[0], column, rows, column_kind, &state_keys) {
                    filters.push(filter);
                }
            }
        }
        if filters.is_empty() {
            continue;
        }
        let unique_keys: BTreeSet<_> = filters.iter().map(|f| &f.key).collect();
        if unique_keys.len() != filters.len() {
            continue;
        }
        let definitions: BTreeMap<String, jian_ops_schema::state::StateEntry> = filters
            .iter()
            .map(|f| {
                (
                    f.key.clone(),
                    serde_json::from_value(json!({"type":"string","default":f.initial}))
                        .expect("valid filter state"),
                )
            })
            .collect();
        sink.apply(EditorCommand::MergeAppState {
            plan_idx: usize::MAX,
            state: definitions,
        });
        let mut conditions = vec![];
        for (index, row) in rows.iter().enumerate() {
            let Some(id) = row["id"].as_str() else {
                continue;
            };
            let condition = filters
                .iter()
                .map(|f| {
                    format!(
                        "($app.{} == {} || $app.{} == {})",
                        f.key,
                        json!(f.all),
                        f.key,
                        json!(f.row_values[index])
                    )
                })
                .collect::<Vec<_>>()
                .join(" && ");
            conditions.push(condition.clone());
            let mut bindings = row["bindings"].as_object().cloned().unwrap_or_default();
            bindings.insert("visible".into(), json!(condition));
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id),
                patch_json: json!({"bindings":bindings,"explain":marker(row,ROW_MARKER)})
                    .to_string(),
                page_id: None,
            });
        }
        for filter in filters {
            let Some(id) = filter.control["id"].as_str() else {
                continue;
            };
            sink.apply(EditorCommand::PatchNodeData{node_id:NodeId::new(id),patch_json:json!({"value":filter.initial,"options":filter.options,
                "bindings":{"bind:value":format!("$state.{}",filter.key)},"explain":marker(filter.control,CONTROL_MARKER)}).to_string(),page_id:None});
        }
        let chinese = children(&children(table)[0]).iter().any(|cell| {
            single_text(cell)
                .is_some_and(|s| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)))
        });
        sink.apply(EditorCommand::PatchNodeData{node_id:NodeId::new(table_id),patch_json:json!({"explain":marker(table,&format!("{TABLE_MARKER} lang={}",if chinese{"zh"}else{"en"}))}).to_string(),page_id:None});
        let count = conditions
            .iter()
            .map(|condition| format!("to_num({condition})"))
            .collect::<Vec<_>>()
            .join(" + ");
        wire_counter(
            sink,
            &root,
            &format!(
                "{} + ({count})",
                json!(if chinese {
                    "当前页数据："
                } else {
                    "Rows on this page: "
                })
            ),
        );
    }
}

fn wire_counter(sink: &mut dyn DocSink, value: &Value, expression: &str) {
    let name = value["name"].as_str().unwrap_or("").to_lowercase();
    if name.contains("pagination") || name.contains("分页") {
        let mut counters = vec![];
        find_counters(value, &mut counters);
        if counters.len() == 1 {
            let id = counters[0]["id"].as_str().unwrap();
            sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id),
                patch_json: json!({"bindings":{"content":expression}}).to_string(),
                page_id: None,
            });
        }
        return;
    }
    for child in children(value) {
        wire_counter(sink, child, expression);
    }
}

fn find_counters<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    if value["type"] == "text"
        && !protected(value)
        && value["content"]
            .as_str()
            .is_some_and(|s| s.starts_with("Showing ") || s.starts_with("显示"))
        && value["id"].is_string()
    {
        out.push(value);
    }
    for child in children(value) {
        find_counters(child, out);
    }
}

#[cfg(test)]
#[path = "table_filter_contract_tests.rs"]
mod tests;
