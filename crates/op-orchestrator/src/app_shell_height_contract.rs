//! Definite height through desktop shell wrappers and a stable pagination footer.

use super::*;
use crate::types::DocSink;
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId};

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn has_table(v: &Value) -> bool {
    is_named_data_table(v) || children(v).iter().any(has_table)
}

fn data_table(v: &Value) -> Option<&Value> {
    if is_named_data_table(v) {
        Some(v)
    } else {
        children(v).iter().find_map(data_table)
    }
}

fn padding(v: &Value) -> Option<[f64; 4]> {
    match v.get("padding")? {
        Value::Number(n) => Some([n.as_f64()?; 4]),
        Value::Array(a) if a.len() == 2 => Some([
            a[0].as_f64()?,
            a[1].as_f64()?,
            a[0].as_f64()?,
            a[1].as_f64()?,
        ]),
        Value::Array(a) if a.len() == 4 => Some([
            a[0].as_f64()?,
            a[1].as_f64()?,
            a[2].as_f64()?,
            a[3].as_f64()?,
        ]),
        _ => None,
    }
}

/// Near-fit tables spend row whitespace before requiring a scroll. Typography,
/// authored row heights and large/intentional overflow remain untouched.
fn compact_near_fit_table(sink: &mut dyn DocSink, section: &Value, viewport_id: &str) -> bool {
    let Some(table) = data_table(section) else {
        return false;
    };
    if num(table, "height").is_some() {
        return false;
    }
    let Some(table_id) = table.get("id").and_then(Value::as_str) else {
        return false;
    };
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return false;
    };
    let (Some(vp), Some(body)) = (page.find(viewport_id), page.find(table_id)) else {
        return false;
    };
    let deficit =
        (body.bounds.origin.y + body.bounds.size.y - vp.bounds.origin.y - vp.bounds.size.y) as f64
            + 1.0;
    if deficit <= 1.0 || deficit > vp.bounds.size.y as f64 * 0.15 {
        return false;
    }
    let mut rows = Vec::new();
    for row in children(table) {
        if layout_str(row) != Some("horizontal")
            || ident_text(row).contains("header")
            || num(row, "height").is_some()
            || row.get("minHeight").is_some()
        {
            continue;
        }
        let (Some(id), Some(pad)) = (row.get("id").and_then(Value::as_str), padding(row)) else {
            continue;
        };
        let Some(rect) = page.find(id) else {
            continue;
        };
        let floor = 6.0_f64.max((44.0 - (rect.bounds.size.y as f64 - pad[0] - pad[2])) / 2.0);
        let top = (pad[0] - floor).max(0.0);
        let bottom = (pad[2] - floor).max(0.0);
        let slack = (top + bottom).min(rect.bounds.size.y as f64 * 0.20);
        if slack > 0.0 {
            rows.push((id, pad, top, bottom, slack));
        }
    }
    let available: f64 = rows.iter().map(|r| r.4).sum();
    if available < deficit {
        return false;
    }
    let share = deficit / available;
    let mut changed = false;
    for (id, mut pad, top, bottom, slack) in rows {
        let reduction = share * slack;
        pad[0] -= reduction * top / (top + bottom);
        pad[2] -= reduction * bottom / (top + bottom);
        changed |= sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: json!({"padding":pad}).to_string(),
            page_id: None,
        });
    }
    changed
}

fn shell(v: &Value) -> Option<(&Value, &Value)> {
    let kids = children(v);
    if num(v, "width")? >= DESKTOP_MIN_WIDTH
        && num(v, "height")? >= 500.0
        && layout_str(v) == Some("horizontal")
        && kids.len() == 2
        && is_narrow_sidebar_column(&kids[0])
        && layout_str(&kids[0]) == Some("vertical")
        && layout_str(&kids[1]) == Some("vertical")
        && !is_sidebar_named(&ident_text(&kids[1]))
        && kids[1].get("role").and_then(Value::as_str) != Some("sidebar")
        && has_table(&kids[1])
    {
        Some((&kids[0], &kids[1]))
    } else {
        None
    }
}

fn sidebar_spine(sidebar: &Value) -> Vec<&Value> {
    let mut spine = vec![sidebar];
    let mut node = sidebar;
    while children(node).len() == 1 {
        let child = &children(node)[0];
        if child.get("type").and_then(Value::as_str) != Some("frame")
            || layout_str(child) != Some("vertical")
            || child.get("constraints").is_some()
        {
            break;
        }
        spine.push(child);
        node = child;
    }
    if is_sidebar_named(&ident_text(node))
        || node.get("height").and_then(Value::as_str) == Some("fill_container")
    {
        spine
    } else {
        vec![sidebar]
    }
}

fn table_and_footer(main: &Value) -> Option<(usize, &Value)> {
    let kids = children(main);
    let footer = kids.last()?;
    let text = ident_text(footer);
    if !text.contains("pagination")
        && !text.contains("分页")
        && footer.get("role").and_then(Value::as_str) != Some("pagination")
    {
        return None;
    }
    let tables: Vec<_> = kids[..kids.len() - 1]
        .iter()
        .enumerate()
        .filter(|(_, n)| has_table(n))
        .collect();
    (tables.len() == 1).then(|| (tables[0].0, footer))
}

fn padding_bottom(v: &Value) -> f64 {
    match v.get("padding") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::Array(a)) if a.len() == 2 => a[0].as_f64().unwrap_or(0.0),
        Some(Value::Array(a)) if a.len() == 4 => a[2].as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

pub(crate) fn repair_height_contract(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) =
        op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(root_id))
    else {
        return false;
    };
    let Ok(value) = serde_json::to_value(root) else {
        return false;
    };
    let Some((sidebar, main)) = shell(&value) else {
        return false;
    };
    let Some(main_id) = main.get("id").and_then(Value::as_str) else {
        return false;
    };
    let mut changed = false;
    for n in sidebar_spine(sidebar)
        .into_iter()
        .chain(std::iter::once(main))
    {
        if n.get("height").and_then(Value::as_str) != Some("fill_container") {
            if let Some(id) = n.get("id").and_then(Value::as_str) {
                changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(id),
                    property: "height".into(),
                    value: LayoutPropValue::Keyword("fill_container".into()),
                });
            }
        }
    }
    let Some((index, _)) = table_and_footer(main) else {
        return changed;
    };
    let table = &children(main)[index];
    // The generated table is kept intact. Only its viewport takes the spare
    // height, so short tables leave breathing room and long tables scroll.
    if table.get("role").and_then(Value::as_str) == Some("scroll-area") {
        if table.get("height").and_then(Value::as_str) != Some("fill_container") {
            if let Some(id) = table.get("id").and_then(Value::as_str) {
                changed |= sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(id),
                    property: "height".into(),
                    value: LayoutPropValue::Keyword("fill_container".into()),
                });
            }
        }
        if let Some(id) = table.get("id").and_then(Value::as_str) {
            changed |= compact_near_fit_table(sink, table, id);
        }
        return changed;
    }
    let Some(table_id) = table.get("id").and_then(Value::as_str) else {
        return changed;
    };
    let viewport = serde_json::from_value(json!({
        "type":"frame","id":"desktop-table-viewport","name":"Table viewport","role":"scroll-area",
        "width":"fill_container","height":"fill_container","layout":"vertical","clipContent":true,
        "events":{"onScroll":[{"delay":{"ms":0}}]},"children":[]
    }))
    .expect("valid table viewport");
    let Some(ids) = sink.insert_subtree_returning_root_ids(vec![viewport], &NodeId::new(main_id))
    else {
        return changed;
    };
    let Some(viewport_id) = ids.first() else {
        return changed;
    };
    sink.apply(EditorCommand::MoveNode {
        node_id: NodeId::new(viewport_id),
        target_parent: NodeId::new(main_id),
        page_id: None,
        index: Some(index),
    });
    sink.apply(EditorCommand::MoveNode {
        node_id: NodeId::new(table_id),
        target_parent: NodeId::new(viewport_id),
        page_id: None,
        index: Some(0),
    });
    compact_near_fit_table(sink, table, viewport_id);
    true
}

pub(crate) fn height_contract_diagnostics<F>(root: &Value, bounds: F, out: &mut Vec<String>)
where
    F: Fn(&str) -> Option<(f64, f64, f64, f64)> + Copy,
{
    let Some((sidebar, main)) = shell(root) else {
        return;
    };
    let Some((_, y, _, h)) = root.get("id").and_then(Value::as_str).and_then(bounds) else {
        return;
    };
    let spine = sidebar_spine(sidebar);
    let surface = spine.last().unwrap();
    let shell_bottom = y + h - padding_bottom(root);
    let expected = shell_bottom
        - spine[..spine.len() - 1]
            .iter()
            .map(|n| padding_bottom(n))
            .sum::<f64>();
    if let Some((_, sy, _, sh)) = surface.get("id").and_then(Value::as_str).and_then(bounds) {
        if (sy + sh - expected).abs() > 4.0 {
            out.push(format!("desktop-shell-sidebar-height: {} ends at {:.1}px instead of {:.1}px; carry the shell height through section wrappers", ident_text(surface), sy + sh, expected));
        }
    }
    if let Some((_, footer)) = table_and_footer(main) {
        if let Some((_, fy, _, fh)) = footer.get("id").and_then(Value::as_str).and_then(bounds) {
            // A hugging minimum can grow the main column beyond its fixed
            // board. That larger resolved height must not redefine the footer
            // contract and hide an off-board pagination bar from diagnostics.
            let bottom = shell_bottom - padding_bottom(main);
            if (fy + fh - bottom).abs() > 4.0 {
                out.push(format!("desktop-shell-pagination-bottom: {} ends at {:.1}px instead of {:.1}px; keep the table in a flexible middle viewport", ident_text(footer), fy + fh, bottom));
            }
        }
    }
}

#[cfg(test)]
#[path = "app_shell_height_contract_tests.rs"]
mod tests;
