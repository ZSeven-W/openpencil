use super::*;
use base64::Engine as _;

#[test]
fn translucent_subtitle_does_not_force_opaque_tiles_behind_a_white_headline() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    let doc = serde_json::from_value(json!({"version":"1.0.0",
        "variables":{"--foreground":{"type":"color","value":"#111111"},"--background":{"type":"color","value":"#FFFFFF"}},
        "children":[{"type":"frame","id":"hero","width":375,"height":300,"layout":"none","children":[
            {"type":"text","id":"title","x":24,"y":100,"width":200,"height":32,"content":"静谧之诗","fontSize":24,"fill":[{"type":"solid","color":"#FFFFFF"}]},
            {"type":"text","id":"subtitle","x":24,"y":146,"width":200,"height":24,"content":"天然植萃","fontSize":16,"fill":[{"type":"solid","color":"#FFFFFFCC"}]},
            {"type":"image","id":"photo","width":375,"height":300,"src":src}
        ]}]})).unwrap();
    let mut state = EditorState::from_document(doc);
    assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 1);
    let value = serde_json::to_value(&state.doc).unwrap();
    assert!(value.to_string().contains("Photo contrast scrim"));
    assert!(!value.to_string().contains("Text contrast backing"));
    assert_eq!(
        value["children"][0]["children"][1]["fill"][0]["color"],
        "#FFFFFFCC"
    );
    assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 0);
}

#[test]
fn image_landing_settles_nested_actions_against_final_mobile_rail() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    let doc = serde_json::from_value(json!({"version":"1.0.0",
        "variables":{"--foreground":{"type":"color","value":"#111111"},"--background":{"type":"color","value":"#FFFFFF"}},
        "children":[{"type":"frame","id":"root","width":390,"height":844,"layout":"vertical","clipContent":true,
            "children":[{"type":"frame","id":"hero","width":342,"height":236,"layout":"none","clipContent":true,
                "children":[
                    {"type":"text","id":"caption","x":24,"y":140,"width":280,"height":32,"content":"今日推荐","fontSize":20,"fill":[{"type":"solid","color":"#FFFFFF"}]},
                    {"type":"frame","id":"actions","x":24,"y":190,"width":342,"height":44,"layout":"none",
                        "children":[{"type":"frame","id":"add","role":"button","x":298,"y":0,"width":44,"height":44,
                            "fill":[{"type":"solid","color":"#111111"}]}]},
                    {"type":"image","id":"photo","width":390,"height":236,"src":src}
                ]}]}]})).unwrap();
    let mut state = EditorState::from_document(doc);
    repair_for_image(&mut state, &NodeId::new("photo"));
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    let page = scene.active_page().unwrap();
    let hero = page.find("hero").unwrap().bounds;
    let add = page.find("add").unwrap().bounds;
    assert!(
        add.origin.x + add.size.x <= hero.origin.x + hero.size.x + 0.5,
        "{add:?} {hero:?}"
    );
    assert_eq!(page.find("root").unwrap().bounds.size.y, 844.0);
    let settled = state.doc.clone();
    assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 0);
    assert_eq!(settled, state.doc);
}

#[test]
fn loose_photo_captions_share_one_gradual_scrim() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    let doc = serde_json::from_value(json!({"version":"1.0.0",
        "variables":{"--foreground":{"type":"color","value":"#111111"},"--background":{"type":"color","value":"#FFFFFF"}},
        "children":[{"type":"frame","id":"root","width":375,"height":300,"layout":"none","children":[
            {"type":"text","id":"title","x":24,"y":100,"width":200,"height":32,"content":"燕麦拿铁","fontSize":20,"fill":[{"type":"solid","color":"#FFFFFF"}]},
            {"type":"frame","id":"price-row","x":24,"y":150,"layout":"horizontal","gap":8,"children":[
                {"type":"text","id":"price","width":"fit_content","height":"fit_content","content":"32元","fontSize":18,"fill":[{"type":"solid","color":"#FFFFFF"}]},
                {"type":"text","id":"caption","width":"fit_content","height":"fit_content","content":"今日推荐","fontSize":12,"fill":[{"type":"solid","color":"#FFFFFF"}]}
            ]},
            {"type":"image","id":"photo","width":375,"height":300,"src":src}
        ]}]})).unwrap();
    let mut state = EditorState::from_document(doc);
    assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 1);
    let value = serde_json::to_value(&state.doc).unwrap();
    let kids = value["children"][0]["children"].as_array().unwrap();
    let price_at = kids.iter().position(|n| n["id"] == "price-row").unwrap();
    assert_eq!(price_at, 1);
    let scrim = &kids[price_at + 1];
    assert_eq!(scrim["name"], "Photo contrast scrim");
    assert_eq!(scrim["width"], 375.0);
    assert_eq!(scrim["height"], 300.0);
    assert_eq!(scrim["fill"][0]["type"], "linear_gradient");
    assert!(!value.to_string().contains("Text contrast backing"));
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
    let page = scene.active_page().unwrap();
    let root = &state.active_children()[0];
    let nodes = node_children_of(root);
    let variables = state.doc.variables.clone().unwrap();
    let theme = op_design_lint::node_util::default_theme(state.doc.themes.as_ref());
    for (id, branch) in [("title", 0), ("price", 1), ("caption", 1)] {
        let rect = page.find(id).unwrap().bounds;
        let colors = layers::composite_samples(
            &[[245, 245, 245]; 25],
            &nodes[branch + 1..nodes.len() - 1],
            page,
            [
                rect.origin.x as f64,
                rect.origin.y as f64,
                rect.size.x as f64,
                rect.size.y as f64,
            ],
            &variables,
            &theme,
        )
        .unwrap();
        assert!(
            colors
                .iter()
                .all(|bg| op_design_lint::color::color_contrast("#FFFFFF", bg) >= TARGET_RATIO),
            "{id}"
        );
    }
    let settled = state.doc.clone();
    assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 0);
    assert_eq!(
        state.doc, settled,
        "a second image callback must not duplicate panels"
    );
}

#[test]
fn a_photo_scrim_is_measured_before_adding_a_caption_panel() {
    let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([245, 245, 245, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
    );
    for (alpha, expected) in [("D9", 0), ("55", 1)] {
        let doc = serde_json::from_value(json!({"version":"1.0.0",
            "variables":{"--foreground":{"type":"color","value":"#111111"},"--background":{"type":"color","value":"#FFFFFF"}},
            "children":[{"type":"frame","id":"root","width":375,"height":300,"layout":"none","children":[
                {"type":"text","id":"caption","x":24,"y":240,"width":200,"height":32,"content":"今日推荐","fontSize":20,"fill":[{"type":"solid","color":"#FFFFFF"}]},
                {"type":"rectangle","id":"scrim","width":375,"height":300,"fill":[{"type":"linear_gradient","angle":180,
                    "stops":[{"offset":0,"color":"#00000000"},{"offset":1,"color":format!("#000000{alpha}")}]}]},
                {"type":"image","id":"photo","width":375,"height":300,"src":src}
            ]}]})).unwrap();
        let mut state = EditorState::from_document(doc);
        assert_eq!(
            repair_for_image(&mut state, &NodeId::new("photo")),
            expected,
            "scrim={alpha}"
        );
        assert_eq!(repair_for_image(&mut state, &NodeId::new("photo")), 0);
        let caption =
            op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("caption"))
                .unwrap();
        assert_eq!(
            serde_json::to_value(caption).unwrap()["fill"][0]["color"],
            "#FFFFFF"
        );
    }
}

#[test]
fn a_translucent_badge_measures_the_photo_beneath_its_tint() {
    for (pixel, expected) in [([30, 30, 30, 255], 1), ([245, 245, 245, 255], 0)] {
        let image = image::RgbaImage::from_pixel(20, 10, image::Rgba(pixel));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let src = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        );
        let doc = serde_json::from_value(json!({"version":"1.0.0",
            "variables":{"--foreground":{"type":"color","value":"#111111"},
                         "--background":{"type":"color","value":"#FFFFFF"}},
            "children":[{"type":"frame","id":"root","width":375,"height":300,"layout":"none","children":[
                {"type":"frame","id":"badge","x":24,"y":120,"layout":"horizontal","padding":[4,12],"cornerRadius":8,
                    "fill":[{"type":"solid","color":"#FF5A1F14"}],"children":[
                        {"type":"text","id":"caption","width":"fit_content","height":"fit_content","content":"今日推荐","fontSize":11,"fill":[{"type":"solid","color":"#111111"}]}
                    ]},
                {"type":"image","id":"photo","width":375,"height":300,"src":src}
            ]}]})).unwrap();
        let mut state = EditorState::from_document(doc);
        assert_eq!(
            repair_for_image(&mut state, &NodeId::new("photo")),
            expected,
            "photo={pixel:?}"
        );
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        assert_eq!(repair(&mut sink, "root"), 0);
        let settled = sink.state().doc.clone();
        assert_eq!(repair(&mut sink, "root"), 0);
        super::super::repair_text_contrast(&mut sink, "root");
        assert_eq!(sink.state().doc, settled);
        let caption = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new("caption"),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(caption).unwrap()["fill"][0]["color"],
            "#111111"
        );
    }
}

#[test]
fn a_bright_photo_gets_readable_scrim_without_recoloring_or_losing_ink() {
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
    assert_eq!(children[1]["name"], "Photo contrast scrim");
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
    assert!(value.to_string().contains("Photo contrast scrim"));
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
