//! Editor-agnostic `PlannedFix` API — a plan for what `apply_fixes` would do,
//! expressed as a data type rather than a direct mutation.
//!
//! `detect_and_plan` runs the same filter logic that `apply_fixes` uses
//! (skip `Info`-severity issues, skip the no-op `Fill` property, skip
//! `Remove` on `status-bar`-role nodes), then emits a `PlannedFix` per
//! issue that would have been applied. The final document produced by
//! applying every `PlannedFix` is byte-identical to the final document
//! produced by `detect_and_fix` — this invariant is verified by the
//! equivalence tests in this module.
//!
//! Host adapters (e.g. `op-host-desktop::pre_validator`) translate each
//! `PlannedFix` to one or more `EditorCommand`s and apply them via
//! `DocSink::apply`. This design keeps `op-design-lint` editor-agnostic
//! and isolates the `FixProperty → EditorCommand` translation table to a
//! single host module.

use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
use crate::node_util;
use jian_ops_schema::PenDocument;

/// An editor-agnostic description of a fix that `detect_and_fix` would
/// have applied to the document. The `PlannedAction` captures the full
/// mutation in a form that a host adapter can translate to `EditorCommand`s.
///
/// Structural equivalence invariant: applying all `PlannedFix`es from
/// `detect_and_plan(doc)` using the mutation primitives in `crate::node_mut`
/// produces a document structurally identical to calling `detect_and_fix(doc)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedFix {
    /// The node the fix targets.
    pub node_id: String,
    /// Which detector category produced this fix — used for per-category
    /// counts in `PreValidationResult`.
    pub category: IssueCategory,
    /// The concrete mutation to apply.
    pub action: PlannedAction,
}

/// The concrete mutation encoded by a `PlannedFix`.
///
/// Variants cover every mutation path in `fixes::apply_fixes` today:
///
/// | `FixProperty`   | `PlannedAction` variant         | Notes                         |
/// |-----------------|--------------------------------|-------------------------------|
/// | `Remove`        | `Remove`                       | detach node from parent       |
/// | `Height`        | `SetHeightFitContent`          | only `"fit_content"` is valid |
/// | `Rotation`      | `SetRotation(f64)`             | degrees                       |
/// | `CornerRadius`  | `SetCornerRadius(f32)`         | uniform radius                |
/// | `FontSize`      | `SetFontSize(f32)`             | doc-px                        |
/// | `Effects`       | `ClearEffects`                 | set effects field to None     |
/// | `Padding`       | `SetPadding(Value)`            | uniform or [L,T,R,B] array    |
/// | `Stroke`        | `SetStroke(Value)`             | full PenStroke JSON object    |
/// | `Fill`          | *(filtered out — no-op)*       | `apply_fixes` returns `false` |
#[derive(Debug, Clone, PartialEq)]
pub enum PlannedAction {
    /// Remove the node from its parent (mirrors `fixes::apply_remove`).
    Remove,
    /// Set the node's height sizing to `fit_content` (text-explicit-height fix).
    SetHeightFitContent,
    /// Set the node's rotation to the given degrees.
    SetRotation(f64),
    /// Set the node's corner-radius to a uniform value.
    SetCornerRadius(f32),
    /// Set the node's font size.
    SetFontSize(f32),
    /// Clear the node's effects list (set to `None`).
    ClearEffects,
    /// Set the node's padding. The `Value` is either a single number
    /// (uniform) or a 4-element array `[L, T, R, B]`.
    SetPadding(serde_json::Value),
    /// Set the node's stroke to the given `PenStroke`-compatible JSON
    /// object (e.g. `{"thickness":1,"fill":[{"type":"solid","color":"#E2E8F0"}]}`).
    SetStroke(serde_json::Value),
}

/// Detect issues over the active page and return a plan of fixes —
/// the set that `apply_fixes` would have ACTUALLY applied (Info-severity
/// issues and `Fill` no-ops are excluded from the plan, matching the
/// filter in `apply_fixes`).
///
/// The returned list is in the same order as `apply_fixes` would have
/// processed the issues: one `PlannedFix` per issue that passes the
/// filter AND whose suggested value is usable (e.g. `Height` only when
/// `suggested_value == "fit_content"`).
///
/// This function reads the doc but does NOT mutate it; it is safe to
/// call on the same doc before handing it to the adapter.
pub fn detect_and_plan(doc: &PenDocument) -> Vec<PlannedFix> {
    // Locate the active-page root. Mirror the logic in `fixes::detect_and_fix`.
    let root = match active_page_root(doc) {
        Some(r) => r,
        None => return Vec::new(),
    };
    let issues = crate::detectors::detect_all(root, doc);

    let mut plan = Vec::new();

    for issue in &issues {
        // Mirror `apply_fixes`: skip Info-severity.
        if issue.severity == IssueSeverity::Info {
            continue;
        }

        if issue.property == FixProperty::Remove {
            // Mirror `apply_remove`: skip status-bar-role nodes.
            // Walk the page roots read-only to check the role before deciding.
            let is_status_bar = active_page_nodes(doc)
                .iter()
                .find_map(|root_node| find_node(root_node, &issue.node_id))
                .map(|n| node_util::role(n) == Some("status-bar"))
                .unwrap_or(false);
            if is_status_bar {
                continue;
            }
            plan.push(PlannedFix {
                node_id: issue.node_id.clone(),
                category: issue.category,
                action: PlannedAction::Remove,
            });
            continue;
        }

        // Translate `(property, suggested_value)` to a `PlannedAction`,
        // mirroring the `set_property` dispatch in `fixes.rs`. Skip combinations
        // that `set_property` returns `false` for (Fill, non-fit_content Height, etc.).
        let action = match issue.property {
            FixProperty::Height => {
                if issue.suggested_value.as_str() == Some("fit_content") {
                    PlannedAction::SetHeightFitContent
                } else {
                    continue; // set_property returns false; skip
                }
            }
            FixProperty::Rotation => {
                let v = match issue.suggested_value.as_f64() {
                    Some(v) => v,
                    None => continue,
                };
                PlannedAction::SetRotation(v)
            }
            FixProperty::CornerRadius => {
                let v = match issue.suggested_value.as_f64() {
                    Some(v) => v as f32,
                    None => continue,
                };
                PlannedAction::SetCornerRadius(v)
            }
            FixProperty::FontSize => {
                let v = match issue.suggested_value.as_f64() {
                    Some(v) => v as f32,
                    None => continue,
                };
                PlannedAction::SetFontSize(v)
            }
            FixProperty::Effects => PlannedAction::ClearEffects,
            FixProperty::Padding => PlannedAction::SetPadding(issue.suggested_value.clone()),
            FixProperty::Stroke => PlannedAction::SetStroke(issue.suggested_value.clone()),
            // Fill is a documented no-op in apply_fixes — skip.
            FixProperty::Fill => continue,
            // Remove is handled above; this arm is unreachable.
            FixProperty::Remove => continue,
        };

        plan.push(PlannedFix {
            node_id: issue.node_id.clone(),
            category: issue.category,
            action,
        });
    }

    plan
}

// ── Private helpers (mirror fixes.rs helpers) ────────────────────────────────

fn active_page_nodes(doc: &PenDocument) -> &[jian_ops_schema::node::PenNode] {
    match doc.pages.as_ref().and_then(|pages| pages.first()) {
        Some(page) => &page.children,
        None => &doc.children,
    }
}

fn active_page_root(doc: &PenDocument) -> Option<&jian_ops_schema::node::PenNode> {
    active_page_nodes(doc).first()
}

fn find_node<'a>(
    root: &'a jian_ops_schema::node::PenNode,
    node_id: &str,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    if node_util::node_id(root) == node_id {
        return Some(root);
    }
    node_util::children(root)
        .iter()
        .find_map(|child| find_node(child, node_id))
}

// ── Equivalence tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixes::detect_and_fix;
    use crate::node_mut;
    use crate::node_util;
    use jian_ops_schema::node::PenNode;
    use jian_ops_schema::PenDocument;
    use serde_json::Value;

    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    /// Apply a single `PlannedAction` to the target node in the document —
    /// mirrors what `apply_fixes` does via `set_property` / `apply_remove`,
    /// reusing the same `node_mut` helpers so the equivalence test is
    /// meaningful (not just checking the same code path twice).
    fn apply_planned_fix_to_doc(doc: &mut PenDocument, fix: &PlannedFix) -> bool {
        if fix.action == PlannedAction::Remove {
            // Mirror `apply_remove` — detach from whichever root contains it.
            for root in active_page_roots_mut(doc) {
                if node_mut::detach_child(root, &fix.node_id) {
                    return true;
                }
            }
            return false;
        }

        // Find the node mutably and apply the action.
        for root in active_page_roots_mut(doc) {
            if let Some(node) = find_node_mut(root, &fix.node_id) {
                return apply_action_to_node(node, &fix.action);
            }
        }
        false
    }

    fn apply_action_to_node(node: &mut PenNode, action: &PlannedAction) -> bool {
        match action {
            PlannedAction::SetHeightFitContent => node_mut::set_text_height_fit_content(node),
            PlannedAction::SetRotation(v) => node_mut::set_rotation(node, *v),
            PlannedAction::SetCornerRadius(v) => node_mut::set_corner_radius(node, *v),
            PlannedAction::SetFontSize(v) => node_mut::set_font_size(node, *v),
            PlannedAction::ClearEffects => node_mut::clear_effects(node),
            PlannedAction::SetPadding(v) => apply_padding(node, v),
            PlannedAction::SetStroke(v) => node_mut::set_stroke_from_json(node, v),
            PlannedAction::Remove => unreachable!("Remove handled in apply_planned_fix_to_doc"),
        }
    }

    fn apply_padding(node: &mut PenNode, suggested: &Value) -> bool {
        if let Some(n) = suggested.as_f64() {
            return node_mut::set_padding_uniform(node, n as f32);
        }
        if let Some(arr) = suggested.as_array() {
            if arr.len() == 4 {
                let v: Vec<f32> = arr
                    .iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect();
                if v.len() == 4 {
                    return node_mut::set_padding_ltrb(node, [v[0], v[1], v[2], v[3]]);
                }
            }
        }
        false
    }

    fn active_page_roots_mut(
        doc: &mut PenDocument,
    ) -> &mut [jian_ops_schema::node::PenNode] {
        let has_page = doc
            .pages
            .as_ref()
            .map(|pages| !pages.is_empty())
            .unwrap_or(false);
        if has_page {
            doc.pages.as_mut().unwrap()[0].children.as_mut_slice()
        } else {
            doc.children.as_mut_slice()
        }
    }

    fn find_node_mut<'a>(
        root: &'a mut PenNode,
        node_id: &str,
    ) -> Option<&'a mut PenNode> {
        if node_util::node_id(root) == node_id {
            return Some(root);
        }
        match root {
            PenNode::Frame(n) => n.children.as_mut(),
            PenNode::Group(n) => n.children.as_mut(),
            PenNode::Rectangle(n) => n.children.as_mut(),
            _ => None,
        }?
        .iter_mut()
        .find_map(|child| find_node_mut(child, node_id))
    }

    /// Load the `invisible-container` fixture and assert that `detect_and_plan`
    /// + apply-each-fix produces a document identical to `detect_and_fix`.
    ///
    /// This fixture exercises `FixProperty::Stroke` (the `Warning`-severity
    /// `light-on-light` node) and the `Info`-severity skip (the `dark-on-dark`
    /// node must NOT be in the plan).
    #[test]
    fn equivalence_invisible_container() {
        let raw = include_str!(
            "../tests/fixtures/docs/invisible-container.json"
        );
        let doc_a: PenDocument = serde_json::from_str(raw).expect("parse");
        let doc_b: PenDocument = serde_json::from_str(raw).expect("parse");

        // Clone A: detect_and_fix (the reference path).
        let mut clone_a = doc_a;
        let report = detect_and_fix(&mut clone_a);

        // Clone B: detect_and_plan + apply each fix via node_mut primitives.
        let mut clone_b = doc_b;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        assert_eq!(
            report.total as usize, applied,
            "fix counts must match: detect_and_fix={}, planned={}",
            report.total, applied
        );
        assert_eq!(plan.len(), applied, "all planned fixes must apply");
        assert_eq!(
            clone_a, clone_b,
            "detect_and_fix and detect_and_plan+apply must produce identical documents"
        );

        // Verify the Info-severity dark-on-dark node is NOT in the plan.
        let dark_in_plan = plan.iter().any(|f| f.node_id == "dark-on-dark");
        assert!(!dark_in_plan, "Info-severity nodes must not appear in the plan");
    }

    /// Load the `text-explicit-height` fixture and assert equivalence.
    ///
    /// This fixture exercises `FixProperty::Height` with `"fit_content"`.
    #[test]
    fn equivalence_text_explicit_height() {
        let raw =
            include_str!("../tests/fixtures/docs/text-explicit-height.json");
        let doc_a: PenDocument = serde_json::from_str(raw).expect("parse");
        let doc_b: PenDocument = serde_json::from_str(raw).expect("parse");

        let mut clone_a = doc_a;
        let report = detect_and_fix(&mut clone_a);

        let mut clone_b = doc_b;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        assert_eq!(report.total as usize, applied);
        assert_eq!(plan.len(), applied);
        assert_eq!(clone_a, clone_b);

        // The fit-text node with explicit `height: "fit_content"` must NOT appear
        // (it already has the keyword, no issue is raised for it).
        let fit_in_plan = plan.iter().any(|f| f.node_id == "fit-text");
        assert!(!fit_in_plan, "fit-text has no issue; must not appear in plan");
    }

    /// Load the `stacked-horizontal-padding` fixture and assert equivalence.
    ///
    /// This fixture exercises an `Info`-severity `StackedHorizontalPadding`
    /// issue: the entire plan should be empty (detect_and_fix applies zero
    /// fixes) and both docs remain unchanged.
    #[test]
    fn equivalence_stacked_horizontal_padding() {
        let raw = include_str!(
            "../tests/fixtures/docs/stacked-horizontal-padding.json"
        );
        let doc_a: PenDocument = serde_json::from_str(raw).expect("parse");
        let doc_b: PenDocument = serde_json::from_str(raw).expect("parse");

        let mut clone_a = doc_a;
        let report = detect_and_fix(&mut clone_a);

        let mut clone_b = doc_b;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        // All issues are Info severity — no fix is applied.
        assert_eq!(report.total, 0, "detect_and_fix must apply zero fixes");
        assert_eq!(plan.len(), 0, "plan must be empty for all-Info fixtures");
        assert_eq!(applied, 0);
        assert_eq!(clone_a, clone_b);
    }

    /// Smoke-test: a `Remove`-action fix (empty-path fixture).
    #[test]
    fn equivalence_empty_path_remove() {
        let d = doc(serde_json::json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "path", "id": "empty"}
            ]}]
        }));

        let mut clone_a = d.clone();
        let report = detect_and_fix(&mut clone_a);

        let mut clone_b = d;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        assert_eq!(report.total as usize, applied);
        assert_eq!(clone_a, clone_b);
    }

    /// The `status-bar` role guard: a `Remove`-action fix on a status-bar node
    /// must NOT appear in the plan (mirrors the guard in `apply_remove`).
    #[test]
    fn equivalence_remove_status_bar_skipped() {
        let d = doc(serde_json::json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "frame", "id": "bar", "role": "status-bar"}
            ]}]
        }));

        let mut clone_a = d.clone();
        let report = detect_and_fix(&mut clone_a);

        let mut clone_b = d;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        assert_eq!(report.total, 0, "status-bar must not be removed");
        assert_eq!(plan.len(), 0, "status-bar must not appear in the plan");
        assert_eq!(applied, 0);
        assert_eq!(clone_a, clone_b);
    }

    /// The `UnexpectedRotation` fix (Rotation property).
    #[test]
    fn equivalence_unexpected_rotation() {
        let raw =
            include_str!("../tests/fixtures/docs/unexpected-rotation.json");
        let doc_a: PenDocument = serde_json::from_str(raw).expect("parse");
        let doc_b: PenDocument = serde_json::from_str(raw).expect("parse");

        let mut clone_a = doc_a;
        let report = detect_and_fix(&mut clone_a);

        let mut clone_b = doc_b;
        let plan = detect_and_plan(&clone_b);
        let applied: usize = plan
            .iter()
            .filter(|fix| apply_planned_fix_to_doc(&mut clone_b, fix))
            .count();

        assert_eq!(report.total as usize, applied);
        assert_eq!(clone_a, clone_b);
    }
}
