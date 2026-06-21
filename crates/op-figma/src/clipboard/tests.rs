//! Clipboard-port tests — base64 decode, HTML extraction, CSS-style
//! hint scanner, and the two PenNode post-process passes
//! (`fix_unresolved_images`, `enrich_nodes_from_html_hints`).
//! Behaviour mirrors `pen-figma/figma-clipboard.ts`.

use super::*;
use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::{
    EllipseNode, FrameNode, ImageNode, PenNode, PenNodeBase, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{ImageFillBody, PenFill, SolidFillBody};

fn base() -> PenNodeBase {
    PenNodeBase {
        id: "n1".to_string(),
        name: Some("hero".to_string()),
        ..Default::default()
    }
}

#[test]
fn is_figma_clipboard_html_detects_both_markers() {
    assert!(is_figma_clipboard_html(
        "<html><!--(figmeta)-->...<!--(figmeta)--></html>"
    ));
    assert!(is_figma_clipboard_html(
        "<span data-buffer=\"AAA=\"></span>"
    ));
    assert!(!is_figma_clipboard_html("<p>just a plain paragraph</p>"));
}

#[test]
fn css_color_to_hex_handles_common_forms() {
    assert_eq!(css_color_to_hex("#abcdef"), Some("#abcdef".to_string()));
    assert_eq!(css_color_to_hex("#abc"), Some("#aabbcc".to_string()));
    assert_eq!(
        css_color_to_hex("rgb(255, 0, 0)"),
        Some("#ff0000".to_string())
    );
    assert_eq!(
        css_color_to_hex("rgba(255, 0, 0, 0.5)"),
        Some("#ff000080".to_string())
    );
    // alpha 1 → drop alpha byte.
    assert_eq!(
        css_color_to_hex("rgba(255, 0, 0, 1)"),
        Some("#ff0000".to_string())
    );
    assert_eq!(css_color_to_hex("named-color"), None);
}

#[test]
fn decode_html_entities_handles_named_and_numeric() {
    assert_eq!(decode_html_entities("&amp;&lt;&gt;&quot;&nbsp;"), "&<>\" ");
    assert_eq!(decode_html_entities("&#65;&#x42;"), "AB");
    assert_eq!(
        decode_html_entities("plain text with &amp; ampersand"),
        "plain text with & ampersand"
    );
}

#[test]
fn decode_base64_handles_padding_and_url_safe() {
    // "Hello" → "SGVsbG8="
    assert_eq!(decode_base64("SGVsbG8="), Some(b"Hello".to_vec()));
    // Without padding.
    assert_eq!(decode_base64("SGVsbG8"), Some(b"Hello".to_vec()));
    // URL-safe alphabet.
    assert_eq!(decode_base64("SGVsbG8_"), Some(b"Hello?".to_vec()));
    // Whitespace tolerance.
    assert_eq!(decode_base64("SGVs\n bG8="), Some(b"Hello".to_vec()));
}

#[test]
fn extract_figma_clipboard_data_via_comment_form() {
    // meta = JSON `{"v":1}` → base64 `eyJ2IjoxfQ==`
    // buffer = bytes `OP` → base64 `T1A=`
    let html = "<html>\
        <!--(figmeta)-->eyJ2IjoxfQ==<!--(figmeta)-->\
        <!--(figma)-->T1A=<!--(figma)-->\
        </html>";
    let data = extract_figma_clipboard_data(html).expect("extracts");
    assert!(data.meta.contains("\"v\":1"));
    assert_eq!(data.buffer, b"OP");
}

#[test]
fn extract_figma_clipboard_data_via_data_attribute_form() {
    let html = "<span data-metadata=\"eyJ2IjoxfQ==\"></span><span data-buffer=\"T1A=\"></span>";
    let data = extract_figma_clipboard_data(html).expect("extracts");
    assert!(data.meta.contains("\"v\":1"));
    assert_eq!(data.buffer, b"OP");
}

#[test]
fn extract_figma_clipboard_data_returns_none_for_random_html() {
    assert!(extract_figma_clipboard_data("<p>nothing</p>").is_none());
}

#[test]
fn parse_clipboard_html_styles_extracts_color_font() {
    let html =
        "<p style=\"color: #ff0000; font-family: Inter, sans; font-size: 18px; font-weight: 700;\">Title</p>";
    let hints = parse_clipboard_html_styles(html);
    let hint = hints.get("Title").expect("hint present");
    assert_eq!(hint.color.as_deref(), Some("#ff0000"));
    assert_eq!(hint.font_family.as_deref(), Some("Inter"));
    assert_eq!(hint.font_size, Some(18.0));
    assert_eq!(hint.font_weight, Some(700));
}

#[test]
fn parse_clipboard_html_styles_handles_background_color() {
    let html = "<div style=\"background-color: rgba(255,0,0,0.5)\">Card</div>";
    let hints = parse_clipboard_html_styles(html);
    let hint = hints.get("Card").expect("hint present");
    assert_eq!(hint.background_color.as_deref(), Some("#ff000080"));
}

#[test]
fn fix_unresolved_images_swaps_blob_image_for_placeholder_rect() {
    let mut nodes = vec![PenNode::Image(ImageNode {
        base: base(),
        src: "__blob:5".into(),
        object_fit: None,
        width: Some(SizingBehavior::Number(100.0)),
        height: Some(SizingBehavior::Number(80.0)),
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    fix_unresolved_images(&mut nodes);
    match &nodes[0] {
        PenNode::Rectangle(rect) => {
            let fill = rect.container.fill.as_ref().expect("placeholder fill");
            assert_eq!(fill.len(), 1);
            match &fill[0] {
                PenFill::Solid(s) => assert_eq!(s.color, "#E5E7EB"),
                _ => panic!("expected solid placeholder"),
            }
        }
        other => panic!("expected Rectangle, got {other:?}"),
    }
}

#[test]
fn fix_unresolved_images_swaps_blob_image_fill_on_rect() {
    let mut nodes = vec![PenNode::Rectangle(RectangleNode {
        base: base(),
        container: ContainerProps {
            fill: Some(vec![PenFill::Image(ImageFillBody {
                url: "__hash:abc".into(),
                mode: None,
                original_size: None,
                transform: None,
                explain: None,
                opacity: None,
                exposure: None,
                contrast: None,
                saturation: None,
                temperature: None,
                tint: None,
                highlights: None,
                shadows: None,
            })]),
            ..ContainerProps::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    fix_unresolved_images(&mut nodes);
    match &nodes[0] {
        PenNode::Rectangle(rect) => {
            let fill = rect.container.fill.as_ref().unwrap();
            match &fill[0] {
                PenFill::Solid(s) => assert_eq!(s.color, "#E5E7EB"),
                _ => panic!("expected placeholder solid"),
            }
        }
        other => panic!("expected Rectangle, got {other:?}"),
    }
}

#[test]
fn fix_unresolved_images_preserves_corner_radius_on_placeholder() {
    use jian_ops_schema::node::container::CornerRadius;
    let mut nodes = vec![PenNode::Image(ImageNode {
        base: base(),
        src: "__blob:1".into(),
        object_fit: None,
        width: Some(SizingBehavior::Number(100.0)),
        height: Some(SizingBehavior::Number(100.0)),
        corner_radius: Some(CornerRadius::Uniform(12.0)),
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    fix_unresolved_images(&mut nodes);
    match &nodes[0] {
        PenNode::Rectangle(rect) => {
            // TS preserves cornerRadius from the original image node.
            match &rect.container.corner_radius {
                Some(CornerRadius::Uniform(r)) => assert!((r - 12.0).abs() < 0.001),
                other => panic!("expected Uniform(12.0), got {other:?}"),
            }
        }
        other => panic!("expected Rectangle, got {other:?}"),
    }
}

#[test]
fn fix_unresolved_images_recurses_into_frame_children() {
    let inner = PenNode::Image(ImageNode {
        base: base(),
        src: "__blob:1".into(),
        object_fit: None,
        width: None,
        height: None,
        corner_radius: None,
        effects: None,
        exposure: None,
        contrast: None,
        saturation: None,
        temperature: None,
        tint: None,
        highlights: None,
        shadows: None,
        image_prompt: None,
        image_search_query: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    });
    let mut nodes = vec![PenNode::Frame(FrameNode {
        base: base(),
        container: ContainerProps::default(),
        children: Some(vec![inner]),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    fix_unresolved_images(&mut nodes);
    let kids = if let PenNode::Frame(f) = &nodes[0] {
        f.children.as_ref().expect("kids preserved")
    } else {
        panic!("expected Frame");
    };
    assert!(matches!(kids[0], PenNode::Rectangle(_)));
}

#[test]
fn enrich_nodes_from_html_hints_fills_missing_text_props() {
    let mut hints = HashMap::new();
    hints.insert(
        "Title".to_string(),
        HtmlStyleHint {
            color: Some("#ff0000".into()),
            font_family: Some("Inter".into()),
            font_size: Some(18.0),
            font_weight: Some(700),
            background_color: None,
        },
    );
    let mut nodes = vec![PenNode::Text(TextNode {
        base: base(),
        width: None,
        height: None,
        content: TextContent::Plain("Title".into()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    enrich_nodes_from_html_hints(&mut nodes, &hints);
    if let PenNode::Text(t) = &nodes[0] {
        assert_eq!(t.font_family.as_deref(), Some("Inter"));
        assert_eq!(t.font_size, Some(18.0));
        assert!(t.font_weight.is_some());
        let fill = t.fill.as_ref().unwrap();
        match &fill[0] {
            PenFill::Solid(s) => assert_eq!(s.color, "#ff0000"),
            _ => panic!("expected solid fill"),
        }
    } else {
        panic!("expected Text");
    }
}

#[test]
fn enrich_does_not_overwrite_explicit_values() {
    let mut hints = HashMap::new();
    hints.insert(
        "Title".into(),
        HtmlStyleHint {
            color: Some("#ff0000".into()),
            ..HtmlStyleHint::default()
        },
    );
    let mut nodes = vec![PenNode::Text(TextNode {
        base: base(),
        width: None,
        height: None,
        content: TextContent::Plain("Title".into()),
        font_family: Some("ExistingFont".into()),
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: Some(vec![PenFill::Solid(SolidFillBody {
            color: "#00ff00".into(),
            explain: None,
            opacity: None,
            blend_mode: None,
        })]),
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    enrich_nodes_from_html_hints(&mut nodes, &hints);
    if let PenNode::Text(t) = &nodes[0] {
        // Explicit pre-existing values must remain.
        assert_eq!(t.font_family.as_deref(), Some("ExistingFont"));
        let fill = t.fill.as_ref().unwrap();
        if let PenFill::Solid(s) = &fill[0] {
            assert_eq!(s.color, "#00ff00", "explicit fill must not be overwritten");
        } else {
            panic!("expected solid");
        }
    }
}

#[test]
fn enrich_applies_background_to_unfilled_rectangle() {
    let mut hints = HashMap::new();
    hints.insert(
        "hero".into(),
        HtmlStyleHint {
            background_color: Some("#abcdef".into()),
            ..HtmlStyleHint::default()
        },
    );
    let mut nodes = vec![PenNode::Rectangle(RectangleNode {
        base: base(),
        container: ContainerProps::default(),
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    enrich_nodes_from_html_hints(&mut nodes, &hints);
    if let PenNode::Rectangle(r) = &nodes[0] {
        let fill = r.container.fill.as_ref().expect("fill applied");
        if let PenFill::Solid(s) = &fill[0] {
            assert_eq!(s.color, "#abcdef");
        } else {
            panic!("expected solid");
        }
    }
}

#[test]
fn ellipse_with_blob_fill_keeps_other_fill_replaced() {
    let _placeholder = EllipseNode {
        base: base(),
        width: None,
        height: None,
        corner_radius: None,
        inner_radius: None,
        start_angle: None,
        sweep_angle: None,
        fill: Some(vec![PenFill::Image(ImageFillBody {
            url: "__blob:7".into(),
            mode: None,
            original_size: None,
            transform: None,
            explain: None,
            opacity: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
        })]),
        stroke: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    };
    let mut nodes = vec![PenNode::Ellipse(_placeholder)];
    fix_unresolved_images(&mut nodes);
    if let PenNode::Ellipse(e) = &nodes[0] {
        let fill = e.fill.as_ref().unwrap();
        match &fill[0] {
            PenFill::Solid(s) => assert_eq!(s.color, "#E5E7EB"),
            _ => panic!("expected placeholder solid"),
        }
    }
}
