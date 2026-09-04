//! Regression tests for fill normalizers.

#[test]
fn normalize_bare_gradient_with_stops_to_linear_gradient() {
    let mut fill = serde_json::json!({
        "type": "gradient",
        "angle": 180,
        "stops": [
            { "color": "#FF0000", "offset": 0.0 },
            { "color": "#0000FF", "offset": 1.0 }
        ]
    });
    super::normalize_fill(&mut fill);
    let items = fill.as_array().expect("fill is an array");
    let obj = items[0].as_object().expect("first item is an object");
    assert_eq!(obj["type"], "linear_gradient");
}

#[test]
fn normalize_bare_gradient_without_stops_left_untouched() {
    let mut fill = serde_json::json!({
        "type": "gradient",
        "angle": 180
    });
    super::normalize_fill(&mut fill);
    let items = fill.as_array().expect("fill is an array");
    let obj = items[0].as_object().expect("first item is an object");
    assert_eq!(
        obj["type"], "gradient",
        "bare gradient without stops is left untouched"
    );
}

#[test]
fn normalize_gradient_stop_pos_to_offset() {
    let mut fill = serde_json::json!({
        "type": "linear_gradient",
        "angle": 45,
        "stops": [
            { "color": "#0D0B0B", "pos": 0 },
            { "color": "#FFFFFF", "pos": 1 }
        ]
    });
    super::normalize_fill(&mut fill);
    let items = fill.as_array().expect("fill is an array");
    let obj = items[0].as_object().expect("first item is an object");
    let stops = obj["stops"].as_array().expect("stops is an array");
    assert!(stops[0].as_object().unwrap().contains_key("offset"));
    assert_eq!(stops[0]["offset"], 0);
}

#[test]
fn normalize_gradient_stop_with_existing_offset_untouched() {
    let mut fill = serde_json::json!({
        "type": "linear_gradient",
        "angle": 45,
        "stops": [
            { "color": "#000000", "offset": 0 }
        ]
    });
    super::normalize_fill(&mut fill);
    let items = fill.as_array().expect("fill is an array");
    let obj = items[0].as_object().expect("first item is an object");
    let stops = obj["stops"].as_array().expect("stops is an array");
    assert_eq!(stops[0]["offset"], 0);
    assert!(!stops[0].as_object().unwrap().contains_key("pos"));
}
