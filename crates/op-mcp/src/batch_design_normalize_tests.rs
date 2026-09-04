//! Regression tests for node-shape normalizers.

#[test]
fn normalize_text_content_number_to_string() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": 2024,
        "id": "test1"
    });
    super::normalize_node_shape(&mut node);
    assert_eq!(node["content"], "2024");
}

#[test]
fn normalize_text_content_float_to_string() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": 8.5,
        "id": "test2"
    });
    super::normalize_node_shape(&mut node);
    assert_eq!(node["content"], "8.5");
}

#[test]
fn normalize_text_content_string_unchanged() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": "Hello World",
        "id": "test3"
    });
    super::normalize_node_shape(&mut node);
    assert_eq!(node["content"], "Hello World");
}

#[test]
fn normalize_text_content_boolean_untouched() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": true,
        "id": "test4"
    });
    super::normalize_node_shape(&mut node);
    assert_eq!(
        node["content"], true,
        "boolean content is left for schema to reject"
    );
}

#[test]
fn normalize_text_content_null_untouched() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": serde_json::Value::Null,
        "id": "test5"
    });
    super::normalize_node_shape(&mut node);
    assert!(
        node["content"].is_null(),
        "null content is left for schema to reject"
    );
}

#[test]
fn normalize_text_content_array_unchanged() {
    let mut node = serde_json::json!({
        "type": "text",
        "content": [
            { "text": "styled", "fontSize": 16 }
        ],
        "id": "test6"
    });
    super::normalize_node_shape(&mut node);
    assert!(node["content"].is_array());
}

#[test]
fn normalize_non_text_node_content_untouched() {
    let mut node = serde_json::json!({
        "type": "frame",
        "content": 2024,
        "id": "test7"
    });
    super::normalize_node_shape(&mut node);
    assert_eq!(
        node["content"], 2024,
        "non-text nodes skip content normalization"
    );
}
