// Tests for Viewer::rebuild_scene / scene().
const DOC: &str = r#"{"version":"1.0","pages":[{"id":"p1","name":"P","children":[{"type":"rectangle","id":"r1","x":0,"y":0,"width":10,"height":10}]}]}"#;

#[test]
fn rebuild_scene_yields_one_page() {
    let mut v = super::Viewer::placeholder();
    v.load(DOC).unwrap();
    v.rebuild_scene();
    let scene = v.scene().expect("scene built");
    assert_eq!(scene.pages.len(), 1);
}
