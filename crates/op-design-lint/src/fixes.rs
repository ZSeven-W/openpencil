//! Auto-fix application — port of TS `applyFixes`
//! (`apps/web/src/services/ai/design-pre-validation.ts`).

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

use crate::issue::{FixProperty, FixReport, Issue, IssueSeverity};
use crate::node_mut;
use crate::node_util;
use serde_json::Value;

/// Apply one issue's suggested value to a node. Returns whether a field
/// was written. `Remove` is handled by the caller (`apply_fixes`), not here.
fn set_property(node: &mut PenNode, property: FixProperty, suggested: &Value) -> bool {
    match property {
        FixProperty::Height => {
            // text-explicit-height → "fit_content"
            if suggested.as_str() == Some("fit_content") {
                node_mut::set_text_height_fit_content(node)
            } else {
                false
            }
        }
        FixProperty::Rotation => match suggested.as_f64() {
            Some(v) => node_mut::set_rotation(node, v),
            None => false,
        },
        FixProperty::CornerRadius => match suggested.as_f64() {
            Some(v) => node_mut::set_corner_radius(node, v as f32),
            None => false,
        },
        FixProperty::FontSize => match suggested.as_f64() {
            Some(v) => node_mut::set_font_size(node, v as f32),
            None => false,
        },
        FixProperty::Effects => {
            // text-effect / excessive-frame-effects → clear
            node_mut::clear_effects(node)
        }
        FixProperty::Padding => set_padding(node, suggested),
        FixProperty::Stroke => node_mut::set_stroke_from_json(node, suggested),
        // text-bg-contrast is Info severity — never reaches apply_fixes.
        FixProperty::Fill => false,
        // widget-a11y is detect-only — the label must be authored, no auto-fix.
        FixProperty::Label => false,
        // Remove is handled in apply_fixes, never dispatched here.
        FixProperty::Remove => false,
    }
}

/// `Padding` suggested values are a number or a 4-element array.
fn set_padding(node: &mut PenNode, suggested: &Value) -> bool {
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

/// Apply auto-fixable issues to the document in place. Mirrors TS
/// `applyFixes` (`design-pre-validation.ts:47-83`): skips `Info` issues,
/// skips `Remove` on a `status-bar`-role node (protected pre-injected
/// chrome), counts only fixes ACTUALLY applied.
pub fn apply_fixes(doc: &mut PenDocument, issues: &[Issue]) -> FixReport {
    let mut report = FixReport::default();
    for issue in issues {
        if issue.severity == IssueSeverity::Info {
            continue;
        }
        let applied = if issue.property == FixProperty::Remove {
            apply_remove(doc, &issue.node_id)
        } else {
            apply_set(doc, &issue.node_id, issue.property, &issue.suggested_value)
        };
        if applied {
            report.total += 1;
            *report.by_category.entry(issue.category).or_insert(0) += 1;
        }
    }
    report
}

/// Detect over the active page, then apply. ← TS `runPreValidationFixes`.
pub fn detect_and_fix(doc: &mut PenDocument) -> FixReport {
    let issues = {
        let root = match active_page_root(doc) {
            Some(r) => r,
            None => return FixReport::default(),
        };
        crate::detectors::detect_all(root, doc)
    };
    apply_fixes(doc, &issues)
}

/// Locate the `Remove` target across the active page. If the node's `role`
/// is `status-bar` it is protected pre-injected chrome — return `false`
/// (not removed, not counted). Otherwise detach it from its parent.
fn apply_remove(doc: &mut PenDocument, node_id: &str) -> bool {
    // Protected-chrome check: find the node read-only and inspect its role.
    if let Some(node) = active_page_roots(doc)
        .iter()
        .find_map(|root| find_node(root, node_id))
    {
        if node_util::role(node) == Some("status-bar") {
            return false;
        }
    } else {
        return false;
    }
    // Detach from whichever active-page root contains it.
    for root in active_page_roots_mut(doc) {
        if node_mut::detach_child(root, node_id) {
            return true;
        }
    }
    false
}

/// Locate a node mutably across the active page and apply `set_property`.
fn apply_set(
    doc: &mut PenDocument,
    node_id: &str,
    property: FixProperty,
    suggested: &Value,
) -> bool {
    for root in active_page_roots_mut(doc) {
        if let Some(node) = find_node_mut(root, node_id) {
            return set_property(node, property, suggested);
        }
    }
    false
}

/// The active page's top-level node list. The detectors walk a single root
/// `PenNode`; a canonical document's active page is `pages[0]` when `pages`
/// is set and non-empty, else the doc's own `children` (legacy single-page
/// shape). Mirrors how the TS store keeps one page of children.
fn active_page_nodes(doc: &PenDocument) -> &[PenNode] {
    match doc.pages.as_ref().and_then(|pages| pages.first()) {
        Some(page) => &page.children,
        None => &doc.children,
    }
}

fn active_page_nodes_mut(doc: &mut PenDocument) -> &mut Vec<PenNode> {
    let has_page = doc
        .pages
        .as_ref()
        .map(|pages| !pages.is_empty())
        .unwrap_or(false);
    if has_page {
        &mut doc.pages.as_mut().unwrap()[0].children
    } else {
        &mut doc.children
    }
}

/// The detector root — the first node of the active page (the root frame).
fn active_page_root(doc: &PenDocument) -> Option<&PenNode> {
    active_page_nodes(doc).first()
}

/// Every top-level node of the active page (`apply_remove` / `apply_set`
/// search each so a multi-root page is covered, not just `children[0]`).
fn active_page_roots(doc: &PenDocument) -> &[PenNode] {
    active_page_nodes(doc)
}

fn active_page_roots_mut(doc: &mut PenDocument) -> &mut [PenNode] {
    active_page_nodes_mut(doc).as_mut_slice()
}

/// Depth-first search for a node by id, including `root` itself.
fn find_node<'a>(root: &'a PenNode, node_id: &str) -> Option<&'a PenNode> {
    if node_util::node_id(root) == node_id {
        return Some(root);
    }
    node_util::children(root)
        .iter()
        .find_map(|child| find_node(child, node_id))
}

/// Mutable depth-first search for a node by id, including `root` itself.
fn find_node_mut<'a>(root: &'a mut PenNode, node_id: &str) -> Option<&'a mut PenNode> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::IssueCategory;
    use jian_ops_schema::node::{CornerRadius, Padding};
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn set_property_height_fit_content_on_text() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi", "height": 40}));
        assert!(set_property(
            &mut n,
            FixProperty::Height,
            &json!("fit_content")
        ));
        match n {
            PenNode::Text(t) => assert!(matches!(
                t.height,
                Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
            )),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn set_property_height_rejects_non_fit_content() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi"}));
        assert!(!set_property(&mut n, FixProperty::Height, &json!(40)));
    }

    #[test]
    fn set_property_rotation_zero_on_frame() {
        let mut n = node(json!({"type": "frame", "id": "f", "rotation": 12}));
        assert!(set_property(&mut n, FixProperty::Rotation, &json!(0)));
        match n {
            PenNode::Frame(f) => assert_eq!(f.base.rotation, Some(0.0)),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_corner_radius_on_frame() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_property(&mut n, FixProperty::CornerRadius, &json!(8)));
        match n {
            PenNode::Frame(f) => assert!(matches!(
                f.container.corner_radius,
                Some(CornerRadius::Uniform(r)) if r == 8.0
            )),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_effects_null_clears_frame_effects() {
        let mut n = node(json!({
            "type": "frame", "id": "f",
            "effects": [{"type": "blur", "radius": 4}]
        }));
        assert!(set_property(&mut n, FixProperty::Effects, &Value::Null));
        match n {
            PenNode::Frame(f) => assert!(f.container.effects.is_none()),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_padding_uniform() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_property(&mut n, FixProperty::Padding, &json!(16)));
        match n {
            PenNode::Frame(f) => {
                assert!(matches!(f.container.padding, Some(Padding::Uniform(p)) if p == 16.0))
            }
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_padding_ltrb_array() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_property(
            &mut n,
            FixProperty::Padding,
            &json!([0, 16, 0, 16])
        ));
        match n {
            PenNode::Frame(f) => assert!(matches!(
                f.container.padding,
                Some(Padding::LtrB([0.0, 16.0, 0.0, 16.0]))
            )),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_stroke_object_on_frame() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        let suggested = json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]});
        assert!(set_property(&mut n, FixProperty::Stroke, &suggested));
        match n {
            PenNode::Frame(f) => assert!(f.container.stroke.is_some()),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_property_font_size_on_text() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi", "fontSize": 24}));
        assert!(set_property(&mut n, FixProperty::FontSize, &json!(14)));
        match n {
            PenNode::Text(t) => assert_eq!(t.font_size, Some(14.0)),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn set_property_fill_is_noop() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi"}));
        assert!(!set_property(&mut n, FixProperty::Fill, &json!("#000000")));
    }

    #[test]
    fn set_property_remove_is_noop() {
        let mut n = node(json!({"type": "path", "id": "p"}));
        assert!(!set_property(&mut n, FixProperty::Remove, &Value::Null));
    }

    fn doc(value: serde_json::Value) -> PenDocument {
        serde_json::from_value(value).expect("fixture must deserialize as PenDocument")
    }

    fn issue(
        node_id: &str,
        category: IssueCategory,
        severity: IssueSeverity,
        property: FixProperty,
        suggested: Value,
    ) -> Issue {
        Issue {
            node_id: node_id.into(),
            category,
            severity,
            property,
            current_value: Value::Null,
            suggested_value: suggested,
            reason: "test".into(),
        }
    }

    #[test]
    fn apply_fixes_skips_info_severity_issues() {
        let mut d = doc(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "text", "id": "t", "content": "hi"}
            ]}]
        }));
        let issues = vec![issue(
            "t",
            IssueCategory::TextBgContrast,
            IssueSeverity::Info,
            FixProperty::Fill,
            json!("#000000"),
        )];
        let report = apply_fixes(&mut d, &issues);
        assert_eq!(report.total, 0);
    }

    #[test]
    fn apply_fixes_skips_remove_on_status_bar_role() {
        let mut d = doc(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "frame", "id": "bar", "role": "status-bar"}
            ]}]
        }));
        let issues = vec![issue(
            "bar",
            IssueCategory::EmptyPath,
            IssueSeverity::Warning,
            FixProperty::Remove,
            Value::Null,
        )];
        let report = apply_fixes(&mut d, &issues);
        assert_eq!(report.total, 0);
        // The protected node is still present.
        let root = active_page_root(&d).unwrap();
        assert_eq!(node_util::children(root).len(), 1);
    }

    #[test]
    fn apply_fixes_removes_ordinary_empty_path() {
        let mut d = doc(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "path", "id": "empty"}
            ]}]
        }));
        let issues = vec![issue(
            "empty",
            IssueCategory::EmptyPath,
            IssueSeverity::Warning,
            FixProperty::Remove,
            Value::Null,
        )];
        let report = apply_fixes(&mut d, &issues);
        assert_eq!(report.total, 1);
        assert_eq!(report.by_category.get(&IssueCategory::EmptyPath), Some(&1));
        let root = active_page_root(&d).unwrap();
        assert_eq!(node_util::children(root).len(), 0);
    }

    #[test]
    fn apply_fixes_clears_rotation() {
        let mut d = doc(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "frame", "id": "tilted", "rotation": 12}
            ]}]
        }));
        let issues = vec![issue(
            "tilted",
            IssueCategory::UnexpectedRotation,
            IssueSeverity::Warning,
            FixProperty::Rotation,
            json!(0),
        )];
        let report = apply_fixes(&mut d, &issues);
        assert_eq!(report.total, 1);
        let root = active_page_root(&d).unwrap();
        match &node_util::children(root)[0] {
            PenNode::Frame(f) => assert_eq!(f.base.rotation, Some(0.0)),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn detect_and_fix_detects_then_applies() {
        let mut d = doc(json!({
            "version": "1.0",
            "children": [{"type": "frame", "id": "root", "children": [
                {"type": "frame", "id": "tilted", "rotation": 12}
            ]}]
        }));
        let report = detect_and_fix(&mut d);
        assert_eq!(report.total, 1);
        let root = active_page_root(&d).unwrap();
        match &node_util::children(root)[0] {
            PenNode::Frame(f) => assert_eq!(node_util::rotation(&PenNode::Frame(f.clone())), 0.0),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn detect_and_fix_uses_first_page_when_pages_present() {
        let mut d = doc(json!({
            "version": "1.0",
            "pages": [{"id": "p1", "name": "Page 1", "children": [
                {"type": "frame", "id": "root", "children": [
                    {"type": "frame", "id": "tilted", "rotation": 30}
                ]}
            ]}],
            "children": []
        }));
        let report = detect_and_fix(&mut d);
        assert_eq!(report.total, 1);
    }

    #[test]
    fn detect_and_fix_on_empty_doc_returns_default() {
        let mut d = doc(json!({"version": "1.0", "children": []}));
        let report = detect_and_fix(&mut d);
        assert_eq!(report.total, 0);
    }
}
