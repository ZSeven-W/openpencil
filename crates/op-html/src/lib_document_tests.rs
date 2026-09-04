use super::*;

fn authored(node: &PenNode) -> &PenNode {
    crate::mapper::unwrap_margin_node(node)
}

#[test]
fn e2e_document_wrapper_produces_pendocument() {
    let result = import_html_document(
        "<html><head><title>T</title></head><body><p>x</p></body></html>",
        &HtmlImportOptions::default(),
        None,
        None,
    );
    assert_eq!(result.document.children.len(), 1);
    assert_eq!(result.document.name.as_deref(), Some("T"));
}

#[test]
fn body_layout_base_styles_and_generated_content_reach_the_root() {
    let result = import_html(
        "<style>body::before{content:'before'}</style>\
         <body style='display:block flex;flex-direction:row;opacity:.4;visibility:hidden;\
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
        Some(jian_ops_schema::node::base::NumberOrExpression::Number(value)) if value == 0.4
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
    let PenNode::Frame(paragraph) = authored(&root.children.as_ref().unwrap()[0]) else {
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
    let PenNode::Frame(paragraph) = authored(&root.children.as_ref().unwrap()[0]) else {
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
