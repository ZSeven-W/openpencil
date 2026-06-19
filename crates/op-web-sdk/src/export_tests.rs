// Tests for Viewer::export_svg and the wasm `export` dispatcher.

#[test]
fn export_svg_emits_svg_root() {
    let mut v = super::Viewer::placeholder();
    v.load(r##"{"version":"1.0","pages":[{"id":"p","name":"P","children":[{"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,"fill":[{"type":"solid","color":"#ff0000"}]}]}]}"##).unwrap();
    v.rebuild_scene();
    let svg = v.export_svg().expect("svg");
    assert!(svg.trim_start().starts_with("<svg"));
}

#[test]
fn export_svg_fails_without_scene() {
    let v = super::Viewer::placeholder();
    let result = v.export_svg();
    assert!(result.is_err());
}
