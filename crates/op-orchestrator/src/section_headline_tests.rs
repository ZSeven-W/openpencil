use super::section_headline;
use jian_ops_schema::node::PenNode;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("valid PenNode fixture")
}

#[test]
fn hero_uses_the_largest_text_node() {
    let hero = node(serde_json::json!({
        "type": "frame",
        "id": "hero",
        "name": "Hero",
        "children": [
            {"type": "text", "id": "title", "content": "Build with Vectra", "fontSize": 48},
            {"type": "text", "id": "body", "content": "A concise product promise.", "fontSize": 16}
        ]
    }));

    assert_eq!(
        section_headline(&hero).as_deref(),
        Some("Build with Vectra")
    );
}

#[test]
fn card_grid_prefers_section_title_over_card_titles() {
    let grid = node(serde_json::json!({
        "type": "frame",
        "id": "grid",
        "name": "Card Grid",
        "children": [
            {"type": "text", "id": "section-title", "content": "Everything you need", "fontSize": 32},
            {"type": "frame", "id": "card-1", "name": "Card", "children": [
                {"type": "text", "id": "card-title-1", "content": "Fast setup", "fontSize": 20}
            ]},
            {"type": "frame", "id": "card-2", "name": "Card", "children": [
                {"type": "text", "id": "card-title-2", "content": "Clear insights", "fontSize": 20}
            ]}
        ]
    }));

    assert_eq!(
        section_headline(&grid).as_deref(),
        Some("Everything you need")
    );
}

#[test]
fn nav_row_is_skipped() {
    let nav = node(serde_json::json!({
        "type": "frame",
        "id": "nav",
        "name": "Navigation",
        "children": [
            {"type": "text", "id": "nav-title", "content": "Explore", "fontSize": 64},
            {"type": "text", "id": "nav-link", "content": "Home", "fontSize": 18}
        ]
    }));

    assert_eq!(section_headline(&nav), None);
}

#[test]
fn empty_section_returns_none() {
    let empty = node(serde_json::json!({
        "type": "frame",
        "id": "empty",
        "name": "Empty Section",
        "children": []
    }));

    assert_eq!(section_headline(&empty), None);
}
