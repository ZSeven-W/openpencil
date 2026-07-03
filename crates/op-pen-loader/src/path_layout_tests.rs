use crate::pen_document_to_payload;

fn load_widths(src: &str) -> (f32, f32) {
    let parsed = jian_ops_schema::load_str(src).expect("canonical load");
    let loaded = pen_document_to_payload(&parsed.value);
    let root = &loaded.payload.pages[0].children[0];
    let fit_path = root
        .children
        .iter()
        .find(|node| node.id == "fit-path")
        .expect("fill path");
    let fixed_path = root
        .children
        .iter()
        .find(|node| node.id == "fixed-path")
        .expect("fixed path");
    (fit_path.w, fixed_path.w)
}

#[test]
fn path_fill_container_width_resolves_inside_none_layout_parent() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{
        "id":"p","name":"P",
        "children":[{
          "type":"frame","id":"root","width":320,"height":260,"layout":"none",
          "children":[
            {"type":"path","id":"fit-path","x":0,"y":0,"width":"fill_container","height":240,
             "d":"M0 0 L640 120 L0 240"},
            {"type":"path","id":"fixed-path","x":0,"y":0,"width":120,"height":40,
             "d":"M0 0 L120 40"}
          ]
        }]
      }],
      "children":[]
    }"##;

    let (fit_w, fixed_w) = load_widths(src);

    assert_eq!(
        fit_w, 320.0,
        "fill_container path should resolve to the numeric parent width"
    );
    assert_eq!(fixed_w, 120.0, "numeric path width must stay authored");
}
