//! HTML and CSS → editable PenNode importer.

use jian_ops_schema::document::PenDocument;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{PenFill, SolidFillBody};

pub mod color;
pub mod css;
pub mod css_encoding;
pub mod dom;
pub(crate) mod font_face;
pub mod html_encoding;
pub mod import_warning;
mod import_warning_display;
#[cfg(test)]
mod import_warning_samples;
#[cfg(test)]
mod import_warning_tests;
pub mod length;
pub(crate) mod list_markers;
pub mod mapper;
mod project;
mod project_zip;
mod project_zip_range;
pub mod resources;
pub mod snapshot;
pub mod special;
pub(crate) mod srcset;
pub mod text;
pub mod transform;
mod zip_encoding;

pub use import_warning::{
    diagnostic_parts, render_warnings, ImportDiagnosticParts, ImportWarning,
    ALL_IMPORT_WARNING_CODES, IMPORT_WARNING_KEY_PREFIX,
};
pub use project::{
    import_html_project, import_html_project_document, import_html_project_document_with_transform,
    import_html_project_with_transform, select_html_entry, HtmlProjectError, HtmlProjectFile,
    MAX_PROJECT_FILES, VIRTUAL_PROJECT_ORIGIN,
};
pub use project_zip::{
    extract_html_zip, extract_html_zip_reader, import_html_zip, import_html_zip_document,
    import_html_zip_document_reader, import_html_zip_document_reader_with_transform,
    import_html_zip_document_with_transform, import_html_zip_reader,
    import_html_zip_reader_document, import_html_zip_reader_document_with_transform,
    import_html_zip_reader_with_transform, import_html_zip_with_transform, MAX_ZIP_ENTRIES,
};
pub use project_zip_range::{
    decode_html_zip_entry, html_zip_entry_data_range, html_zip_local_header_length,
    html_zip_project_url, locate_html_zip_directory, parse_html_zip_central_directory,
    validate_html_zip_local_header, HtmlZipByteRange, HtmlZipCompressionMethod,
    HtmlZipDirectoryLocator, HtmlZipRangeEntry, HtmlZipRangeManifest, HTML_ZIP_EOCD_MIN_BYTES,
    HTML_ZIP_LOCAL_HEADER_FIXED_BYTES, HTML_ZIP_TAIL_BYTES,
};
pub use snapshot::{import_snapshot, import_snapshot_document};

pub use css::MediaQueryError;

#[cfg(test)]
mod e2e_tests;

#[cfg(test)]
mod e2e_tailwind_tests;

#[cfg(test)]
mod e2e_content_tests;

#[cfg(test)]
mod stylesheet_resource_tests;

#[cfg(test)]
mod project_zip_range_unicode_tests;

pub struct HtmlImportOptions {
    pub viewport_width: f64,
    /// Import viewport height. `None` keeps the historical 16:10 estimate
    /// (`viewport_width * 0.625`). Every `vh` / `100vh` / `max-height` media
    /// query resolves against `HtmlImportOptions::viewport_height()`.
    pub viewport_height: Option<f64>,
    pub base_font_size: f64,
    pub document_name: Option<String>,
    pub base_url: Option<String>,
}

/// The 16:10 fallback applied when the caller does not pin a viewport height.
pub const DEFAULT_VIEWPORT_ASPECT: f64 = 0.625;

impl HtmlImportOptions {
    /// Single resolution point for the import viewport height. Non-finite or
    /// non-positive overrides fall back to the aspect-derived default so a
    /// bad caller value can never poison percentage/`vh` resolution.
    pub fn viewport_height(&self) -> f64 {
        self.viewport_height
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(self.viewport_width * DEFAULT_VIEWPORT_ASPECT)
    }
}

impl Default for HtmlImportOptions {
    fn default() -> Self {
        Self {
            viewport_width: 1440.0,
            viewport_height: None,
            base_font_size: 16.0,
            document_name: None,
            base_url: None,
        }
    }
}

pub struct HtmlImportResult {
    pub nodes: Vec<PenNode>,
    /// Rendered warning text, in emission order. Kept for the CLI JSON and
    /// MCP payloads, whose wording callers already depend on.
    pub warnings: Vec<String>,
    /// The same warnings, typed. Each carries a stable
    /// [`ImportWarning::code`] and an [`ImportWarning::i18n_key`] for the
    /// post-import diagnostics panel.
    pub diagnostics: Vec<ImportWarning>,
}

impl HtmlImportResult {
    /// Append one more diagnostic, keeping the rendered mirror in step.
    pub fn push_warning(&mut self, warning: ImportWarning) {
        self.warnings.push(warning.to_string());
        self.diagnostics.push(warning);
    }

    /// Build a result from typed diagnostics, rendering the compatibility
    /// string list from them so the two views can never drift.
    pub fn new(nodes: Vec<PenNode>, diagnostics: Vec<ImportWarning>) -> Self {
        Self {
            nodes,
            warnings: import_warning::render_warnings(&diagnostics),
            diagnostics,
        }
    }
}

/// A whole imported document plus both views of what degraded.
///
/// **Mirror invariant:** `warnings` is `render_warnings(&diagnostics)` — same
/// length, same order, element `i` of one is the rendered form of element `i`
/// of the other. Callers rely on it (`op-cli` prints `warnings` while the
/// hosts localize `diagnostics`, and a test asserts the two agree), so build
/// one with [`HtmlDocumentResult::new`] and extend it only through
/// [`HtmlDocumentResult::push_warning`]; a bare struct literal or a direct
/// `warnings.push` can silently break the pairing.
pub struct HtmlDocumentResult {
    pub document: PenDocument,
    pub warnings: Vec<String>,
    /// Typed mirror of `warnings`; see [`HtmlImportResult::diagnostics`].
    pub diagnostics: Vec<ImportWarning>,
}

impl HtmlDocumentResult {
    /// Build a result from typed diagnostics, rendering the compatibility
    /// string list from them so the two views can never drift.
    pub fn new(document: PenDocument, diagnostics: Vec<ImportWarning>) -> Self {
        Self {
            document,
            warnings: import_warning::render_warnings(&diagnostics),
            diagnostics,
        }
    }

    /// Append one more diagnostic, keeping the rendered mirror in step.
    pub fn push_warning(&mut self, warning: ImportWarning) {
        self.warnings.push(warning.to_string());
        self.diagnostics.push(warning);
    }
}

pub fn import_html(source: &str, opts: &HtmlImportOptions) -> HtmlImportResult {
    import_html_with_resources(source, opts, None, None)
}

pub fn import_html_with_resources(
    source: &str,
    opts: &HtmlImportOptions,
    fetcher: Option<&resources::ResourceFetcher<'_>>,
    transform: Option<&resources::ImageTransform<'_>>,
) -> HtmlImportResult {
    let mut warnings = Vec::new();
    if source.trim().is_empty() {
        warnings.push(ImportWarning::EmptyInput);
        return HtmlImportResult::new(Vec::new(), warnings);
    }
    let mut parsed = dom::parse_dom(source);
    if parsed.depth_truncated {
        warnings.push(ImportWarning::DomDepthTruncated {
            max_depth: dom::MAX_DOM_DEPTH,
        });
    }
    if parsed.body.is_empty() {
        warnings.push(ImportWarning::EmptyBody);
        return HtmlImportResult::new(Vec::new(), warnings);
    }
    let mut remaining = MAX_OUTPUT_NODES - 1;
    if truncate_dom_nodes(&mut parsed.body, &mut remaining) {
        warnings.push(ImportWarning::NodeLimitTruncated);
    }
    let resource_base = resources::select_document_base(
        opts.base_url.as_deref(),
        &parsed.base_hrefs,
        &mut warnings,
    );

    let viewport_height = opts.viewport_height();
    let mut ua_parser =
        css::cascade::StylesheetParser::new(css::cascade::StyleOrigin::UserAgent, 0);
    let (mut rules, ua_warnings) = ua_parser.parse_for_viewport(
        css::cascade::UA_STYLESHEET,
        opts.viewport_width,
        viewport_height,
    );
    warnings.extend(ua_warnings);
    let mut budget = resources::ResourceBudget;
    let mut author_parser =
        css::cascade::StylesheetParser::new(css::cascade::StyleOrigin::Author, 500);
    for stylesheet_source in std::mem::take(&mut parsed.stylesheet_sources) {
        let stylesheet = match stylesheet_source {
            dom::StylesheetSource::Inline(stylesheet) => resources::expand_stylesheet_imports(
                &stylesheet,
                resource_base.as_deref(),
                None,
                fetcher,
                &mut budget,
                &mut warnings,
            ),
            dom::StylesheetSource::Link(href) => {
                let resolved = resources::resolve_resource_url(resource_base.as_deref(), &href);
                let display_url = resolved.as_deref().unwrap_or(&href);
                if !budget.take(&mut warnings) {
                    continue;
                }
                let Some(bytes) = resolved
                    .as_deref()
                    .and_then(|url| fetcher.and_then(|fetch| fetch(url)))
                else {
                    warnings.push(ImportWarning::ExternalStylesheetSkipped {
                        url: display_url.to_string(),
                    });
                    continue;
                };
                let decoded = css_encoding::decode_css_bytes(&bytes);
                resources::expand_stylesheet_imports(
                    &decoded,
                    Some(display_url),
                    Some(display_url),
                    fetcher,
                    &mut budget,
                    &mut warnings,
                )
            }
        };
        let (author_rules, stylesheet_warnings) =
            author_parser.parse_for_viewport(&stylesheet, opts.viewport_width, viewport_height);
        rules.extend(author_rules);
        warnings.extend(stylesheet_warnings);
    }
    // Kept before the parser is dropped; reported once the mapped nodes reveal
    // which of the declared families text actually asks for.
    let web_fonts = author_parser.web_fonts().to_vec();

    let body = dom::DomElement {
        tag: "body".to_string(),
        attrs: parsed.body_attrs,
        children: parsed.body,
    };
    // Keep the browser-created document chain for selector matching and
    // inheritance, while continuing to emit the body as the import root.
    let document_element = dom::DomElement {
        tag: "html".to_string(),
        attrs: parsed.html_attrs,
        children: vec![
            dom::DomNode::Element(dom::DomElement {
                tag: "head".to_string(),
                attrs: parsed.head_attrs,
                children: Vec::new(),
            }),
            dom::DomNode::Element(body),
        ],
    };
    let dom::DomNode::Element(body) = &document_element.children[1] else {
        unreachable!("the synthesized document chain always contains a body")
    };
    let document_style = css::cascade::compute_style_for_viewport(
        &[&document_element],
        &rules,
        None,
        opts.base_font_size,
        opts.viewport_width,
        viewport_height,
    );
    let root_font_size = document_style.font_size;
    let body_path = [&document_element, body];
    let body_style = css::cascade::compute_style_for_viewport(
        &body_path,
        &rules,
        Some(&document_style),
        root_font_size,
        opts.viewport_width,
        viewport_height,
    );
    let mapping_opts = HtmlImportOptions {
        viewport_width: opts.viewport_width,
        viewport_height: Some(viewport_height),
        base_font_size: root_font_size,
        document_name: opts.document_name.clone(),
        base_url: resource_base.clone(),
    };
    let mut context = mapper::MapCtx {
        opts: &mapping_opts,
        rules: &rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 1,
        containing_width: opts.viewport_width,
        containing_height: viewport_height,
        containing_width_is_definite: true,
        positioned_width: opts.viewport_width,
        positioned_height: viewport_height,
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    let root_id = context.generate_id();
    let mut container = mapper::container_props_from(&body_style, &mut context);
    container.width = Some(SizingBehavior::Number(opts.viewport_width));
    container.height = container
        .height
        .or(Some(SizingBehavior::Keyword(SizingKeyword::FitContent)));
    let mut body_children_centered = false;
    if container.align_items.is_none() {
        container.align_items =
            mapper::infer_child_alignment(&context, &body_path, &body_style, &body.children);
        body_children_centered = container.align_items.is_some();
    }
    if container.fill.is_none() {
        container.fill = Some(vec![solid_fill("#ffffff")]);
    }
    let name = opts
        .document_name
        .clone()
        .or(parsed.title)
        .unwrap_or_else(|| "HTML Import".to_string());
    let mut base = PenNodeBase {
        id: root_id,
        name: Some(name),
        ..Default::default()
    };
    // The document root has no parent to reserve an un-shifted box in, so an
    // in-flow offset on `<body>` is reported rather than silently applied.
    let body_outcome = mapper::apply_base_style(&mut base, &body_style, &mut context);
    mapper::warn_dropped_flow_offset(&mut context, body_outcome);
    if base
        .explain
        .as_deref()
        .is_some_and(|value| value.starts_with("__op_html_z_index:"))
    {
        base.explain = None;
    }
    let body_display_none = body_style.get("display") == Some("none");
    if body_display_none {
        base.visible = Some(false);
    }
    context.containing_width = opts.viewport_width;
    context.containing_height = match container.height.as_ref() {
        Some(SizingBehavior::Number(value)) => *value,
        _ => viewport_height,
    };
    if !matches!(body_style.get("position"), None | Some("static"))
        || body_style
            .get("transform")
            .is_some_and(|value| value != "none")
    {
        context.positioned_width = context.containing_width;
        context.positioned_height = context.containing_height;
    }
    context.auto_margin_handled_by_parent = body_children_centered;
    let children = if body_display_none {
        Vec::new()
    } else {
        let children =
            mapper::map_container_children(&mut context, &body_path, &body_style, &body.children);
        mapper::apply_flex_wrap(&mut context, &body_style, &mut container, children)
    };
    let root = PenNode::Frame(FrameNode {
        base,
        container,
        children: Some(children),
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
        screen: None,
        breakpoint: None,
    });
    warnings.extend(context.warnings);
    let mut nodes = vec![root];
    font_face::warn_undownloaded(&web_fonts, &nodes, &mut warnings);
    if let Some(fetcher) = fetcher {
        resources::embed_images(
            &mut nodes,
            resource_base.as_deref(),
            fetcher,
            transform,
            &mut budget,
            &mut warnings,
        );
    }
    HtmlImportResult::new(nodes, warnings)
}

pub fn import_html_document(
    source: &str,
    opts: &HtmlImportOptions,
    fetcher: Option<&resources::ResourceFetcher<'_>>,
    transform: Option<&resources::ImageTransform<'_>>,
) -> HtmlDocumentResult {
    wrap_imported_document(import_html_with_resources(source, opts, fetcher, transform))
}

pub(crate) fn wrap_imported_document(imported: HtmlImportResult) -> HtmlDocumentResult {
    let name = imported.nodes.first().and_then(|node| match node {
        PenNode::Frame(frame) => frame.base.name.clone(),
        _ => None,
    });
    HtmlDocumentResult::new(
        PenDocument {
            version: "1.0".to_string(),
            name,
            themes: None,
            variables: None,
            pages: None,
            children: imported.nodes,
            format_version: None,
            responsive: None,
            id: None,
            app: None,
            routes: None,
            state: None,
            lifecycle: None,
            logic_modules: None,
            design_md: None,
            conversion: None,
        },
        imported.diagnostics,
    )
}

pub(crate) const MAX_OUTPUT_NODES: usize = 40_000;

fn truncate_dom_nodes(nodes: &mut Vec<dom::DomNode>, remaining: &mut usize) -> bool {
    let original_len = nodes.len();
    let mut keep = 0;
    let mut truncated = false;
    for node in nodes.iter_mut() {
        if *remaining == 0 {
            truncated = true;
            break;
        }
        *remaining -= 1;
        keep += 1;
        if let dom::DomNode::Element(element) = node {
            truncated |= truncate_dom_nodes(&mut element.children, remaining);
        }
    }
    if keep < original_len {
        nodes.truncate(keep);
        truncated = true;
    }
    truncated
}

fn solid_fill(color: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: color.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_nodes_and_a_warning() {
        let r = import_html("", &HtmlImportOptions::default());
        assert!(r.nodes.is_empty());
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("no importable content"));
    }

    #[test]
    fn options_default_values() {
        let o = HtmlImportOptions::default();
        assert_eq!(o.viewport_width, 1440.0);
        assert_eq!(o.base_font_size, 16.0);
        assert!(o.document_name.is_none());
        assert!(o.base_url.is_none());
    }

    #[test]
    fn external_stylesheet_participates_in_cascade() {
        let html = r#"<html><head><link rel="stylesheet" href="site.css"></head>
            <body><p class="hot">x</p></body></html>"#;
        let fetcher = |url: &str| -> Option<Vec<u8>> {
            (url == "https://a.dev/site.css").then(|| b".hot { color: #ff0000 }".to_vec())
        };
        let opts = HtmlImportOptions {
            base_url: Some("https://a.dev/page.html".into()),
            ..Default::default()
        };
        let r = import_html_with_resources(html, &opts, Some(&fetcher), None);
        let PenNode::Frame(root) = &r.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(p) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Text(t) = &p.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let Some(fills) = &t.fill else {
            panic!("text should carry color fill")
        };
        assert!(matches!(&fills[0], PenFill::Solid(s) if s.color == "#ff0000"));
    }

    #[test]
    fn linked_and_inline_stylesheets_follow_document_order() {
        let html = r#"<head><style>.hot{color:red}</style>
            <link rel="stylesheet" href="site.css"></head><body><p class="hot">x</p></body>"#;
        let fetcher = |_: &str| Some(b".hot{color:blue}".to_vec());
        let opts = HtmlImportOptions {
            base_url: Some("https://a.dev/page.html".into()),
            ..Default::default()
        };
        let result = import_html_with_resources(html, &opts, Some(&fetcher), None);
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(paragraph) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert!(matches!(
            text.fill.as_deref(),
            Some([PenFill::Solid(fill)]) if fill.color == "#0000ff"
        ));
    }

    #[test]
    fn missing_fetcher_degrades_with_warning() {
        let html = r#"<link rel="stylesheet" href="https://a.dev/s.css"><p>x</p>"#;
        let r = import_html(html, &HtmlImportOptions::default());
        assert!(r
            .warnings
            .iter()
            .any(|warning| warning.contains("external stylesheet skipped")));
    }

    #[test]
    fn imports_more_than_the_retired_resource_count_limit() {
        let links = (0..205)
            .map(|index| format!(r#"<link rel="stylesheet" href="{index}.css">"#))
            .collect::<String>();
        let html = format!("<head>{links}</head><body><p>ok</p></body>");
        let fetched = std::cell::Cell::new(0usize);
        let fetcher = |_: &str| {
            fetched.set(fetched.get() + 1);
            Some(Vec::new())
        };
        let result = import_html_with_resources(
            &html,
            &HtmlImportOptions {
                base_url: Some("https://a.dev/index.html".into()),
                ..Default::default()
            },
            Some(&fetcher),
            None,
        );

        assert_eq!(fetched.get(), 205);
        assert!(!result
            .warnings
            .iter()
            .any(|warning| warning.contains("resource limit")));
    }

    #[test]
    fn e2e_images_embed_via_fetcher_with_dedup_and_placeholder() {
        let html = r#"<div><img src="a.png"><img src="a.png"><img src="missing.png"></div>"#;
        let png: Vec<u8> = vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3];
        let fetched = std::cell::RefCell::new(0usize);
        let fetcher = |url: &str| -> Option<Vec<u8>> {
            *fetched.borrow_mut() += 1;
            (url == "https://a.dev/a.png").then(|| png.clone())
        };
        let opts = HtmlImportOptions {
            base_url: Some("https://a.dev/p.html".into()),
            ..Default::default()
        };
        let r = import_html_with_resources(html, &opts, Some(&fetcher), None);
        let PenNode::Frame(root) = &r.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(div) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Frame(inline_row) = &div.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let kids = inline_row.children.as_ref().unwrap();
        let PenNode::Image(i1) = &kids[0] else {
            panic!()
        };
        let PenNode::Image(i2) = &kids[1] else {
            panic!()
        };
        let PenNode::Image(i3) = &kids[2] else {
            panic!()
        };
        assert!(i1.src.as_str().starts_with("data:image/png;base64,"));
        assert_eq!(i1.src.as_str(), i2.src.as_str());
        assert_eq!(*fetched.borrow(), 2);
        assert!(i3.src.as_str().starts_with("data:image/png;base64,"));
        assert!(r
            .warnings
            .iter()
            .any(|warning| warning.contains("missing.png")));
    }

    #[test]
    fn e2e_transform_callback_rewrites_bytes() {
        let html = r#"<img src="https://a.dev/big.jpg">"#;
        let fetcher = |_: &str| Some(vec![0xffu8, 0xd8, 9, 9, 9, 9]);
        let transform = |_: &[u8]| Some(vec![0xffu8, 0xd8, 1]);
        let r = import_html_with_resources(
            html,
            &HtmlImportOptions::default(),
            Some(&fetcher),
            Some(&transform),
        );
        let PenNode::Frame(root) = &r.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(inline_row) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Image(img) = &inline_row.children.as_ref().unwrap()[0] else {
            panic!()
        };
        use base64::Engine as _;
        let b64 = img
            .src
            .as_str()
            .strip_prefix("data:image/jpeg;base64,")
            .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .unwrap(),
            vec![0xff, 0xd8, 1]
        );
    }

    #[test]
    fn e2e_document_wrapper_produces_pendocument() {
        let r = import_html_document(
            "<html><head><title>T</title></head><body><p>x</p></body></html>",
            &HtmlImportOptions::default(),
            None,
            None,
        );
        assert_eq!(r.document.children.len(), 1);
        assert_eq!(r.document.name.as_deref(), Some("T"));
    }

    #[test]
    fn body_layout_base_styles_and_generated_content_reach_the_root() {
        let result = import_html(
            "<style>body::before{content:'before'}</style>\
             <body style='display:flex;flex-direction:row;opacity:.4;visibility:hidden;\
                          background:#123456'><p>copy</p></body>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        assert_eq!(
            root.container.layout,
            Some(jian_ops_schema::node::container::LayoutMode::Horizontal)
        );
        assert_eq!(root.base.visible, Some(false));
        assert!(matches!(
            root.base.opacity,
            Some(jian_ops_schema::node::base::NumberOrExpression::Number(value))
                if value == 0.4
        ));
        assert!(root
            .children
            .as_ref()
            .is_some_and(|children| children.len() == 2));
        assert!(matches!(
            root.container.fill.as_deref(),
            Some([PenFill::Solid(fill)]) if fill.color == "#123456"
        ));
    }

    #[test]
    fn document_root_styles_inherit_through_body_and_ancestor_selectors_match() {
        let result = import_html(
            "<style>\
                html[data-theme=night]{--ink:#123456}\
                :root{--copy-size:2rem}\
                body{background:var(--ink)}\
                html body p{color:var(--ink);font-size:var(--copy-size)}\
             </style>\
             <html data-theme=night style='font-size:20px'><body><p>copy</p></body></html>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        assert!(matches!(
            root.container.fill.as_deref(),
            Some([PenFill::Solid(fill)]) if fill.color == "#123456"
        ));
        let PenNode::Frame(paragraph) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(text.font_size, Some(40.0));
        assert!(matches!(
            text.fill.as_deref(),
            Some([PenFill::Solid(fill)]) if fill.color == "#123456"
        ));
    }

    #[test]
    fn fragment_uses_the_generated_html_and_body_ancestor_chain() {
        let result = import_html(
            "<style>:root{--ink:#654321}html body > p{color:var(--ink)}</style><p>x</p>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(paragraph) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert!(matches!(
            text.fill.as_deref(),
            Some([PenFill::Solid(fill)]) if fill.color == "#654321"
        ));
    }

    #[test]
    fn pathological_nesting_reports_depth_truncation() {
        let mut html = String::from("<body>");
        for _ in 0..300 {
            html.push_str("<div>");
        }
        html.push('x');
        for _ in 0..300 {
            html.push_str("</div>");
        }
        let result = import_html(&html, &HtmlImportOptions::default());
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("levels")));
    }
}
