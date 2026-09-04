//! Extract the visually dominant headline from one generated section.

use jian_ops_schema::node::{PenNode, TextContent};
use op_editor_core::PenNodeExt;

const MIN_HEADLINE_CHARS: usize = 2;
const MAX_HEADLINE_CHARS: usize = 80;

/// Return the text of the largest eligible text node in `root`.
///
/// Ties are resolved by keeping the first node encountered during the
/// document-order walk. Navigation and footer subtrees are ignored so their
/// labels cannot become page-section headlines.
pub(crate) fn section_headline(root: &PenNode) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    collect_headline(root, false, &mut best);
    best.map(|(_, text)| text)
}

fn collect_headline(node: &PenNode, in_chrome: bool, best: &mut Option<(f64, String)>) {
    let in_chrome = in_chrome || is_nav_or_footer(node);
    if !in_chrome {
        if let PenNode::Text(text_node) = node {
            let text = text_content(text_node);
            let char_count = text.chars().count();
            if (MIN_HEADLINE_CHARS..=MAX_HEADLINE_CHARS).contains(&char_count) {
                if let Some(font_size) = effective_font_size(text_node) {
                    if font_size.is_finite()
                        && best
                            .as_ref()
                            .is_none_or(|(best_size, _)| font_size > *best_size)
                    {
                        *best = Some((font_size, text));
                    }
                }
            }
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_headline(child, in_chrome, best);
        }
    }
}

fn effective_font_size(text_node: &jian_ops_schema::node::TextNode) -> Option<f64> {
    text_node.font_size.or_else(|| match &text_node.content {
        TextContent::Plain(_) => None,
        TextContent::Styled(segments) => segments
            .iter()
            .filter_map(|segment| segment.font_size.map(f64::from))
            .max_by(f64::total_cmp),
    })
}

fn text_content(text_node: &jian_ops_schema::node::TextNode) -> String {
    match &text_node.content {
        TextContent::Plain(text) => text.trim().to_owned(),
        TextContent::Styled(segments) => segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>()
            .trim()
            .to_owned(),
    }
}

fn is_nav_or_footer(node: &PenNode) -> bool {
    let role = node
        .base()
        .role
        .as_deref()
        .or_else(|| crate::role_infer::infer_role_from_name(node));
    let Some(role) = role else {
        return false;
    };
    let role = role.trim().to_ascii_lowercase().replace('_', "-");
    role == "footer" || role == "header" || role.contains("nav") || role.contains("tab-bar")
}

#[cfg(test)]
#[path = "section_headline_tests.rs"]
mod tests;
