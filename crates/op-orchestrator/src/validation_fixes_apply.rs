//! B2: Fix application — `apply_validation_fixes` and helpers.
//!
//! Wired from `validation_fixes.rs` via:
//! `#[path = "validation_fixes_apply.rs"] pub(crate) mod apply;`
//!
//! Faithful port of `applyValidationFixes` (lines 163-301) and
//! `autoFixParentLayoutAfterAddChild` (lines 378-433) in
//! `apps/web/src/services/ai/design-validation-fixes.ts`.

use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};

use crate::validation_fixes::{is_valid_fix_value, SAFE_FIX_PROPERTIES};
use crate::validation_fixes_b3::{
    build_node_from_spec, node_type_str, snapshot_justify, snapshot_layout,
};

// ── B2: Types for fix application ─────────────────────────────────────────────

/// A single property fix returned by the vision LLM.
///
/// Port of `ValidationFix` in `design-validation-fixes.ts:15-19`.
#[derive(Debug, Clone)]
pub(crate) struct ValidationFix {
    pub node_id: String,
    pub property: String,
    pub value: serde_json::Value,
}

/// A structural fix returned by the vision LLM.
///
/// Port of `StructuralFix` in `design-validation-fixes.ts:21-48`.
#[derive(Debug, Clone)]
pub(crate) enum StructuralFix {
    /// Remove an existing node.  Carries the target node id.
    RemoveNode { node_id: String },
    /// Add a child node under a parent.  `spec` is the raw JSON object
    /// the LLM provided — full builder lives in B3.
    AddChild {
        parent_id: String,
        index: Option<usize>,
        spec: serde_json::Value,
    },
}

/// Result of `apply_validation_fixes`.
#[derive(Debug, Clone, Default)]
pub(crate) struct ApplyResult {
    /// Number of fixes that were applied successfully.
    pub applied: usize,
    /// Human-readable log of skipped fixes (for diagnostics).
    pub errors: Vec<String>,
}

// ── B2: apply_validation_fixes ────────────────────────────────────────────────

/// Apply property fixes and structural fixes to the document.
///
/// Faithful port of `applyValidationFixes` in
/// `design-validation-fixes.ts:163-301`.
///
/// **B3 note:** `addChild` structural fixes are deferred.
/// `auto_fix_parent_layout_after_add_child` is only called when
/// `addChild` fixes are processed (B3 will call it from its branch).
pub(crate) fn apply_validation_fixes(
    sink: &mut dyn crate::types::DocSink,
    fixes: &[ValidationFix],
    structural_fixes: &[StructuralFix],
) -> ApplyResult {
    let mut result = ApplyResult::default();

    // ── Property fixes ────────────────────────────────────────────────────────
    for fix in fixes {
        let node_id = NodeId::new(fix.node_id.clone());

        // Skip if node not found in active page.
        {
            let state = sink.state();
            if op_editor_core::walkers::find_node(state.active_children(), &node_id).is_none() {
                result.errors.push(format!("{} (not found)", fix.node_id));
                continue;
            }
        }

        // Whitelist check.
        if !SAFE_FIX_PROPERTIES.iter().any(|p| p.name == fix.property) {
            result.errors.push(format!(
                "{}.{} (unsupported property)",
                fix.node_id, fix.property
            ));
            continue;
        }

        // Value shape validation.
        if !is_valid_fix_value(&fix.property, &fix.value) {
            result.errors.push(format!(
                "{}.{}={} (invalid value)",
                fix.node_id, fix.property, fix.value
            ));
            continue;
        }

        // fit_content → fixed-pixel guard (TS lines 191-208).
        if (fix.property == "height" || fix.property == "width") && fix.value.is_number() {
            let state = sink.state();
            if let Some(node) =
                op_editor_core::walkers::find_node(state.active_children(), &node_id)
            {
                use op_editor_core::walkers as wk;
                let current_sizing = if fix.property == "width" {
                    node_sizing_width(node)
                } else {
                    node_sizing_height(node)
                };
                let is_fit_content = matches!(
                    current_sizing,
                    Some(jian_ops_schema::sizing::SizingBehavior::Keyword(
                        jian_ops_schema::sizing::SizingKeyword::FitContent
                    ))
                );
                let has_layout = node_has_layout(node);
                let child_count = node_child_count(node);
                let _ = wk::find_node; // suppress unused import warning
                let _ = node.base(); // use trait to suppress dead-code warning

                if is_fit_content && has_layout && child_count > 1 {
                    result.errors.push(format!(
                        "{}.{} (refusing fit_content→{}px on layout container)",
                        fix.node_id, fix.property, fix.value
                    ));
                    continue;
                }
            }
        }

        // ── Virtual property translation ──────────────────────────────────────

        if fix.property == "fillColor" {
            if let Some(color) = fix.value.as_str() {
                sink.apply(EditorCommand::SetNodeFillHex {
                    node_id,
                    hex: color.to_string(),
                });
                result.applied += 1;
                continue;
            }
        }

        if fix.property == "strokeColor" {
            if let Some(color) = fix.value.as_str() {
                sink.apply(EditorCommand::SetNodeStrokeHex {
                    node_id,
                    hex: color.to_string(),
                });
                result.applied += 1;
                continue;
            }
        }

        if fix.property == "strokeWidth" {
            if let Some(w) = fix.value.as_f64() {
                sink.apply(EditorCommand::SetNodeStrokeWidth {
                    node_id,
                    width: w as f32,
                });
                result.applied += 1;
                continue;
            }
        }

        // ── Typed property dispatch ───────────────────────────────────────────

        if fix.property == "cornerRadius" {
            if let Some(v) = fix.value.as_f64() {
                sink.apply(EditorCommand::SetNodeCornerRadius {
                    node_id,
                    radius: v as f32,
                });
                result.applied += 1;
                continue;
            }
        }

        if fix.property == "fontSize" {
            if let Some(v) = fix.value.as_f64() {
                sink.apply(EditorCommand::SetNodeFontSize {
                    node_id,
                    font_size: v as f32,
                });
                result.applied += 1;
                continue;
            }
        }

        if fix.property == "fontWeight" {
            if let Some(v) = fix.value.as_u64() {
                sink.apply(EditorCommand::SetNodeFontWeight {
                    node_id,
                    font_weight: v as u16,
                });
                result.applied += 1;
                continue;
            }
        }

        // width / height: number → UpdateNode, keyword → SetNodeLayoutProp
        if fix.property == "width" || fix.property == "height" {
            if let Some(px) = fix.value.as_f64() {
                let is_width = fix.property == "width";
                sink.apply(EditorCommand::UpdateNode {
                    node_id,
                    x: None,
                    y: None,
                    width: if is_width {
                        Some(px.round() as i32)
                    } else {
                        None
                    },
                    height: if !is_width {
                        Some(px.round() as i32)
                    } else {
                        None
                    },
                    name: None,
                    fill_hex: None,
                });
                result.applied += 1;
                continue;
            } else if let Some(kw) = fix.value.as_str() {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: fix.property.clone(),
                    value: LayoutPropValue::Keyword(kw.to_string()),
                });
                result.applied += 1;
                continue;
            }
        }

        // opacity → SetNodeLayoutProp Number
        if fix.property == "opacity" {
            if let Some(v) = fix.value.as_f64() {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: "opacity".to_string(),
                    value: LayoutPropValue::Number(v),
                });
                result.applied += 1;
                continue;
            }
        }

        // padding → SetNodeLayoutProp Number or NumberArray
        if fix.property == "padding" {
            let lv = if let Some(n) = fix.value.as_f64() {
                Some(LayoutPropValue::Number(n))
            } else if let Some(arr) = fix.value.as_array() {
                let nums: Option<Vec<f64>> = arr.iter().map(|v| v.as_f64()).collect();
                nums.map(LayoutPropValue::NumberArray)
            } else {
                None
            };
            if let Some(lv) = lv {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: "padding".to_string(),
                    value: lv,
                });
                result.applied += 1;
                continue;
            }
        }

        // Keyword layout / text properties → SetNodeLayoutProp Keyword
        if matches!(
            fix.property.as_str(),
            "alignItems" | "justifyContent" | "textAlign" | "textGrowth"
        ) {
            if let Some(s) = fix.value.as_str() {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: fix.property.clone(),
                    value: LayoutPropValue::Keyword(s.to_string()),
                });
                result.applied += 1;
                continue;
            }
        }

        // Numeric layout / text properties → SetNodeLayoutProp Number
        if matches!(
            fix.property.as_str(),
            "gap" | "letterSpacing" | "lineHeight"
        ) {
            if let Some(v) = fix.value.as_f64() {
                sink.apply(EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property: fix.property.clone(),
                    value: LayoutPropValue::Number(v),
                });
                result.applied += 1;
                continue;
            }
        }

        // Any whitelisted property that fell through without a handler.
        result.errors.push(format!(
            "{}.{} (no handler for value {})",
            fix.node_id, fix.property, fix.value
        ));
    }

    // ── Structural fixes ──────────────────────────────────────────────────────
    for sf in structural_fixes {
        match sf {
            StructuralFix::RemoveNode { node_id } => {
                let id = NodeId::new(node_id.clone());
                let state = sink.state();
                let maybe_node = op_editor_core::walkers::find_node(state.active_children(), &id);

                let Some(node) = maybe_node else {
                    result
                        .errors
                        .push(format!("removeNode: {node_id} not found"));
                    continue;
                };

                // Never remove pre-injected chrome (e.g. iPhone status bar).
                if node.base().role.as_deref() == Some("status-bar") {
                    result.errors.push(format!(
                        "removeNode: {node_id} is a protected status-bar node"
                    ));
                    continue;
                }

                sink.apply(EditorCommand::DeleteNode { node_id: id });
                result.applied += 1;
            }
            StructuralFix::AddChild {
                parent_id,
                index,
                spec,
            } => {
                let pid = NodeId::new(parent_id.clone());
                // Parent must exist.
                let parent_node = {
                    let state = sink.state();
                    op_editor_core::walkers::find_node(state.active_children(), &pid).cloned()
                };
                let Some(parent_node) = parent_node else {
                    result
                        .errors
                        .push(format!("addChild: parent {parent_id} not found"));
                    continue;
                };

                // Never add children to the injected status-bar chrome.
                if parent_node.base().role.as_deref() == Some("status-bar") {
                    result.errors.push(format!(
                        "addChild: parent {parent_id} is a protected status-bar node"
                    ));
                    continue;
                }

                // Snapshot parent properties we need for auto-fix BEFORE the add.
                let parent_justify_before = snapshot_justify(&parent_node);
                let parent_layout_before = snapshot_layout(&parent_node);
                let parent_name_before = parent_node.base().name.clone().unwrap_or_default();
                let parent_type_before = node_type_str(&parent_node);

                // Build the node from the spec.
                let Some(new_node) = build_node_from_spec(spec, *index) else {
                    result.errors.push(format!(
                        "addChild: could not build node for spec {:?}",
                        spec.get("type").and_then(|v| v.as_str()).unwrap_or("?")
                    ));
                    continue;
                };

                // Insert via InsertSubtree under the parent.
                // NOTE: InsertSubtree appends to the parent's children;
                // the optional `index` in the spec is not yet supported
                // by EditorCommand::InsertSubtree (it always appends).
                // This matches the TS behaviour when index is None.
                sink.apply(EditorCommand::InsertSubtree {
                    nodes: vec![new_node],
                    parent_id: pid.clone(),
                });
                result.applied += 1;

                // Auto-fix parent layout to match sibling container if applicable.
                auto_fix_parent_layout_after_add_child(
                    sink,
                    parent_id,
                    parent_justify_before.as_deref(),
                    parent_layout_before.as_deref(),
                    &parent_name_before,
                    parent_type_before,
                );
            }
        }
    }

    result
}

// ── B2: auto_fix_parent_layout_after_add_child ────────────────────────────────

/// Adjust a parent node's layout properties (justifyContent, gap)
/// to match a sibling container of the same type / name base, after
/// a child was added to that parent.
///
/// Faithful port of `autoFixParentLayoutAfterAddChild` in
/// `design-validation-fixes.ts:378-433`.
///
/// In B2 (addChild not yet wired), this function is unreachable from
/// the main path.  It is fully implemented here so B3 can call it
/// from the `AddChild` branch of `apply_validation_fixes` without
/// additional changes.
///
/// `parent_justify_before` — the `justifyContent` string of the
/// parent **before** the child was added (needed because the state
/// has already changed by the time this is called).
pub(crate) fn auto_fix_parent_layout_after_add_child(
    sink: &mut dyn crate::types::DocSink,
    parent_id: &str,
    parent_justify_before: Option<&str>,
    parent_layout_before: Option<&str>,
    parent_name_before: &str,
    parent_type_before: &str,
) -> usize {
    // TS: if (!justify || justify === 'start') return; — only proceed when
    // the parent had a non-trivial justifyContent before the add.
    let justify = match parent_justify_before {
        None | Some("start") => return 0,
        Some(j) => j,
    };

    let parent_id_node = NodeId::new(parent_id.to_string());

    // Current child count of the parent (after add).
    let current_child_count = {
        let state = sink.state();
        match op_editor_core::walkers::find_node(state.active_children(), &parent_id_node) {
            Some(n) => node_child_count(n),
            None => return 0,
        }
    };

    // Extract the last "word" of the parent's name as the name base.
    let parent_name_base = extract_name_base(parent_name_before);

    // Walk all active-page nodes to find a matching sibling container.
    let flat: Vec<_> = {
        let state = sink.state();
        collect_all_nodes(state.active_children())
    };

    for candidate in flat {
        // Skip the parent itself.
        if candidate.id == parent_id {
            continue;
        }
        // Must be the same node type.
        if candidate.node_type != parent_type_before {
            continue;
        }
        // Must have the same layout mode.
        if candidate.layout.as_deref() != parent_layout_before {
            continue;
        }
        // Name-base match.
        let cand_name_base = extract_name_base(&candidate.name);
        if parent_name_base.is_empty()
            || cand_name_base.is_empty()
            || parent_name_base != cand_name_base
        {
            continue;
        }
        // Candidate must have ≤ children than current parent (TS lines 404-412).
        if current_child_count > candidate.child_count {
            return 0;
        }

        // Build the updates.
        let cand_justify = candidate.justify_content.as_deref();
        let mut applied = 0usize;

        if (cand_justify.unwrap_or("start")) != justify {
            let new_justify = cand_justify.unwrap_or("start");
            sink.apply(EditorCommand::SetNodeLayoutProp {
                node_id: parent_id_node.clone(),
                property: "justifyContent".to_string(),
                value: LayoutPropValue::Keyword(new_justify.to_string()),
            });
            applied += 1;
        }

        if let Some(cand_gap) = candidate.gap {
            // Only update gap when candidate's gap differs from what the
            // parent had (we don't know the parent's old gap here, so we
            // unconditionally align it with the sibling — same semantics
            // as the TS which checks `candGap !== parentNode.gap`).
            sink.apply(EditorCommand::SetNodeLayoutProp {
                node_id: parent_id_node.clone(),
                property: "gap".to_string(),
                value: LayoutPropValue::Number(cand_gap),
            });
            applied += 1;
        }

        return applied;
    }

    0
}

// ── helpers for node introspection ───────────────────────────────────────────

fn node_sizing_width(
    node: &jian_ops_schema::node::PenNode,
) -> Option<&jian_ops_schema::sizing::SizingBehavior> {
    use jian_ops_schema::node::PenNode;
    match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Text(n) => n.width.as_ref(),
        PenNode::TextInput(n) => n.width.as_ref(),
        _ => None,
    }
}

fn node_sizing_height(
    node: &jian_ops_schema::node::PenNode,
) -> Option<&jian_ops_schema::sizing::SizingBehavior> {
    use jian_ops_schema::node::PenNode;
    match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        PenNode::Text(n) => n.height.as_ref(),
        PenNode::TextInput(n) => n.height.as_ref(),
        _ => None,
    }
}

fn node_has_layout(node: &jian_ops_schema::node::PenNode) -> bool {
    use jian_ops_schema::node::container::LayoutMode;
    use jian_ops_schema::node::PenNode;
    match node {
        PenNode::Frame(n) => !matches!(n.container.layout, None | Some(LayoutMode::None)),
        PenNode::Group(n) => !matches!(n.container.layout, None | Some(LayoutMode::None)),
        PenNode::Rectangle(n) => !matches!(n.container.layout, None | Some(LayoutMode::None)),
        _ => false,
    }
}

fn node_child_count(node: &jian_ops_schema::node::PenNode) -> usize {
    use jian_ops_schema::node::PenNode;
    match node {
        PenNode::Frame(n) => n.children.as_ref().map(|c| c.len()).unwrap_or(0),
        PenNode::Group(n) => n.children.as_ref().map(|c| c.len()).unwrap_or(0),
        PenNode::Rectangle(n) => n.children.as_ref().map(|c| c.len()).unwrap_or(0),
        _ => 0,
    }
}

/// A flat snapshot of the properties we need for
/// `auto_fix_parent_layout_after_add_child`.
struct NodeSnapshot {
    id: String,
    name: String,
    node_type: &'static str,
    layout: Option<String>,
    justify_content: Option<String>,
    gap: Option<f64>,
    child_count: usize,
}

fn collect_all_nodes(roots: &[jian_ops_schema::node::PenNode]) -> Vec<NodeSnapshot> {
    let mut out = Vec::new();
    for root in roots {
        collect_node(root, &mut out);
    }
    out
}

fn collect_node(node: &jian_ops_schema::node::PenNode, out: &mut Vec<NodeSnapshot>) {
    use jian_ops_schema::node::base::NumberOrExpression;
    use jian_ops_schema::node::container::{JustifyContent, LayoutMode};
    use jian_ops_schema::node::PenNode;

    let node_type = match node {
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
    };

    let (layout, justify_content, gap, child_count) = match node {
        PenNode::Frame(n) => {
            let layout = n.container.layout.as_ref().map(|l| match l {
                LayoutMode::None => "none".to_string(),
                LayoutMode::Vertical => "vertical".to_string(),
                LayoutMode::Horizontal => "horizontal".to_string(),
            });
            let jc = n.container.justify_content.as_ref().map(|j| match j {
                JustifyContent::Start => "start".to_string(),
                JustifyContent::Center => "center".to_string(),
                JustifyContent::End => "end".to_string(),
                JustifyContent::SpaceBetween => "space_between".to_string(),
                JustifyContent::SpaceAround => "space_around".to_string(),
            });
            let gap = n.container.gap.as_ref().and_then(|g| match g {
                NumberOrExpression::Number(n) => Some(*n),
                _ => None,
            });
            let cc = n.children.as_ref().map(|c| c.len()).unwrap_or(0);
            (layout, jc, gap, cc)
        }
        PenNode::Group(n) => {
            let layout = n.container.layout.as_ref().map(|l| match l {
                LayoutMode::None => "none".to_string(),
                LayoutMode::Vertical => "vertical".to_string(),
                LayoutMode::Horizontal => "horizontal".to_string(),
            });
            let jc = n.container.justify_content.as_ref().map(|j| match j {
                JustifyContent::Start => "start".to_string(),
                JustifyContent::Center => "center".to_string(),
                JustifyContent::End => "end".to_string(),
                JustifyContent::SpaceBetween => "space_between".to_string(),
                JustifyContent::SpaceAround => "space_around".to_string(),
            });
            let gap = n.container.gap.as_ref().and_then(|g| match g {
                NumberOrExpression::Number(n) => Some(*n),
                _ => None,
            });
            let cc = n.children.as_ref().map(|c| c.len()).unwrap_or(0);
            (layout, jc, gap, cc)
        }
        PenNode::Rectangle(n) => {
            let layout = n.container.layout.as_ref().map(|l| match l {
                LayoutMode::None => "none".to_string(),
                LayoutMode::Vertical => "vertical".to_string(),
                LayoutMode::Horizontal => "horizontal".to_string(),
            });
            let jc = n.container.justify_content.as_ref().map(|j| match j {
                JustifyContent::Start => "start".to_string(),
                JustifyContent::Center => "center".to_string(),
                JustifyContent::End => "end".to_string(),
                JustifyContent::SpaceBetween => "space_between".to_string(),
                JustifyContent::SpaceAround => "space_around".to_string(),
            });
            let gap = n.container.gap.as_ref().and_then(|g| match g {
                NumberOrExpression::Number(n) => Some(*n),
                _ => None,
            });
            let cc = n.children.as_ref().map(|c| c.len()).unwrap_or(0);
            (layout, jc, gap, cc)
        }
        _ => (None, None, None, 0),
    };

    out.push(NodeSnapshot {
        id: node.id_str().to_string(),
        name: node.base().name.clone().unwrap_or_default(),
        node_type,
        layout,
        justify_content,
        gap,
        child_count,
    });

    // Recurse into children.
    if let Some(children) = match node {
        PenNode::Frame(n) => n.children.as_ref(),
        PenNode::Group(n) => n.children.as_ref(),
        PenNode::Rectangle(n) => n.children.as_ref(),
        _ => None,
    } {
        for child in children {
            collect_node(child, out);
        }
    }
}

/// Extract the last whitespace-delimited word from `name` and lowercase it.
///
/// Faithful port of `extractNameBase` in
/// `design-validation-fixes.ts:435-438`:
/// ```ts
/// const words = name.trim().toLowerCase().split(/\s+/);
/// return words.length > 0 ? words[words.length - 1] : '';
/// ```
/// B2 nit: the TS calls `.toLowerCase()` BEFORE splitting and taking the last
/// word, so "Nav ITEM" → "item" and "Tab item" → "item" share the same base.
/// The B2 Rust implementation was missing the lowercase step; B3 adds it.
fn extract_name_base(name: &str) -> String {
    let lowered = name.trim().to_lowercase();
    if lowered.is_empty() {
        return String::new();
    }
    lowered.split_whitespace().last().unwrap_or("").to_string()
}
