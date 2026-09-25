use super::*;
use serde_json::json;

fn node(v: Value) -> PenNode {
    serde_json::from_value(v).expect("fixture parses")
}

/// arena-m03 minimized: the chart header's period switcher.
fn period_header(radio_width: Value, radio_height: Value) -> PenNode {
    node(json!({
        "type": "frame", "id": "hdr", "name": "收益时间范围标题栏",
        "width": 327, "height": 44, "layout": "horizontal",
        "justifyContent": "space_between", "alignItems": "center",
        "children": [
            {"type": "text", "id": "title", "content": "累计收益率", "fontSize": 15},
            {"type": "radio_group", "id": "range", "name": "收益时间范围切换",
             "width": radio_width, "height": radio_height, "value": "近1月",
             "options": [
                {"value": "近1月", "label": "近1月"},
                {"value": "近3月", "label": "近3月"},
                {"value": "今年", "label": "今年"}
             ],
             "fill": [{"type": "solid", "color": "$--muted"}],
             "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "$--border"}]},
             "cornerRadius": 8}
        ]
    }))
}

fn radio_json(root: &PenNode) -> Value {
    serde_json::to_value(root).unwrap()["children"][1].clone()
}

#[test]
fn single_row_radio_box_becomes_a_segmented_control() {
    let mut root = period_header(json!(194), json!(36));
    assert!(segment_single_row_radio_groups(&mut root));
    let radio = radio_json(&root);
    assert_eq!(radio["type"], json!("tabs"));
    assert_eq!(radio["id"], json!("range"));
    assert_eq!(radio["value"], json!("近1月"));
    assert_eq!(radio["tabs"].as_array().map(Vec::len), Some(3));
    assert_eq!(radio["width"], json!(194.0));
    assert_eq!(radio["height"], json!(36.0));
    assert_eq!(radio["cornerRadius"], json!(8.0));
    assert!(radio.get("options").is_none());
}

#[test]
fn radio_box_tall_enough_for_its_rows_is_untouched() {
    let mut root = period_header(json!(194), json!(84));
    assert!(!segment_single_row_radio_groups(&mut root));
    assert_eq!(radio_json(&root)["type"], json!("radio_group"));
}

#[test]
fn hugging_radio_height_is_untouched() {
    let mut root = period_header(json!(194), json!("fit_content"));
    assert!(!segment_single_row_radio_groups(&mut root));
}

#[test]
fn radio_box_too_narrow_for_a_row_is_untouched() {
    let mut root = period_header(json!(90), json!(36));
    assert!(!segment_single_row_radio_groups(&mut root));
}
