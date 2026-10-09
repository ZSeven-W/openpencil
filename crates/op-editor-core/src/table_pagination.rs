//! Local pagination over contract-owned records, retaining the canonical data.
use crate::table_filter_contract::is_row;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MARKER: &str = "op-table-paging:v1 ";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub page: String,
    pub size: String,
    pub total: String,
    pub pages: String,
    pub start: String,
    pub end: String,
    pub filter_keys: Vec<String>,
    pub sizes: Vec<u32>,
    pub default_size: u32,
    pub items_id: String,
    pub previous: Value,
    pub next: Value,
    pub active: Value,
    pub inactive: Value,
}
impl Spec {
    pub fn from_node(node: &Value) -> Option<Self> {
        let line = node["explain"]
            .as_str()?
            .lines()
            .find_map(|line| line.strip_prefix(MARKER))?;
        let spec: Self = serde_json::from_str(line).ok()?;
        (!spec.sizes.is_empty()
            && spec.sizes.iter().all(|n| (5..=100).contains(n))
            && spec.sizes.contains(&spec.default_size))
        .then_some(spec)
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.page.as_str())
            .chain(std::iter::once(self.size.as_str()))
            .chain(self.filter_keys.iter().map(String::as_str))
    }
}
pub fn specs(value: &Value, out: &mut Vec<Spec>) {
    if let Some(spec) = Spec::from_node(value) {
        out.push(spec);
    }
    for field in ["children", "pages"] {
        if let Some(kids) = value[field].as_array() {
            for child in kids {
                specs(child, out);
            }
        }
    }
}
fn remap_ids(value: &mut Value, suffix: &str) {
    if let Some(id) = value["id"].as_str() {
        value["id"] = json!(format!("{id}{suffix}"));
    }
    if let Some(kids) = value["children"].as_array_mut() {
        for child in kids {
            remap_ids(child, suffix);
        }
    }
}
fn label(value: &mut Value, copy: &str) {
    if value["type"] == "text" {
        value["content"] = json!(copy);
    }
    if let Some(kids) = value["children"].as_array_mut() {
        for child in kids {
            label(child, copy);
        }
    }
}
pub fn buttons(spec: &Spec, page: u32, pages: u32) -> Vec<Value> {
    let mut previous = spec.previous.clone();
    previous["enabled"] = json!(page > 1);
    previous["opacity"] =
        json!(spec.previous["opacity"].as_f64().unwrap_or(1.0) * if page > 1 { 1.0 } else { 0.35 });
    let mut next = spec.next.clone();
    next["enabled"] = json!(page < pages);
    next["opacity"] =
        json!(spec.next["opacity"].as_f64().unwrap_or(1.0) * if page < pages { 1.0 } else { 0.35 });
    let mut out = vec![previous];
    let mut last = 0;
    for number in
        (1..=pages).filter(|n| pages <= 7 || *n == 1 || *n == pages || n.abs_diff(page) <= 1)
    {
        if last > 0 && number > last + 1 {
            let mut gap = spec.inactive.clone();
            remap_ids(&mut gap, &format!("-local-gap-{last}"));
            label(&mut gap, "…");
            gap["enabled"] = json!(false);
            gap.as_object_mut().unwrap().remove("events");
            out.push(gap);
        }
        let mut button = if number == page {
            spec.active.clone()
        } else {
            spec.inactive.clone()
        };
        remap_ids(&mut button, &format!("-local-page-{number}"));
        label(&mut button, &number.to_string());
        button["id"] = json!(format!("{}-local-page-{number}", spec.items_id));
        button["name"] = json!(format!("Page {number} Button"));
        button["enabled"] = json!(true);
        button["events"] =
            json!({"onTap":[{"set":{format!("$app.{}",spec.page):number.to_string()}}]});
        out.push(button);
        last = number;
    }
    out.push(next);
    out
}
fn set_children(value: &mut Value, id: &str, kids: &[Value]) {
    if value["id"] == id {
        value["children"] = json!(kids);
        return;
    }
    for field in ["children", "pages"] {
        if let Some(children) = value[field].as_array_mut() {
            for child in children {
                set_children(child, id, kids);
            }
        }
    }
}
/// Call after enum/search visibility has been materialized. Computed values
/// are returned to the host's state graph, never persisted into the dataset.
pub fn materialize(
    value: &mut Value,
    get: &impl Fn(&str) -> Option<Value>,
) -> Vec<(String, Value)> {
    fn walk(
        value: &mut Value,
        get: &impl Fn(&str) -> Option<Value>,
        updates: &mut Vec<(String, Value)>,
        pagers: &mut Vec<(String, Vec<Value>)>,
    ) {
        if let Some(spec) = Spec::from_node(value) {
            if let Some(kids) = value["children"].as_array_mut() {
                let total = kids.iter().filter(|child| is_row(child)).count() as u32;
                let requested = get(&spec.size)
                    .and_then(|v| {
                        v.as_str()
                            .and_then(|s| s.parse::<u32>().ok())
                            .or_else(|| v.as_u64().and_then(|n| u32::try_from(n).ok()))
                    })
                    .unwrap_or(spec.default_size);
                let size = if spec.sizes.contains(&requested) {
                    requested
                } else {
                    spec.default_size
                };
                let pages = total.div_ceil(size).max(1);
                let page = get(&spec.page)
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(1)
                    .clamp(1, pages);
                let offset = (page - 1) * size;
                let mut index = 0;
                kids.retain(|child| {
                    if !is_row(child) {
                        return true;
                    }
                    let keep = index >= offset && index < offset + size;
                    index += 1;
                    keep
                });
                for (key, number) in [
                    (&spec.page, page),
                    (&spec.total, total),
                    (&spec.pages, pages),
                    (&spec.start, if total == 0 { 0 } else { offset + 1 }),
                    (&spec.end, (offset + size).min(total)),
                ] {
                    updates.push((key.clone(), json!(number)));
                }
                updates.push((spec.size.clone(), json!(size.to_string())));
                pagers.push((spec.items_id.clone(), buttons(&spec, page, pages)));
            }
        }
        for field in ["children", "pages"] {
            if let Some(kids) = value[field].as_array_mut() {
                for child in kids {
                    walk(child, get, updates, pagers);
                }
            }
        }
    }
    let mut updates = vec![];
    let mut pagers = vec![];
    walk(value, get, &mut updates, &mut pagers);
    for (id, kids) in pagers {
        set_children(value, &id, &kids);
    }
    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_button_ids_stay_stable_when_selection_changes() {
        let button = |id: &str| json!({"type":"frame","id":id,"opacity":0.8,"children":[{"type":"text","id":format!("{id}-label"),"content":"1"}]});
        let spec = Spec {
            page: "page".into(),
            size: "size".into(),
            total: "total".into(),
            pages: "pages".into(),
            start: "start".into(),
            end: "end".into(),
            filter_keys: vec![],
            sizes: vec![10],
            default_size: 10,
            items_id: "pager".into(),
            previous: button("prev"),
            next: button("next"),
            active: button("selected"),
            inactive: button("normal"),
        };
        let a = buttons(&spec, 1, 3);
        let b = buttons(&spec, 2, 3);
        assert_eq!(a[2]["id"], b[2]["id"]);
        assert_eq!(a[2]["children"][0]["content"], "2");
        assert_eq!(b[0]["opacity"], 0.8);
        assert_eq!(a[0]["enabled"], false);
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    #[test]
    fn large_page_counts_keep_a_bounded_window_and_real_last_page() {
        let button = |id: &str| json!({"type":"frame","id":id,"children":[{"type":"text","id":format!("{id}-label"),"content":"1"}]});
        let spec = Spec {
            page: "p".into(),
            size: "s".into(),
            total: "t".into(),
            pages: "ps".into(),
            start: "a".into(),
            end: "b".into(),
            filter_keys: vec![],
            sizes: vec![5],
            default_size: 5,
            items_id: "pager".into(),
            previous: button("prev"),
            next: button("next"),
            active: button("active"),
            inactive: button("idle"),
        };
        let nodes = buttons(&spec, 10, 20);
        assert!(nodes.len() <= 9);
        for number in [1, 9, 10, 11, 20] {
            assert!(nodes
                .iter()
                .any(|v| v["id"] == format!("pager-local-page-{number}")));
        }
        assert_eq!(
            nodes
                .iter()
                .filter(|v| v["children"][0]["content"] == "…")
                .count(),
            2
        );
    }
}
