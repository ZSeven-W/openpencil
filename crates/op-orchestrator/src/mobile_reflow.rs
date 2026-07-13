//! Structural reflow for a mobile root whose bottom nav was appended after a
//! construction-time, fixed-height content wrapper.

use jian_ops_schema::node::{
    container::{ContainerProps, JustifyContent, LayoutMode, Padding},
    NumberOrExpression, PenNode,
};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId, PenNodeExt};

use crate::types::DocSink;

const MOBILE_MIN_WIDTH: f64 = 280.0;
const MOBILE_MAX_WIDTH: f64 = 480.0;
const MOBILE_MIN_HEIGHT: f64 = 500.0;
const INTRINSIC_PROBE_HEIGHT: i32 = 1_000_000;

#[derive(Debug, Clone)]
struct ReflowCandidate {
    root_id: String,
    wrapper_id: String,
    nav_id: String,
    root_height: f64,
    root_bottom_padding: f64,
    root_gap_needs_default: bool,
    wrapper_needs_hug: bool,
}

/// Repair a construction-time mobile content height after a bottom tab bar is
/// appended in a later batch.
///
/// Returns `true` when at least one wrapper or root was changed. The narrow
/// status/content/trailing-nav contract also restores the canonical 16 px gap
/// between those major regions when the model omitted it. The root keeps its
/// authored numeric height as a minimum: it grows to contain long intrinsic
/// content, but never shrinks when the content is shorter than the artboard.
pub fn repair_mobile_trailing_nav_reflow(state: &mut EditorState) -> bool {
    let mut sink = EditorStateSink { state };
    repair_mobile_trailing_nav_reflow_in_sink(&mut sink)
}

pub(crate) fn repair_mobile_trailing_nav_reflow_in_sink(sink: &mut dyn DocSink) -> bool {
    let candidates = collect_candidates(sink.state());
    apply_candidates(sink, candidates)
}

pub(crate) fn repair_mobile_trailing_nav_reflow_for_root_in_sink(
    sink: &mut dyn DocSink,
    root_id: &str,
) -> bool {
    let candidate = sink
        .state()
        .active_children()
        .iter()
        .find(|root| root.id_str() == root_id)
        .and_then(candidate_for_root);
    apply_candidates(sink, candidate.into_iter().collect())
}

/// Whether `root` has the narrow structure managed by this reflow pass.
///
/// Cleanup uses this after the first scene-backed reflow to prevent its generic
/// tree-height estimator from overwriting the result (notably by counting
/// absolute overlays as flow content). A final reflow still measures the real
/// scene and grows the numeric root if later repairs made the flow content
/// genuinely taller.
pub(crate) fn has_mobile_trailing_nav_reflow_contract(root: &PenNode) -> bool {
    candidate_for_root(root).is_some()
}

fn apply_candidates(sink: &mut dyn DocSink, candidates: Vec<ReflowCandidate>) -> bool {
    let mut changed = false;

    for candidate in candidates {
        // Resolve intrinsic content on a clone first. This keeps the real edit
        // atomic if a malformed tree cannot produce the expected scene nodes.
        let mut probe = sink.state().clone();
        if candidate.wrapper_needs_hug && !set_fit_content(&mut probe, &candidate.wrapper_id) {
            continue;
        }
        if candidate.root_gap_needs_default
            && !set_gap(
                &mut probe,
                &candidate.root_id,
                crate::plan_normalize::MOBILE_DEFAULT_ROOT_GAP,
            )
        {
            continue;
        }
        if !probe.apply(update_height_command(
            &candidate.root_id,
            INTRINSIC_PROBE_HEIGHT,
        )) {
            continue;
        }
        let Some(required_height) = resolved_required_root_height(&probe, &candidate) else {
            continue;
        };

        if candidate.wrapper_needs_hug {
            changed |= sink.apply(fit_content_command(&candidate.wrapper_id));
        }
        if candidate.root_gap_needs_default {
            changed |= sink.apply(gap_command(
                &candidate.root_id,
                crate::plan_normalize::MOBILE_DEFAULT_ROOT_GAP,
            ));
        }
        if required_height > candidate.root_height + 0.5 {
            let height = required_height.ceil().min(f64::from(i32::MAX)) as i32;
            changed |= sink.apply(update_height_command(&candidate.root_id, height));
        }
    }

    changed
}

fn collect_candidates(state: &EditorState) -> Vec<ReflowCandidate> {
    state
        .active_children()
        .iter()
        .filter_map(candidate_for_root)
        .collect()
}

fn candidate_for_root(root: &PenNode) -> Option<ReflowCandidate> {
    let root_props = container_props(root)?;
    if root_props.layout.as_ref() != Some(&LayoutMode::Vertical)
        || !matches!(
            root_props.justify_content.as_ref(),
            None | Some(JustifyContent::Start)
        )
    {
        return None;
    }

    let root_width = numeric_sizing(root_props.width.as_ref())?;
    let root_height = numeric_sizing(root_props.height.as_ref())?;
    if !(MOBILE_MIN_WIDTH..=MOBILE_MAX_WIDTH).contains(&root_width)
        || root_height < MOBILE_MIN_HEIGHT
    {
        return None;
    }

    let children = root.children()?;
    let (nav, prefix) = children.split_last()?;
    if prefix.is_empty()
        || nav.base().x.is_some()
        || nav.base().y.is_some()
        || !has_resolvable_trailing_nav_height(nav)
        || !crate::cleanup::is_trailing_bottom_nav_section(nav)
    {
        return None;
    }

    // Absolute root children are overlays, not members of the vertical flow.
    // They may legitimately sit between the content shell and trailing nav in
    // authored order, so identify the wrapper and stale prefix from in-flow
    // children only.
    // A positioned child without the overlay role is ambiguous in the current
    // scene repairer: deterministic reflow must not guess whether its x/y are
    // real absolute intent or stale flow debris. Role-marked overlays are the
    // shared, explicit out-of-flow contract and can be ignored safely.
    if prefix
        .iter()
        .any(|child| (child.base().x.is_some() || child.base().y.is_some()) && !is_overlay(child))
    {
        return None;
    }
    let flow_prefix = prefix
        .iter()
        .filter(|child| !is_overlay(child))
        .collect::<Vec<_>>();
    let wrapper = *flow_prefix.last()?;
    let wrapper_props = container_props(wrapper)?;
    let wrapper_needs_hug = match wrapper_props.height.as_ref() {
        Some(SizingBehavior::Number(value)) if value.is_finite() && *value > 0.0 => true,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => false,
        _ => return None,
    };
    if wrapper.base().x.is_some()
        || wrapper.base().y.is_some()
        || wrapper_props.layout.as_ref() != Some(&LayoutMode::Vertical)
        || !matches!(
            wrapper_props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || wrapper.children().is_none_or(Vec::is_empty)
        || is_explicit_scroll_viewport(wrapper)
    {
        return None;
    }

    // The repair is safe only for the single main wrapper shape. A root with
    // several direct content sections has no evidence that the final numeric
    // section was a temporary remainder shell.
    let content_wrappers = flow_prefix
        .iter()
        .filter(|child| {
            !is_mobile_status_bar(child)
                && container_props(child).is_some()
                && child
                    .children()
                    .is_some_and(|children| !children.is_empty())
        })
        .collect::<Vec<_>>();
    if content_wrappers.len() != 1 || content_wrappers[0].id_str() != wrapper.id_str() {
        return None;
    }

    let [padding_top, _, padding_bottom, _] = match root_props.padding.as_ref() {
        Some(padding) => padding_sides(padding)?,
        None => [0.0; 4],
    };
    let (gap, gap_is_missing) = match root_props.gap.as_ref() {
        // Plan normalization may default a weak model's provisional `0`, but
        // once a numeric value is authored in the document it is intent. Only
        // an omitted property is safe to restore during deterministic finalize.
        Some(NumberOrExpression::Number(value)) => (*value, false),
        Some(NumberOrExpression::Expression(_)) => return None,
        None => (0.0, true),
    };
    if wrapper_needs_hug {
        let prefix_children_height = flow_prefix
            .iter()
            .map(|child| child.height_px())
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .sum::<f64>();
        let prefix_height = padding_top
            + padding_bottom
            + prefix_children_height
            + gap * flow_prefix.len().saturating_sub(1) as f64;
        let tolerance = (root_height * 0.02).clamp(12.0, 24.0);
        if (prefix_height - root_height).abs() > tolerance {
            return None;
        }
    }

    // Spacing is a stricter contract than containment. Only infer the mobile
    // region gap for the exact in-flow stack mirrored by Pencil:
    // status bar -> one content wrapper -> trailing bottom nav. Reflow remains
    // available to older no-status or spacer-bearing trees without rewriting
    // their authored rhythm.
    let root_gap_needs_default = gap_is_missing
        && flow_prefix.len() == 2
        && is_mobile_status_bar(flow_prefix[0])
        && flow_prefix[1].id_str() == wrapper.id_str();

    Some(ReflowCandidate {
        root_id: root.id_str().to_string(),
        wrapper_id: wrapper.id_str().to_string(),
        nav_id: nav.id_str().to_string(),
        root_height,
        root_bottom_padding: padding_bottom,
        root_gap_needs_default,
        wrapper_needs_hug,
    })
}

fn resolved_required_root_height(state: &EditorState, candidate: &ReflowCandidate) -> Option<f64> {
    // Scene geometry is the only reliable evidence here: an authored-tree sum
    // cannot distinguish ordinary flow children from absolute overlays. If the
    // active page cannot resolve the expected root/nav pair, leave the document
    // untouched instead of guessing a larger root.
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    scene
        .pages
        .get(state.ui.active_page_index)
        .or_else(|| scene.pages.first())
        .and_then(|page| find_scene_node(&page.children, &candidate.root_id))
        .and_then(|root| {
            let nav = root
                .children
                .iter()
                .find(|node| node.id == candidate.nav_id)?;
            let root_top = f64::from(root.bounds.origin.y);
            let nav_bottom = f64::from(nav.bounds.origin.y + nav.bounds.size.y);
            Some(nav_bottom - root_top + candidate.root_bottom_padding)
        })
        .filter(|required| required.is_finite() && *required >= 0.0)
}

fn find_scene_node<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    nodes.iter().find_map(|node| {
        (node.id == id)
            .then_some(node)
            .or_else(|| find_scene_node(&node.children, id))
    })
}

fn set_fit_content(state: &mut EditorState, node_id: &str) -> bool {
    state.apply(fit_content_command(node_id))
}

fn set_gap(state: &mut EditorState, node_id: &str, gap: f64) -> bool {
    state.apply(gap_command(node_id, gap))
}

fn fit_content_command(node_id: &str) -> EditorCommand {
    EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(node_id.to_string()),
        property: "height".to_string(),
        value: LayoutPropValue::Keyword("fit_content".to_string()),
    }
}

fn gap_command(node_id: &str, gap: f64) -> EditorCommand {
    EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(node_id.to_string()),
        property: "gap".to_string(),
        value: LayoutPropValue::Number(gap),
    }
}

fn update_height_command(node_id: &str, height: i32) -> EditorCommand {
    EditorCommand::UpdateNode {
        node_id: NodeId::new(node_id.to_string()),
        x: None,
        y: None,
        width: None,
        height: Some(height),
        name: None,
        fill_hex: None,
        page_id: None,
    }
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(node) => Some(&node.container),
        PenNode::Group(node) => Some(&node.container),
        PenNode::Rectangle(node) => Some(&node.container),
        _ => None,
    }
}

fn is_mobile_status_bar(node: &PenNode) -> bool {
    crate::cleanup::is_status_bar(node)
        || matches!(
            node.base()
                .role
                .as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("status-bar" | "status_bar" | "statusbar")
        )
}

fn numeric_sizing(value: Option<&SizingBehavior>) -> Option<f64> {
    match value {
        Some(SizingBehavior::Number(value)) if value.is_finite() && *value > 0.0 => Some(*value),
        _ => None,
    }
}

fn padding_sides(padding: &Padding) -> Option<[f64; 4]> {
    match padding {
        Padding::Uniform(value) => Some([*value; 4]),
        Padding::XY([vertical, horizontal]) => {
            Some([*vertical, *horizontal, *vertical, *horizontal])
        }
        Padding::LtrB(values) => Some(*values),
        Padding::Expression(_) => None,
    }
}

fn has_resolvable_trailing_nav_height(node: &PenNode) -> bool {
    let Some(props) = container_props(node) else {
        return false;
    };
    match props.height.as_ref() {
        Some(SizingBehavior::Number(value)) => value.is_finite() && *value > 0.0,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) | None => {
            node.children().is_some_and(|children| !children.is_empty())
        }
        _ => false,
    }
}

fn is_overlay(node: &PenNode) -> bool {
    matches!(node.base().role.as_deref(), Some("overlay"))
}

pub(crate) fn is_explicit_scroll_viewport(node: &PenNode) -> bool {
    matches!(
        node.base()
            .role
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("scroll" | "scroll-area" | "scroll-viewport" | "viewport")
    )
}

struct EditorStateSink<'a> {
    state: &'a mut EditorState,
}

impl DocSink for EditorStateSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }

    fn apply(&mut self, command: EditorCommand) -> bool {
        self.state.apply(command)
    }

    fn begin_undo_batch(&mut self) {}

    fn end_undo_batch(&mut self) {}
}
