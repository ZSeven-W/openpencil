//! Search a bounded authored table without changing its canonical records.
use super::*;

pub(super) struct Search<'a> {
    control: &'a Value,
    pub(super) key: String,
    corpus: Vec<String>,
}

fn search_inputs<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    let hints = format!(
        "{} {}",
        value["name"].as_str().unwrap_or(""),
        value["placeholder"].as_str().unwrap_or("")
    )
    .to_lowercase();
    if value["type"] == "text_input" && (hints.contains("search") || hints.contains("搜索")) {
        out.push(value);
    }
    for child in children(value) {
        search_inputs(child, out);
    }
}

fn searchable_text(value: &Value, out: &mut Vec<String>, row: bool) -> Option<()> {
    if value["visible"] == false {
        return Some(());
    }
    // A static snapshot cannot truthfully search dynamically replaced copy.
    if (!row && value["bindings"]["visible"].is_string())
        || value["bindings"]["content"].is_string()
    {
        return None;
    }
    if value["type"] == "text" {
        let text = match serde_json::from_value::<jian_ops_schema::node::TextContent>(
            value["content"].clone(),
        )
        .ok()?
        {
            jian_ops_schema::node::TextContent::Plain(text) => text,
            jian_ops_schema::node::TextContent::Styled(spans) => {
                spans.into_iter().map(|span| span.text).collect()
            }
        };
        if !text.trim().is_empty() {
            out.push(text);
        }
    }
    for child in children(value) {
        searchable_text(child, out, false)?;
    }
    Some(())
}

pub(super) fn candidate<'a>(
    root: &'a Value,
    rows: &[Value],
    state_keys: &BTreeSet<String>,
) -> Option<Search<'a>> {
    let mut inputs = vec![];
    search_inputs(root, &mut inputs);
    if inputs.len() != 1 {
        return None;
    }
    let control = inputs[0];
    if protected(control)
        || owned(control, CONTROL_MARKER)
        || control["value"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    {
        return None;
    }
    let key = key(control["id"].as_str()?);
    if state_keys.contains(&key) {
        return None;
    }
    let corpus: Option<Vec<_>> = rows
        .iter()
        .map(|row| {
            let mut copy = vec![];
            searchable_text(row, &mut copy, true)?;
            (!copy.is_empty()).then(|| copy.join(" ").to_lowercase())
        })
        .collect();
    Some(Search {
        control,
        key,
        corpus: corpus?,
    })
}

pub(super) fn state_entry() -> jian_ops_schema::state::StateEntry {
    serde_json::from_value(json!({"type":"string","default":""})).expect("valid search state")
}

impl Search<'_> {
    pub(super) fn condition(&self, index: usize) -> String {
        format!(
            "contains({}, lower(trim($app.{})))",
            json!(self.corpus[index]),
            self.key
        )
    }
    pub(super) fn bind(&self, sink: &mut dyn DocSink) {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(self.control["id"].as_str().unwrap()),
            patch_json: json!({"bindings":{"bind:value":format!("$state.{}",self.key)},"explain":marker(self.control, CONTROL_MARKER)}).to_string(),page_id:None,
        });
    }
}

/// Older contract-owned drafts may gain search, but their existing predicates
/// remain part of the AND and custom counters are never overwritten.
pub(super) fn upgrade(
    sink: &mut dyn DocSink,
    root: &Value,
    table: &Value,
    state_keys: &BTreeSet<String>,
) {
    let rows = &children(table)[1..];
    if rows.is_empty()
        || rows
            .iter()
            .any(|row| !owned(row, ROW_MARKER) || !row["id"].is_string())
    {
        return;
    }
    let Some(search) = candidate(root, rows, state_keys) else {
        return;
    };
    let Some(previous): Option<Vec<String>> = rows
        .iter()
        .map(|row| row["bindings"]["visible"].as_str().map(str::to_owned))
        .collect()
    else {
        return;
    };
    let chinese = table["explain"]
        .as_str()
        .is_some_and(|s| s.contains("lang=zh"));
    let mut conditions = vec![];
    sink.apply(EditorCommand::MergeAppState {
        plan_idx: usize::MAX,
        state: BTreeMap::from([(search.key.clone(), state_entry())]),
    });
    for (index, row) in rows.iter().enumerate() {
        let condition = format!("({}) && ({})", previous[index], search.condition(index));
        let mut bindings = row["bindings"].as_object().cloned().unwrap_or_default();
        bindings.insert("visible".into(), json!(condition));
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(row["id"].as_str().unwrap()),
            patch_json: json!({"bindings":bindings}).to_string(),
            page_id: None,
        });
        conditions.push(condition);
    }
    search.bind(sink);
    wire_counter(
        sink,
        root,
        &counter_expression(&conditions, chinese),
        Some(&counter_expression(&previous, chinese)),
    );
}

#[cfg(test)]
#[path = "table_filter_search_tests.rs"]
mod tests;
