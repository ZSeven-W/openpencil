use super::*;
use base64::Engine as _;

#[test]
fn a_bright_photo_gets_readable_backing_without_recoloring_or_losing_ink() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    let doc=serde_json::from_value(json!({"version":"1.0.0","themes":{"Mode":["Light"]},
        "variables":{"--foreground":{"type":"color","value":[{"value":"#111111","theme":{"Mode":"Light"}}]},"--background":{"type":"color","value":[{"value":"#FFFFFF","theme":{"Mode":"Light"}}]}},
        "children":[{"type":"frame","id":"root","width":390,"height":186,"layout":"none","children":[
            {"type":"text","id":"caption","x":20,"y":100,"width":220,"height":48,"fontSize":32,"content":"Classic latte","fill":[{"type":"solid","color":"#FFFFFF"}]},
            {"type":"image","id":"photo","width":390,"height":186,"src":src}
        ]}]})).unwrap();
    let mut state = EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    assert_eq!(repair(&mut sink, "root"), 1);
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    let children = value["children"][0]["children"].as_array().unwrap();
    assert_eq!(children[0]["id"], "caption");
    assert_eq!(children[0]["fill"][0]["color"], "#FFFFFF");
    assert_eq!(children[1]["name"], "Text contrast backing");
    assert_eq!(children[2]["id"], "photo");
    assert!(op_design_lint::color::color_contrast("#FFFFFF", "#111111") >= TARGET_RATIO);
    assert_eq!(repair(&mut sink, "root"), 0);
    super::super::repair_text_contrast(&mut sink, "root");
    let current = serde_json::to_value(&sink.state().doc).unwrap();
    assert_eq!(
        current["children"][0]["children"][0]["fill"][0]["color"],
        "#FFFFFF"
    );
}

#[test]
fn whole_finalize_measures_photo_contrast_after_fitting_the_caption() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    let doc = serde_json::from_value(json!({"version":"1.0.0","themes":{"Mode":["Light"]},
        "variables":{"--foreground":{"type":"color","value":"#111111"},
                     "--background":{"type":"color","value":"#FFFFFF"}},
        "children":[{"type":"frame","id":"root","width":390,"height":844,"layout":"vertical",
            "children":[{"type":"frame","id":"hero","width":390,"height":204,"layout":"none","clipContent":true,
                "children":[
                    {"type":"image","id":"photo","width":390,"height":204,"src":src},
                    {"type":"frame","id":"copy","x":24,"y":118,"width":280,"layout":"vertical","gap":8,
                        "children":[
                            {"type":"text","id":"title","content":"经典拿铁","fontSize":32,"lineHeight":1.5,"fill":[{"type":"solid","color":"#FFFFFF"}]},
                            {"type":"text","id":"description","content":"双份浓缩 × 丝滑奶泡","fontSize":16,"lineHeight":1.5,"fill":[{"type":"solid","color":"#FFFFFF"}]},
                            {"type":"text","id":"price","content":"¥28","fontSize":34,"lineHeight":1.5,"fill":[{"type":"solid","color":"#FFFFFF"}]}
                        ]}
                ]}]}]})).unwrap();
    let mut state = EditorState::from_document(doc);
    crate::apply_loop_finalize(&mut state);
    let price =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("price")).unwrap();
    assert_eq!(
        serde_json::to_value(price).unwrap()["fill"][0]["color"],
        "#FFFFFF"
    );
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    let page = scene.active_page().unwrap();
    let hero = page.find("hero").unwrap().bounds;
    let copy = page.find("copy").unwrap().bounds;
    assert!(copy.origin.y + copy.size.y <= hero.origin.y + hero.size.y + 0.5);
    let value = serde_json::to_value(&state.doc).unwrap();
    assert!(value.to_string().contains("Text contrast backing"));
    let photo =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("photo")).unwrap();
    assert_eq!(serde_json::to_value(photo).unwrap()["src"], src);
    crate::apply_loop_finalize(&mut state);
    let price =
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("price")).unwrap();
    assert_eq!(
        serde_json::to_value(price).unwrap()["fill"][0]["color"],
        "#FFFFFF"
    );
}
