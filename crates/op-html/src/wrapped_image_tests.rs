use base64::Engine as _;
use jian_ops_schema::node::{FrameNode, ImageNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use crate::{import_html, import_html_with_resources, HtmlImportOptions};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0; 24];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

fn data_url(width: u32, height: u32) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(width, height))
    )
}

fn children(node: &PenNode) -> Option<&[PenNode]> {
    match node {
        PenNode::Frame(node) => node.children.as_deref(),
        PenNode::Group(node) => node.children.as_deref(),
        PenNode::Rectangle(node) => node.children.as_deref(),
        PenNode::Ref(node) => node.children.as_deref(),
        PenNode::Tabs(node) => node.children.as_deref(),
        _ => None,
    }
}

fn frame_named<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a FrameNode> {
    for node in nodes {
        if let PenNode::Frame(frame) = node {
            if frame.base.name.as_deref() == Some(name) {
                return Some(frame);
            }
        }
        if let Some(frame) = children(node).and_then(|children| frame_named(children, name)) {
            return Some(frame);
        }
    }
    None
}

fn collect_images<'a>(nodes: &'a [PenNode], output: &mut Vec<&'a ImageNode>) {
    for node in nodes {
        if let PenNode::Image(image) = node {
            output.push(image);
        }
        if let Some(children) = children(node) {
            collect_images(children, output);
        }
    }
}

fn number(axis: Option<&SizingBehavior>) -> Option<f64> {
    match axis {
        Some(SizingBehavior::Number(value)) => Some(*value),
        _ => None,
    }
}

#[test]
fn linked_image_is_boxed_instead_of_disappearing_into_rich_text() {
    let imported = import_html(
        &format!(
            r##"<p><a href="#x"><img src="{}"></a></p>"##,
            data_url(100, 50)
        ),
        &HtmlImportOptions::default(),
    );
    let mut images = Vec::new();
    collect_images(&imported.nodes, &mut images);
    assert_eq!(images.len(), 1);
    let link = frame_named(&imported.nodes, "a").expect("boxed link");
    assert_eq!(number(link.container.width.as_ref()), Some(100.0));
    assert_eq!(number(link.container.height.as_ref()), Some(50.0));
}

#[test]
fn linked_images_are_measurable_flex_items_before_wrapping() {
    let image = data_url(220, 100);
    let imported = import_html(
        &format!(
            r#"<div style="display:flex;flex-wrap:wrap;width:500px;gap:0">
                <a><img src="{image}"></a><a><img src="{image}"></a><a><img src="{image}"></a>
            </div>"#
        ),
        &HtmlImportOptions::default(),
    );
    let flex = frame_named(&imported.nodes, "div").expect("flex frame");
    let rows = flex.children.as_deref().expect("wrap rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(children(&rows[0]).map(<[PenNode]>::len), Some(2));
    assert_eq!(children(&rows[1]).map(<[PenNode]>::len), Some(1));
}

#[test]
fn picture_promotes_density_corrected_image_size_through_single_child_wrappers() {
    let bytes = png(440, 200);
    let fetcher = |url: &str| (url == "https://assets.test/photo.png").then(|| bytes.clone());
    let imported = import_html_with_resources(
        r#"<picture><source srcset="photo.png 2x"><img></picture>"#,
        &HtmlImportOptions {
            base_url: Some("https://assets.test/page.html".to_string()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );
    let picture = frame_named(&imported.nodes, "picture").expect("picture frame");
    assert_eq!(number(picture.container.width.as_ref()), Some(220.0));
    assert_eq!(number(picture.container.height.as_ref()), Some(100.0));
}

#[test]
fn boxed_link_promotes_border_box_but_keeps_dynamic_and_multi_child_guards() {
    let image = data_url(100, 50);
    let padded = import_html(
        &format!(r#"<a style="padding:10px;border:2px solid #000"><img src="{image}"></a>"#),
        &HtmlImportOptions::default(),
    );
    let link = frame_named(&padded.nodes, "a").expect("padded link");
    assert_eq!(number(link.container.width.as_ref()), Some(124.0));
    assert_eq!(number(link.container.height.as_ref()), Some(74.0));

    let dynamic = import_html(
        &format!(r#"<a style="width:100%"><img src="{image}"></a>"#),
        &HtmlImportOptions::default(),
    );
    let link = frame_named(&dynamic.nodes, "a").expect("dynamic link");
    assert!(matches!(
        link.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));

    let mixed = import_html(
        &format!(r#"<a>label<img src="{image}"></a>"#),
        &HtmlImportOptions::default(),
    );
    let link = frame_named(&mixed.nodes, "a").expect("mixed link");
    assert!(!matches!(
        link.container.width,
        Some(SizingBehavior::Number(_))
    ));
}
