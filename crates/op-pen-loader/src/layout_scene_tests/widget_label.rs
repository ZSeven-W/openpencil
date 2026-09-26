//! A checkbox / radio label is page text: the scene builder hands the
//! painter the document's `--foreground` so the label never derives its
//! colour from checked-state accent paint.

use super::*;
use jian_scene::layout_scene::SceneWidget;

fn checkbox_doc(variables: &str) -> String {
    format!(
        r##"{{
      "version":"1.0.0",
      "variables":{{{variables}}},
      "pages":[{{"id":"p","name":"P","children":[
        {{"type":"checkbox","id":"cb","x":0,"y":0,"width":160,"height":22,
         "checked":true,"label":"Ship it",
         "fill":[{{"type":"solid","color":"#2563EB"}}],
         "stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#2563EB"}}]}}}}
      ]}}],"children":[]
    }}"##
    )
}

fn widget(src: &str) -> SceneWidget {
    let scene = editor_state_to_layout_scene(&state_from(src));
    scene.pages[0]
        .children
        .iter()
        .find(|n| n.id == "cb")
        .and_then(|n| n.widget.clone())
        .expect("checkbox scene widget")
}

#[test]
fn document_foreground_reaches_the_checkbox_label() {
    let w = widget(&checkbox_doc(
        r##""--foreground":{"type":"color","value":"#111827"}"##,
    ));
    assert_eq!(w.label_foreground, Some(Color::rgb_u8(0x11, 0x18, 0x27)));
}

#[test]
fn missing_foreground_leaves_the_label_to_the_widget_policy() {
    let w = widget(&checkbox_doc(""));
    assert_eq!(w.label_foreground, None);
}
