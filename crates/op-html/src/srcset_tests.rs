use super::*;

/// First `ImageNode` src in document order.
fn first_image_src(nodes: &[jian_ops_schema::node::PenNode]) -> Option<String> {
    use jian_ops_schema::node::PenNode;
    for node in nodes {
        if let PenNode::Image(image) = node {
            return Some(image.src.as_str().to_string());
        }
        let children: &[PenNode] = match node {
            PenNode::Frame(frame) => frame.children.as_deref().unwrap_or_default(),
            PenNode::Group(group) => group.children.as_deref().unwrap_or_default(),
            PenNode::Rectangle(rectangle) => rectangle.children.as_deref().unwrap_or_default(),
            _ => &[],
        };
        if let Some(found) = first_image_src(children) {
            return Some(found);
        }
    }
    None
}

fn urls(value: &str) -> Vec<String> {
    parse_srcset(value)
        .into_iter()
        .map(|candidate| candidate.url.to_string())
        .collect()
}

#[test]
fn parses_width_and_density_descriptors() {
    let candidates = parse_srcset("a.png 480w, b.png 800w ,  c.png 1200w");
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[1].url, "b.png");
    assert_eq!(candidates[1].width, Some(800.0));
    assert_eq!(candidates[1].density, None);

    let density = parse_srcset("a.png, a@2x.png 2x, a@3x.png 3x");
    assert_eq!(density[0].density, None);
    assert_eq!(density[2].density, Some(3.0));
}

#[test]
fn a_data_url_keeps_its_commas() {
    assert_eq!(
        urls("data:image/svg+xml;base64,AAA,BBB=  1x , b.png 2x"),
        ["data:image/svg+xml;base64,AAA,BBB=", "b.png"]
    );
}

#[test]
fn large_data_candidates_borrow_the_source_and_candidate_count_is_bounded() {
    let data = format!("data:image/png;base64,{} 1x", "A".repeat(1024 * 1024));
    let candidates = parse_srcset(&data);
    assert_eq!(candidates.len(), 1);
    let start = data.as_ptr() as usize;
    let end = start + data.len();
    let pointer = candidates[0].url.as_ptr() as usize;
    assert!((start..end).contains(&pointer));

    let many = (0..300)
        .map(|index| format!("{index}.png 1x"))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(parse_srcset(&many).len(), 256);
    assert!(!is_decodable(&format!("image/{}", "x".repeat(1024))));
}

#[test]
fn width_selection_takes_the_narrowest_candidate_that_still_covers_the_slot() {
    let candidates = parse_srcset("s.png 320w, m.png 640w, l.png 1280w");
    assert_eq!(select(&candidates, 600.0).unwrap().url, "m.png");
    assert_eq!(select(&candidates, 640.0).unwrap().url, "m.png");
    assert_eq!(select(&candidates, 641.0).unwrap().url, "l.png");
    // Nothing covers the slot: the widest available is the closest match.
    assert_eq!(select(&candidates, 4000.0).unwrap().url, "l.png");
    assert!(select(&[], 100.0).is_none());
}

#[test]
fn density_selection_prefers_one_x() {
    let candidates = parse_srcset("hi.png 2x, base.png 1x, low.png 0.5x");
    assert_eq!(select(&candidates, 1000.0).unwrap().url, "base.png");

    // A bare candidate is 1x by definition.
    let bare = parse_srcset("bare.png, hi.png 2x");
    assert_eq!(select(&bare, 1000.0).unwrap().url, "bare.png");
}

#[test]
fn source_sizes_split_their_media_condition_from_their_length() {
    assert_eq!(split_source_size("100vw"), (None, "100vw"));
    assert_eq!(
        split_source_size("(max-width: 600px) 480px"),
        (Some("(max-width: 600px)"), "480px")
    );
    assert_eq!(
        split_source_size("(min-width:36em) and (max-width:80em) 33.3vw"),
        (Some("(min-width:36em) and (max-width:80em)"), "33.3vw")
    );
    // A `calc()` size is not a media condition even though it starts with a
    // parenthesised run once the function name is stripped.
    assert_eq!(
        split_source_size("calc(100vw - 40px)"),
        (None, "calc(100vw - 40px)")
    );
    assert_eq!(
        split_source_size("(max-width:600px) calc(100vw - 2rem)"),
        (Some("(max-width:600px)"), "calc(100vw - 2rem)")
    );
}

#[test]
fn sizes_resolve_the_first_matching_entry_at_the_import_viewport() {
    let options = crate::HtmlImportOptions {
        viewport_width: 1200.0,
        ..Default::default()
    };
    let rules = Vec::new();
    let mut context = MapCtx {
        opts: &options,
        rules: &rules,
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
    let viewport = (options.viewport_width, options.viewport_height());
    assert_eq!(
        resolve_sizes("(max-width: 600px) 480px, 50vw", viewport, &mut context),
        Some(600.0)
    );
    assert_eq!(
        resolve_sizes("(min-width: 900px) 300px, 50vw", viewport, &mut context),
        Some(300.0)
    );
    assert_eq!(
        resolve_sizes("(min-width: 5000px) 300px", viewport, &mut context),
        None
    );
}

#[test]
fn only_known_raster_and_vector_types_are_decodable() {
    assert!(is_decodable("image/webp"));
    assert!(is_decodable("image/svg+xml"));
    assert!(is_decodable(""));
    assert!(!is_decodable("image/avif"));
    assert!(!is_decodable("image/jxl"));
}

#[test]
fn picture_sources_are_selected_by_media_then_type() {
    let result = crate::import_html(
        r#"<picture>
             <source media="(max-width: 500px)" srcset="small.png 400w">
             <source type="image/avif" srcset="modern.avif 1200w">
             <source srcset="wide-800.png 800w, wide-1600.png 1600w">
             <img src="fallback.png">
           </picture>"#,
        &crate::HtmlImportOptions {
            viewport_width: 1200.0,
            ..Default::default()
        },
    );
    let source = first_image_src(&result.nodes);
    assert_eq!(source.as_deref(), Some("wide-1600.png"));
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn a_narrow_viewport_takes_the_matching_media_source() {
    let result = crate::import_html(
        r#"<picture>
             <source media="(max-width: 500px)" srcset="small.png">
             <source srcset="wide.png">
             <img src="fallback.png">
           </picture>"#,
        &crate::HtmlImportOptions {
            viewport_width: 400.0,
            ..Default::default()
        },
    );
    assert_eq!(first_image_src(&result.nodes).as_deref(), Some("small.png"));
}

/// Without an `<img>` fallback there is nothing else to import, so the
/// undecodable source is taken and reported.
#[test]
fn an_avif_only_picture_without_a_fallback_is_imported_with_a_warning() {
    let result = crate::import_html(
        r#"<picture><source type="image/avif" srcset="only.avif"><img alt="x"></picture>"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(first_image_src(&result.nodes).as_deref(), Some("only.avif"));
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("cannot decode")),
        "{:?}",
        result.warnings
    );
}

/// With an `<img src>` the guaranteed-decodable fallback wins, exactly as it
/// does in a browser that cannot decode AVIF. Nothing is lost, so nothing is
/// reported.
#[test]
fn a_decodable_img_fallback_beats_an_undecodable_source() {
    let result = crate::import_html(
        r#"<picture><source type="image/avif" srcset="only.avif"><img src="fallback.png"></picture>"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(
        first_image_src(&result.nodes).as_deref(),
        Some("fallback.png")
    );
    assert!(
        !result
            .warnings
            .iter()
            .any(|warning| warning.contains("cannot decode")),
        "{:?}",
        result.warnings
    );

    // An `<img srcset>` is a fallback too, even without `src`.
    let via_srcset = crate::import_html(
        r#"<picture><source type="image/avif" srcset="only.avif"><img srcset="fallback.png 800w"></picture>"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(
        first_image_src(&via_srcset.nodes).as_deref(),
        Some("fallback.png")
    );
}

/// `src` is not a candidate source inside `<picture>`, and a `<source>` without
/// `sizes` measures against `100vw` rather than the `<img>`'s `sizes`.
#[test]
fn source_src_is_ignored_and_sizes_do_not_leak_from_the_img() {
    let result = crate::import_html(
        r#"<picture><source src="ignored.png"><img src="used.png"></picture>"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(first_image_src(&result.nodes).as_deref(), Some("used.png"));

    // The `<img sizes>` would pick `narrow.png`; the source's own 100vw default
    // covers the 1200px viewport and takes `wide.png`.
    let sizes = crate::import_html(
        r#"<picture><source srcset="narrow.png 400w, wide.png 1600w">
             <img src="fallback.png" sizes="200px"></picture>"#,
        &crate::HtmlImportOptions {
            viewport_width: 1200.0,
            ..Default::default()
        },
    );
    assert_eq!(first_image_src(&sizes.nodes).as_deref(), Some("wide.png"));
}

/// An or-joined media condition is evaluated, not silently dropped.
#[test]
fn an_or_joined_source_media_condition_is_evaluated() {
    let html = r#"<picture>
           <source media="(max-width: 500px) or (min-width: 1000px)" srcset="edge.png">
           <source srcset="middle.png">
           <img src="fallback.png">
         </picture>"#;
    let wide = crate::import_html(
        html,
        &crate::HtmlImportOptions {
            viewport_width: 1200.0,
            ..Default::default()
        },
    );
    assert_eq!(first_image_src(&wide.nodes).as_deref(), Some("edge.png"));
    assert!(wide.warnings.is_empty(), "{:?}", wide.warnings);

    let middle = crate::import_html(
        html,
        &crate::HtmlImportOptions {
            viewport_width: 800.0,
            ..Default::default()
        },
    );
    assert_eq!(
        first_image_src(&middle.nodes).as_deref(),
        Some("middle.png")
    );
}

/// A condition the evaluator cannot read still does not match, but the reason
/// now reaches the import's warning list instead of being swallowed.
#[test]
fn an_unreadable_source_media_condition_is_reported() {
    let result = crate::import_html(
        r#"<picture><source media="(hover: hover)" srcset="hoverable.png">
             <source srcset="plain.png"><img src="fallback.png"></picture>"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(first_image_src(&result.nodes).as_deref(), Some("plain.png"));
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("@media feature 'hover'")),
        "{:?}",
        result.warnings
    );
}

#[test]
fn a_plain_img_srcset_is_honoured_and_src_remains_the_fallback() {
    let result = crate::import_html(
        r#"<img src="fallback.png" srcset="a-400.png 400w, a-1600.png 1600w" sizes="(min-width: 1000px) 1500px, 100vw">"#,
        &crate::HtmlImportOptions {
            viewport_width: 1200.0,
            ..Default::default()
        },
    );
    assert_eq!(
        first_image_src(&result.nodes).as_deref(),
        Some("a-1600.png")
    );

    let plain = crate::import_html(
        r#"<img src="only.png">"#,
        &crate::HtmlImportOptions::default(),
    );
    assert_eq!(first_image_src(&plain.nodes).as_deref(), Some("only.png"));
}
