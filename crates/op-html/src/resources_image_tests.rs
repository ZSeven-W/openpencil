use super::*;
use jian_ops_schema::node::{FrameNode, ImageNode};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use serde_json::json;
use std::cell::{Cell, RefCell};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0; 24];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

fn number(axis: Option<&SizingBehavior>) -> Option<f64> {
    match axis {
        Some(SizingBehavior::Number(value)) => Some(*value),
        _ => None,
    }
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("numeric axis");
    assert!((actual - expected).abs() < 1.0e-6, "{actual} != {expected}");
}

fn collect_images<'a>(nodes: &'a [PenNode], output: &mut Vec<&'a ImageNode>) {
    for node in nodes {
        if let PenNode::Image(image) = node {
            output.push(image);
        }
        if let Some(children) = node_children(node) {
            collect_images(children, output);
        }
    }
}

fn images(nodes: &[PenNode]) -> Vec<&ImageNode> {
    let mut output = Vec::new();
    collect_images(nodes, &mut output);
    output
}

fn node_children(node: &PenNode) -> Option<&[PenNode]> {
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
        if let Some(frame) = node_children(node).and_then(|children| frame_named(children, name)) {
            return Some(frame);
        }
    }
    None
}

fn options() -> crate::HtmlImportOptions {
    crate::HtmlImportOptions {
        base_url: Some("https://assets.test/page.html".to_string()),
        ..Default::default()
    }
}

fn embed_with_fetcher(
    nodes: &mut [PenNode],
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
) {
    embed_images(
        nodes,
        Some("https://assets.test/page.html"),
        Some(fetcher),
        transform,
        &mut ResourceBudget::default(),
        &mut Vec::new(),
        &mut ImageResourceCache::default(),
    );
}

#[test]
fn fetched_image_cache_deduplicates_bytes_and_intrinsic_dimensions() {
    let fetched = Cell::new(0);
    let bytes = png(320, 180);
    let fetcher = |url: &str| {
        assert_eq!(url, "https://assets.test/photo.png");
        fetched.set(fetched.get() + 1);
        Some(bytes.clone())
    };
    let imported = crate::import_html_with_resources(
        r#"<img src="photo.png"><img src="photo.png">"#,
        &options(),
        Some(&fetcher),
        None,
    );

    let images = images(&imported.nodes);
    assert_eq!(fetched.get(), 1);
    assert_eq!(images.len(), 2);
    for image in &images {
        assert_eq!(number(image.width.as_ref()), Some(320.0));
        assert_eq!(number(image.height.as_ref()), Some(180.0));
        assert!(image.src.as_str().starts_with("data:image/png;base64,"));
    }
    assert_eq!(images[0].src, images[1].src);
}

#[test]
fn numeric_and_authored_axes_are_resolved_before_embedding() {
    let bytes = png(320, 180);
    let fetcher = |_: &str| Some(bytes.clone());
    let imported = crate::import_html_with_resources(
        r#"<img src="photo.png" width="160">
            <img src="photo.png" height="45">
            <img src="photo.png" width="10" height="20">"#,
        &options(),
        Some(&fetcher),
        None,
    );

    let images = images(&imported.nodes);
    assert_eq!(
        (
            number(images[0].width.as_ref()),
            number(images[0].height.as_ref())
        ),
        (Some(160.0), Some(90.0))
    );
    assert_eq!(
        (
            number(images[1].width.as_ref()),
            number(images[1].height.as_ref())
        ),
        (Some(80.0), Some(45.0))
    );
    assert_eq!(
        (
            number(images[2].width.as_ref()),
            number(images[2].height.as_ref())
        ),
        (Some(10.0), Some(20.0))
    );
}

#[test]
fn intrinsic_ratio_applies_all_limits_and_minimum_wins_conflicts() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(320, 180))
    );
    let imported = crate::import_html(
        &format!(
            r#"<img src="{data_url}" width="160" style="max-width:100px">
                <img src="{data_url}" style="max-width:100px">
                <img src="{data_url}" width="160" style="max-height:50px">
                <img src="{data_url}" style="min-width:10px;max-width:0">
                <img src="{data_url}" width="160" height="90" style="max-width:100px;height:auto">"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let images = images(&imported.nodes);
    for index in [0, 1, 4] {
        assert_close(number(images[index].width.as_ref()), 100.0);
        assert_close(number(images[index].height.as_ref()), 56.25);
    }
    assert_close(number(images[2].width.as_ref()), 800.0 / 9.0);
    assert_close(number(images[2].height.as_ref()), 50.0);
    assert_close(number(images[3].width.as_ref()), 10.0);
    assert_close(number(images[3].height.as_ref()), 5.625);
}

#[test]
fn definite_fill_width_derives_height_but_indefinite_dynamic_axis_warns() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(320, 180))
    );
    let definite = crate::import_html(
        &format!(r#"<div style="width:200px"><img src="{data_url}" style="width:100%"></div>"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&definite.nodes)[0];
    assert!(matches!(
        image.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));
    assert_eq!(number(image.height.as_ref()), Some(112.5));

    let constrained = crate::import_html(
        &format!(
            r#"<div style="width:200px"><img src="{data_url}" style="width:100%;height:auto;max-height:50px"></div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&constrained.nodes)[0];
    assert_close(number(image.width.as_ref()), 800.0 / 9.0);
    assert_eq!(number(image.height.as_ref()), Some(50.0));

    let indefinite = crate::import_html(
        &format!(
            r#"<div style="display:inline-flex"><img src="{data_url}" style="width:100%"></div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&indefinite.nodes)[0];
    assert!(matches!(
        image.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));
    assert!(image.height.is_none());
    assert!(indefinite
        .diagnostics
        .contains(&ImportWarning::ImageIntrinsicAxisUnresolved));
}

#[test]
fn fill_anchor_uses_parent_content_box_without_double_counting_root_margins() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(320, 180))
    );
    let imported = crate::import_html(
        &format!(
            r#"<div style="width:200px;padding:10px;border:5px solid">
                    <img src="{data_url}" style="width:100%;height:auto">
                </div>
                <div style="height:200px;padding:10px;border:5px solid">
                    <img src="{data_url}" style="height:100%;width:auto">
                </div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let imported_images = images(&imported.nodes);
    assert_eq!(number(imported_images[0].height.as_ref()), Some(112.5));
    assert_eq!(
        number(imported_images[1].width.as_ref()),
        Some(3200.0 / 9.0)
    );

    let root = crate::import_html(
        &format!(r#"<img src="{data_url}" style="width:100%;height:auto">"#),
        &crate::HtmlImportOptions {
            viewport_width: 200.0,
            ..Default::default()
        },
    );
    let image = images(&root.nodes)[0];
    // UA body margin is 8px on both inline edges: 200 - 16 = 184.
    assert_close(number(image.height.as_ref()), 103.5);

    let nested_fill = crate::import_html(
        &format!(
            r#"<div style="padding:10px"><div style="padding:10px"><img src="{data_url}" style="width:100%;height:auto"></div></div>"#
        ),
        &crate::HtmlImportOptions {
            viewport_width: 200.0,
            ..Default::default()
        },
    );
    let image = images(&nested_fill.nodes)[0];
    // 200 viewport - 16 body margin - 20 - 20 nested padding = 144.
    assert_eq!(number(image.height.as_ref()), Some(81.0));

    let percentage_padding = crate::import_html(
        &format!(
            r#"<div style="width:200px;padding:10%"><img src="{data_url}" style="width:100%;height:auto"></div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&percentage_padding.nodes)[0];
    assert!(matches!(
        image.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));
    assert_eq!(number(image.height.as_ref()), Some(112.5));
    assert!(!percentage_padding
        .diagnostics
        .contains(&ImportWarning::ImageIntrinsicAxisUnresolved));
}

#[test]
fn definite_fill_height_derives_width_and_applies_both_axis_limits() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(100, 50))
    );
    let imported = crate::import_html(
        &format!(
            r#"<div style="height:200px"><img src="{data_url}" style="height:100%;width:auto"></div>
                <div style="height:200px"><img src="{data_url}" style="height:100%;width:auto;max-height:150px;max-width:350px"></div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let images = images(&imported.nodes);
    assert_eq!(number(images[0].width.as_ref()), Some(400.0));
    assert!(matches!(
        images[0].height,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));
    assert_eq!(number(images[1].width.as_ref()), Some(300.0));
    assert_eq!(number(images[1].height.as_ref()), Some(150.0));
}

#[test]
fn expression_axis_is_preserved_and_reports_the_dedicated_diagnostic() {
    let options = crate::HtmlImportOptions::default();
    let mut context = crate::mapper::MapCtx {
        opts: &options,
        rules: &[],
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: options.viewport_width,
        containing_height: options.viewport_height(),
        containing_width_is_definite: true,
        positioned_width: options.viewport_width,
        positioned_height: options.viewport_height(),
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    let mut width = Some(SizingBehavior::Expression("$image-width".to_string()));
    let mut height = None;

    crate::special::apply_intrinsic_axes(
        &mut width,
        &mut height,
        &SizeLimits::default(),
        BrowserImageMetadata {
            dimensions: (320.0, 180.0),
            preferred_ratio: Some(16.0 / 9.0),
        },
        (None, None),
        false,
        &mut context,
    );

    assert!(
        matches!(width, Some(SizingBehavior::Expression(ref value)) if value == "$image-width")
    );
    assert!(height.is_none());
    assert_eq!(
        context.warnings,
        [ImportWarning::ImageIntrinsicAxisUnresolved]
    );
}

#[test]
fn base64_probe_is_bounded_uncached_and_malformed_inputs_stay_unsized() {
    let valid_payload = base64::engine::general_purpose::STANDARD.encode(png(48, 24));
    let valid = format!("data:image/png;base64,{valid_payload}");
    let mut oversized = valid.clone();
    oversized.extend(std::iter::repeat_n(
        'A',
        MAX_IMAGE_METADATA_BASE64_CHARS * 2,
    ));
    assert_eq!(data_url_image_dimensions(&oversized), Some((48.0, 24.0)));
    assert_eq!(data_url_image_dimensions("data:image/png;base64,%%%"), None);
    assert_eq!(
        data_url_image_dimensions("data:image/png;base64,iVBORw0KGgo="),
        None
    );

    let mut cache = HashMap::new();
    let embedded = embed_url(
        &oversized,
        None,
        None,
        None,
        &mut ResourceBudget::default(),
        &mut Vec::new(),
        &mut cache,
        false,
    )
    .expect("valid data URL metadata");
    assert_eq!(embedded.dimensions, Some((48.0, 24.0)));
    assert!(cache.is_empty(), "data URLs must not become cache keys");
}

#[test]
fn data_scheme_and_svg_metadata_follow_browser_replaced_sizing() {
    let percent_square = "   DATA:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20100%20100%22%2F%3E";
    assert_eq!(
        data_url_image_dimensions(percent_square),
        Some((150.0, 150.0))
    );
    let one_axis = format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD
            .encode(br#"<svg width="100%" height="20" viewBox="0 0 200 100"/>"#)
    );
    assert_eq!(data_url_image_dimensions(&one_axis), Some((40.0, 20.0)));

    let imported = crate::import_html(
        &format!(r#"<img src="{percent_square}"><img src="{one_axis}" width="80">"#),
        &crate::HtmlImportOptions::default(),
    );
    let images = images(&imported.nodes);
    assert!(images[0].src.as_str().starts_with("data:"));
    assert!(!images[0].src.as_str().starts_with(char::is_whitespace));
    assert_eq!(number(images[0].width.as_ref()), Some(150.0));
    assert_eq!(number(images[0].height.as_ref()), Some(150.0));
    assert_eq!(number(images[1].width.as_ref()), Some(80.0));
    assert_eq!(number(images[1].height.as_ref()), Some(40.0));

    let mismatched = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(br#"<svg width="10" height="10"/>"#)
    );
    assert_eq!(data_url_image_dimensions(&mismatched), None);
    let oversized_header = format!("data:image/{}", "x".repeat(2_000));
    assert_eq!(data_url_image_dimensions(&oversized_header), None);
    assert_eq!(
        image_mime(br#"<?xml version="1.0"?><!-- icon --><svg viewBox="0 0 2 1"/>"#),
        "image/svg+xml"
    );
}

#[test]
fn inline_svg_applies_authored_aspect_ratio_before_intrinsic_fallback() {
    let imported = crate::import_html(
        r#"<svg viewBox="0 0 320 180" style="width:160px;aspect-ratio:1/1"></svg>"#,
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&imported.nodes)[0];
    assert_eq!(number(image.width.as_ref()), Some(160.0));
    assert_eq!(number(image.height.as_ref()), Some(160.0));
}

#[test]
fn special_leaf_fallback_descendants_are_not_prefetched_or_annotated() {
    let fetched = RefCell::new(Vec::new());
    let bytes = png(32, 32);
    let fetcher = |url: &str| {
        fetched.borrow_mut().push(url.to_string());
        Some(bytes.clone())
    };
    let imported = crate::import_html_with_resources(
        r#"<canvas><img src="canvas-fallback.png"></canvas>
            <svg viewBox="0 0 10 10"><foreignObject><img src="svg-foreign.png"></foreignObject></svg>
            <img src="visible.png">"#,
        &options(),
        Some(&fetcher),
        None,
    );
    assert_eq!(fetched.into_inner(), ["https://assets.test/visible.png"]);
    let svg = images(&imported.nodes)
        .into_iter()
        .find(|image| image.base.name.as_deref() == Some("svg"))
        .expect("inline SVG placeholder");
    let payload = svg.src.as_str().split_once(',').expect("SVG data URL").1;
    let serialized = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .expect("valid SVG payload");
    assert!(
        !serialized.contains(&0),
        "private NUL attrs leaked into SVG"
    );
}

#[test]
fn plain_html_import_runs_intrinsic_sizing_without_a_fetcher() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(96, 54))
    );
    let imported = crate::import_html(
        &format!(r#"<img src="{data_url}">"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&imported.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(96.0), Some(54.0))
    );
}

#[test]
fn authored_aspect_ratio_result_wins_over_intrinsic_ratio() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(320, 180))
    );
    let imported = crate::import_html(
        &format!(r#"<img src="{data_url}" style="width:160px;aspect-ratio:1/1">"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&imported.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(160.0), Some(160.0))
    );

    let auto_fallback = crate::import_html(
        &format!(r#"<img src="{data_url}" style="width:160px;aspect-ratio:auto 1/1">"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&auto_fallback.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(160.0), Some(90.0))
    );

    let reversed_auto = crate::import_html(
        &format!(r#"<img src="{data_url}" style="width:160px;aspect-ratio:1/1 auto">"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&reversed_auto.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(160.0), Some(90.0))
    );
}

#[test]
fn intrinsic_box_is_available_to_transform_and_absolute_positioning() {
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(100, 50))
    );
    let transformed = crate::import_html(
        &format!(r#"<img src="{data_url}" style="transform:scale(2)">"#),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&transformed.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(200.0), Some(100.0))
    );
    assert!(transformed
        .diagnostics
        .contains(&ImportWarning::TransformScaleBaked));
    assert!(!transformed
        .diagnostics
        .contains(&ImportWarning::TransformScaleNotBaked));

    let positioned = crate::import_html(
        &format!(
            r#"<div style="position:relative;width:300px;height:200px">
                <img src="{data_url}" style="position:absolute;right:10px;bottom:20px">
            </div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&positioned.nodes)[0];
    assert_eq!(image.base.x, Some(190.0));
    assert_eq!(image.base.y, Some(130.0));

    let centred = crate::import_html(
        &format!(
            r#"<div style="position:relative;width:300px;height:200px">
                <img src="{data_url}" style="position:absolute;left:10px;right:10px;margin-left:auto;margin-right:auto">
            </div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&centred.nodes)[0];
    assert_eq!(number(image.width.as_ref()), Some(100.0));
    assert_eq!(image.base.x, Some(100.0));

    let wide_data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(320, 90))
    );
    let attributed = crate::import_html(
        &format!(
            r#"<div style="position:relative;width:500px;height:200px">
                <img src="{wide_data_url}" width="100" style="position:absolute;left:0;right:0;margin-left:auto;margin-right:auto">
            </div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    let image = images(&attributed.nodes)[0];
    assert_eq!(number(image.width.as_ref()), Some(100.0));
    assert_close(number(image.height.as_ref()), 28.125);
    assert_eq!(image.base.x, Some(200.0));
}

#[test]
fn original_dimensions_survive_an_image_transform() {
    let original = png(320, 180);
    let fetcher = |_: &str| Some(original.clone());
    let transformed = png(1, 1);
    let transform = |_: &[u8]| Some(transformed.clone());
    let imported = crate::import_html_with_resources(
        r#"<img src="photo.png">"#,
        &options(),
        Some(&fetcher),
        Some(&transform),
    );

    let image = images(&imported.nodes)[0];
    assert_eq!(
        (number(image.width.as_ref()), number(image.height.as_ref())),
        (Some(320.0), Some(180.0))
    );
    let payload = image
        .src
        .as_str()
        .strip_prefix("data:image/png;base64,")
        .expect("transformed PNG data URL");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(payload)
            .unwrap(),
        png(1, 1)
    );
}

#[test]
fn image_fill_embedding_does_not_change_container_sizing() {
    let mut nodes = vec![serde_json::from_value(json!({
        "type": "rectangle",
        "id": "background",
        "fill": [{"type": "image", "url": "photo.png", "mode": "crop"}]
    }))
    .expect("valid rectangle fixture")];
    let bytes = png(320, 180);
    let fetcher = |_: &str| Some(bytes.clone());
    embed_with_fetcher(&mut nodes, &fetcher, None);

    let PenNode::Rectangle(rectangle) = &nodes[0] else {
        panic!("expected rectangle")
    };
    assert!(rectangle.container.width.is_none());
    assert!(rectangle.container.height.is_none());
    assert!(matches!(
        rectangle.container.fill.as_deref(),
        Some([PenFill::Image(fill)]) if fill.url.as_str().starts_with("data:image/png;base64,")
    ));
}

#[test]
fn unavailable_and_malformed_remote_images_do_not_gain_placeholder_dimensions() {
    let fetcher = |url: &str| match url {
        "https://assets.test/malformed.png" => Some(b"not an image".to_vec()),
        _ => None,
    };
    let imported = crate::import_html_with_resources(
        r#"<img src="missing.png"><img src="malformed.png">"#,
        &options(),
        Some(&fetcher),
        None,
    );
    for image in images(&imported.nodes) {
        assert!(image.width.is_none());
        assert!(image.height.is_none());
    }
}

#[test]
fn remote_and_data_images_shape_flex_wrap_before_row_chunking() {
    fn assert_two_plus_one(imported: &crate::HtmlImportResult) {
        let frame = frame_named(&imported.nodes, "div").expect("wrapping div");
        let rows = frame.children.as_deref().expect("wrap rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(node_children(&rows[0]).map(<[PenNode]>::len), Some(2));
        assert_eq!(node_children(&rows[1]).map(<[PenNode]>::len), Some(1));
    }

    let bytes = png(120, 80);
    let fetcher = |_: &str| Some(bytes.clone());
    let remote = crate::import_html_with_resources(
        r#"<div style="display:flex;flex-wrap:wrap;width:250px;gap:0">
            <img src="photo.png"><img src="photo.png"><img src="photo.png">
        </div>"#,
        &options(),
        Some(&fetcher),
        None,
    );
    assert_two_plus_one(&remote);

    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(120, 80))
    );
    let data = crate::import_html(
        &format!(
            r#"<div style="display:flex;flex-wrap:wrap;width:250px;gap:0">
                <img src="{data_url}"><img src="{data_url}"><img src="{data_url}">
            </div>"#
        ),
        &crate::HtmlImportOptions::default(),
    );
    assert_two_plus_one(&data);
}

#[test]
fn srcset_density_corrects_natural_size_per_element_without_refetching() {
    let fetched = Cell::new(0);
    let bytes = png(320, 180);
    let fetcher = |url: &str| {
        assert_eq!(url, "https://assets.test/shared.png");
        fetched.set(fetched.get() + 1);
        Some(bytes.clone())
    };
    let imported = crate::import_html_with_resources(
        r#"<img src="shared.png"><img srcset="shared.png 2x">
            <img srcset="shared.png 640w" sizes="320px">"#,
        &options(),
        Some(&fetcher),
        None,
    );
    let images = images(&imported.nodes);
    assert_eq!(fetched.get(), 1);
    assert_eq!(number(images[0].width.as_ref()), Some(320.0));
    assert_eq!(number(images[1].width.as_ref()), Some(160.0));
    assert_eq!(number(images[2].width.as_ref()), Some(160.0));
    assert_eq!(number(images[2].height.as_ref()), Some(90.0));
}

#[test]
fn hidden_images_are_not_prefetched() {
    let fetched = RefCell::new(Vec::new());
    let bytes = png(32, 32);
    let fetcher = |url: &str| {
        fetched.borrow_mut().push(url.to_string());
        Some(bytes.clone())
    };
    let imported = crate::import_html_with_resources(
        r#"<img src="self-hidden.png" style="display:none">
            <div style="display:none"><img src="ancestor-hidden.png"></div>
            <img src="visible.png">"#,
        &options(),
        Some(&fetcher),
        None,
    );
    assert_eq!(fetched.into_inner(), ["https://assets.test/visible.png"]);
    assert_eq!(images(&imported.nodes).len(), 1);
}

#[test]
fn standalone_and_background_share_fetch_without_changing_fill_sizing() {
    let fetched = Cell::new(0);
    let bytes = png(200, 100);
    let fetcher = |_: &str| {
        fetched.set(fetched.get() + 1);
        Some(bytes.clone())
    };
    let imported = crate::import_html_with_resources(
        r#"<div style="width:100px;background-image:url(photo.png)">
            <img src="photo.png">
        </div>"#,
        &options(),
        Some(&fetcher),
        None,
    );
    assert_eq!(fetched.get(), 1);
    let frame = frame_named(&imported.nodes, "div").expect("background frame");
    assert_eq!(number(frame.container.width.as_ref()), Some(100.0));
    assert!(matches!(
        frame.container.fill.as_deref(),
        Some([PenFill::Image(fill)]) if fill.url.as_str().starts_with("data:image/png;base64,")
    ));
}

#[test]
fn prefetched_resource_warnings_keep_final_tree_order() {
    let fetcher = |_: &str| None;
    let imported = crate::import_html_with_resources(
        r#"<div style="background-image:url(background.png)">
            <img src="foreground.png">
        </div>"#,
        &options(),
        Some(&fetcher),
        None,
    );
    let resource_warnings: Vec<_> = imported
        .warnings
        .iter()
        .filter(|warning| warning.contains("resource unavailable"))
        .collect();
    assert_eq!(resource_warnings.len(), 2);
    assert!(resource_warnings[0].contains("background.png"));
    assert!(resource_warnings[1].contains("foreground.png"));
}
