//! Tests for the browser-snapshot importer.

use super::*;
use crate::HtmlImportOptions;
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::PenNode;

const SAMPLE: &str = include_str!("../tests/fixtures/snapshot_v1_sample.json");

#[test]
fn snapshot_extractor_contract_markers_are_present() {
    for marker in [
        "getComputedStyle",
        "getBoundingClientRect",
        "createRange",
        "toDataURL",
        "clipboard.writeText",
        "snapshot.json",
        "version: 1",
        "truncated",
    ] {
        assert!(
            SNAPSHOT_EXTRACTOR_JS.contains(marker),
            "extractor is missing {marker}"
        );
    }
}

/// Source-level guards for the capture-side invariants this importer depends
/// on. They cannot be exercised from Rust — they need a live DOM — so the
/// contract is pinned where it is written instead.
#[test]
fn snapshot_extractor_pins_its_geometry_contract() {
    // The vector box is the union of the shapes that contribute path data.
    // Sizing it from the root `svg.getBBox()` counted the skipped ones too —
    // Material's `<path fill="none" d="M0 0h24v24H0z"/>` sizing rect doubled
    // every glyph.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("unionBox(current.box, segmentBox)"),
        "vector box must be the union of contributing shapes"
    );
    assert!(
        !SNAPSHOT_EXTRACTOR_JS.contains("= svg.getBBox()"),
        "the root bbox counts shapes that paint nothing"
    );
    // Generated primitives all wind clockwise, so merging them under the
    // `nonzero` rule unions instead of punching a hole.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("\" 0 1 1 \""),
        "ellipse arcs must sweep clockwise like the rect branch"
    );
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("if (signedArea(points) < 0) points.reverse();"),
        "a points ring must be normalized to a clockwise winding"
    );
    // A rotated / skewed CTM cannot be expressed by an axis-aligned rect.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("CTM_SKEW_EPSILON"),
        "a non-axis-aligned CTM must fall back to the image path"
    );
    // Boundary whitespace is only kept where a sibling actually sits.
    assert!(
        SNAPSHOT_EXTRACTOR_JS
            .contains("if (!textNode.previousSibling) text = text.replace(/^ /, \"\");"),
        "leading boundary space needs a previous sibling"
    );
    assert!(
        SNAPSHOT_EXTRACTOR_JS
            .contains("if (!textNode.nextSibling) text = text.replace(/ $/, \"\");"),
        "trailing boundary space needs a next sibling"
    );
    // Vector geometry rides alongside the image serialization; the node keeps
    // `kind: "image"` so an importer that predates it still reads a node.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("result.vectorRect = fragments[0].rect;"),
        "the artwork box must travel next to the element rect"
    );
    assert!(
        !SNAPSHOT_EXTRACTOR_JS.contains("kind: \"vector\""),
        "a vector-only node kind is dropped by importers that predate it"
    );
    // The parent's display decides whether a static child's z-index applies.
    assert!(
        SNAPSHOT_EXTRACTOR_JS.contains("\"display\","),
        "display is a stacking input for flex / grid items"
    );
}

#[test]
fn sample_snapshot_converts_to_absolute_tree() {
    let result = import_snapshot(SAMPLE, &HtmlImportOptions::default());
    assert!(result.nodes.len() == 1, "warnings: {:?}", result.warnings);
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    assert!(matches!(
        root.container.layout,
        None | Some(LayoutMode::None)
    ));
    // Canonical children are front-to-back, so the image declared last in the
    // DOM (and therefore painted on top by CSS) comes first.
    let children = root.children.as_ref().unwrap();
    let PenNode::Frame(card) = &children[1] else {
        panic!("card frame")
    };
    assert_eq!(card.base.x, Some(24.0));
    assert_eq!(card.base.y, Some(24.0));
    use jian_ops_schema::sizing::SizingBehavior;
    use jian_ops_schema::style::StrokeThickness;
    assert!(matches!(card.container.width, Some(SizingBehavior::Number(w)) if w == 300.0));
    assert!(matches!(
        card.container.stroke.as_ref().map(|stroke| &stroke.thickness),
        Some(StrokeThickness::Uniform(width)) if *width == 1.0
    ));
    let PenNode::Text(text) = &card.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    assert_eq!(text.base.x, Some(16.0));
    assert_eq!(text.font_size_px(), Some(16.0));
    assert_eq!(text.line_height, Some(1.5));
    let PenNode::Image(image) = &children[0] else {
        panic!("image")
    };
    assert!(image.src.as_str().starts_with("data:image/png"));
}

#[test]
fn computed_order_box_shadow_parses() {
    let result = import_snapshot(SAMPLE, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Frame(card) = &root.children.as_ref().unwrap()[1] else {
        panic!()
    };
    let effects = card.container.effects.as_ref().expect("shadow");
    assert!(
        matches!(&effects[0], jian_ops_schema::style::PenEffect::Shadow(shadow)
        if shadow.offset_y == 4.0 && shadow.blur == 8.0 && shadow.color == "#00000040")
    );
}

/// The hero pattern that wiped whole pages out: a full-bleed background
/// declared first, overlay content after it. CSS paints the overlay on top;
/// canonical order has to put it in `children[0]`.
#[test]
fn full_bleed_background_does_not_bury_the_hero_content() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "section",
        "rect": { "x": 0, "y": 0, "w": 1440, "h": 760 },
        "styles": {},
        "children": [
          { "kind": "image", "tag": "video",
            "rect": { "x": 0, "y": 0, "w": 1440, "h": 760 },
            "src": "https://cdn.example.com/hero.jpg",
            "styles": { "position": "absolute", "object-fit": "cover" } },
          { "kind": "element", "tag": "h1",
            "rect": { "x": 60, "y": 200, "w": 400, "h": 90 },
            "styles": { "position": "relative", "z-index": "2" },
            "children": [
              { "kind": "text", "rect": { "x": 60, "y": 200, "w": 400, "h": 90 },
                "text": "hero heading", "lines": 1, "styles": {} }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!("root frame")
    };
    let children = root.children.as_ref().unwrap();
    assert!(
        matches!(&children[0], PenNode::Frame(frame) if frame.base.name.as_deref() == Some("h1")),
        "hero content must paint above the background: {:?}",
        children
            .iter()
            .map(|node| match node {
                PenNode::Frame(frame) => frame.base.name.clone(),
                PenNode::Image(image) => image.base.name.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
    assert!(matches!(&children[1], PenNode::Image(_)));
}

#[test]
fn single_line_text_can_outgrow_the_captured_box() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 200, "h": 40 },
        "styles": {},
        "children": [
          { "kind": "text", "rect": { "x": 8, "y": 8, "w": 96, "h": 20 },
            "text": "openpencil", "lines": 1,
            "styles": { "font-size": "14px" } }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    use jian_ops_schema::node::text::TextGrowth;
    assert!(text.width.is_none(), "single-line text must hug");
    assert_eq!(text.limits.min_width, Some(96.0));
    assert_eq!(text.text_growth, Some(TextGrowth::Auto));
}

#[test]
fn wrapped_text_keeps_its_width_and_grows_downward() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "p",
        "rect": { "x": 0, "y": 0, "w": 300, "h": 60 },
        "styles": {},
        "children": [
          { "kind": "text", "rect": { "x": 0, "y": 0, "w": 300, "h": 60 },
            "text": "a wrapped paragraph", "lines": 3, "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    use jian_ops_schema::node::text::TextGrowth;
    use jian_ops_schema::sizing::SizingBehavior;
    assert!(matches!(text.width, Some(SizingBehavior::Number(w)) if w == 300.0));
    assert!(text.height.is_none());
    assert_eq!(text.limits.min_height, Some(60.0));
    assert_eq!(text.text_growth, Some(TextGrowth::FixedWidth));
}

/// Inline `<svg>` reaches the importer as an image node carrying an SVG data
/// URI (the extractor serializes it), and must keep its full box.
#[test]
fn inline_svg_image_fills_its_box_and_keeps_percentage_radius() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 40, "h": 40 },
        "styles": {},
        "children": [
          { "kind": "image", "tag": "svg",
            "rect": { "x": 8, "y": 8, "w": 24, "h": 24 },
            "src": "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
            "styles": { "border-top-left-radius": "50%",
                        "border-top-right-radius": "50%",
                        "border-bottom-right-radius": "50%",
                        "border-bottom-left-radius": "50%" } }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Image(image) = &root.children.as_ref().unwrap()[0] else {
        panic!("svg image")
    };
    use jian_ops_schema::node::image::ImageFitMode;
    use jian_ops_schema::node::CornerRadius;
    assert_eq!(image.object_fit, Some(ImageFitMode::Fill));
    assert!(image.src.as_str().starts_with("data:image/svg+xml"));
    // 50% of the 24 px box, resolved against the node — not the viewport.
    assert!(matches!(
        image.corner_radius,
        Some(CornerRadius::Uniform(radius)) if (radius - 12.0).abs() < f64::EPSILON
    ));
}

/// The extractor omits computed values that equal the CSS initial value, so
/// the importer must read a missing property as that default rather than as
/// "unknown".
#[test]
fn pruned_default_styles_import_without_warnings() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 100, "h": 100 },
        "children": [
          { "kind": "image", "tag": "img",
            "rect": { "x": 0, "y": 0, "w": 100, "h": 100 },
            "src": "data:image/png;base64,iVBORw0KGgo=" }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    assert_eq!(result.nodes.len(), 1);
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

/// A vectorized inline `<svg>` becomes a native path node — Skia has no SVG
/// codec, so the data-URI fallback would paint as a placeholder box. The path
/// data rides on the ordinary `kind: "image"` node next to the serialization
/// an older importer reads, and `vectorRect` (the artwork's own bounds, which
/// the renderer fits the path to) overrides the element's used box.
#[test]
fn path_data_on_an_image_node_wins_over_its_serialization() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "button",
        "rect": { "x": 0, "y": 0, "w": 32, "h": 32 },
        "children": [
          { "kind": "image", "tag": "svg",
            "rect": { "x": 4, "y": 4, "w": 24, "h": 24 },
            "src": "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
            "vectorRect": { "x": 6, "y": 8, "w": 20, "h": 16 },
            "d": "M1 8a7 7 0 1 1 14 0Z", "fill": "rgb(255, 0, 0)",
            "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Path(path) = &root.children.as_ref().unwrap()[0] else {
        panic!("path node")
    };
    use jian_ops_schema::sizing::SizingBehavior;
    assert_eq!(path.d.as_deref(), Some("M1 8a7 7 0 1 1 14 0Z"));
    // The artwork box, not the 24x24 element box: sizing the path to the
    // element would stretch the glyph by exactly their ratio.
    assert_eq!(path.base.x, Some(6.0));
    assert_eq!(path.base.y, Some(8.0));
    assert!(matches!(path.width, Some(SizingBehavior::Number(w)) if w == 20.0));
    assert!(matches!(path.height, Some(SizingBehavior::Number(h)) if h == 16.0));
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

/// Multi-colour flat art arrives as a plain container holding one
/// path-carrying image node per colour group (the extractor's expansion for
/// SVGs `vectorizeSvg` used to bail on). Each child must land as a native
/// path with its own fill and artwork box — the raster fallback these
/// replaced was an undecodable SVG data URI.
#[test]
fn a_multi_colour_svg_wrapper_imports_as_one_path_per_colour() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 120, "h": 40 },
        "children": [
          { "kind": "element", "tag": "svg",
            "rect": { "x": 10, "y": 4, "w": 96, "h": 32 },
            "styles": {},
            "children": [
              { "kind": "image", "tag": "svg",
                "rect": { "x": 10, "y": 4, "w": 44, "h": 32 },
                "src": "data:image/png;base64,iVBORw0KGgo=",
                "vectorRect": { "x": 10, "y": 4, "w": 44, "h": 32 },
                "d": "M0 0h20v32H0z", "fill": "rgb(66, 133, 244)",
                "styles": {} },
              { "kind": "image", "tag": "svg",
                "rect": { "x": 62, "y": 4, "w": 44, "h": 32 },
                "src": "data:image/png;base64,iVBORw0KGgo=",
                "vectorRect": { "x": 62, "y": 4, "w": 44, "h": 32 },
                "d": "M24 0h20v32H24z", "fill": "rgb(234, 67, 53)",
                "styles": {} }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Frame(svg) = &root.children.as_ref().unwrap()[0] else {
        panic!("svg wrapper frame")
    };
    let children = svg.children.as_ref().unwrap();
    assert_eq!(children.len(), 2);
    let mut fills = Vec::new();
    for (index, child) in children.iter().enumerate() {
        let PenNode::Path(path) = child else {
            panic!("path child {index}")
        };
        assert!(path.d.is_some(), "path {index} keeps its data");
        fills.push(path.fill.clone().expect("path fill"));
    }
    assert_ne!(fills[0], fills[1], "each colour group keeps its own fill");
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

/// A capture from an extension build that predates the vector fields carries
/// only `src`, and must keep importing as the image node it always was.
#[test]
fn an_svg_without_path_data_still_imports_as_an_image() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "button",
        "rect": { "x": 0, "y": 0, "w": 32, "h": 32 },
        "children": [
          { "kind": "image", "tag": "svg",
            "rect": { "x": 4, "y": 4, "w": 24, "h": 24 },
            "src": "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
            "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Image(image) = &root.children.as_ref().unwrap()[0] else {
        panic!("image node")
    };
    assert!(image.src.as_str().starts_with("data:image/svg+xml"));
    assert_eq!(image.base.x, Some(4.0));
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

/// Unusable path data degrades to the image serialization instead of dropping
/// the node — an icon that imports as a placeholder box is recoverable, one
/// that vanishes is not.
#[test]
fn empty_path_data_falls_back_to_the_image_instead_of_dropping() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "button",
        "rect": { "x": 0, "y": 0, "w": 32, "h": 32 },
        "children": [
          { "kind": "vector", "tag": "svg",
            "rect": { "x": 4, "y": 4, "w": 24, "h": 24 },
            "src": "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
            "d": "   ", "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let children = root.children.as_ref().unwrap();
    assert_eq!(children.len(), 1, "the node must survive");
    assert!(matches!(&children[0], PenNode::Image(_)));
}

/// The path node's box already comes out of the capture's root CTM, which
/// carries every transform between the artwork and the page. Re-applying the
/// element's CSS `transform` on top would rotate geometry that is already in
/// its final place.
#[test]
fn a_vector_node_does_not_re_apply_its_css_rotation() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 32, "h": 32 },
        "children": [
          { "kind": "image", "tag": "svg",
            "rect": { "x": 0, "y": 0, "w": 16, "h": 16 },
            "src": "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
            "d": "M0 0h8v8H0Z", "fill": "rgb(0, 0, 0)",
            "styles": { "transform": "matrix(0, 1, -1, 0, 0, 0)", "opacity": "0.5" } }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Path(path) = &root.children.as_ref().unwrap()[0] else {
        panic!("path node")
    };
    assert_eq!(path.base.rotation, None, "the CTM already placed the art");
    assert!(matches!(
        path.base.opacity,
        Some(jian_ops_schema::node::base::NumberOrExpression::Number(value)) if value == 0.5
    ));
}

/// The interim capture shape that carried path data on a node of its own.
#[test]
fn vector_node_becomes_a_path_with_its_fill() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "button",
        "rect": { "x": 0, "y": 0, "w": 32, "h": 32 },
        "children": [
          { "kind": "vector", "tag": "svg",
            "rect": { "x": 8, "y": 8, "w": 16, "h": 16 },
            "d": "M1 8a7 7 0 1 0 14 0Z", "fill": "rgb(255, 0, 0)",
            "fillRule": "evenodd", "styles": {} }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Path(path) = &root.children.as_ref().unwrap()[0] else {
        panic!("path node")
    };
    use jian_ops_schema::node::path::PathFillRule;
    use jian_ops_schema::sizing::SizingBehavior;
    assert_eq!(path.d.as_deref(), Some("M1 8a7 7 0 1 0 14 0Z"));
    assert_eq!(path.base.x, Some(8.0));
    assert!(matches!(path.width, Some(SizingBehavior::Number(w)) if w == 16.0));
    assert_eq!(path.fill_rule, Some(PathFillRule::Evenodd));
    assert!(matches!(
        path.fill.as_deref(),
        Some([jian_ops_schema::style::PenFill::Solid(body)]) if body.color == "#ff0000"
    ));
}

/// `overflow: hidden` that clipped nothing in the browser is dropped, so a
/// wider fallback font can spill instead of losing its tail; a box whose
/// content really did overflow keeps clipping.
#[test]
fn inert_clip_is_dropped_but_a_real_overflow_still_clips() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 400, "h": 100 },
        "children": [
          { "kind": "element", "tag": "div",
            "rect": { "x": 0, "y": 0, "w": 100, "h": 20 },
            "styles": { "overflow": "hidden" },
            "children": [
              { "kind": "text", "rect": { "x": 0, "y": 0, "w": 90, "h": 20 },
                "text": "fits", "lines": 1, "styles": {} }
            ] },
          { "kind": "element", "tag": "div",
            "rect": { "x": 0, "y": 40, "w": 100, "h": 20 },
            "styles": { "overflow": "hidden" },
            "children": [
              { "kind": "text", "rect": { "x": 0, "y": 40, "w": 220, "h": 20 },
                "text": "truncated by the browser", "lines": 1, "styles": {} }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    // Front-to-back: the overflowing box was declared last, so it comes first.
    let children = root.children.as_ref().unwrap();
    let PenNode::Frame(overflowing) = &children[0] else {
        panic!()
    };
    let PenNode::Frame(fitting) = &children[1] else {
        panic!()
    };
    assert_eq!(fitting.container.clip_content, None, "inert clip must go");
    assert_eq!(overflowing.container.clip_content, Some(true));
}

/// A rounded box keeps its clip however contained its children are:
/// `overflow: hidden` + `border-radius` is the rounded-card idiom, where the
/// clip rounds off a child that fills the box rather than truncating it. The
/// child rects cannot reveal that — only the child's *corners* stick out.
#[test]
fn a_rounded_wrapper_keeps_its_clip_even_when_nothing_overflows() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 400, "h": 200 },
        "children": [
          { "kind": "element", "tag": "div",
            "rect": { "x": 0, "y": 0, "w": 100, "h": 60 },
            "styles": { "overflow": "hidden",
                        "border-top-left-radius": "8px",
                        "border-top-right-radius": "8px",
                        "border-bottom-right-radius": "8px",
                        "border-bottom-left-radius": "8px" },
            "children": [
              { "kind": "element", "tag": "div",
                "rect": { "x": 0, "y": 0, "w": 100, "h": 30 },
                "styles": { "background-color": "rgb(255, 0, 0)" },
                "children": [] }
            ] },
          { "kind": "element", "tag": "div",
            "rect": { "x": 0, "y": 100, "w": 100, "h": 60 },
            "styles": { "overflow": "hidden" },
            "children": [
              { "kind": "text", "rect": { "x": 0, "y": 100, "w": 90, "h": 20 },
                "text": "fits", "lines": 1, "styles": {} }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    // Front-to-back: the square wrapper was declared last, so it comes first.
    let children = root.children.as_ref().unwrap();
    let PenNode::Frame(square) = &children[0] else {
        panic!()
    };
    let PenNode::Frame(rounded) = &children[1] else {
        panic!()
    };
    assert_eq!(
        rounded.container.clip_content,
        Some(true),
        "a radius makes the clip load-bearing"
    );
    assert_eq!(square.container.clip_content, None, "inert clip must go");
}

/// A vector descendant may be relying on the clip for its crop exactly the way
/// an image descendant can, so it blocks the inert-clip drop too.
#[test]
fn a_vector_child_keeps_its_parents_clip() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 400, "h": 200 },
        "children": [
          { "kind": "element", "tag": "div",
            "rect": { "x": 0, "y": 0, "w": 100, "h": 60 },
            "styles": { "overflow": "hidden" },
            "children": [
              { "kind": "vector", "tag": "svg",
                "rect": { "x": 0, "y": 0, "w": 40, "h": 40 },
                "d": "M0 0h8v8H0Z", "fill": "rgb(0, 0, 0)", "styles": {} }
            ] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Frame(wrapper) = &root.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(wrapper.container.clip_content, Some(true));
}

/// `white-space: pre` suppresses automatic wrapping in the browser, but the
/// capture collapses the newlines that gave a `<pre>` block its lines — so
/// hugging it lays the whole block out on one long line, three times wider
/// than the slot it sits in.
#[test]
fn a_pre_block_keeps_the_captured_width_instead_of_hugging() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "pre",
        "rect": { "x": 0, "y": 0, "w": 101, "h": 54 },
        "styles": {},
        "children": [
          { "kind": "text", "rect": { "x": 0, "y": 0, "w": 101, "h": 54 },
            "text": "one two three", "lines": 3,
            "styles": { "white-space": "pre" } }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    use jian_ops_schema::node::text::TextGrowth;
    use jian_ops_schema::sizing::SizingBehavior;
    assert!(matches!(text.width, Some(SizingBehavior::Number(w)) if w == 101.0));
    assert_eq!(text.text_growth, Some(TextGrowth::FixedWidth));
    // `nowrap` is still a hug: there the browser really did keep one line.
    let nowrap = json.replace("\"white-space\": \"pre\"", "\"white-space\": \"nowrap\"");
    let result = import_snapshot(&nowrap, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Text(text) = &root.children.as_ref().unwrap()[0] else {
        panic!("text run")
    };
    assert!(text.width.is_none());
    assert_eq!(text.text_growth, Some(TextGrowth::Auto));
}

/// The avatar-stack idiom: static flex items overlapped by a negative margin
/// and ordered by `z-index` alone. CSS honours `z-index` on a flex item, so
/// the first one declared paints on top.
#[test]
fn z_index_on_static_flex_items_is_honoured() {
    let json = r#"{
      "version": 1,
      "root": {
        "kind": "element", "tag": "div",
        "rect": { "x": 0, "y": 0, "w": 100, "h": 40 },
        "styles": { "display": "flex" },
        "children": [
          { "kind": "element", "tag": "span",
            "rect": { "x": 0, "y": 0, "w": 40, "h": 40 },
            "styles": { "z-index": "3" }, "children": [] },
          { "kind": "element", "tag": "b",
            "rect": { "x": 28, "y": 0, "w": 40, "h": 40 },
            "styles": { "z-index": "2" }, "children": [] },
          { "kind": "element", "tag": "i",
            "rect": { "x": 56, "y": 0, "w": 40, "h": 40 },
            "styles": { "z-index": "1" }, "children": [] }
        ]
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let names: Vec<_> = root
        .children
        .as_ref()
        .unwrap()
        .iter()
        .map(|node| match node {
            PenNode::Frame(frame) => frame.base.name.clone().unwrap_or_default(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(names, ["span", "b", "i"], "z-index orders flex items");
    // The very same children under a block parent keep document order,
    // because there a static box's z-index is inert.
    let block = json.replace("\"display\": \"flex\"", "\"position\": \"static\"");
    let result = import_snapshot(&block, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let names: Vec<_> = root
        .children
        .as_ref()
        .unwrap()
        .iter()
        .map(|node| match node {
            PenNode::Frame(frame) => frame.base.name.clone().unwrap_or_default(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(names, ["i", "b", "span"]);
}

/// The page's own backdrop travels with the capture, so an element pick out
/// of a dark-theme page does not land on a white canvas.
#[test]
fn page_background_fills_the_root_frame() {
    let json = r#"{
      "version": 1,
      "background": "rgb(13, 17, 23)",
      "root": {
        "kind": "element", "tag": "table",
        "rect": { "x": 0, "y": 0, "w": 100, "h": 100 },
        "children": []
      }
    }"#;
    let result = import_snapshot(json, &HtmlImportOptions::default());
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    assert!(matches!(
        root.container.fill.as_deref(),
        Some([jian_ops_schema::style::PenFill::Solid(body)]) if body.color == "#0d1117"
    ));
}

#[test]
fn bad_version_and_bad_json_warn_not_panic() {
    let result = import_snapshot("{\"version\":2,\"root\":{}}", &HtmlImportOptions::default());
    assert!(result.nodes.is_empty());
    assert!(result.warnings[0].contains("version"));
    let malformed = import_snapshot("not json", &HtmlImportOptions::default());
    assert!(malformed.nodes.is_empty());
}
