use jian_ops_schema::node::PenNode;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelfCheckSeverity {
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfCheckIssue {
    pub code: &'static str,
    pub node_id: Option<String>,
    pub message: String,
    pub severity: SelfCheckSeverity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelfCheckReport {
    pub issues: Vec<SelfCheckIssue>,
}

impl SelfCheckReport {
    pub(crate) fn has_fatal(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == SelfCheckSeverity::Fatal)
    }

    pub(crate) fn failure_message(&self) -> String {
        self.issues
            .iter()
            .filter(|issue| issue.severity == SelfCheckSeverity::Fatal)
            .map(|issue| match issue.node_id.as_deref() {
                Some(id) if !id.is_empty() => {
                    format!("{} at {}: {}", issue.code, id, issue.message)
                }
                _ => format!("{}: {}", issue.code, issue.message),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub(crate) fn check_generated_nodes(nodes: &[PenNode], canvas_width: f64) -> SelfCheckReport {
    let value = serde_json::to_value(nodes).unwrap_or(Value::Null);
    check_value_forest(&value, canvas_width)
}

pub(crate) fn check_value_forest(value: &Value, canvas_width: f64) -> SelfCheckReport {
    let mut report = SelfCheckReport::default();
    match value {
        Value::Array(nodes) => {
            for node in nodes {
                check_node(node, canvas_width, &mut report);
            }
        }
        Value::Object(_) => check_node(value, canvas_width, &mut report),
        _ => {}
    }
    report
}

fn check_node(node: &Value, canvas_width: f64, report: &mut SelfCheckReport) {
    if is_mobile_product_row_overflow(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-product-row-overflow",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "fixed-width product cards exceed the mobile content rail; use two fill_container cards with gap 12"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if is_mobile_category_row_loose_spacing(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-category-row-loose-spacing",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "mobile category rows must use start alignment, gap 12, fit_content height, and no wide fixed row"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
    if is_mobile_featured_card_split_badly(node, canvas_width) {
        report.issues.push(SelfCheckIssue {
            code: "mobile-featured-card-bad-split",
            node_id: string_prop(node, "id").map(str::to_string),
            message:
                "mobile featured food cards must not leave a blank half beside the image; use an image-top product card or a deliberate 50/50 promo banner with compact action"
                    .into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }

    if let Some(children) = children(node) {
        for child in children {
            check_node(child, canvas_width, report);
        }
    }
}

fn is_mobile_product_row_overflow(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
    {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() < 2 || !children.iter().all(is_product_card_child) {
        return false;
    }

    let fixed_widths: Vec<f64> = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .collect();
    if fixed_widths.len() < 2 {
        return false;
    }
    let gap = numeric_prop(node, "gap").unwrap_or(0.0);
    let total = fixed_widths.iter().sum::<f64>() + gap * (children.len().saturating_sub(1) as f64);
    total > available_row_width(node, canvas_width)
}

fn is_mobile_category_row_loose_spacing(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
    {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() < 2 || !children.iter().all(is_category_item_child) {
        return false;
    }

    // `space_between` / `space_around` are NO LONGER flagged: spreading a small
    // chip set across the row is the desired distribution (the user's
    // "撑不满就把间距放大一点"). Only genuinely broken spacing is fatal — a huge
    // literal gap, a row wider than the canvas, or an over-tall row.
    numeric_prop(node, "gap")
        .map(|gap| gap > 48.0)
        .unwrap_or(false)
        || numeric_prop(node, "width")
            .map(|width| width > canvas_width)
            .unwrap_or(false)
        || numeric_prop(node, "height")
            .map(|height| height > 120.0)
            .unwrap_or(false)
}

fn is_mobile_featured_card_split_badly(node: &Value, canvas_width: f64) -> bool {
    if canvas_width > 480.0
        || string_prop(node, "type") != Some("frame")
        || string_prop(node, "layout") != Some("horizontal")
        || !looks_like_featured_food_card(node)
    {
        return false;
    }
    let Some(children) = children(node) else {
        return false;
    };
    if children.len() >= 2 && children.iter().all(is_product_card_child) {
        return false;
    }
    if children.len() < 2 || !has_descendant_type(node, "image") || !has_text_descendant(node) {
        return false;
    }

    let Some(card_width) = effective_node_width(node, canvas_width) else {
        return false;
    };
    let content_width = (card_width - horizontal_padding(node)).max(0.0);
    if content_width <= 0.0 {
        return false;
    }

    let gap = numeric_prop(node, "gap").unwrap_or(0.0);
    let fixed_child_total = children
        .iter()
        .filter_map(|child| numeric_prop(child, "width"))
        .sum::<f64>()
        + gap * (children.len().saturating_sub(1) as f64);
    let image_width_ratio = largest_descendant_image_width(node) / content_width;

    image_width_ratio < 0.45
        || (fixed_child_total > 0.0 && fixed_child_total < content_width * 0.82)
        || has_oversized_square_action(node)
}

fn looks_like_featured_food_card(node: &Value) -> bool {
    if !is_product_card_child(node) {
        return false;
    }
    let label = semantic_label(node);
    contains_any(
        &label,
        &[
            "featured",
            "feature",
            "special",
            "popular",
            "hero",
            "dish",
            "menu",
            "product",
            "truffle",
            "tagliatelle",
            "dumpling",
            "饺子",
            "特色",
            "推荐",
            "热门",
            "菜品",
        ],
    )
}

fn available_row_width(node: &Value, canvas_width: f64) -> f64 {
    let nominal = numeric_prop(node, "width")
        .filter(|width| *width > 0.0 && *width <= canvas_width)
        .unwrap_or_else(|| (canvas_width - 40.0).max(0.0));
    (nominal - horizontal_padding(node)).max(0.0)
}

fn effective_node_width(node: &Value, canvas_width: f64) -> Option<f64> {
    match node.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) if s == "fill_container" => Some((canvas_width - 40.0).max(0.0)),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn horizontal_padding(node: &Value) -> f64 {
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

fn is_product_card_child(node: &Value) -> bool {
    if string_prop(node, "type") != Some("frame") {
        return false;
    }
    if matches!(
        string_prop(node, "role"),
        Some("card")
            | Some("image-card")
            | Some("product-card")
            | Some("restaurant-card")
            | Some("menu-card")
            | Some("feature-card")
    ) {
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

fn is_category_item_child(node: &Value) -> bool {
    if string_prop(node, "type") != Some("frame") {
        return false;
    }
    matches!(
        string_prop(node, "role"),
        Some("chip") | Some("tag") | Some("pill") | Some("button")
    ) || {
        let label = semantic_label(node);
        label.contains("chip") || label.contains("category") || label.contains("类别")
    } || (has_descendant_type(node, "icon_font") && has_text_descendant(node))
}

fn has_descendant_type(node: &Value, type_name: &str) -> bool {
    string_prop(node, "type") == Some(type_name)
        || children(node)
            .map(|children| {
                children
                    .iter()
                    .any(|child| has_descendant_type(child, type_name))
            })
            .unwrap_or(false)
}

fn has_text_descendant(node: &Value) -> bool {
    (string_prop(node, "type") == Some("text")
        && string_prop(node, "content")
            .map(|content| !content.trim().is_empty())
            .unwrap_or(false))
        || children(node)
            .map(|children| children.iter().any(has_text_descendant))
            .unwrap_or(false)
}

fn largest_descendant_image_width(node: &Value) -> f64 {
    let own = if string_prop(node, "type") == Some("image") {
        numeric_prop(node, "width").unwrap_or(0.0)
    } else {
        0.0
    };
    let child = children(node)
        .map(|children| {
            children
                .iter()
                .map(largest_descendant_image_width)
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0);
    own.max(child)
}

fn has_oversized_square_action(node: &Value) -> bool {
    let label = semantic_label(node);
    let looks_action = contains_any(
        &label,
        &["button", "action", "add", "plus", "cta", "加入", "添加"],
    );
    let size = numeric_prop(node, "width").zip(numeric_prop(node, "height"));
    if looks_action
        && size
            .map(|(w, h)| w >= 56.0 && h >= 56.0 && (w - h).abs() <= 8.0)
            .unwrap_or(false)
    {
        return true;
    }
    children(node)
        .map(|children| children.iter().any(has_oversized_square_action))
        .unwrap_or(false)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn semantic_label(node: &Value) -> String {
    [
        "id",
        "name",
        "role",
        "content",
        "placeholder",
        "value",
        "iconFontName",
    ]
    .iter()
    .filter_map(|key| string_prop(node, key))
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase()
}

fn children(node: &Value) -> Option<&Vec<Value>> {
    node.get("children").and_then(Value::as_array)
}

fn string_prop<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
    node.get(key).and_then(Value::as_str)
}

fn numeric_prop(node: &Value, key: &str) -> Option<f64> {
    node.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn flags_mobile_product_row_that_will_clip_cards() {
        let nodes = json!([
            {
                "type": "frame",
                "id": "popular",
                "name": "Popular Now",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "gap": 20,
                "children": [
                    {"type": "frame", "id": "card-1", "role": "card", "width": 170, "height": 220,
                     "children": [{"type": "image", "id": "img-1", "width": 170, "height": 120}, {"type": "text", "id": "t1", "content": "Truffle Carbonara"}]},
                    {"type": "frame", "id": "card-2", "role": "card", "width": 170, "height": 220,
                     "children": [{"type": "image", "id": "img-2", "width": 170, "height": 120}, {"type": "text", "id": "t2", "content": "Smash Deluxe"}]},
                    {"type": "frame", "id": "card-3", "role": "card", "width": 170, "height": 220,
                     "children": [{"type": "image", "id": "img-3", "width": 170, "height": 120}, {"type": "text", "id": "t3", "content": "Poke Salmon"}]}
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            report.has_fatal(),
            "fixed-width mobile product row should be rejected before insertion"
        );
        assert!(
            report
                .failure_message()
                .contains("mobile-product-row-overflow"),
            "{report:?}"
        );
    }

    #[test]
    fn accepts_two_equal_fill_product_cards() {
        let nodes = json!([
            {
                "type": "frame",
                "id": "popular",
                "name": "Popular Now",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "gap": 12,
                "children": [
                    {"type": "frame", "id": "card-1", "role": "card", "width": "fill_container", "height": "fit_content",
                     "children": [{"type": "image", "id": "img-1", "width": "fill_container", "height": 120}, {"type": "text", "id": "t1", "content": "Truffle Carbonara"}]},
                    {"type": "frame", "id": "card-2", "role": "card", "width": "fill_container", "height": "fit_content",
                     "children": [{"type": "image", "id": "img-2", "width": "fill_container", "height": 120}, {"type": "text", "id": "t2", "content": "Smash Deluxe"}]}
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            !report.has_fatal(),
            "valid two-card mobile grid should pass: {report:?}"
        );
    }

    #[test]
    fn allows_mobile_category_row_with_space_between() {
        // space_between is now the desired way to spread an under-filling chip
        // set across the row — it must NOT be flagged (user direction 2026-06-23).
        let nodes = json!([
            {
                "type": "frame",
                "id": "categories",
                "name": "Categories Section",
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "heading", "content": "Categories"},
                    {
                        "type": "frame",
                        "id": "options-row",
                        "name": "Options Row",
                        "layout": "horizontal",
                        "width": "fill_container",
                        "height": 96,
                        "justifyContent": "space_between",
                        "children": [
                            {"type": "frame", "id": "pizza", "layout": "vertical",
                             "children": [{"type": "icon_font", "iconFontName": "pizza"}, {"type": "text", "content": "Pizza"}]},
                            {"type": "frame", "id": "burger", "layout": "vertical",
                             "children": [{"type": "icon_font", "iconFontName": "hamburger"}, {"type": "text", "content": "Burger"}]}
                        ]
                    }
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            !report.has_fatal(),
            "space_between on a category row must be allowed now: {report:?}"
        );
    }

    #[test]
    fn flags_mobile_category_row_that_overflows_or_is_too_tall() {
        // Genuinely broken spacing is still fatal: an over-tall row.
        let nodes = json!([
            {
                "type": "frame",
                "id": "categories",
                "name": "Categories Section",
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "heading", "content": "Categories"},
                    {
                        "type": "frame",
                        "id": "options-row",
                        "name": "Options Row",
                        "layout": "horizontal",
                        "width": "fill_container",
                        "height": 160,
                        "justifyContent": "start",
                        "children": [
                            {"type": "frame", "id": "pizza", "layout": "vertical",
                             "children": [{"type": "icon_font", "iconFontName": "pizza"}, {"type": "text", "content": "Pizza"}]},
                            {"type": "frame", "id": "burger", "layout": "vertical",
                             "children": [{"type": "icon_font", "iconFontName": "hamburger"}, {"type": "text", "content": "Burger"}]}
                        ]
                    }
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            report.has_fatal(),
            "an over-tall category row should still be rejected"
        );
        assert!(
            report
                .failure_message()
                .contains("mobile-category-row-loose-spacing"),
            "{report:?}"
        );
    }

    #[test]
    fn flags_mobile_featured_food_card_with_blank_split() {
        let nodes = json!([
            {
                "type": "frame",
                "id": "featured-card",
                "name": "Featured Dish Card",
                "role": "card",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "gap": 0,
                "fill": [{"type": "solid", "color": "#FFFFFF"}],
                "children": [
                    {
                        "type": "frame",
                        "id": "featured-left",
                        "width": 122,
                        "height": "fit_content",
                        "layout": "vertical",
                        "children": [
                            {"type": "text", "id": "badge", "content": "Free delivery"},
                            {"type": "text", "id": "title", "content": "Truffle Tagliatelle"},
                            {"type": "text", "id": "price", "content": "$18.50"}
                        ]
                    },
                    {
                        "type": "image",
                        "id": "featured-photo",
                        "width": 170,
                        "height": 136,
                        "imageSearchQuery": "pasta plate"
                    },
                    {
                        "type": "frame",
                        "id": "plus-action",
                        "name": "Plus Action Button",
                        "role": "button",
                        "width": 64,
                        "height": 64,
                        "children": [{"type": "icon_font", "iconFontName": "plus"}]
                    }
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            report.has_fatal(),
            "bad split featured food card should be rejected before insertion"
        );
        assert!(
            report
                .failure_message()
                .contains("mobile-featured-card-bad-split"),
            "{report:?}"
        );
    }

    #[test]
    fn accepts_mobile_featured_food_card_with_image_top() {
        let nodes = json!([
            {
                "type": "frame",
                "id": "featured-card",
                "name": "Featured Dish Card",
                "role": "card",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 12,
                "fill": [{"type": "solid", "color": "#FFFFFF"}],
                "children": [
                    {
                        "type": "image",
                        "id": "featured-photo",
                        "width": "fill_container",
                        "height": 148,
                        "imageSearchQuery": "dumpling plate"
                    },
                    {
                        "type": "frame",
                        "id": "body",
                        "layout": "horizontal",
                        "children": [
                            {"type": "text", "id": "title", "content": "Dumpling House"},
                            {"type": "text", "id": "price", "content": "$18.50"}
                        ]
                    }
                ]
            }
        ]);

        let report = check_value_forest(&nodes, 390.0);

        assert!(
            !report.has_fatal(),
            "valid image-top featured food card should pass: {report:?}"
        );
    }
}
