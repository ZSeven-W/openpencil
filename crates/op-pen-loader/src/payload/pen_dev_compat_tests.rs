//! pen.dev 2.21 documents load through the canonical loader (issues #270,
//! #272).

use super::*;
use crate::payload::load_canonical;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

fn names(nodes: &[PenNode]) -> Vec<&str> {
    nodes
        .iter()
        .map(|node| node.base().name.as_deref().unwrap_or(""))
        .collect()
}

const PEN_DEV_DOC: &str = r##"{
  "version": "2.21",
  "variables": {
    "font-body": { "type": "number", "value": 15 },
    "radius-md": { "type": "number", "value": 12 },
    "surface-background": { "type": "color", "value": "#F4F4F5" },
    "pc-font": { "type": "string", "value": "Noto Sans SC" }
  },
  "children": [
    { "type": "frame", "id": "a", "name": "Board A", "x": 0, "y": 0, "width": 375, "height": 812 },
    { "type": "frame", "id": "screen", "name": "Screen", "x": 400, "y": 0,
      "width": 375, "height": "fit_content(812)", "layout": "vertical",
      "fill": "$surface-background", "clip": true, "cornerRadius": "$radius-md",
      "children": [
        { "type": "frame", "id": "nav", "name": "topNav", "width": "fill_container", "height": 44 },
        { "type": "frame", "id": "body", "name": "body", "width": "fill_container", "height": 600,
          "stroke": "#E4E4E7", "strokeWidth": 2, "strokeAlignment": "inner",
          "effect": { "type": "shadow", "offset": { "x": 0, "y": 4 }, "blur": 12, "color": "#00000020" } },
        { "type": "frame", "id": "mask", "name": "mask", "layoutPosition": "absolute",
          "x": 0, "y": 0, "width": 375, "height": 812, "fill": "#00000066" },
        { "type": "frame", "id": "sheet", "name": "actionSheet", "layoutPosition": "absolute",
          "x": 0, "y": 500, "width": 375, "height": 312,
          "fill": [{ "type": "image", "url": "sheet.png", "mode": "contain" }] },
        { "type": "text", "id": "label", "name": "label", "fontSize": "$font-body", "fontFamily": "$pc-font" },
        { "type": "note", "id": "memo", "name": "memo", "content": "Check copy" }
      ] }
  ]
}"##;

#[test]
fn only_native_pen_dev_versions_are_detected() {
    assert!(is_pen_dev_document(&json!({"version": "2.21"})));
    assert!(is_pen_dev_document(&json!({"version": "2.8"})));
    assert!(!is_pen_dev_document(&json!({"version": "1.0.0"})));
    assert!(!is_pen_dev_document(
        &json!({"version": "2.21", "formatVersion": "1.0"})
    ));
}

#[test]
fn pen_dev_document_loads_with_op_paint_order() {
    let doc = load_canonical(PEN_DEV_DOC)
        .expect("pen.dev 2.21 document loads")
        .value;
    // Boards: last-on-top → first-on-top.
    assert_eq!(names(&doc.children), ["Screen", "Board A"]);
    let screen = &doc.children[0];
    // Overlays stay topmost; flow children keep their flow order.
    assert_eq!(
        names(screen.children().expect("screen children")),
        ["topNav", "body", "actionSheet", "mask", "label", "memo"]
    );
    let value = serde_json::to_value(screen).expect("serialize");
    assert_eq!(value["height"], "fit_content");
    assert_eq!(value["clipContent"], true);
    // Variable links survive the load (issue #270).
    assert_eq!(value["cornerRadius"], "$radius-md");
    assert_eq!(value["fill"][0]["color"], "$surface-background");
    let child = |name: &str| {
        value["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .cloned()
            .unwrap()
    };
    assert!(child("mask")["constraints"].is_object());
    assert_eq!(child("actionSheet")["fill"][0]["mode"], "fit");
    let body = child("body");
    assert_eq!(body["stroke"]["thickness"], 2.0);
    assert_eq!(body["stroke"]["align"], "inside");
    assert_eq!(body["effects"][0]["offsetY"], 4.0);
    let label = child("label");
    assert_eq!(label["type"], "text");
    assert_eq!(label["fontSize"], "$font-body");
    assert_eq!(label["content"], "");
    assert_eq!(child("memo")["type"], "text");
}

#[test]
fn pen_dev_linear_gradient_rotation_maps_to_op_angle() {
    let mut fill = json!([{ "type": "gradient", "gradientType": "linear", "rotation": 90 }]);
    assert!(convert_paints(&mut fill));
    assert_eq!(fill[0]["rotation"], 270.0);
}

/// Issue #270: linked font sizes and radii render at the variable's value
/// and keep the link, so editing the token reaches every node.
#[test]
fn variable_font_size_and_radius_resolve_for_canvas_and_round_trip() {
    let doc = load_canonical(PEN_DEV_DOC).expect("loads").value;
    let resolved =
        op_editor_core::variables_resolve::resolve_document_for_canvas(&doc, &Default::default());
    let screen = serde_json::to_value(&resolved.children[0]).expect("serialize");
    assert_eq!(screen["cornerRadius"], 12.0);
    let label = screen["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "label")
        .cloned()
        .unwrap();
    assert_eq!(label["fontSize"], 15.0);
    // The stored document keeps the refs through a save / reload.
    let saved = serde_json::to_string(&doc).expect("save");
    assert!(saved.contains(r#""fontSize":"$font-body""#), "{saved}");
    assert!(saved.contains(r#""cornerRadius":"$radius-md""#), "{saved}");
    let reloaded = load_canonical(&saved).expect("reload").value;
    assert_eq!(reloaded, doc);
}
