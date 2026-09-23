//! Mobile product-card rows, featured media cards and cart count badges.

use super::*;

pub(super) fn is_product_card_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("card")
            | Some("image-card")
            | Some("product-card")
            | Some("restaurant-card")
            | Some("menu-card")
            | Some("feature-card")
    )
}

pub(super) fn is_mobile_product_card_child(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    if is_product_card_role(role_of(node)) {
        return true;
    }
    let label = semantic_label(node);
    [
        "card",
        "product",
        "restaurant",
        "popular",
        "dish",
        "menu",
        "nearby",
        "餐厅",
        "美食",
        "热门",
        "菜品",
    ]
    .iter()
    .any(|needle| label.contains(needle))
        || (has_descendant_type(node, "image") && has_text_descendant(node))
}

pub(super) fn should_normalize_mobile_product_card_row(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || node.get("type").and_then(Value::as_str) != Some("frame")
        || node.get("layout").and_then(Value::as_str) != Some("horizontal")
    {
        return false;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return false;
    };
    if children.len() != 2 || !children.iter().all(is_mobile_product_card_child) {
        return false;
    }

    let fixed_children_width: f64 = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .sum();
    let fixed_total = fixed_children_width + numeric_prop(node, "gap").unwrap_or(0.0);
    let content_rail_width = (canvas_width - 40.0).max(0.0);
    fixed_total > content_rail_width
        || numeric_prop(node, "width")
            .map(|width| width > content_rail_width)
            .unwrap_or(false)
        || matches!(
            node.get("justifyContent").and_then(Value::as_str),
            Some("space_between") | Some("space_around")
        )
}

pub(super) fn normalize_mobile_product_card_row(node: &mut Value, canvas_width: f64) {
    if !should_normalize_mobile_product_card_row(node, canvas_width) {
        return;
    }

    node["width"] = json!("fill_container");
    node["height"] = json!("fit_content");
    node["gap"] = json!(12);
    node["clipContent"] = json!(false);
    node["justifyContent"] = json!("start");
    node["alignItems"] = json!("stretch");

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        child["width"] = json!("fill_container");
        child["height"] = json!("fit_content");
        child["cornerRadius"] = json!(8);
    }
}

pub(super) fn horizontal_padding_sum(node: &Value) -> f64 {
    match node.get("padding") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) * 2.0,
        Some(Value::Array(values)) if values.len() == 2 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0) * 2.0
        }
        Some(Value::Array(values)) if values.len() >= 4 => {
            values.get(1).and_then(Value::as_f64).unwrap_or(0.0)
                + values.get(3).and_then(Value::as_f64).unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

pub(super) fn mobile_card_content_width(node: &Value, canvas_width: f64) -> f64 {
    let fallback = (canvas_width - 40.0).max(0.0);
    let outer = match node.get("width") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(fallback),
        Some(Value::String(s)) if s == "fill_container" => fallback,
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(fallback),
        _ => fallback,
    };
    (outer - horizontal_padding_sum(node)).max(0.0)
}

pub(super) fn direct_image_child_indices_over_height(node: &Value, max_height: f64) -> Vec<usize> {
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return Vec::new();
    };
    children
        .iter()
        .enumerate()
        .filter_map(|(idx, child)| {
            (child.get("type").and_then(Value::as_str) == Some("image")
                && numeric_prop(child, "height")
                    .map(|height| height > max_height + 24.0)
                    .unwrap_or(false))
            .then_some(idx)
        })
        .collect()
}

pub(super) fn looks_like_mobile_food_media_card(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("frame")
        || !is_mobile_product_card_child(node)
        || !has_descendant_type(node, "image")
        || !has_text_descendant(node)
    {
        return false;
    }
    let label = semantic_label(node);
    [
        "featured",
        "feature",
        "dish",
        "food",
        "menu",
        "product",
        "restaurant",
        "card",
        "推荐",
        "主题",
        "美食",
        "菜品",
        "餐厅",
    ]
    .iter()
    .any(|needle| label.contains(needle))
}

pub(super) fn normalize_mobile_featured_card_media(node: &mut Value, canvas_width: f64) {
    if canvas_width > 480.0 || !looks_like_mobile_food_media_card(node) {
        return;
    }
    let content_width = mobile_card_content_width(node, canvas_width);
    if content_width <= 0.0 {
        return;
    }
    let max_height = (content_width * 0.6).round().clamp(148.0, 204.0);
    let image_indices = direct_image_child_indices_over_height(node, max_height);
    if image_indices.is_empty() {
        return;
    }

    node["height"] = json!("fit_content");
    node["clipContent"] = json!(true);
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for idx in image_indices {
        if let Some(image) = children.get_mut(idx) {
            image["width"] = json!("fill_container");
            image["height"] = json!(max_height as i64);
        }
    }
}

// ── normalizeCartCountBadges ────────────────────────────────────────────────

pub(super) fn is_short_count_text(node: &Value) -> bool {
    if node.get("type").and_then(Value::as_str) != Some("text") {
        return false;
    }
    let Some(content) = text_content(node).map(str::trim) else {
        return false;
    };
    !content.is_empty() && content.len() <= 2 && content.chars().all(|c| c.is_ascii_digit())
}

/// Largest box a cart ICON (glyph, or the small button wrapping one) can
/// occupy. Without it every node NAMED "cart…" qualified — a GLM-5.3-Flash
/// and an Opus 5.5 delivery page (0923) both had their whole floating
/// `cart-bar` row matched as "the cart icon", and the pass then styled the
/// bar itself as a 16px count badge, squashing it flat.
const CART_ICON_MAX_BOX: f64 = 56.0;
/// Largest box a count badge can occupy.
const COUNT_BADGE_MAX_BOX: f64 = 28.0;

fn fits_box(node: &Value, max: f64) -> bool {
    ["width", "height"].iter().all(|key| {
        node.get(*key)
            .and_then(Value::as_f64)
            .is_none_or(|size| size <= max)
    }) && ["width", "height"]
        .iter()
        .all(|key| node.get(*key).and_then(Value::as_str) != Some("fill_container"))
}

fn is_glyph(node: &Value) -> bool {
    matches!(
        node.get("type").and_then(Value::as_str),
        Some("icon_font" | "path")
    )
}

pub(super) fn is_cart_icon_node(node: &Value) -> bool {
    let is_icon = is_glyph(node)
        || (fits_box(node, CART_ICON_MAX_BOX)
            && node
                .get("children")
                .and_then(Value::as_array)
                .is_some_and(|children| children.iter().any(is_glyph)));
    if !is_icon {
        return false;
    }
    let label = semantic_label(node);
    label.contains("shopping-cart")
        || label.contains("shopping cart")
        || label.contains("cart")
        || label.contains("basket")
        || label.contains("checkout")
        || label.contains("购物车")
}

/// A count badge is the number itself, or a SMALL frame whose only content is
/// that number. Recursing into any descendant made every ancestor of the
/// badge — the icon cluster, the whole cart bar — a "count badge" too.
pub(super) fn is_count_badge_candidate(node: &Value) -> bool {
    if !fits_box(node, COUNT_BADGE_MAX_BOX) {
        return false;
    }
    if matches!(role_of(node), Some("badge")) || is_short_count_text(node) {
        return true;
    }
    node.get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| children.len() == 1 && is_short_count_text(&children[0]))
}

pub(super) fn style_count_badge(node: &mut Value) {
    node["role"] = json!("badge");
    node["width"] = json!(16);
    node["height"] = json!(16);
    node["cornerRadius"] = json!(999);
    node["padding"] = json!([0, 0]);
    node["layout"] = json!("horizontal");
    node["justifyContent"] = json!("center");
    node["alignItems"] = json!("center");
    node["fill"] = solid_fill("$--primary");
    if let Some(obj) = node.as_object_mut() {
        obj.remove("stroke");
        obj.remove("effects");
    }
    set_count_badge_text_style(node);
}

pub(super) fn set_count_badge_text_style(node: &mut Value) {
    if is_short_count_text(node) {
        node["fontSize"] = json!(10);
        node["fontWeight"] = json!(700);
        node["fill"] = solid_fill("#FFFFFF");
        node["textAlign"] = json!("center");
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            set_count_badge_text_style(child);
        }
    }
}

pub(super) fn normalize_cart_count_badges(node: &mut Value) {
    if node.get("type").and_then(Value::as_str) != Some("frame") {
        return;
    }
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    let has_cart_icon = children.iter().any(is_cart_icon_node);
    let has_count_badge = children.iter().any(is_count_badge_candidate);
    if !has_cart_icon || !has_count_badge {
        return;
    }

    if matches!(role_of(node), Some("icon-button") | Some("button"))
        || (semantic_label(node).contains("cart") && fits_box(node, CART_ICON_MAX_BOX + 8.0))
    {
        node["fill"] = solid_fill("#FFFFFF");
        node["stroke"] = neutral_stroke("#E5E7EB");
        node["cornerRadius"] = json!(8);
    }

    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if is_count_badge_candidate(child) {
            style_count_badge(child);
        }
    }
}

// ── fixInputSiblingConsistency ───────────────────────────────────────────────
