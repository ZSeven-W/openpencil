//! Marker-only table filtering semantics shared with the preview compositor.

use serde_json::{json, Value};

pub const ROW_MARKER: &str = "op-table-filter-row:v1";
pub const TABLE_MARKER: &str = "op-table-filter-table:v1";
pub const CONTROL_MARKER: &str = "op-table-filter-control:v1";

pub fn is_row(value: &Value) -> bool {
    value
        .get("explain")
        .and_then(Value::as_str)
        .is_some_and(|s| s.starts_with(ROW_MARKER))
}

pub fn row_ids(value: &Value, ids: &mut std::collections::BTreeSet<String>) {
    if is_row(value) {
        if let Some(id) = value["id"].as_str() {
            ids.insert(id.to_owned());
        }
    }
    if let Some(kids) = value.get("children").and_then(Value::as_array) {
        for child in kids {
            row_ids(child, ids);
        }
    }
    if let Some(pages) = value.get("pages").and_then(Value::as_array) {
        for page in pages {
            row_ids(page, ids);
        }
    }
}

/// Remove only contract-owned rows whose evaluated visibility is false. The
/// caller operates on a derived clone, keeping the canonical dataset intact.
pub fn materialize(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let table = object
                .get("explain")
                .and_then(Value::as_str)
                .is_some_and(|s| s.starts_with(TABLE_MARKER));
            let id = object
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("table")
                .to_owned();
            let chinese = object
                .get("explain")
                .and_then(Value::as_str)
                .is_some_and(|s| s.contains("lang=zh"));
            if let Some(kids) = object.get_mut("children").and_then(Value::as_array_mut) {
                for child in kids.iter_mut() {
                    materialize(child);
                }
                kids.retain(|child| !is_row(child) || child["visible"] != false);
                if table
                    && !kids.iter().any(is_row)
                    && !kids
                        .iter()
                        .any(|child| child["id"] == format!("{id}-filter-empty"))
                {
                    kids.push(json!({"type":"frame","id":format!("{id}-filter-empty"),"name":"Empty filter result",
                        "width":"fill_container","height":72,"layout":"vertical","gap":4,"justifyContent":"center","alignItems":"center",
                        "children":[{"type":"text","id":format!("{id}-filter-empty-label"),"content":if chinese{"没有匹配的数据"}else{"No matching rows"},
                            "fontSize":14,"width":"fill_container","height":"fit_content","textAlign":"center","fill":[{"type":"solid","color":"$--muted-foreground"}]},
                            {"type":"text","id":format!("{id}-filter-empty-hint"),"content":if chinese{"调整筛选条件后再试"}else{"Try adjusting the filters."},
                            "fontSize":12,"width":"fill_container","height":"fit_content","textAlign":"center","fill":[{"type":"solid","color":"$--muted-foreground"}]}]}));
                }
            }
            if let Some(pages) = object.get_mut("pages").and_then(Value::as_array_mut) {
                for page in pages {
                    materialize(page);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                materialize(child);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pruning_is_scoped_and_empty_state_is_idempotent() {
        let mut value = json!({"id":"table","explain":format!("{TABLE_MARKER} lang=zh"),"children":[
            {"id":"header"},{"id":"authored-hidden","visible":false},
            {"id":"row","explain":ROW_MARKER,"visible":false}
        ]});
        materialize(&mut value);
        let children = value["children"].as_array().unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[1]["id"], "authored-hidden");
        assert_eq!(children[2]["children"][0]["content"], "没有匹配的数据");
        let settled = value.clone();
        materialize(&mut value);
        assert_eq!(value, settled);
    }
}
