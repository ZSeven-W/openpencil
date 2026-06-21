// Tests for Viewer read-only JSON snapshot getters.

// A document with an explicit pages array (multi-page format).
const DOC_WITH_PAGE: &str =
    r#"{"version":"1.0","pages":[{"id":"pageX","name":"P","children":[]}]}"#;

// A document that uses the single-page fallback: no `pages` key, nodes live in
// top-level `children`. PenNode is a serde tagged enum so `type` is required.
const DOC_SINGLE_PAGE_FALLBACK: &str =
    r#"{"version":"1.0","children":[{"type":"rectangle","id":"r1","name":"rect"}]}"#;

#[test]
fn document_json_contains_page_id() {
    let mut v = super::Viewer::placeholder();
    v.load(DOC_WITH_PAGE).unwrap();
    assert!(v.document_json().unwrap().contains("pageX"));
}

#[test]
fn pages_json_contains_page_id() {
    let mut v = super::Viewer::placeholder();
    v.load(DOC_WITH_PAGE).unwrap();
    assert!(v.pages_json().unwrap().contains("pageX"));
}

#[test]
fn viewport_json_round_trips() {
    let mut v = super::Viewer::placeholder();
    v.set_viewport(5.0, -10.0, 2.0);
    let j = v.viewport_json().unwrap();
    // Must contain the pan_x field and the exact values we set.
    assert!(j.contains("pan_x"), "expected pan_x field in: {j}");
    assert!(j.contains('5'), "expected pan_x value 5 in: {j}");
    assert!(j.contains('2'), "expected zoom value 2 in: {j}");
}

#[test]
fn document_json_returns_empty_on_no_doc() {
    let v = super::Viewer::placeholder();
    // No document loaded — must return exactly "{}" (the documented default).
    let j = v.document_json().unwrap();
    assert_eq!(
        j, "{}",
        "document_json must return \"{{}}\" when no doc is loaded, got: {j}"
    );
}

#[test]
fn pages_json_returns_empty_array_on_no_doc() {
    let v = super::Viewer::placeholder();
    assert_eq!(v.pages_json().unwrap(), "[]");
}

#[test]
fn pages_json_single_page_fallback_exposes_children() {
    let mut v = super::Viewer::placeholder();
    v.load(DOC_SINGLE_PAGE_FALLBACK).unwrap();
    let j = v.pages_json().unwrap();
    // Must be a non-empty JSON array containing our node.
    assert!(
        j.starts_with('[') && !j.starts_with("[]"),
        "expected non-empty array for single-page fallback, got: {j}"
    );
    assert!(
        j.contains("rectangle"),
        "expected node type value in pages_json result: {j}"
    );
    // Synthetic page must carry the documented sentinel fields.
    assert!(
        j.contains("\"id\":\"default\""),
        "expected synthetic page id in: {j}"
    );
    assert!(
        j.contains("\"name\":\"Page 1\""),
        "expected synthetic page name in: {j}"
    );
}

#[test]
fn pages_json_no_pages_no_children_returns_empty_array() {
    // Document with neither pages nor children → "[]"
    let src = r#"{"version":"1.0","children":[]}"#;
    let mut v = super::Viewer::placeholder();
    v.load(src).unwrap();
    assert_eq!(v.pages_json().unwrap(), "[]");
}
