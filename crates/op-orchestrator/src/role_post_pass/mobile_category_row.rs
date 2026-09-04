//! Mobile category-row / category-section normalizers plus the icon-tile
//! styling they apply.

use super::*;

pub(super) fn is_category_row_label(label: &str) -> bool {
    label.contains("category")
        || label.contains("categories")
        || label.contains("cuisine")
        || label.contains("分类")
        || label.contains("品类")
}

pub(super) fn is_chip_like_child(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    matches!(
        role_of(node),
        Some("chip") | Some("tag") | Some("pill") | Some("button")
    ) || {
        let label = semantic_label(node);
        label.contains("chip") || label.contains("category") || label.contains("类别")
    }
}

pub(super) fn has_descendant_type(node: &Value, type_name: &str) -> bool {
    node.get("type").and_then(Value::as_str) == Some(type_name)
        || node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| {
                children
                    .iter()
                    .any(|child| has_descendant_type(child, type_name))
            })
            .unwrap_or(false)
}

pub(super) fn has_text_descendant(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("text")
        && text_content(node)
            .map(|content| !content.trim().is_empty())
            .unwrap_or(false)
        || node
            .get("children")
            .and_then(Value::as_array)
            .map(|children| children.iter().any(has_text_descendant))
            .unwrap_or(false)
}

pub(super) fn is_icon_label_category_item(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("frame")
        && has_descendant_type(node, "icon_font")
        && has_text_descendant(node)
        // A product/travel card commonly contains a favorite icon over a
        // photo plus text. That does not make the whole card an icon+label
        // category chip; treating it as one shrank 140px photos to 56px tiles.
        && !has_descendant_type(node, "image")
}

pub(super) fn is_category_item_like_child(node: &Value) -> bool {
    is_chip_like_child(node) || is_icon_label_category_item(node)
}

pub(super) fn has_loose_category_spacing(node: &Value, canvas_width: f64) -> bool {
    numeric_prop(node, "width")
        .map(|width| width > canvas_width)
        .unwrap_or(false)
        || matches!(
            node.get("justifyContent").and_then(Value::as_str),
            Some("space_between") | Some("space_around")
        )
        || numeric_prop(node, "gap")
            .map(|gap| gap > 48.0)
            .unwrap_or(false)
}

pub(super) fn category_item_row_count(node: &Value) -> Option<usize> {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return None;
    }
    let children = node.get("children").and_then(Value::as_array)?;
    let item_count = children
        .iter()
        .filter(|child| is_category_item_like_child(child))
        .count();
    if item_count < 2 {
        return None;
    }
    if children
        .iter()
        .any(|child| !is_category_item_like_child(child))
    {
        return None;
    }
    Some(item_count)
}

pub(super) fn should_normalize_mobile_category_row(node: &Value, canvas_width: f64) -> bool {
    let Some(item_count) = category_item_row_count(node) else {
        return false;
    };
    let loose_spacing = has_loose_category_spacing(node, canvas_width);
    let explicitly_category = is_category_row_label(&semantic_label(node))
        || node
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| {
                children.iter().all(|child| {
                    matches!(role_of(child), Some("chip") | Some("tag") | Some("pill"))
                        || is_category_row_label(&semantic_label(child))
                })
            });
    if !explicitly_category {
        return false;
    }
    item_count >= 4 || loose_spacing
}

pub(super) fn should_normalize_mobile_category_row_in_section(node: &Value) -> bool {
    category_item_row_count(node).is_some()
}

pub(super) fn category_section_has_direct_item_row(node: &Value, canvas_width: f64) -> bool {
    node.get("children")
        .and_then(Value::as_array)
        .map(|children| {
            children.iter().any(|child| {
                should_normalize_mobile_category_row(child, canvas_width)
                    || should_normalize_mobile_category_row_in_section(child)
            })
        })
        .unwrap_or(false)
}

pub(super) fn normalize_mobile_category_section(node: &mut Value, canvas_width: f64) {
    if canvas_width > 480.0
        || node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("vertical")
        || !is_category_row_label(&semantic_label(node))
        || !category_section_has_direct_item_row(node, canvas_width)
    {
        return;
    }

    node["height"] = json!("fit_content");
    node["clipContent"] = json!(false);
    if node.get("gap").is_none()
        || numeric_prop(node, "gap")
            .map(|gap| gap > 12.0)
            .unwrap_or(false)
    {
        node["gap"] = json!(12);
    }

    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            normalize_mobile_category_row_in_section(child);
        }
    }
}

pub(super) fn normalize_mobile_category_row(node: &mut Value, canvas_width: f64) {
    // This pass is intentionally mobile-only. Desktop category rails commonly
    // carry six or more wide tiles; applying the phone overflow guard there
    // used to truncate the authored rail and made finalize command recording
    // fail closed because a valid node disappeared during the semantic phase.
    if canvas_width > 480.0 || !should_normalize_mobile_category_row(node, canvas_width) {
        return;
    }

    normalize_mobile_category_row_unchecked(node);
}

pub(super) fn normalize_mobile_category_row_in_section(node: &mut Value) {
    if !should_normalize_mobile_category_row_in_section(node) {
        return;
    }

    normalize_mobile_category_row_unchecked(node);
}

pub(super) fn normalize_mobile_category_row_unchecked(node: &mut Value) {
    node["width"] = json!("fill_container");
    node["height"] = json!("fit_content");
    node["gap"] = json!(12);
    node["alignItems"] = json!("center");
    // Distribute a small fixed set of chips across the row instead of clustering
    // them on the left with a lopsided empty band on the right. 3+ chips use
    // space_between so the leftover width becomes even gaps (the user's
    // "撑不满就把间距放大一点"); 1-2 chips stay start-aligned (space_between would
    // throw two chips to opposite edges). The chip COUNT follows the model — no
    // truncation — so the screen isn't forced to exactly four categories.
    let child_count = node
        .get("children")
        .and_then(Value::as_array)
        .map(|c| c.len())
        .unwrap_or(0);
    const MAX_VISIBLE_CHIPS_PER_ROW: usize = 5;
    let is_scroller = child_count > MAX_VISIBLE_CHIPS_PER_ROW;
    node["clipContent"] = json!(is_scroller);
    node["justifyContent"] = json!(if is_scroller || child_count < 3 {
        "start"
    } else {
        "space_between"
    });

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    // Six or more items become a clipped, start-aligned horizontal rail. Keep
    // every authored node: semantic finalization is a field transform and must
    // never delete product/category content to satisfy a viewport heuristic.
    for (idx, child) in children.iter_mut().enumerate() {
        if is_category_item_like_child(child) {
            child["width"] = json!("fit_content");
            child["height"] = json!("fit_content");
            child["cornerRadius"] = json!(8);
            if is_icon_label_category_item(child) {
                child["gap"] = json!(8);
                child["alignItems"] = json!("center");
                normalize_category_icon_tile(child, idx == 0);
            }
        }
    }
}

pub(super) fn normalize_category_icon_tile(item: &mut Value, active: bool) {
    let Some(children) = item.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    let Some(tile) = children.iter_mut().find(|child| {
        child.get("type").and_then(Value::as_str) == Some("frame")
            && has_descendant_type(child, "icon_font")
    }) else {
        return;
    };

    tile["width"] = json!(56);
    tile["height"] = json!(56);
    tile["cornerRadius"] = json!(8);
    tile["layout"] = json!("horizontal");
    tile["justifyContent"] = json!("center");
    tile["alignItems"] = json!("center");
    if active {
        tile["fill"] = solid_fill("#FFF0E3");
        if let Some(obj) = tile.as_object_mut() {
            obj.remove("stroke");
            obj.remove("effects");
        }
        set_subtree_foreground(tile, "$--primary");
    } else {
        tile["fill"] = solid_fill("#FFFFFF");
        tile["stroke"] = neutral_stroke("#EAD8C8");
        set_subtree_foreground(tile, "#8A5F49");
    }
}

// ── normalizeMobileProductCardRows ──────────────────────────────────────────
