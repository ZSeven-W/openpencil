//! Deterministic cleanup used by `EditorCommand::RefineDesign`.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;

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
    /// Refine a generated design root. Returns `None` when the root is
    /// invalid/missing and `Some(changed)` when the command is accepted.
    pub(crate) fn cmd_refine_design(
        &mut self,
        root_id: &NodeId,
        _canvas_width: Option<i32>,
    ) -> Option<bool> {
        if !root_id.is_real() {
            return None;
        }
        let root = walkers::find_node_mut(self.active_children_mut(), root_id)?;
        Some(!refine_subtree(root).is_empty())
    }
}

/// Apply the deterministic refine transforms to `root` in place and return the
/// fix report (one entry per change). SHARED by `cmd_refine_design` (the host
/// apply path) and the `design_refine` MCP tool (which simulates on a clone to
/// report `fixes[]`), so the reported fixes always match what apply does.
pub fn refine_subtree(root: &mut PenNode) -> Vec<RefineFix> {
    let mut fixes = Vec::new();
    // Pass order mirrors TS `design-refine.ts:60-71` for the
    // deterministic subset (role / icon-path resolution are the
    // live-hook passes; see the module docs for coverage).
    apply_no_emoji_icon_heuristic(root, &mut fixes);
    ensure_unique_node_ids(root, &mut fixes);
    sanitize_auto_layout_child_positions(root, &mut fixes);
    sanitize_screen_frame_bounds(root, &mut fixes);
    adjust_root_height_to_content(root, &mut fixes);
    fixes
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
