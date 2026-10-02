//! Document-derived turn context: append-target detection, the design-md
//! auto-generation gate, the design-variable context block, and the
//! selection-scoped modify plan. Split out of `chat_intent.rs` to keep the
//! spine under the 800-line cap.

use super::*;

pub(super) type PenNode = jian_ops_schema::node::PenNode;

pub(super) fn is_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
}

pub(super) fn is_status_bar_like(node: &PenNode) -> bool {
    let name = node.base().name.as_deref().unwrap_or("");
    is_status_bar_like_text(&format!("{name} {}", node.id_str()))
}

pub(super) fn node_label(node: &PenNode) -> String {
    node.base()
        .name
        .clone()
        .unwrap_or_else(|| node.id_str().to_string())
}

/// TS `pickContentRoot` — prefer a child frame named like a content
/// root, else the page frame itself.
pub(super) fn pick_content_root(page: &PenNode) -> (&PenNode, Vec<String>) {
    let children: &[PenNode] = page.children().map(Vec::as_slice).unwrap_or(&[]);
    let content_frames: Vec<&PenNode> = children
        .iter()
        .filter(|n| is_frame(n) && !is_status_bar_like(n))
        .collect();

    const CONTENT_NAME: &[&str] = &["content", "main", "body", "root"];
    let candidate = content_frames.iter().find(|f| {
        let name = f.base().name.as_deref().unwrap_or("").to_lowercase();
        matches_any_word_phrase(&name, CONTENT_NAME)
    });
    if let Some(candidate) = candidate {
        let grand = candidate.children();
        let labels = grand
            .map(|kids| {
                kids.iter()
                    .filter(|n| is_frame(n) && !is_status_bar_like(n))
                    .map(node_label)
                    .collect()
            })
            .unwrap_or_default();
        return (candidate, labels);
    }

    (page, content_frames.iter().map(|n| node_label(n)).collect())
}

/// Detect append intent against the live editor state. Append is a targeted
/// mutation, so continuation wording alone never chooses an arbitrary canvas
/// frame. Exactly one selected Frame is required; callers otherwise stay on
/// the new-design/chat route.
pub fn detect_append_intent(state: &EditorState, prompt: &str) -> Option<AppendContext> {
    if prompt.trim().is_empty() {
        return None;
    }
    let lower = prompt.to_lowercase();
    let has_append = matches_any_word_phrase(&lower, APPEND_EN)
        || APPEND_CJK.iter().any(|k| prompt.contains(k))
        || matches_cjk_add_section(prompt);
    if !has_append {
        return None;
    }
    if is_new_screen_veto(prompt) {
        return None;
    }

    let [selected_id] = state.selection.set.as_slice() else {
        return None;
    };
    let page_frame = op_editor_core::walkers::find_node(state.active_children(), selected_id)?;
    if !is_frame(page_frame) {
        return None;
    }
    let page_has_content = page_frame
        .children()
        .is_some_and(|kids| kids.iter().any(|c| is_frame(c) && !is_status_bar_like(c)));
    if !page_has_content {
        return None;
    }

    let (target, section_labels) = pick_content_root(page_frame);
    let width = page_frame.width_px().unwrap_or(375.0);

    Some(AppendContext {
        target_parent_id: target.id_str().to_string(),
        target_width: target.width_px().unwrap_or(width),
        existing_section_labels: section_labels,
        is_mobile: width <= 480.0,
    })
}

/// Design generation should ask the selected LLM to extract a design.md from
/// the current canvas for named follow-on pages (Discover / Orders / Profile,
/// etc.). A document-bound design.md wins, and append mode keeps its
/// append-specific context instead of creating a new sibling screen.
pub fn should_auto_generate_design_md(
    state: &EditorState,
    prompt: &str,
    append_context: Option<&AppendContext>,
) -> bool {
    state.doc.design_md.is_none()
        && append_context.is_none()
        && !state.active_children().is_empty()
        && (is_named_follow_on_screen(prompt) || requests_new_whole_screen(prompt))
}

pub use op_chat_agent::chat_modify::{
    build_modify_plan, build_modify_plan_for_route, build_variable_context,
    modify_target_frame_ids, parse_modify_nodes, ModifyPlan,
};
