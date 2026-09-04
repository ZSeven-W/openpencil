use super::*;
use jian_ops_schema::node::{ImageNode, PenNode};
use jian_ops_schema::sizing::SizingBehavior;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0; 24];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

fn data_url(bytes: &[u8], mime: &str) -> String {
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn collect_images<'a>(nodes: &'a [PenNode], output: &mut Vec<&'a ImageNode>) {
    for node in nodes {
        if let PenNode::Image(image) = node {
            output.push(image);
        }
        let children = match node {
            PenNode::Frame(node) => node.children.as_deref(),
            PenNode::Group(node) => node.children.as_deref(),
            PenNode::Rectangle(node) => node.children.as_deref(),
            PenNode::Ref(node) => node.children.as_deref(),
            PenNode::Tabs(node) => node.children.as_deref(),
            _ => None,
        };
        if let Some(children) = children {
            collect_images(children, output);
        }
    }
}

fn images(nodes: &[PenNode]) -> Vec<&ImageNode> {
    let mut output = Vec::new();
    collect_images(nodes, &mut output);
    output
}

fn number(axis: Option<&SizingBehavior>) -> Option<f64> {
    match axis {
        Some(SizingBehavior::Number(value)) => Some(*value),
        _ => None,
    }
}

fn assert_close(actual: Option<f64>, expected: f64) {
    assert!((actual.unwrap_or(f64::NAN) - expected).abs() < 1.0e-6);
}

#[test]
fn svg_default_box_does_not_masquerade_as_a_natural_ratio() {
    let no_ratio = data_url(
        br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
        "image/svg+xml",
    );
    let natural = data_url(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 9"/>"#,
        "image/svg+xml",
    );
    let width_only = data_url(
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="100"/>"#,
        "image/svg+xml",
    );
    let height_only = data_url(
        br#"<svg xmlns="http://www.w3.org/2000/svg" height="50"/>"#,
        "image/svg+xml",
    );
    let imported = crate::import_html(
        &format!(
            r#"<img src="{no_ratio}" style="width:160px;aspect-ratio:auto 1/1">
                <img src="{no_ratio}" style="width:160px;aspect-ratio:1/1 auto">
                <img src="{natural}" style="width:160px;aspect-ratio:auto 1/1">
                <img src="{width_only}" style="width:160px;height:auto">
                <img src="{height_only}" style="width:auto;height:100px">"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let images = images(&imported.nodes);
    assert_eq!(number(images[0].height.as_ref()), Some(160.0));
    assert_eq!(number(images[1].height.as_ref()), Some(160.0));
    assert_close(number(images[2].height.as_ref()), 90.0);
    assert_eq!(number(images[3].height.as_ref()), Some(150.0));
    assert_eq!(number(images[4].width.as_ref()), Some(300.0));
}

#[test]
fn inline_svg_uses_the_same_natural_ratio_rule() {
    let imported = crate::import_html(
        r#"<svg style="width:160px;aspect-ratio:auto 1/1"></svg>
            <svg viewBox="0 0 16 9" style="width:160px;aspect-ratio:auto 1/1"></svg>"#,
        &crate::HtmlImportOptions::default(),
    );
    let images = images(&imported.nodes);
    assert_eq!(number(images[0].height.as_ref()), Some(160.0));
    assert_close(number(images[1].height.as_ref()), 90.0);
}

#[test]
fn root_font_recompute_uses_the_initial_rem_basis_once() {
    let image = data_url(&png(320, 180), "image/png");
    let imported = crate::import_html(
        &format!(
            r#"<style>html{{font-size:2rem}}body{{margin:0}}</style>
                <div style="padding:1rem"><img src="{image}" style="width:100%;height:auto"></div>"#
        ),
        &crate::HtmlImportOptions {
            viewport_width: 200.0,
            ..Default::default()
        },
    );
    // Root font is 32px, so the two 1rem padding edges leave 136px.
    assert_eq!(
        number(images(&imported.nodes)[0].height.as_ref()),
        Some(76.5)
    );
}

#[test]
fn reverse_fill_height_survives_an_indefinite_inline_width() {
    let image = data_url(&png(100, 50), "image/png");
    let imported = crate::import_html(
        &format!(
            r#"<span style="display:inline-block;height:200px"><img src="{image}" style="height:100%;width:auto"></span>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(
        number(images(&imported.nodes)[0].width.as_ref()),
        Some(400.0)
    );
}

#[test]
fn auto_fill_ancestor_limits_constrain_the_content_anchor() {
    let image = data_url(&png(320, 180), "image/png");
    let imported = crate::import_html(
        &format!(
            r#"<div style="max-width:200px"><img src="{image}" style="width:100%;height:auto"></div>
                <section style="height:300px"><div style="height:100%;max-height:100px"><img src="{image}" style="height:100%;width:auto"></div></section>"#
        ),
        &crate::HtmlImportOptions {
            viewport_width: 500.0,
            ..Default::default()
        },
    );
    let images = images(&imported.nodes);
    assert_eq!(number(images[0].height.as_ref()), Some(112.5));
    assert_close(number(images[1].width.as_ref()), 1600.0 / 9.0);
}
