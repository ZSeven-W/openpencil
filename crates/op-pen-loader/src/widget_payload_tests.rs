use crate::pen_document_to_payload;

fn assert_child_payload(src: &str, radius: f32, kind: &str) {
    let parsed = jian_ops_schema::load_str(src).expect("canonical load");
    let loaded = pen_document_to_payload(&parsed.value);
    let node = &loaded.payload.pages[0].children[0];

    assert_eq!(node.corner_radius, radius);
    assert_eq!(node.widget.as_ref().expect("widget payload").kind, kind);
}

#[test]
fn text_input_payload_carries_corner_radius() {
    assert_child_payload(
        r##"{
          "version":"1.0.0",
          "pages":[{"id":"p","name":"P","children":[{
            "type":"text_input","id":"search","width":160,"height":36,
            "placeholder":"Search","cornerRadius":8,
            "fill":[{"type":"solid","color":"#F8FAFC"}],
            "stroke":{"fill":[{"type":"solid","color":"#CBD5E1"}],"thickness":1}
          }]}],
          "children":[]
        }"##,
        8.0,
        "text_input",
    );
}

#[test]
fn number_input_payload_carries_corner_radius() {
    assert_child_payload(
        r##"{
          "version":"1.0.0",
          "pages":[{"id":"p","name":"P","children":[{
            "type":"number_input","id":"amount","width":120,"height":36,
            "placeholder":"0","cornerRadius":6,
            "fill":[{"type":"solid","color":"#F8FAFC"}],
            "stroke":{"fill":[{"type":"solid","color":"#CBD5E1"}],"thickness":1}
          }]}],
          "children":[]
        }"##,
        6.0,
        "number_input",
    );
}
