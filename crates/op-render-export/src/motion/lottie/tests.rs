use super::*;
use crate::motion::{export_motion_bytes, ClipTrigger, MotionFormat};

fn export(source: &str, options: MotionExportOptions) -> Value {
    let doc = jian_ops_schema::load_str(source).unwrap().value;
    let scene = op_pen_loader::pen_document_to_layout_scene(&doc, &Default::default(), 0);
    let bytes =
        export_motion_bytes(&scene, &doc.children[0], MotionFormat::Lottie, &options).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn compositions(value: &Value) -> impl Iterator<Item = &Value> {
    value["assets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v.get("layers").is_some())
}

fn named_layer<'a>(value: &'a Value, name: &str) -> &'a Value {
    compositions(value)
        .flat_map(|v| v["layers"].as_array().unwrap())
        .find(|v| v["nm"] == name)
        .unwrap()
}

#[test]
fn vector_export_keeps_initial_transparency_and_the_exact_final_sample() {
    let (root, scene) = super::super::tests::fixture();
    let bytes = export_motion_bytes(
        &scene,
        &root,
        MotionFormat::Lottie,
        &MotionExportOptions::default(),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        (value["w"].as_i64(), value["h"].as_i64()),
        (Some(16), Some(12))
    );
    assert_eq!(value["op"], 4);
    assert_eq!(value["fr"], 30);
    assert_eq!(value["meta"]["warnings"], json!([]));
    let box_layer = named_layer(&value, "box");
    assert_eq!(box_layer["ty"], 4);
    let fill = find_shape(box_layer, "fl").unwrap();
    assert_eq!(fill["o"]["k"][0]["s"], json!([0.0]));
    assert_eq!(fill["o"]["k"][3]["s"], json!([100.0]));
    assert!(compositions(&value).all(|v| v.get("p").is_none()));
}

#[test]
fn text_is_outlined_and_can_play_without_installed_fonts() {
    let value = export(
        r##"{"version":"1.1","children":[{"type":"frame","id":"board","width":180,"height":80,
        "layout":"none","children":[{"type":"text","id":"text","content":"Hello","fontSize":32,
        "fontFamily":"Arial","width":160,"height":60,"fill":[{"type":"solid","color":"#ff0000"}],
        "animations":[{"trigger":"mount","durationMs":100,"keyframes":[{"offset":0,"values":{"translateX":-8}},
        {"offset":1,"values":{"translateX":0}}]}]}]}]}"##,
        MotionExportOptions::default(),
    );
    assert_eq!(value["meta"]["warnings"], json!([]));
    assert!(value["assets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v.get("p").is_none()));
    let text = named_layer(&value, "text");
    assert_eq!(text["ty"], 4);
    assert!(!text["shapes"].as_array().unwrap().is_empty());
}

#[test]
fn clips_and_opacity_apply_to_the_assembled_subtree() {
    let value = export(
        r##"{"version":"1.1","children":[{"type":"frame","id":"board","width":100,"height":80,
        "clipContent":true,"cornerRadius":12,"layout":"none","animations":[{"trigger":"mount","durationMs":100,
        "keyframes":[{"offset":0,"values":{"rotation":0}},{"offset":1,"values":{"rotation":5}}]}],
        "children":[{"type":"rectangle","id":"child","x":-20,"y":0,"width":160,"height":50,
        "fill":[{"type":"solid","color":"#ff0000"}]}]}]}"##,
        MotionExportOptions::default(),
    );
    let mask = named_layer(&value, "Clipped children");
    assert_eq!(mask["hasMask"], true);
    assert_eq!(mask["masksProperties"][0]["pt"]["k"]["c"], true);
    assert_eq!(
        mask["masksProperties"][0]["pt"]["k"]["v"][0],
        json!([12.0, 0.0])
    );
    assert_eq!(value["layers"][0]["ks"]["r"]["a"], 1);
    assert_eq!(named_layer(&value, "child")["ks"]["o"]["k"], 100.0);
}

#[test]
fn unsupported_effects_have_embedded_pixels_and_a_warning() {
    let value = export(
        r##"{"version":"1.1","children":[{"type":"rectangle","id":"shadowed","width":20,"height":20,
        "fill":[{"type":"solid","color":"#00ff00"}],"effects":[{"type":"blur","radius":3}],
        "animations":[{"trigger":"mount","durationMs":100,"keyframes":[{"offset":0,"values":{"opacity":0}},
        {"offset":1,"values":{"opacity":1}}]}]}]}"##,
        MotionExportOptions::default(),
    );
    let images: Vec<_> = value["assets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v.get("p").is_some())
        .collect();
    assert!(!images.is_empty());
    assert!(images.iter().all(|v| v["p"]
        .as_str()
        .unwrap()
        .starts_with("data:image/png;base64,")));
    assert!(!value["meta"]["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn manual_trigger_scale_background_and_reduced_motion_share_existing_options() {
    let value = export(
        r##"{"version":"1.1","children":[{"type":"rectangle","id":"box","width":20,"height":12,
        "fill":[{"type":"solid","color":"#00ff00"}],"animations":[{"trigger":"manual","id":"intro","durationMs":100,
        "keyframes":[{"offset":0,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}]}]}]}"##,
        MotionExportOptions {
            trigger: ClipTrigger::Manual("intro".into()),
            scale: 2.0,
            background: MotionExportBackground::White,
            reduced_motion: true,
            ..Default::default()
        },
    );
    assert_eq!(value["w"], 40);
    assert_eq!(value["h"], 24);
    assert_eq!(value["layers"][1]["sc"], "#ffffff");
    assert_eq!(named_layer(&value, "box")["ks"]["o"]["a"], 0);
}

#[test]
fn glyph_outlines_follow_the_native_alphabetic_baseline() {
    use op_editor_ui::{Color, Point2D, TextLayout};
    let mut backend = op_host_native::NativeBackend::with_dpi(1.0);
    let layout = TextLayout::single_run("H", "Arial", 32.0, Color::BLACK.to_jian(), Point2D::ZERO);
    let first = backend.outline_text(&layout, Point2D::ZERO).unwrap();
    let moved = backend
        .outline_text(&layout, Point2D::new(20.0, 40.0))
        .unwrap();
    let first = first[0].0.bounds();
    let moved = moved[0].0.bounds();
    assert!(first.top() < 0.0);
    assert!(first.bottom() <= 0.1);
    assert!((moved.left() - first.left() - 20.0).abs() < 0.01);
    assert!((moved.top() - first.top() - 40.0).abs() < 0.01);
}

fn find_shape<'a>(value: &'a Value, kind: &str) -> Option<&'a Value> {
    match value {
        Value::Object(fields) => {
            if fields.get("ty").and_then(Value::as_str) == Some(kind) {
                return Some(value);
            }
            fields.values().find_map(|field| find_shape(field, kind))
        }
        Value::Array(values) => values.iter().find_map(|field| find_shape(field, kind)),
        _ => None,
    }
}
