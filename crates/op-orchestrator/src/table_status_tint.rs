//! Correct a copied status tint using the badge's own authored dot colour.
use super::*;

fn solid(value: &Value) -> Option<&Value> {
    let fills = value["fill"].as_array()?;
    (fills.len() == 1 && fills[0]["type"] == "solid").then(|| &fills[0])
}
fn hex(value: &str) -> Option<String> {
    let raw = value.strip_prefix('#')?;
    (raw.is_ascii() && matches!(raw.len(), 6 | 8) && raw.bytes().all(|c| c.is_ascii_hexdigit()))
        .then(|| value.to_uppercase())
}
struct Badge<'a> {
    node: &'a Value,
    tint: String,
    dot: String,
}
fn badge(value: &Value) -> Option<Badge<'_>> {
    let name = value["name"].as_str()?.to_lowercase();
    if value["type"] != "frame"
        || !name.contains("status")
        || !(name.contains("pill") || name.contains("badge") || name.contains("chip"))
        || protected(value)
    {
        return None;
    }
    let kids = children(value);
    if kids.len() != 2 || kids.iter().filter(|c| c["type"] == "text").count() != 1 {
        return None;
    }
    let dot = kids.iter().find(|child| child["type"] == "ellipse")?;
    if protected(dot) {
        return None;
    }
    let dot = hex(solid(dot)?["color"].as_str()?)?;
    if dot.len() != 7 {
        return None;
    }
    let fill = solid(value)?;
    let tint = hex(fill["color"].as_str()?)?;
    if tint.len() != 9 || fill["blendMode"].as_str().is_some_and(|s| s != "normal") {
        return None;
    }
    let alpha = u8::from_str_radix(&tint[7..9], 16).ok()?;
    if !(1..=64).contains(&alpha) {
        return None;
    }
    value["id"].as_str()?;
    Some(Badge {
        node: value,
        tint,
        dot,
    })
}
fn collect<'a>(value: &'a Value, out: &mut Vec<Badge<'a>>) {
    if let Some(badge) = badge(value) {
        out.push(badge);
        return;
    }
    for child in children(value) {
        collect(child, out);
    }
}
pub(super) fn repair(sink: &mut dyn DocSink, table: &Value) {
    let mut badges = vec![];
    collect(table, &mut badges);
    if badges.len() < 3 {
        return;
    }
    let shared = &badges[0].tint;
    // Neutral, intentionally varied, and custom-themed surfaces are preserved.
    if !badges.iter().all(|b| b.tint == *shared)
        || !badges.iter().any(|b| b.dot == shared[..7])
        || badges.iter().map(|b| &b.dot).collect::<BTreeSet<_>>().len() < 2
    {
        return;
    }
    for badge in badges {
        if badge.dot == badge.tint[..7] {
            continue;
        }
        let mut fill = badge.node["fill"].clone();
        fill[0]["color"] = json!(format!("{}{}", badge.dot, &badge.tint[7..9]));
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(badge.node["id"].as_str().unwrap()),
            patch_json: json!({"fill":fill}).to_string(),
            page_id: None,
        });
    }
}

#[cfg(test)]
#[path = "table_status_tint_tests.rs"]
mod tests;
