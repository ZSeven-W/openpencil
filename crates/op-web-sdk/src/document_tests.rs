// Tests for Viewer document parsing and page snapshot accessors.
const DOC: &str = r#"{"version":"1.0","pages":[{"id":"p1","name":"Page 1","children":[]}]}"#;

#[test]
fn load_parses_pages() {
    let mut v = super::Viewer::placeholder();
    v.load(DOC).expect("parse");
    assert_eq!(v.page_count(), 1);
    assert_eq!(v.active_page_index(), 0);
}
