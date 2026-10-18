//! Presence evidence for an explicitly owned mobile business bottom bar.

use super::{local_item_text, CompletenessFailure};
use crate::image_fallback_policy::{resolved_rects, ResolvedRect};
use crate::plan::Subtask;
use crate::types::{DocSink, SubtaskOutcome};
use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::style::PenFill;
use op_design_lint::node_util::{has_stroke, is_node_visible, node_fills, opacity};
use op_editor_core::{EditorState, PenNodeExt};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

pub(super) fn inspect(
    sink: &dyn DocSink,
    task: &Subtask,
    outcome: &SubtaskOutcome,
) -> Option<CompletenessFailure> {
    // The shared section gate has no trusted sibling plan. Enforce only this
    // task's own unmasked promise, never an alleged owner in generated code.
    if !(280.0..=480.0).contains(&task.region.width) || !owns_bar(task) {
        return None;
    }
    let state = sink.state();
    let mut nodes = op_editor_core::ref_resolve::resolve_refs_for_canvas_roots(
        state.active_children(),
        &state.doc,
    );
    op_editor_core::variables_resolve::resolve_roots_for_canvas(
        &mut nodes,
        &state.doc,
        &state.ui.variables.active_theme,
    );
    let mut owned = Vec::new();
    collect_owned(&nodes, &outcome.inserted_root_ids, &mut owned);
    // Measure intrinsic/fill-container output at its actual planned width. The
    // scratch tree is read-only and preserves the canvas's theme/layout context.
    let mut scratch = EditorState::new();
    scratch.doc.variables = state.doc.variables.clone();
    scratch.doc.themes = state.doc.themes.clone();
    scratch.ui.variables.active_theme = state.ui.variables.active_theme.clone();
    scratch.editor_ui.preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
    let parent: PenNode = serde_json::from_value(serde_json::json!({
        "type":"frame", "id":"__bottom_action_presence_measure",
        "width":task.region.width, "height":"fit_content", "layout":"vertical",
        "children":owned,
    }))
    .expect("valid bottom action measurement wrapper");
    scratch.doc.children = vec![parent];
    let rects = resolved_rects(&scratch);
    if owned
        .iter()
        .any(|node| contains_bar(node, task.region.width, &rects, true, true))
    {
        return None;
    }
    Some(CompletenessFailure {
        expected: 1,
        delivered: 0,
        feedback: "The plan asks for a bottom business bar in this section; none was delivered. Regenerate the section with its explicitly requested cart, add-to-cart, or checkout bar as a visible grouped rail. Preserve the other content. An empty spacer, individual product add buttons, layer names, or a comment claiming a global bar do not supply this bar. No additional behavior is required beyond the plan.".into(),
    })
}

fn owns_bar(task: &Subtask) -> bool {
    static BAR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)购物车(?:条|栏)|(?:底部|底栏|底边)?(?:加购|结算)(?:条|栏)|底部(?:加入购物车|立即购买|下单)(?:操作)?(?:条|栏)|\b(?:cart|checkout|add[ -]to[ -]cart)\s+(?:bar|footer|rail)\b")
            .expect("valid explicit business bar contract")
    });
    let local = local_item_text(task);
    BAR.find_iter(&local).any(|found| {
        // Recheck a comma-introduced owner as part of the same clause. The
        // ordinary item mask already handles quotes, metadata and exclusions.
        let start = local[..found.start()]
            .rfind([';', '；', '\n', '。'])
            .map_or(0, |i| i + local[i..].chars().next().unwrap().len_utf8());
        let end = local[found.end()..]
            .find([';', '；', '\n', '。'])
            .map_or(local.len(), |i| found.end() + i);
        let raw = format!("{}\n{}", task.label, task.elements.as_deref().unwrap_or(""));
        let mut bytes = raw.as_bytes()[start..end].to_vec();
        for (i, ch) in raw[start..end]
            .char_indices()
            .filter(|(_, ch)| matches!(ch, ',' | '，'))
        {
            bytes[i..i + ch.len_utf8()].fill(b' ');
        }
        let clause = String::from_utf8(bytes).expect("comma masking preserves UTF-8");
        let clause = super::mask_metadata_copy(&super::mask_quoted_copy(&clause));
        let scoped = super::scope::mask_nonlocal_references(&clause, task);
        !scoped[found.start() - start..found.end() - start]
            .trim()
            .is_empty()
            && !["不要", "无需", "不需要", "without ", "no "]
                .iter()
                .any(|prefix| {
                    scoped[..found.start() - start]
                        .trim_end()
                        .to_lowercase()
                        .ends_with(prefix.trim_end())
                })
    })
}

fn bindings(node: &PenNode) -> Option<&jian_ops_schema::events::Bindings> {
    match node {
        PenNode::Frame(n) => n.bindings.as_ref(),
        PenNode::Rectangle(n) => n.bindings.as_ref(),
        PenNode::Group(n) => n.bindings.as_ref(),
        PenNode::Text(n) => n.bindings.as_ref(),
        _ => None,
    }
}

fn potential(node: &PenNode) -> bool {
    (is_node_visible(node) && opacity(node) > 0.0)
        || bindings(node).is_some_and(|b| b.contains_key("visible") || b.contains_key("opacity"))
}

fn collect_owned(nodes: &[PenNode], roots: &[String], out: &mut Vec<PenNode>) {
    for node in nodes {
        if !potential(node) || matches!(node, PenNode::Ref(_)) {
            continue;
        }
        if roots.iter().any(|id| id == node.id_str()) {
            out.push(node.clone());
        } else if let Some(children) = node.children() {
            collect_owned(children, roots, out);
        }
    }
}

fn contains_bar(
    node: &PenNode,
    width: f64,
    rects: &HashMap<String, ResolvedRect>,
    terminal: bool,
    owned_root: bool,
) -> bool {
    if !potential(node) || matches!(node, PenNode::Ref(_)) {
        return false;
    }
    if bar_evidence(node, width, rects, terminal, owned_root) {
        return true;
    }
    // An ordinary media card's purchase controls are not a separate footer.
    // Large page/section containers remain traversable, including hero pages.
    if has_media(node) && rects.get(node.id_str()).is_some_and(|r| r.height <= 360.0) {
        return false;
    }
    let children: Vec<_> = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| meaningful(child))
        .collect();
    children.iter().enumerate().any(|(index, child)| {
        contains_bar(
            child,
            width,
            rects,
            terminal && index + 1 == children.len(),
            false,
        )
    })
}

fn has_media(node: &PenNode) -> bool {
    potential(node)
        && (matches!(node, PenNode::Image(_))
            || node_fills(node)
                .is_some_and(|fills| fills.iter().any(|fill| matches!(fill, PenFill::Image(_))))
            || node.children().into_iter().flatten().any(has_media))
}

fn bar_evidence(
    node: &PenNode,
    width: f64,
    rects: &HashMap<String, ResolvedRect>,
    terminal: bool,
    owned_root: bool,
) -> bool {
    if !matches!(
        node,
        PenNode::Frame(_) | PenNode::Rectangle(_) | PenNode::Group(_)
    ) || has_media(node)
    {
        return false;
    }
    let compact = rects.get(node.id_str()).is_some_and(|rect| {
        rect.width >= width * 0.6
            && rect.width <= width + 2.0
            && (24.0..=180.0).contains(&rect.height)
            && rect.width >= rect.height * 1.5
    });
    let mut controls = Vec::new();
    collect_controls(node, &mut controls, !(owned_root && compact));
    if controls.is_empty() {
        return false;
    }
    let mut support = false;
    let mut ordinary_copy = false;
    inspect_remainder(node, &controls, &mut support, &mut ordinary_copy);
    // Strong business grouping is independent of a particular rail size/style.
    // A single CTA additionally needs its own compact terminal wrapper (or be
    // the entire owned action section), rather than an ordinary page button.
    !ordinary_copy && (support || controls.len() >= 2 || (compact && terminal))
}

fn text(node: &PenNode) -> Option<(String, bool)> {
    let PenNode::Text(n) = node else { return None };
    let content = match &n.content {
        TextContent::Plain(value) => value.clone(),
        TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
    };
    Some((
        content.trim().to_owned(),
        n.bindings
            .as_ref()
            .is_some_and(|b| b.contains_key("content")),
    ))
}

fn action_label(label: &str) -> bool {
    static ACTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)^(?:去|立即|确认|提交)?(?:结算|下单|购买|支付|付款|订单)|^(?:加入购物车|加购)|^(?:checkout|check\s+out|(?:continue|proceed)\s+to\s+checkout|add\s+to\s+cart|buy\s+now|place\s+order|pay)\b")
            .expect("valid visible business action label")
    });
    label.chars().count() <= 48 && ACTION.is_match(label.trim())
}

fn number_badge(label: &str) -> bool {
    let label = label.trim().trim_matches(['(', ')', '（', '）']);
    !label.is_empty() && label.chars().all(|ch| ch.is_ascii_digit())
}

fn collect_texts(node: &PenNode, out: &mut Vec<(String, bool)>) {
    if !potential(node) || matches!(node, PenNode::Ref(_)) {
        return;
    }
    if let Some(label) = text(node) {
        if !label.0.is_empty() || label.1 {
            out.push(label);
        }
    }
    for child in node.children().into_iter().flatten() {
        collect_texts(child, out);
    }
}

fn surface(node: &PenNode) -> bool {
    node_fills(node).is_some_and(|fills| {
        fills.iter().any(|fill| match fill {
            PenFill::Solid(solid) => {
                solid.opacity.unwrap_or(1.0) > 0.0
                    && !matches!(
                        solid.color.to_lowercase().as_str(),
                        "transparent" | "#00000000" | "#ffffff00"
                    )
            }
            PenFill::Image(_) => false,
            _ => true,
        })
    })
}

fn collect_controls(node: &PenNode, out: &mut Vec<String>, root: bool) {
    if !potential(node) || matches!(node, PenNode::Ref(_)) {
        return;
    }
    if !root
        && matches!(node, PenNode::Frame(_) | PenNode::Rectangle(_))
        && (surface(node)
            || has_stroke(node)
            || matches!(node.base().role.as_deref(), Some("button" | "icon-button"))
            || crate::wire_screen_navigation::node_has_events(node))
    {
        let mut labels = Vec::new();
        collect_texts(node, &mut labels);
        let business = labels
            .iter()
            // A bound numeric badge remains supporting copy, not a second
            // potential action label. Unknown nonnumeric labels still defer.
            .filter(|(label, dynamic)| action_label(label) || (*dynamic && !number_badge(label)))
            .count();
        if business == 1
            && labels
                .iter()
                .all(|(label, dynamic)| *dynamic || action_label(label) || number_badge(label))
        {
            out.push(node.id_str().into());
            return;
        }
    }
    for child in node.children().into_iter().flatten() {
        collect_controls(child, out, false);
    }
}

fn inspect_remainder(node: &PenNode, controls: &[String], support: &mut bool, ordinary: &mut bool) {
    if !potential(node)
        || controls.iter().any(|id| id == node.id_str())
        || matches!(node, PenNode::Ref(_))
    {
        return;
    }
    if let PenNode::IconFont(icon) = node {
        *support |= matches!(
            icon.icon_font_name.as_str(),
            "shopping-cart" | "shopping-bag" | "shopping-basket" | "basket" | "headset" | "heart"
        );
    }
    if let Some((label, dynamic)) = text(node) {
        static SUMMARY: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?ix)[¥￥$]\s*\d|(?:合计|总计|实付|应付|配送费|购物车|已选|起送|客服|收藏|店铺)|\b(?:total|subtotal|delivery|cart)\b").expect("valid business summary")
        });
        if SUMMARY.is_match(&label) {
            *support = true;
        } else if !dynamic
            && !label.is_empty()
            && !label
                .chars()
                .all(|ch| ch.is_ascii_digit() || ".+- ".contains(ch))
        {
            *ordinary = true;
        }
    }
    for child in node.children().into_iter().flatten() {
        inspect_remainder(child, controls, support, ordinary);
    }
}

fn meaningful(node: &PenNode) -> bool {
    potential(node)
        && (matches!(node, PenNode::IconFont(_) | PenNode::Image(_))
            || text(node).is_some_and(|(label, dynamic)| !label.is_empty() || dynamic)
            || node.children().into_iter().flatten().any(meaningful))
}
