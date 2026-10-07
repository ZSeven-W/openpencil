//! Preserve decorative ink only when the actual foreground remains readable.
use super::*;

pub(super) fn covered_by_readable_front(
    node: &PenNode,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, ResolvedRect>,
) -> bool {
    let Some(parent) = ancestors.last() else {
        return false;
    };
    let Ok(value) = serde_json::to_value(parent) else {
        return false;
    };
    let Some(pair) = crate::text_effect_overlay::pair(&value) else {
        return false;
    };
    if pair.front_index != 0
        || pair.back.get("id").and_then(serde_json::Value::as_str) != Some(node.id_str())
    {
        return false;
    }
    let Some(front) = node_children_of(parent).first() else {
        return false;
    };
    if let PenNode::Text(text) = front {
        if let jian_ops_schema::node::TextContent::Styled(runs) = &text.content {
            if runs.iter().any(|run| run.fill.is_some()) {
                return false;
            }
        }
    }
    let ResolvedFill::Solid(rgba) = resolve_fill_kind(node_fills(front), variables, theme) else {
        return false;
    };
    if rgba[3] != 255 {
        return false;
    }
    let Some(background) = photo::effective_background(front, ancestors, variables, theme, rects)
    else {
        return false;
    };
    let colors = background_colors_at_text(&background, front.id_str(), rects);
    !colors.is_empty()
        && colors
            .iter()
            .all(|bg| op_design_lint::color::color_contrast(&rgb_hex(rgba), bg) >= TARGET_RATIO)
}
