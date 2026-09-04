//! Mobile normalizers for search shells, favourite icon buttons and
//! section-header "see all" actions.

use super::*;

pub(super) fn child_role(child: &Value) -> Option<&str> {
    child.get("role").and_then(Value::as_str)
}

pub(super) fn is_search_input_child(child: &Value) -> bool {
    let label = semantic_label(child);
    matches!(
        child_role(child),
        Some("input") | Some("form-input") | Some("search-bar")
    ) || (child.get("type").and_then(Value::as_str) == Some("text_input")
        && (label.contains("search") || label.contains("搜索")))
        || label.contains("search")
        || label.contains("搜索")
}

pub(super) fn is_filter_button_child(child: &Value) -> bool {
    let label = semantic_label(child);
    if (matches!(child_role(child), Some("icon-button") | Some("button"))
        || child.get("type").and_then(Value::as_str) == Some("icon_font"))
        && mentions_filter_affordance(&label)
    {
        return true;
    }
    child
        .get("children")
        .and_then(Value::as_array)
        .map(|children| {
            children.iter().any(|grandchild| {
                let label = semantic_label(grandchild);
                mentions_filter_affordance(&label)
            })
        })
        .unwrap_or(false)
}

pub(super) fn mentions_filter_affordance(label: &str) -> bool {
    label.contains("filter")
        || label.contains("slider")
        || label.contains("sliders")
        || label.contains("筛选")
        || label.contains("过滤")
        || label.contains("调节")
}

pub(super) fn normalize_nested_search_shell(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return;
    }
    // The shell shape this pass exists for is a WRAPPER around a search
    // input CONTAINER and a filter button CONTAINER. A search bar whose
    // children are bare icon/text leaves IS the input itself — clearing
    // its chrome stripped the authored background, repainted the search
    // icon white (invisible on a light field) and re-inked the filter
    // glyph with a dangling `$--primary` (measured: test0711-1-ds).
    if node.get("role").and_then(Value::as_str) == Some("search-bar") {
        return;
    }

    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    let container_child = |c: &Value| {
        matches!(
            c.get("type").and_then(Value::as_str),
            Some("frame" | "text_input")
        )
    };
    let search_child_count = children
        .iter()
        .filter(|c| container_child(c) && is_search_input_child(c))
        .count();
    let has_filter = children
        .iter()
        .any(|c| container_child(c) && !is_search_input_child(c) && is_filter_button_child(c));
    if search_child_count != 1 || !has_filter {
        return;
    }

    clear_visual_chrome(node);
    node["gap"] = json!(12);
    node["padding"] = json!([0, 0]);
    node["alignItems"] = json!("center");

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if !matches!(
            child.get("type").and_then(Value::as_str),
            Some("frame" | "text_input")
        ) {
            continue;
        }
        if is_search_input_child(child) {
            child["fill"] = solid_fill("#FFFFFF");
            child["stroke"] = neutral_stroke("#E5E7EB");
            child["cornerRadius"] = json!(8);
        } else if is_filter_button_child(child) {
            // The button's own saturated fill IS the design's accent — move
            // it onto the glyph instead of a symbolic `$--primary`,
            // which dangled on variable-less documents and rendered the
            // fallback blue (measured: test0711-1-ds).
            let demoted_accent = child
                .get("fill")
                .and_then(|f| f.pointer("/0/color"))
                .and_then(Value::as_str)
                .filter(|hex| hex.starts_with('#'))
                .map(str::to_string);
            child["fill"] = solid_fill("#FFFFFF");
            child["stroke"] = neutral_stroke("#E5E7EB");
            child["cornerRadius"] = json!(8);
            if let Some(accent) = demoted_accent {
                set_subtree_foreground(child, &accent);
            }
        }
    }
}

pub(super) fn set_subtree_foreground(node: &mut Value, color: &str) {
    match node.get("type").and_then(Value::as_str) {
        Some("text") | Some("icon_font") => {
            node["fill"] = solid_fill(color);
        }
        Some("path") => {
            if node.get("stroke").map(|s| !s.is_null()).unwrap_or(false) {
                node["stroke"]["fill"] = solid_fill(color);
            } else {
                node["fill"] = solid_fill(color);
            }
        }
        _ => {}
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            set_subtree_foreground(child, color);
        }
    }
}

// ── normalizeFavoriteIconButtons ────────────────────────────────────────────

pub(super) fn subtree_mentions_heart(node: &Value) -> bool {
    let label = identity_label(node);
    if label.contains("favorite")
        || label.contains("favourite")
        || label.contains("heart")
        || label.contains("like")
        || label.contains("收藏")
        || label.contains("喜欢")
    {
        return true;
    }
    node.get("children")
        .and_then(Value::as_array)
        .map(|children| children.iter().any(subtree_mentions_heart))
        .unwrap_or(false)
}

pub(super) fn normalize_favorite_icon_buttons(node: &mut Value) {
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if child_role(child) == Some("icon-button") && subtree_mentions_heart(child) {
            clear_visual_chrome(child);
            child["width"] = json!(32);
            child["height"] = json!(32);
        }
    }
}

// ── normalizeSectionHeaderActions ────────────────────────────────────────────

pub(super) fn text_content(node: &Value) -> Option<&str> {
    node.get("content").and_then(Value::as_str)
}

pub(super) fn numeric_prop(node: &Value, key: &str) -> Option<f64> {
    node.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

pub(super) fn is_see_all_action_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") {
        return false;
    }
    let Some(content) = text_content(node) else {
        return false;
    };
    let compact = content
        .trim()
        .trim_matches(|c: char| matches!(c, '>' | '›' | '→' | '»'))
        .to_lowercase()
        .replace(char::is_whitespace, "");
    matches!(
        compact.as_str(),
        "查看全部" | "查看更多" | "seeall" | "viewall" | "viewmore"
    )
}

pub(super) fn is_section_heading_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") || is_see_all_action_text(node) {
        return false;
    }
    let label = identity_label(node);
    if matches!(role_of(node), Some("heading") | Some("subheading"))
        || label.contains("heading")
        || label.contains("title")
        || label.contains("header")
    {
        return true;
    }
    let weight = numeric_prop(node, "fontWeight").unwrap_or(0.0);
    let size = numeric_prop(node, "fontSize").unwrap_or(0.0);
    weight >= 600.0 && size >= 16.0
}

pub(super) fn rewrite_text_node_as_chevron(child: &mut Value) {
    let Some(obj) = child.as_object_mut() else {
        return;
    };
    obj.insert("type".to_string(), json!("icon_font"));
    obj.insert("iconFontName".to_string(), json!("chevron-right"));
    obj.insert("width".to_string(), json!(20));
    obj.insert("height".to_string(), json!(20));
    obj.insert("fill".to_string(), solid_fill("$--primary"));
    obj.remove("content");
    obj.remove("fontFamily");
    obj.remove("fontSize");
    obj.remove("fontWeight");
    obj.remove("fontStyle");
    obj.remove("textAlign");
    obj.remove("textGrowth");
    obj.remove("lineHeight");
    obj.remove("letterSpacing");
}

pub(super) fn normalize_section_header_actions(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if !children.iter().any(is_see_all_action_text) || !children.iter().any(is_section_heading_text)
    {
        return;
    }
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if is_see_all_action_text(child) {
            rewrite_text_node_as_chevron(child);
        }
    }
}

// ── normalizeMobileCategoryRows ──────────────────────────────────────────────
