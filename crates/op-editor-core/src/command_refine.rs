//! Deterministic cleanup used by `EditorCommand::RefineDesign`.

use crate::id_allocator::{IdAllocError, IdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;

const DEFAULT_ICON_FONT_SIZE: f64 = 24.0;

/// One fix applied during `RefineDesign`, mirroring TS design-refine's
/// `RefineFix` (`{ nodeId, nodeName?, fix }`). Surfaced by the `design_refine`
/// MCP tool as its `fixes[]` result so the Rust + TS tool results share shape.
#[derive(Debug, Clone, PartialEq)]
pub struct RefineFix {
    pub node_id: String,
    pub node_name: Option<String>,
    pub fix: String,
}

impl EditorState {
    /// Allocator-aware document refine used by command collaboration paths.
    pub(crate) fn cmd_refine_design_with_allocator(
        &mut self,
        root_id: &NodeId,
        _canvas_width: Option<i32>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<Option<bool>, IdAllocError> {
        if !root_id.is_real() {
            return Ok(None);
        }
        let Some(mut staged) = walkers::find_node(self.active_children(), root_id).cloned() else {
            return Ok(None);
        };
        let mut taken = ids_outside_subtree(&self.doc, &staged);
        let fixes = refine_subtree_with_allocator(&mut staged, allocator, &mut taken)?;
        let Some(live) = walkers::find_node_mut(self.active_children_mut(), root_id) else {
            return Ok(None);
        };
        *live = staged;
        Ok(Some(!fixes.is_empty()))
    }
}

/// Apply the deterministic refine transforms to `root` in place and return the
/// fix report (one entry per change). SHARED by `cmd_refine_design` (the host
/// apply path) and the `design_refine` MCP tool (which simulates on a clone to
/// report `fixes[]`), so the reported fixes always match what apply does.
pub fn refine_subtree(root: &mut PenNode) -> Vec<RefineFix> {
    refine_subtree_impl(root, true)
}

/// Apply the deterministic refine transforms to a subtree that will be
/// installed below an existing document node. This intentionally omits the
/// root-only height expansion: before installation, `root` is merely the
/// insertion payload, so treating its horizontal children as document-root
/// content can inflate an authored fixed height (for example, a 232 px
/// category rail with five 160 px cards) to 800 px.
pub fn refine_child_subtree(root: &mut PenNode) -> Vec<RefineFix> {
    refine_subtree_impl(root, false)
}

fn refine_subtree_impl(root: &mut PenNode, adjust_root_height: bool) -> Vec<RefineFix> {
    let mut fixes = Vec::new();
    // Pass order mirrors TS `design-refine.ts:60-71` for the
    // deterministic subset (role / icon-path resolution are the
    // live-hook passes; see the module docs for coverage).
    apply_no_emoji_icon_heuristic(root, &mut fixes);
    ensure_unique_node_ids(root, &mut fixes);
    normalize_icon_font_dimensions(root, &mut fixes);
    sanitize_auto_layout_child_positions(root, &mut fixes);
    sanitize_screen_frame_bounds(root, &mut fixes);
    if adjust_root_height {
        adjust_root_height_to_content(root, &mut fixes);
    }
    fixes
}

/// Refine an installed subtree while minting every repaired id from
/// `allocator`. The caller supplies ids owned by the rest of the document.
pub fn refine_subtree_with_allocator(
    root: &mut PenNode,
    allocator: &mut dyn IdAllocator,
    taken: &mut std::collections::HashSet<NodeId>,
) -> Result<Vec<RefineFix>, IdAllocError> {
    let mut fixes = Vec::new();
    apply_no_emoji_icon_heuristic(root, &mut fixes);
    ensure_unique_node_ids_with_allocator(root, allocator, taken, &mut fixes)?;
    normalize_icon_font_dimensions(root, &mut fixes);
    sanitize_auto_layout_child_positions(root, &mut fixes);
    sanitize_screen_frame_bounds(root, &mut fixes);
    adjust_root_height_to_content(root, &mut fixes);
    Ok(fixes)
}

fn ensure_unique_node_ids_with_allocator(
    node: &mut PenNode,
    allocator: &mut dyn IdAllocator,
    taken: &mut std::collections::HashSet<NodeId>,
    fixes: &mut Vec<RefineFix>,
) -> Result<(), IdAllocError> {
    let original = node.base().id.clone();
    let trimmed = original.trim();
    let candidate = NodeId::new(trimmed);
    let preserve = !trimmed.is_empty() && trimmed == original && taken.insert(candidate);
    if !preserve {
        let fresh = allocator.allocate(taken)?;
        node.base_mut().id = fresh.as_str().to_string();
        fixes.push(RefineFix {
            node_id: fresh.into(),
            node_name: node.base().name.clone(),
            fix: format!("Reminted duplicate/blank id (was {original:?})"),
        });
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            ensure_unique_node_ids_with_allocator(child, allocator, taken, fixes)?;
        }
    }
    Ok(())
}

fn ids_outside_subtree(
    doc: &jian_ops_schema::PenDocument,
    subtree: &PenNode,
) -> std::collections::HashSet<NodeId> {
    use std::collections::HashMap;

    fn count_node(node: &PenNode, counts: &mut HashMap<NodeId, usize>) {
        if let Some(id) = NodeId::new_opt(node.id_str()) {
            *counts.entry(id).or_default() += 1;
        }
        if let Some(children) = node.children() {
            for child in children {
                count_node(child, counts);
            }
        }
    }

    fn subtract_node(node: &PenNode, counts: &mut HashMap<NodeId, usize>) {
        if let Some(id) = NodeId::new_opt(node.id_str()) {
            if let Some(count) = counts.get_mut(&id) {
                *count = count.saturating_sub(1);
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                subtract_node(child, counts);
            }
        }
    }

    let mut counts = HashMap::new();
    for node in &doc.children {
        count_node(node, &mut counts);
    }
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(id) = NodeId::new_opt(&page.id) {
                *counts.entry(id).or_default() += 1;
            }
            for node in &page.children {
                count_node(node, &mut counts);
            }
        }
    }
    subtract_node(subtree, &mut counts);
    counts
        .into_iter()
        .filter_map(|(id, count)| (count > 0).then_some(id))
        .collect()
}

/// TS `ensureUniqueNodeIds` port (`design-node-sanitization.ts:142`):
/// empty/blank ids normalize to `{type}-node`, duplicates get a
/// `-{n}` suffix from a per-base counter starting at 2.
fn ensure_unique_node_ids(root: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    use std::collections::{HashMap, HashSet};
    fn type_label(node: &PenNode) -> &'static str {
        match node {
            PenNode::Frame(_) => "frame",
            PenNode::Group(_) => "group",
            PenNode::Rectangle(_) => "rectangle",
            PenNode::Ellipse(_) => "ellipse",
            PenNode::Line(_) => "line",
            PenNode::Polygon(_) => "polygon",
            PenNode::Path(_) => "path",
            PenNode::Text(_) => "text",
            PenNode::TextInput(_) => "text_input",
            PenNode::Image(_) => "image",
            PenNode::IconFont(_) => "icon_font",
            PenNode::TextArea(_) => "text_area",
            PenNode::Select(_) => "select",
            PenNode::Switch(_) => "switch",
            PenNode::Checkbox(_) => "checkbox",
            PenNode::Slider(_) => "slider",
            PenNode::RadioGroup(_) => "radio_group",
            PenNode::NumberInput(_) => "number_input",
            PenNode::Progress(_) => "progress",
            PenNode::Tabs(_) => "tabs",
            PenNode::Ref(_) => "ref",
        }
    }
    fn walk(
        node: &mut PenNode,
        used: &mut HashSet<String>,
        counters: &mut HashMap<String, usize>,
        fixes: &mut Vec<RefineFix>,
    ) {
        let original = node.base().id.clone();
        let trimmed = original.trim();
        let base = if trimmed.is_empty() {
            format!("{}-node", type_label(node))
        } else {
            trimmed.to_string()
        };
        let mut final_id = base.clone();
        if used.contains(&final_id) {
            let mut next = counters.get(&base).copied().unwrap_or(2);
            let mut candidate = format!("{base}-{next}");
            while used.contains(&candidate) {
                next += 1;
                candidate = format!("{base}-{next}");
            }
            counters.insert(base.clone(), next + 1);
            final_id = candidate;
        }
        if final_id != original {
            let name = node.base().name.clone();
            node.base_mut().id = final_id.clone();
            fixes.push(RefineFix {
                node_id: final_id.clone(),
                node_name: name,
                fix: format!("Reminted duplicate/blank id (was {original:?})"),
            });
        }
        used.insert(final_id);
        if let Some(children) = node.children_mut() {
            for child in children {
                walk(child, used, counters, fixes);
            }
        }
    }
    let mut used = HashSet::new();
    let mut counters = HashMap::new();
    walk(root, &mut used, &mut counters, fixes);
}

/// Give every icon-font node deterministic literal geometry before the live
/// canvas observes a post-processed batch. A single valid authored axis is the
/// best square-size signal for its missing/invalid twin; otherwise use the
/// editor's stable 24 px icon default. Two valid authored axes are preserved,
/// including intentionally non-square icons.
fn normalize_icon_font_dimensions(node: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    if let PenNode::IconFont(icon) = node {
        let width = valid_icon_font_dimension(&icon.width);
        let height = valid_icon_font_dimension(&icon.height);
        let target = match (width, height) {
            (Some(_), Some(_)) => None,
            (Some(width), None) => {
                icon.height = Some(jian_ops_schema::sizing::SizingBehavior::Number(width));
                Some((width, width))
            }
            (None, Some(height)) => {
                icon.width = Some(jian_ops_schema::sizing::SizingBehavior::Number(height));
                Some((height, height))
            }
            (None, None) => {
                icon.width = Some(jian_ops_schema::sizing::SizingBehavior::Number(
                    DEFAULT_ICON_FONT_SIZE,
                ));
                icon.height = Some(jian_ops_schema::sizing::SizingBehavior::Number(
                    DEFAULT_ICON_FONT_SIZE,
                ));
                Some((DEFAULT_ICON_FONT_SIZE, DEFAULT_ICON_FONT_SIZE))
            }
        };
        if let Some((width, height)) = target {
            fixes.push(RefineFix {
                node_id: icon.base.id.clone(),
                node_name: icon.base.name.clone(),
                fix: format!("Set missing/invalid icon font dimensions to {width}x{height}"),
            });
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            normalize_icon_font_dimensions(child, fixes);
        }
    }
}

fn valid_icon_font_dimension(
    size: &Option<jian_ops_schema::sizing::SizingBehavior>,
) -> Option<f64> {
    match size {
        Some(jian_ops_schema::sizing::SizingBehavior::Number(value))
            if value.is_finite() && *value > 0.0 =>
        {
            Some(*value)
        }
        _ => None,
    }
}

/// TS `sanitizeScreenFrameBounds` port: a free-layout "screen" frame
/// (mobile 320-480 × ≥640 or desktop ≥900 × ≥600) clamps its sized,
/// absolutely-positioned children into the frame with a 10% bleed
/// allowance per axis.
fn sanitize_screen_frame_bounds(node: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    if let PenNode::Frame(frame) = node {
        let dims = (
            frame_size_px(&frame.container.width),
            frame_size_px(&frame.container.height),
        );
        if let (Some(w), Some(h)) = dims {
            let is_mobile = (320.0..=480.0).contains(&w) && h >= 640.0;
            let is_desktop = w >= 900.0 && h >= 600.0;
            let free_layout = !node.is_auto_layout_container();
            if (is_mobile || is_desktop) && free_layout {
                clamp_children_into_screen(node, w, h, fixes);
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            sanitize_screen_frame_bounds(child, fixes);
        }
    }
}

fn frame_size_px(size: &Option<jian_ops_schema::sizing::SizingBehavior>) -> Option<f64> {
    match size {
        Some(jian_ops_schema::sizing::SizingBehavior::Number(n)) => Some(*n),
        _ => None,
    }
}

fn clamp_children_into_screen(
    frame: &mut PenNode,
    frame_w: f64,
    frame_h: f64,
    fixes: &mut Vec<RefineFix>,
) {
    let max_bleed_x = frame_w * 0.1;
    let max_bleed_y = frame_h * 0.1;
    let Some(children) = frame.children_mut() else {
        return;
    };
    for child in children {
        let (Some(cw), Some(ch)) = (child.width_px(), child.height_px()) else {
            continue;
        };
        let base = child.base_mut();
        let (Some(x), Some(y)) = (base.x, base.y) else {
            continue;
        };
        let clamped_x = x.clamp(-max_bleed_x, (frame_w - cw + max_bleed_x).max(-max_bleed_x));
        let clamped_y = y.clamp(-max_bleed_y, (frame_h - ch + max_bleed_y).max(-max_bleed_y));
        if (clamped_x - x).abs() > f64::EPSILON || (clamped_y - y).abs() > f64::EPSILON {
            base.x = Some(clamped_x);
            base.y = Some(clamped_y);
            let id = base.id.clone();
            let name = base.name.clone();
            fixes.push(RefineFix {
                node_id: id,
                node_name: name,
                fix: "Clamped child into screen-frame bounds".into(),
            });
        }
    }
}

/// TS `applyNoEmojiIconHeuristic` port — STRIP branch only: emoji
/// characters are removed from text content (and runs of whitespace
/// collapsed). The TS all-emoji branch swaps the text node for a
/// fallback path icon; that requires parent surgery mid-walk and is
/// deferred — an all-emoji text keeps its content here.
fn apply_no_emoji_icon_heuristic(node: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    use jian_ops_schema::node::text::TextContent;
    if let PenNode::Text(text) = node {
        if let TextContent::Plain(content) = &text.content {
            if content.chars().any(is_emoji_char) {
                let stripped: String = content.chars().filter(|c| !is_emoji_char(*c)).collect();
                let cleaned = collapse_spaces(stripped.trim());
                if !cleaned.is_empty() && &cleaned != content {
                    text.content = TextContent::Plain(cleaned);
                    fixes.push(RefineFix {
                        node_id: text.base.id.clone(),
                        node_name: text.base.name.clone(),
                        fix: "Stripped emoji from text content".into(),
                    });
                }
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            apply_no_emoji_icon_heuristic(child, fixes);
        }
    }
}

/// Conservative approximation of TS `\p{Extended_Pictographic}` +
/// `\p{Emoji_Presentation}` + U+FE0F: the major emoji blocks.
fn is_emoji_char(c: char) -> bool {
    matches!(c as u32,
        0x1F300..=0x1FAFF // symbols, emoticons, transport, supplemental
        | 0x2600..=0x27BF // misc symbols + dingbats
        | 0x1F1E6..=0x1F1FF // regional indicators (flags)
        | 0x2B00..=0x2BFF // arrows/stars subset used by emoji
        | 0xFE0F..=0xFE0F // variation selector-16
        | 0x1F000..=0x1F0FF // mahjong/dominoes/cards
        | 0x231A..=0x231B // watch, hourglass
        | 0x23E9..=0x23FA // av symbols
        | 0x25FB..=0x25FE // geometric squares
    )
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim().to_string()
}

fn sanitize_auto_layout_child_positions(node: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    let parent_auto_layout = node.is_auto_layout_container();
    if let Some(children) = node.children_mut() {
        for child in children {
            if parent_auto_layout {
                let id = child.base().id.clone();
                let name = child.base().name.clone();
                let base = child.base_mut();
                let removed_x = base.x.take().is_some();
                let removed_y = base.y.take().is_some();
                if removed_x || removed_y {
                    fixes.push(RefineFix {
                        node_id: id,
                        node_name: name,
                        fix: "Removed absolute position from auto-layout child".into(),
                    });
                }
            }
            sanitize_auto_layout_child_positions(child, fixes);
        }
    }
}

fn adjust_root_height_to_content(root: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    let total: f64 = match root.children() {
        Some(children) if !children.is_empty() => {
            children.iter().filter_map(PenNodeExt::height_px).sum()
        }
        _ => return,
    };
    if total <= 0.0 {
        return;
    }
    if root.height_px().is_some_and(|current| current >= total) {
        return;
    }
    let id = root.base().id.clone();
    let name = root.base().name.clone();
    root.set_height_px(total);
    fixes.push(RefineFix {
        node_id: id,
        node_name: name,
        fix: "Adjusted root height to fit content".into(),
    });
}
