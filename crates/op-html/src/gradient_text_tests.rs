use jian_ops_schema::node::text::{TextContent, TextNode};
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::style::PenFill;

use crate::{import_html, HtmlImportOptions, ImportWarning};

fn root(html: &str) -> (crate::HtmlImportResult, FrameNode) {
    let result = import_html(html, &HtmlImportOptions::default());
    let PenNode::Frame(frame) = result.nodes[0].clone() else {
        panic!("import root must be a frame")
    };
    (result, frame)
}

fn children(frame: &FrameNode) -> &[PenNode] {
    frame.children.as_deref().unwrap_or_default()
}

fn content(text: &TextNode) -> String {
    match &text.content {
        TextContent::Plain(value) => value.clone(),
        TextContent::Styled(segments) => segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect(),
    }
}

fn find_frame<'a>(frame: &'a FrameNode, name: &str) -> Option<&'a FrameNode> {
    children(frame).iter().find_map(|node| match node {
        PenNode::Frame(child) if child.base.name.as_deref() == Some(name) => Some(child),
        PenNode::Frame(child) => find_frame(child, name),
        _ => None,
    })
}

fn find_frame_with_text<'a>(frame: &'a FrameNode, expected: &str) -> Option<&'a FrameNode> {
    children(frame).iter().find_map(|node| match node {
        PenNode::Frame(child) if find_text(child, expected).is_some() => Some(child),
        PenNode::Frame(child) => find_frame_with_text(child, expected),
        _ => None,
    })
}

fn find_text<'a>(frame: &'a FrameNode, expected: &str) -> Option<&'a TextNode> {
    children(frame).iter().find_map(|node| match node {
        PenNode::Text(text) if content(text) == expected => Some(text),
        PenNode::Frame(child) => find_text(child, expected),
        _ => None,
    })
}

fn text_fill(text: &TextNode) -> Option<&str> {
    text.fill.as_deref()?.iter().find_map(|fill| match fill {
        PenFill::Solid(fill) => Some(fill.color.as_str()),
        _ => None,
    })
}

fn frame_solid_fill(frame: &FrameNode) -> Option<&str> {
    frame
        .container
        .fill
        .as_deref()?
        .iter()
        .find_map(|fill| match fill {
            PenFill::Solid(fill) => Some(fill.color.as_str()),
            _ => None,
        })
}

fn has_clip_warning(result: &crate::HtmlImportResult) -> bool {
    result.diagnostics.iter().any(|warning| {
        matches!(warning, ImportWarning::PropertyNotRepresentable { property }
            if property == "background-clip")
    })
}

#[test]
fn webkit_gradient_text_is_scoped_and_preserves_opaque_nested_segments() {
    let html = r#"<style>
      .gradient {
        background: linear-gradient(90deg, #123456, #abcdef);
        -webkit-background-clip: text;
        color: #0f172a;
        -webkit-text-fill-color: transparent;
      }
      .opaque { color: #010203; -webkit-text-fill-color: #ef4444; }
      .sibling { color: transparent; }
    </style>
    <h1 class="gradient">alpha <span class="opaque">beta</span> omega</h1>
    <p class="sibling">transparent sibling</p>"#;
    let (result, root) = root(html);
    let heading = find_frame(&root, "h1").expect("gradient heading");
    assert!(heading.container.fill.is_none());
    let text = find_text(heading, "alpha beta omega").expect("heading text");
    assert_eq!(text_fill(text), Some("#123456"));
    let TextContent::Styled(segments) = &text.content else {
        panic!("the explicit nested colour must remain a rich-text segment")
    };
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].fill.as_deref(), Some("#123456"));
    assert_eq!(segments[1].fill.as_deref(), Some("#ef4444"));
    assert_eq!(segments[2].fill.as_deref(), Some("#123456"));

    let sibling = find_text(&root, "transparent sibling").expect("sibling text");
    assert_eq!(text_fill(sibling), Some("#00000000"));
    assert!(!has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn solid_and_radial_text_clips_use_the_first_paint_colour() {
    for (background, expected) in [
        ("background-color:#0a1b2c", "#0a1b2c"),
        (
            "background-image:radial-gradient(circle,#405060,#f0f0f0)",
            "#405060",
        ),
    ] {
        let html =
            format!("<div style='{background};background-clip:text;color:transparent'>copy</div>");
        let (result, root) = root(&html);
        let frame = find_frame_with_text(&root, "copy").expect("text clip frame");
        assert!(frame.container.fill.is_none(), "{background}");
        assert_eq!(
            text_fill(find_text(frame, "copy").expect("copy text")),
            Some(expected),
            "{background}"
        );
        assert!(!has_clip_warning(&result), "{:?}", result.warnings);
    }
}

#[test]
fn ordinary_gradients_and_unclipped_transparent_text_keep_their_own_paints() {
    let html = r#"
      <div style="background:linear-gradient(#112233,#ddeeff);color:transparent">ordinary</div>
      <div style="background:linear-gradient(#223344,#ccddee);background-clip:padding-box;color:transparent">unsupported clip</div>
    "#;
    let (result, root) = root(html);
    for expected in ["ordinary", "unsupported clip"] {
        let frame = find_frame_with_text(&root, expected).expect("gradient frame");
        assert!(matches!(
            frame.container.fill.as_deref(),
            Some([PenFill::LinearGradient(_)])
        ));
        assert_eq!(
            text_fill(find_text(frame, expected).expect("transparent text")),
            Some("#00000000")
        );
    }
    assert!(has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn image_backed_text_clip_keeps_the_box_paint_and_warning() {
    let html = r#"<div style="background:url(texture.png) #123456;
      background-clip:text;color:transparent">copy</div>"#;
    let (result, root) = root(html);
    let frame = find_frame_with_text(&root, "copy").expect("image-backed frame");
    assert!(frame.container.fill.as_deref().is_some_and(|fills| {
        fills.iter().any(|fill| matches!(fill, PenFill::Image(_)))
            && fills.iter().any(|fill| matches!(fill, PenFill::Solid(_)))
    }));
    assert!(has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn partial_alpha_glyphs_keep_the_clipped_background_and_warning() {
    let html = r#"<div style="background:linear-gradient(#ff0000,#00ff00);
      background-clip:text;-webkit-text-fill-color:rgb(0 0 255 / 50%)">copy</div>"#;
    let (result, root) = root(html);
    let frame = find_frame_with_text(&root, "copy").expect("partial-alpha text frame");
    assert!(matches!(
        frame.container.fill.as_deref(),
        Some([PenFill::LinearGradient(_)])
    ));
    assert_eq!(
        text_fill(find_text(frame, "copy").expect("partial-alpha text")),
        Some("#0000ff80")
    );
    assert!(has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn opaque_scope_can_supply_a_transparent_descendant() {
    let html = r#"<div style="background:linear-gradient(#123456,#abcdef);
      background-clip:text;color:#010203"><span style="color:transparent">copy</span></div>"#;
    let (result, root) = root(html);
    let frame = find_frame_with_text(&root, "copy").expect("gradient scope");
    assert!(frame.container.fill.is_none());
    assert_eq!(
        text_fill(find_text(frame, "copy").expect("transparent descendant")),
        Some("#123456")
    );
    assert!(!has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn replaced_and_form_elements_do_not_suppress_text_clip_warnings() {
    for html in [
        r#"<input style="background:linear-gradient(#123456,#abcdef);
          background-clip:text;color:transparent">"#,
        r#"<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw="
          style="background:linear-gradient(#123456,#abcdef);
          background-clip:text;color:transparent">"#,
    ] {
        let result = import_html(html, &HtmlImportOptions::default());
        assert!(has_clip_warning(&result), "{html}: {:?}", result.warnings);
    }
}

#[test]
fn hidden_pseudo_does_not_block_an_exact_text_clip_transfer() {
    for hidden in ["display:none", "visibility:hidden", "opacity:0"] {
        let html = format!(
            r#"<style>
              .gradient {{ background:linear-gradient(#123456,#abcdef);
                background-clip:text;color:transparent }}
              .gradient::before {{ content:'hidden';{hidden};color:rgb(0 0 0 / 50%) }}
            </style><div class="gradient">copy</div>"#
        );
        let (result, root) = root(&html);
        let frame = find_frame_with_text(&root, "copy").expect("gradient frame");
        assert!(frame.container.fill.is_none(), "{hidden}");
        assert_eq!(
            text_fill(find_text(frame, "copy").expect("gradient text")),
            Some("#123456"),
            "{hidden}"
        );
        assert!(
            !has_clip_warning(&result),
            "{hidden}: {:?}",
            result.warnings
        );
    }
}

#[test]
fn partial_alpha_list_marker_keeps_the_clipped_background_and_warning() {
    let html = r#"<ul><li style="background:linear-gradient(#123456,#abcdef);
      background-clip:text;color:rgb(0 0 0 / 50%)"></li></ul>"#;
    let (result, root) = root(html);
    let item = find_frame(&root, "li").expect("list item");
    assert!(matches!(
        item.container.fill.as_deref(),
        Some([PenFill::LinearGradient(_)])
    ));
    assert!(has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn markerless_list_item_can_transfer_an_exact_text_clip() {
    let html = r#"<ul><li style="list-style-type:none;
      background:linear-gradient(#123456,#abcdef);
      background-clip:text;color:rgb(0 0 0 / 50%)"></li></ul>"#;
    let (result, root) = root(html);
    let item = find_frame(&root, "li").expect("list item");
    assert!(item.container.fill.is_none());
    assert!(!has_clip_warning(&result), "{:?}", result.warnings);
}

#[test]
fn body_and_generated_content_get_local_text_clip_scopes() {
    let body_html = r#"<style>body {
      background:linear-gradient(90deg,#314159,#ffffff);
      background-clip:text;color:transparent
    }</style>body copy"#;
    let (result, body) = root(body_html);
    assert_eq!(frame_solid_fill(&body), Some("#ffffff"));
    assert_eq!(
        text_fill(find_text(&body, "body copy").expect("body text")),
        Some("#314159")
    );
    assert!(!has_clip_warning(&result), "{:?}", result.warnings);

    let pseudo_html = r#"<style>
      p { color:#556677 }
      p::before { content:'generated'; display:inline-block;
        background:linear-gradient(#765432,#ffffff);
        background-clip:text;color:transparent }
    </style><p>ordinary copy</p>"#;
    let (result, root) = root(pseudo_html);
    let generated = find_frame(&root, "::before").expect("generated frame");
    assert!(generated.container.fill.is_none());
    assert_eq!(
        text_fill(find_text(generated, "generated").expect("generated text")),
        Some("#765432")
    );
    assert_eq!(
        text_fill(find_text(&root, "ordinary copy").expect("ordinary text")),
        Some("#556677")
    );
    assert!(!has_clip_warning(&result), "{:?}", result.warnings);
}
