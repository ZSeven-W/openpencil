use crate::{import_html, HtmlImportOptions};
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode};
use jian_ops_schema::node::text::{FontWeight, TextAlign, TextContent, TextGrowth, TextNode};
use jian_ops_schema::node::{FrameNode, ImageFitMode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{BlendMode, PenFill};

const LANDING: &str = r#"<html><head><title>Acme</title><style>
  .hero { display:flex; flex-direction:column; align-items:center; gap:24px;
          padding:64px; background:linear-gradient(180deg,#0b1220,#1a2740); }
  .hero h1 { color:#ffffff; margin:0 }
  .cta { background-color:#3b82f6; color:#ffffff; padding:12px 24px; border-radius:8px }
  .row { display:flex; gap:16px }
</style></head><body>
  <section class="hero">
    <h1>Build faster</h1>
    <p style="color:#94a3b8">Ship <b>beautiful</b> designs</p>
    <div class="row">
      <button class="cta">Start</button>
      <input type="text" placeholder="Email"/>
    </div>
  </section>
</body></html>"#;

const CSS_FEATURE_MATRIX: &str = r##"<!doctype html>
<html>
<head>
  <title>CSS feature matrix</title>
  <style>
    :root {
      --ink: #172033;
      --card: #d6e4ff;
    }
    * { box-sizing: border-box; opacity: .98; }
    body { margin: 0; color: var(--ink); }
    .wrap { width: min(1180px, calc(100% - 40px)); margin-inline: auto; }
    .nav { display: flex; gap: clamp(8px, 2vw, 24px); }
    .nav a { display: block; }
    .hero {
      position: relative;
      display: flex;
      align-items: center;
      min-height: calc(460px + 40px);
      overflow: hidden;
    }
    .hero::after {
      content: "";
      position: absolute;
      inset: 0;
      background: linear-gradient(90deg, #00000000, #00000099);
    }
    .hero img {
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    .copy { position: relative; z-index: 1; }
    .title { color: var(--ink); font-size: clamp(40px, 5vw, 72px); }
    .cards {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 20px;
    }
    .cards article { background-color: var(--card); }
    .cards article:first-child { background-color: #ffcfcc; }
    .cards article:nth-child(2) { background-color: #ffe6ae; }
    @media (max-width: 720px) {
      .nav { display: none; }
      .cards { grid-template-columns: repeat(2, 1fr); }
    }
  </style>
</head>
<body>
  <main class="wrap">
    <nav class="nav">
      <a href="#alpha">Alpha</a>
      <a href="#beta">Beta</a>
      <a href="#gamma">Gamma</a>
    </nav>
    <section class="hero">
      <img src="hero.jpg" alt="" />
      <div class="copy"><h1 class="title">Responsive parser</h1></div>
    </section>
    <section class="cards">
      <article>One</article>
      <article>Two</article>
      <article>Three</article>
      <article>Four</article>
    </section>
  </main>
</body>
</html>"##;

const NOVA_STORE: &str = include_str!("../tests/fixtures/nova_store.html");

fn frame_children(frame: &FrameNode) -> &[PenNode] {
    frame.children.as_deref().unwrap_or_default()
}

fn append_text(node: &PenNode, output: &mut String) {
    match node {
        PenNode::Text(text) => match &text.content {
            TextContent::Plain(value) => output.push_str(value),
            TextContent::Styled(segments) => {
                for segment in segments {
                    output.push_str(&segment.text);
                }
            }
        },
        PenNode::Frame(frame) => {
            for child in frame_children(frame) {
                append_text(child, output);
            }
        }
        _ => {}
    }
}

fn node_text(node: &PenNode) -> String {
    let mut output = String::new();
    append_text(node, &mut output);
    output
}

fn recursive_node_count(node: &PenNode) -> usize {
    let children = match node {
        PenNode::Frame(node) => node.children.as_deref(),
        PenNode::Group(node) => node.children.as_deref(),
        PenNode::Rectangle(node) => node.children.as_deref(),
        _ => None,
    };
    1 + children
        .unwrap_or_default()
        .iter()
        .map(recursive_node_count)
        .sum::<usize>()
}

fn direct_frame_containing<'a>(frame: &'a FrameNode, text: &str) -> &'a FrameNode {
    frame_children(frame)
        .iter()
        .find_map(|node| match node {
            PenNode::Frame(child) if node_text(node).contains(text) => Some(child),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing direct frame containing `{text}`"))
}

fn descendant_frame_with_exact_text<'a>(
    frame: &'a FrameNode,
    expected: &str,
) -> Option<&'a FrameNode> {
    for child in frame_children(frame) {
        let PenNode::Frame(child_frame) = child else {
            continue;
        };
        if node_text(child) == expected {
            return Some(child_frame);
        }
        if let Some(found) = descendant_frame_with_exact_text(child_frame, expected) {
            return Some(found);
        }
    }
    None
}

fn descendant_frame_named<'a>(frame: &'a FrameNode, expected: &str) -> Option<&'a FrameNode> {
    for child in frame_children(frame) {
        let PenNode::Frame(child_frame) = child else {
            continue;
        };
        if child_frame.base.name.as_deref() == Some(expected) {
            return Some(child_frame);
        }
        if let Some(found) = descendant_frame_named(child_frame, expected) {
            return Some(found);
        }
    }
    None
}

fn descendant_frame_with_direct_child_named<'a>(
    frame: &'a FrameNode,
    expected: &str,
) -> Option<&'a FrameNode> {
    for child in frame_children(frame) {
        let PenNode::Frame(child_frame) = child else {
            continue;
        };
        if frame_children(child_frame).iter().any(|node| {
            matches!(node, PenNode::Frame(direct)
                if direct.base.name.as_deref() == Some(expected))
        }) {
            return Some(child_frame);
        }
        if let Some(found) = descendant_frame_with_direct_child_named(child_frame, expected) {
            return Some(found);
        }
    }
    None
}

fn descendant_text_with_exact_content<'a>(
    frame: &'a FrameNode,
    expected: &str,
) -> Option<&'a TextNode> {
    for child in frame_children(frame) {
        match child {
            PenNode::Text(text) if node_text(child) == expected => return Some(text),
            PenNode::Frame(child_frame) => {
                if let Some(found) = descendant_text_with_exact_content(child_frame, expected) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn solid_fill_color(frame: &FrameNode) -> Option<&str> {
    frame
        .container
        .fill
        .as_ref()?
        .iter()
        .find_map(|fill| match fill {
            PenFill::Solid(solid) => Some(solid.color.as_str()),
            _ => None,
        })
}

fn assert_grid_rows(frame: &FrameNode, expected_columns: &[usize]) {
    assert_eq!(frame.container.layout, Some(LayoutMode::Vertical));
    let rows: Vec<_> = frame_children(frame)
        .iter()
        .filter_map(|node| match node {
            PenNode::Frame(row) if row.container.layout == Some(LayoutMode::Horizontal) => {
                Some(row)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rows.len(),
        expected_columns.len(),
        "unexpected grid row count"
    );
    for (row, expected) in rows.iter().zip(expected_columns) {
        assert_eq!(
            frame_children(row).len(),
            *expected,
            "unexpected grid column count"
        );
    }
}

fn feature_matrix(viewport_width: f64) -> crate::HtmlImportResult {
    import_html(
        CSS_FEATURE_MATRIX,
        &HtmlImportOptions {
            viewport_width,
            ..Default::default()
        },
    )
}

fn feature_root(result: &crate::HtmlImportResult) -> &FrameNode {
    let [PenNode::Frame(root)] = result.nodes.as_slice() else {
        panic!("import should produce exactly one root frame")
    };
    root
}

fn nova_store(viewport_width: f64) -> crate::HtmlImportResult {
    import_html(
        NOVA_STORE,
        &HtmlImportOptions {
            viewport_width,
            ..Default::default()
        },
    )
}

#[test]
fn landing_page_imports_as_editable_tree() {
    let result = import_html(LANDING, &HtmlImportOptions::default());
    assert_eq!(result.nodes.len(), 1);
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!("root must be frame")
    };
    assert_eq!(root.base.name.as_deref(), Some("Acme"));
    assert_eq!(root.container.width, Some(SizingBehavior::Number(1440.0)));
    assert_eq!(
        root.container.height,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );
    let PenNode::Frame(hero) = &root.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(hero.container.layout, Some(LayoutMode::Vertical));
    let children = hero.children.as_ref().unwrap();
    assert!(children.len() >= 3);
    let PenNode::Frame(row) = children.last().unwrap() else {
        panic!("row")
    };
    let row_children = row.children.as_ref().unwrap();
    assert!(matches!(&row_children[0], PenNode::Frame(button)
        if button.base.role.as_deref() == Some("button")));
    assert!(matches!(&row_children[1], PenNode::TextInput(_)));
}

#[test]
fn auto_flex_item_uses_intrinsic_width_without_fill_container_cycles() {
    let result = import_html(
        r#"<style>
          .card { display:flex; width:260px; gap:12px; }
          .icon { width:40px; height:40px; }
          .copy b { display:block; }
        </style>
        <div class="card"><i class="icon">*</i><div class="copy">
          <b>Short</b><span>A much longer description that may wrap</span>
        </div></div>"#,
        &HtmlImportOptions::default(),
    );
    let root = feature_root(&result);
    let card = direct_frame_containing(root, "Short");
    let copy = frame_children(card)
        .iter()
        .find_map(|node| match node {
            PenNode::Frame(frame) if node_text(node).contains("Short") => Some(frame),
            _ => None,
        })
        .expect("copy flex item");
    assert_eq!(
        copy.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );
    let title = descendant_text_with_exact_content(copy, "Short").expect("title");
    let description =
        descendant_text_with_exact_content(copy, "A much longer description that may wrap")
            .expect("description");
    for text in [title, description] {
        assert_eq!(text.width, None);
        assert_eq!(text.text_growth, Some(TextGrowth::Auto));
    }
}

#[test]
fn node_limit_truncates_with_warning() {
    let mut html = String::from("<div>");
    // ~2 output nodes per <p>; 45,000 keeps a >2x margin over the 40k cap.
    for _ in 0..45_000 {
        html.push_str("<p>x</p>");
    }
    html.push_str("</div>");
    let result = import_html(&html, &HtmlImportOptions::default());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("node limit")));
    assert!(
        result.nodes.iter().map(recursive_node_count).sum::<usize>() <= crate::MAX_OUTPUT_NODES
    );
}

#[test]
fn generated_content_also_obeys_the_output_node_limit() {
    let mut html = String::from(
        "<style>p::before{content:'before';display:block}\
         p::after{content:'after';display:block}</style><div>",
    );
    // Each <p> expands to ~6 output nodes with the generated content, so
    // 7,500 paragraphs lands ~45k — just over MAX_OUTPUT_NODES (40k).
    for _ in 0..7_500 {
        html.push_str("<p>x</p>");
    }
    html.push_str("</div>");
    let result = import_html(&html, &HtmlImportOptions::default());
    assert!(
        result.nodes.iter().map(recursive_node_count).sum::<usize>() <= crate::MAX_OUTPUT_NODES
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("node limit")));
}

#[test]
fn document_name_option_overrides_title() {
    let options = HtmlImportOptions {
        document_name: Some("Custom".into()),
        ..Default::default()
    };
    let result = import_html(
        "<html><head><title>T</title></head><body><p>x</p></body></html>",
        &options,
    );
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    assert_eq!(root.base.name.as_deref(), Some("Custom"));
}

#[test]
fn css_feature_matrix_maps_variables_functions_and_flex_items() {
    let result = feature_matrix(1440.0);
    let root = feature_root(&result);
    assert_eq!(root.base.name.as_deref(), Some("CSS feature matrix"));
    assert_eq!(root.container.align_items, Some(AlignItems::Center));

    let PenNode::Frame(main) = &frame_children(root)[0] else {
        panic!("body should contain the main frame")
    };
    assert_eq!(main.base.name.as_deref(), Some("main"));
    assert_eq!(main.container.width, Some(SizingBehavior::Number(1180.0)));

    let nav = direct_frame_containing(main, "Alpha");
    assert_eq!(nav.container.layout, Some(LayoutMode::Horizontal));
    assert!(matches!(
        nav.container.gap,
        Some(NumberOrExpression::Number(value)) if value == 24.0
    ));
    assert!(matches!(
        nav.base.opacity,
        Some(NumberOrExpression::Number(value)) if value == 0.98
    ));
    let links = frame_children(nav);
    assert_eq!(links.len(), 3, "flex links must remain independent items");
    assert!(links.iter().all(|node| matches!(node, PenNode::Frame(_))));
    assert_eq!(node_text(&links[0]), "Alpha");
    assert_eq!(node_text(&links[1]), "Beta");
    assert_eq!(node_text(&links[2]), "Gamma");

    let title = descendant_text_with_exact_content(main, "Responsive parser")
        .expect("title text should be imported");
    assert_eq!(title.font_size, Some(72.0));
    assert!(matches!(
        title.fill.as_deref(),
        Some([PenFill::Solid(solid)]) if solid.color == "#172033"
    ));
}

#[test]
fn css_feature_matrix_maps_grid_pseudo_element_and_positional_selectors() {
    let result = feature_matrix(1440.0);
    let root = feature_root(&result);
    let PenNode::Frame(main) = &frame_children(root)[0] else {
        panic!()
    };

    let hero = direct_frame_containing(main, "Responsive parser");
    assert_eq!(hero.container.limits.min_height, Some(500.0));
    let image = frame_children(hero)
        .iter()
        .find_map(|node| match node {
            PenNode::Image(image) => Some(image),
            _ => None,
        })
        .expect("hero image should be imported");
    assert_eq!(image.base.x, Some(0.0));
    assert_eq!(image.base.y, Some(0.0));
    assert_eq!(image.object_fit, Some(ImageFitMode::Crop));
    assert_eq!(image.width, Some(SizingBehavior::Number(1180.0)));
    assert_eq!(image.height, Some(SizingBehavior::Number(500.0)));

    let overlay = frame_children(hero)
        .iter()
        .find_map(|node| match node {
            PenNode::Frame(frame)
                if frame.container.fill.as_ref().is_some_and(|fills| {
                    fills
                        .iter()
                        .any(|fill| matches!(fill, PenFill::LinearGradient(_)))
                }) =>
            {
                Some(frame)
            }
            _ => None,
        })
        .expect("hero::after should become a gradient overlay frame");
    assert_eq!(overlay.base.x, Some(0.0));
    assert_eq!(overlay.base.y, Some(0.0));

    let cards = direct_frame_containing(main, "One");
    assert!(matches!(
        cards.container.gap,
        Some(NumberOrExpression::Number(value)) if value == 20.0
    ));
    assert_grid_rows(cards, &[4]);
    let first = descendant_frame_with_exact_text(cards, "One").expect("first card");
    let second = descendant_frame_with_exact_text(cards, "Two").expect("second card");
    let third = descendant_frame_with_exact_text(cards, "Three").expect("third card");
    assert_eq!(solid_fill_color(first), Some("#ffcfcc"));
    assert_eq!(solid_fill_color(second), Some("#ffe6ae"));
    assert_eq!(solid_fill_color(third), Some("#d6e4ff"));
}

#[test]
fn css_feature_matrix_applies_max_width_media_query_at_720px() {
    let result = feature_matrix(720.0);
    let root = feature_root(&result);
    let PenNode::Frame(main) = &frame_children(root)[0] else {
        panic!()
    };
    assert_eq!(main.container.width, Some(SizingBehavior::Number(680.0)));
    assert!(
        !node_text(&result.nodes[0]).contains("Alpha"),
        "the media query should hide desktop navigation"
    );

    let cards = direct_frame_containing(main, "One");
    assert_grid_rows(cards, &[2, 2]);
    let title = descendant_text_with_exact_content(main, "Responsive parser")
        .expect("title text should be imported");
    assert_eq!(title.font_size, Some(40.0));
}

#[test]
fn nova_store_desktop_preserves_cascade_geometry_layers_and_grid() {
    let result = nova_store(1440.0);
    let root = feature_root(&result);
    assert_eq!(root.base.name.as_deref(), Some("NOVA｜发现新好物"));
    assert_eq!(solid_fill_color(root), Some("#ffffff"));
    assert_eq!(frame_children(root).len(), 2);

    let PenNode::Frame(header) = &frame_children(root)[0] else {
        panic!("header should be a frame")
    };
    let PenNode::Frame(main) = &frame_children(root)[1] else {
        panic!("main should be a frame")
    };
    assert_eq!(header.container.width, Some(SizingBehavior::Number(1180.0)));
    assert_eq!(header.container.height, Some(SizingBehavior::Number(76.0)));
    assert_eq!(header.container.layout, Some(LayoutMode::Horizontal));
    assert_eq!(main.container.width, Some(SizingBehavior::Number(1180.0)));

    let nav = direct_frame_containing(header, "首页");
    assert_eq!(nav.container.layout, Some(LayoutMode::Horizontal));
    assert!(matches!(
        nav.container.gap,
        Some(NumberOrExpression::Number(value)) if value == 30.0
    ));
    assert_eq!(frame_children(nav).len(), 4);

    let actions = direct_frame_containing(header, "⌕");
    assert_eq!(
        frame_children(actions)
            .iter()
            .map(node_text)
            .collect::<Vec<_>>(),
        ["⌕", "2♧"],
        "position:relative with z-index:auto must not reorder flex items"
    );
    let bag = descendant_frame_with_exact_text(header, "2♧").expect("shopping bag frame");
    assert_eq!(solid_fill_color(bag), Some("#17223b"));
    assert!(
        bag.container.stroke.is_none(),
        "border:0 must reset the shared border"
    );
    let badge = descendant_frame_with_exact_text(bag, "2").expect("bag badge");
    assert_eq!((badge.base.x, badge.base.y), (Some(25.0), Some(-5.0)));
    assert_eq!(solid_fill_color(badge), Some("#ff6565"));

    let hero = direct_frame_containing(main, "把未来，握在手里。");
    assert_eq!(hero.container.height, Some(SizingBehavior::Number(500.0)));
    assert_eq!(hero.container.limits.min_height, Some(500.0));
    assert_eq!(solid_fill_color(hero), Some("#f1f5ff"));
    let hero_children = frame_children(hero);
    assert!(matches!(&hero_children[0], PenNode::Frame(frame)
        if frame.container.width == Some(SizingBehavior::Number(613.6))
            && node_text(&hero_children[0]).contains("把未来，握在手里。")));
    assert!(matches!(&hero_children[1], PenNode::Frame(frame)
        if frame.base.name.as_deref() == Some("::after")
            && frame.container.width == Some(SizingBehavior::Number(1180.0))));
    assert!(matches!(&hero_children[2], PenNode::Image(image)
        if image.base.x == Some(0.0)
            && image.base.y == Some(0.0)
            && image.object_fit == Some(ImageFitMode::Crop)
            && image.base.blend_mode == Some(BlendMode::Multiply)));
    let overlay = descendant_frame_named(hero, "::after").expect("gradient overlay");
    assert!(matches!(
        overlay.container.fill.as_deref(),
        Some([PenFill::LinearGradient(gradient)]) if gradient.angle == Some(90.0)
    ));

    let heading = descendant_frame_named(hero, "h1").expect("h1 frame");
    assert_eq!(heading.container.width, Some(SizingBehavior::Number(470.0)));
    assert_eq!(heading.container.limits.max_width, Some(470.0));
    let heading_text =
        descendant_text_with_exact_content(hero, "把未来，握在手里。").expect("heading text");
    assert_eq!(heading_text.font_size, Some(72.0));
    assert_eq!(heading_text.width, Some(SizingBehavior::Number(470.0)));
    let cta = descendant_frame_named(hero, "button").expect("CTA frame");
    assert_eq!(node_text(&PenNode::Frame(cta.clone())), "探索新品→");
    assert_eq!(solid_fill_color(cta), Some("#2859f6"));
    assert!(cta.container.stroke.is_none());
    let cta_label = descendant_text_with_exact_content(cta, "探索新品").expect("CTA label");
    assert_eq!(cta_label.font_family.as_deref(), Some("Arial"));
    assert!((cta_label.font_size.unwrap_or_default() - 13.3333).abs() < 1e-6);
    assert_eq!(cta_label.font_weight, Some(FontWeight::Number(400)));
    assert_eq!(cta_label.line_height, None);

    let search_button = descendant_frame_with_exact_text(root, "⌕").expect("search button");
    let search_label =
        descendant_text_with_exact_content(search_button, "⌕").expect("search icon label");
    assert_eq!(search_label.font_family.as_deref(), Some("Arial"));
    assert_eq!(search_label.font_size, Some(17.0));
    assert_eq!(search_label.font_weight, Some(FontWeight::Number(400)));
    assert_eq!(search_label.line_height, None);

    let categories_section = direct_frame_containing(main, "逛逛分类");
    let categories = descendant_frame_with_direct_child_named(categories_section, "Grid row")
        .expect("categories grid");
    assert_grid_rows(categories, &[4]);
    let products_section = direct_frame_containing(main, "人气上新");
    let products = descendant_frame_with_direct_child_named(products_section, "Grid row")
        .expect("products grid");
    assert_grid_rows(products, &[3]);
    assert_eq!(
        solid_fill_color(descendant_frame_with_exact_text(products, "📱").unwrap()),
        Some("#edf0ff")
    );
    assert_eq!(
        solid_fill_color(descendant_frame_with_exact_text(products, "🎧").unwrap()),
        Some("#fff4ed")
    );
    assert_eq!(
        solid_fill_color(descendant_frame_with_exact_text(products, "⌚").unwrap()),
        Some("#eef9f6")
    );
    let add_button = descendant_frame_with_exact_text(products, "+").expect("product add button");
    assert_eq!(
        add_button.container.justify_content,
        Some(JustifyContent::Center)
    );
    assert_eq!(add_button.container.align_items, Some(AlignItems::Center));
    let add_label =
        descendant_text_with_exact_content(add_button, "+").expect("product add button label");
    assert_eq!(add_label.text_align, Some(TextAlign::Center));
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn nova_store_mobile_applies_media_query_and_single_column_products() {
    let result = nova_store(720.0);
    let root = feature_root(&result);
    let PenNode::Frame(header) = &frame_children(root)[0] else {
        panic!("header should be a frame")
    };
    let PenNode::Frame(main) = &frame_children(root)[1] else {
        panic!("main should be a frame")
    };
    assert_eq!(header.container.width, Some(SizingBehavior::Number(600.0)));
    assert_eq!(header.container.height, Some(SizingBehavior::Number(64.0)));
    assert_eq!(main.container.width, Some(SizingBehavior::Number(600.0)));
    assert!(!node_text(&result.nodes[0]).contains("首页"));

    let hero = direct_frame_containing(main, "把未来，握在手里。");
    assert_eq!(hero.container.height, Some(SizingBehavior::Number(580.0)));
    assert_eq!(hero.container.limits.min_height, Some(580.0));
    let heading = descendant_text_with_exact_content(hero, "把未来，握在手里。")
        .expect("mobile heading text");
    assert_eq!(heading.font_size, Some(42.0));
    assert_eq!(heading.width, Some(SizingBehavior::Number(470.0)));
    let overlay = descendant_frame_named(hero, "::after").expect("mobile gradient overlay");
    assert!(matches!(
        overlay.container.fill.as_deref(),
        Some([PenFill::LinearGradient(gradient)]) if gradient.angle == Some(180.0)
    ));

    let categories_section = direct_frame_containing(main, "逛逛分类");
    let categories = descendant_frame_with_direct_child_named(categories_section, "Grid row")
        .expect("mobile categories grid");
    assert_grid_rows(categories, &[2, 2]);

    let products_section = direct_frame_containing(main, "人气上新");
    let products = descendant_frame_with_direct_child_named(products_section, "article")
        .expect("mobile products grid");
    let articles = frame_children(products)
        .iter()
        .filter(|node| {
            matches!(node, PenNode::Frame(frame)
            if frame.base.name.as_deref() == Some("article"))
        })
        .count();
    assert_eq!(articles, 3);
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("object-position")));
    assert!(result
        .warnings
        .iter()
        .all(|warning| !warning.contains("mix-blend-mode")));
}
