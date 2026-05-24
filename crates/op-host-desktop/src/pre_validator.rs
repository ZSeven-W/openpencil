//! D′ architecture host adapter — translates `PlannedFix → EditorCommand`.
//!
//! `LintPreValidator` implements `op_orchestrator::PreValidator` by:
//!
//! 1. Cloning the current document from `sink.state().doc`.
//! 2. Calling `op_design_lint::detect_and_plan` to get a `Vec<PlannedFix>`.
//! 3. Translating each `PlannedFix` to one or more `EditorCommand`s via
//!    [`planned_fix_to_commands`].
//! 4. Applying each command via `sink.apply` and counting successes.
//!
//! ## FixProperty → EditorCommand translation table
//!
//! | `PlannedAction`         | `EditorCommand`(s)                                      |
//! |------------------------|----------------------------------------------------------|
//! | `Remove`               | `DeleteNode { node_id }`                                |
//! | `SetHeightFitContent`  | `SetNodeLayoutProp { property:"height", Keyword("fit_content") }` |
//! | `SetRotation(deg)`     | `SetNodeRotation { degrees }`                           |
//! | `SetCornerRadius(r)`   | `SetNodeCornerRadius { radius }`                        |
//! | `SetFontSize(px)`      | `SetNodeFontSize { font_size }`                         |
//! | `ClearEffects`         | N × `RemoveNodeEffect { index }` (highest→lowest)       |
//! | `SetPadding(v)`        | `SetNodeLayoutProp { property:"padding", NumberArray/Number }` |
//! | `SetStroke(v)`         | `SetNodeStrokeHex { hex }` + `SetNodeStrokeWidth { width }` |
//!
//! ### `ClearEffects`
//! `EditorCommand` has no "clear all effects" variant — the adapter reads the
//! current effect count from `sink.state()` and emits `RemoveNodeEffect` for
//! each index, iterating highest-to-lowest so index shifts don't affect earlier
//! removals. After all removals the effects field is `None`, matching
//! `op_design_lint::node_mut::clear_effects`.
//!
//! ### `SetStroke`
//! `EditorCommand` has no "set full PenStroke object" variant. The adapter
//! decomposes the JSON object into:
//! - `SetNodeStrokeHex { hex }` — extracts the first solid-fill color from
//!   the suggested value's `fill[0].color`; creates a fresh 1px stroke if
//!   the node has none.
//! - `SetNodeStrokeWidth { width }` — extracts `thickness` as `f32`.
//!
//! The resulting `PenStroke` is structurally identical to what
//! `node_mut::set_stroke_from_json` produces for the current fixture set
//! (all invisible-container stroke fixes use `{thickness:1, fill:[{type:solid,
//! color:"#E2E8F0"}]}`). This is verified by the parity integration test.
//! If a future detector emits a stroke with non-solid fills or per-side
//! thickness, extend the translation or report a gap.

use op_design_lint::plan::{PlannedAction, PlannedFix};
use op_editor_core::{EditorCommand, EditorState};
use op_orchestrator::{DocSink, PreValidationResult, PreValidator};

/// Real `PreValidator` implementation backed by `op-design-lint`.
///
/// Swap `SkippedPreValidator` for this in `chat_orchestrator.rs` to
/// enable pre-validation fixes on every design generation run.
pub struct LintPreValidator;

impl PreValidator for LintPreValidator {
    fn run_pre_validation_fixes(&self, sink: &mut dyn DocSink) -> PreValidationResult {
        // Clone the document snapshot so detect_and_plan can read it
        // immutably while sink remains available for apply calls.
        let doc_clone = sink.state().doc.clone();
        let plan = op_design_lint::detect_and_plan(&doc_clone);

        let mut total = 0usize;
        let mut by_category: std::collections::BTreeMap<String, usize> = Default::default();

        for fix in &plan {
            let commands = planned_fix_to_commands(fix, sink.state());
            // Count this fix as applied only if at least one command lands.
            let mut any_applied = false;
            for cmd in commands {
                if sink.apply(cmd) {
                    any_applied = true;
                }
            }
            if any_applied {
                total += 1;
                let key = serde_json::to_value(fix.category)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| format!("{:?}", fix.category));
                *by_category.entry(key).or_insert(0) += 1;
            }
        }

        PreValidationResult { total, by_category }
    }
}

/// Translate one `PlannedFix` into the `EditorCommand`(s) needed to apply it.
/// Returns an empty vec only for configurations where no command is appropriate
/// (e.g. a `SetStroke` whose JSON has no extractable hex — should not happen
/// in practice since `detect_and_plan` only emits valid suggested values).
fn planned_fix_to_commands(fix: &PlannedFix, state: &EditorState) -> Vec<EditorCommand> {
    use op_editor_core::node_id::NodeId;

    let node_id = NodeId::new(&*fix.node_id);
    let mut cmds = Vec::new();

    match &fix.action {
        PlannedAction::Remove => {
            cmds.push(EditorCommand::DeleteNode {
                node_id: node_id.clone(),
            });
        }

        PlannedAction::SetHeightFitContent => {
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: node_id.clone(),
                property: "height".to_string(),
                value: op_editor_core::LayoutPropValue::Keyword("fit_content".to_string()),
            });
        }

        PlannedAction::SetRotation(deg) => {
            cmds.push(EditorCommand::SetNodeRotation {
                node_id: node_id.clone(),
                degrees: *deg as f32,
            });
        }

        PlannedAction::SetCornerRadius(r) => {
            cmds.push(EditorCommand::SetNodeCornerRadius {
                node_id: node_id.clone(),
                radius: *r,
            });
        }

        PlannedAction::SetFontSize(px) => {
            cmds.push(EditorCommand::SetNodeFontSize {
                node_id: node_id.clone(),
                font_size: *px,
            });
        }

        PlannedAction::ClearEffects => {
            // Read the current effect count from the state, then emit
            // RemoveNodeEffect for each index from highest to lowest so
            // earlier indices are not invalidated by the removal.
            let effect_count = effect_count_for_node(state, &node_id);
            for i in (0..effect_count).rev() {
                cmds.push(EditorCommand::RemoveNodeEffect {
                    node_id: node_id.clone(),
                    index: i as u32,
                });
            }
        }

        PlannedAction::SetPadding(v) => {
            // Translate to SetNodeLayoutProp "padding" with Number or NumberArray.
            let prop_value = if let Some(n) = v.as_f64() {
                Some(op_editor_core::LayoutPropValue::Number(n))
            } else if let Some(arr) = v.as_array() {
                let nums: Option<Vec<f64>> = arr.iter().map(|x| x.as_f64()).collect();
                nums.map(op_editor_core::LayoutPropValue::NumberArray)
            } else {
                None
            };
            if let Some(pv) = prop_value {
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: node_id.clone(),
                    property: "padding".to_string(),
                    value: pv,
                });
            }
        }

        PlannedAction::SetStroke(v) => {
            // Decompose the PenStroke JSON into SetNodeStrokeHex + SetNodeStrokeWidth.
            // Both commands are emitted; the hex command creates a fresh stroke if needed.
            //
            // Extract hex from fill[0].color (first solid fill entry).
            let hex = v
                .get("fill")
                .and_then(|f| f.as_array())
                .and_then(|arr| arr.first())
                .and_then(|entry| entry.get("color"))
                .and_then(|c| c.as_str())
                .map(str::to_string);

            // Extract thickness as f32.
            let thickness = v
                .get("thickness")
                .and_then(|t| t.as_f64())
                .map(|t| t as f32);

            if let Some(hex) = hex {
                cmds.push(EditorCommand::SetNodeStrokeHex {
                    node_id: node_id.clone(),
                    hex,
                });
            }
            if let Some(width) = thickness {
                cmds.push(EditorCommand::SetNodeStrokeWidth {
                    node_id: node_id.clone(),
                    width,
                });
            }
        }
    }

    cmds
}

/// Count the number of effects on the named node in the active page.
/// Returns 0 if the node is not found or has no effects.
fn effect_count_for_node(state: &EditorState, node_id: &op_editor_core::node_id::NodeId) -> usize {
    use op_editor_core::walkers::find_node;

    let children = state.active_children();
    let Some(node) = find_node(children, node_id) else {
        return 0;
    };
    effects_from_pen_node(node).len()
}

/// Extract the effect list from a `PenNode`, returning an empty slice
/// when the node kind has no effects or effects is `None`.
fn effects_from_pen_node(
    node: &jian_ops_schema::node::PenNode,
) -> &[jian_ops_schema::style::PenEffect] {
    use jian_ops_schema::node::PenNode;
    let opt = match node {
        PenNode::Frame(n) => n.container.effects.as_deref(),
        PenNode::Group(n) => n.container.effects.as_deref(),
        PenNode::Rectangle(n) => n.container.effects.as_deref(),
        PenNode::Ellipse(n) => n.effects.as_deref(),
        PenNode::Polygon(n) => n.effects.as_deref(),
        PenNode::Path(n) => n.effects.as_deref(),
        PenNode::Line(n) => n.effects.as_deref(),
        PenNode::Text(n) => n.effects.as_deref(),
        PenNode::TextInput(n) => n.effects.as_deref(),
        PenNode::Image(n) => n.effects.as_deref(),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    };
    opt.unwrap_or(&[])
}

// ── Parity integration tests ─────────────────────────────────────────────────
//
// For each of the same 3 fixtures used in `op-design-lint`'s equivalence tests:
// - Build an `EditorState` whose `doc` is the fixture's `PenDocument`.
// - Wrap it in a `TestSink`.
// - Call `LintPreValidator.run_pre_validation_fixes(&mut sink)`.
// - Compare `sink.state().doc` against the result of
//   `op_design_lint::detect_and_fix(&mut fixture_doc.clone())`.
// - Assert both produce structurally-identical `PenDocument`s.
//
// This is the LOAD-BEARING test that proves the cross-crate adapter doesn't
// silently lose mutations.

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::EditorState;
    use op_orchestrator::PreValidator;

    /// Minimal `DocSink` for tests — wraps an `EditorState`, applies
    /// commands through `EditorState::apply`. Modeled on
    /// `op-orchestrator::test_support::VecDocSink`.
    struct TestSink {
        state: EditorState,
    }

    impl TestSink {
        fn from_doc(doc: jian_ops_schema::PenDocument) -> Self {
            Self {
                state: EditorState::from_document(doc),
            }
        }
    }

    impl op_orchestrator::DocSink for TestSink {
        fn state(&self) -> &EditorState {
            &self.state
        }
        fn apply(&mut self, cmd: op_editor_core::EditorCommand) -> bool {
            self.state.apply(cmd)
        }
        fn begin_undo_batch(&mut self) {}
        fn end_undo_batch(&mut self) {}
    }

    fn load_doc(json: &str) -> jian_ops_schema::PenDocument {
        serde_json::from_str(json).expect("fixture must parse as PenDocument")
    }

    /// Normalize a `serde_json::Value` for structural comparison.
    ///
    /// `EditorState::find_node_mut` calls `children_mut()` which uses
    /// `get_or_insert_with(Vec::new)` — this side-effects frames that
    /// were traversed but not mutated from `children: None` →
    /// `children: Some([])`. The `detect_and_fix` path does NOT do
    /// this. We normalize away this difference by removing any JSON
    /// `"children"` key whose value is an empty array (`[]`).
    ///
    /// Every other field is compared exactly. This is the only known
    /// structural difference between the two paths.
    fn normalize_doc_json(v: serde_json::Value) -> serde_json::Value {
        use serde_json::Value;
        match v {
            Value::Object(mut map) => {
                // Remove "children": [] (empty array) — artifact of
                // EditorState walker's get_or_insert_with(Vec::new).
                if let Some(Value::Array(arr)) = map.get("children") {
                    if arr.is_empty() {
                        map.remove("children");
                    }
                }
                Value::Object(
                    map.into_iter()
                        .map(|(k, v)| (k, normalize_doc_json(v)))
                        .collect(),
                )
            }
            Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_doc_json).collect()),
            other => other,
        }
    }

    /// Assert that `LintPreValidator` + `EditorState` produces the same
    /// `PenDocument` as `op_design_lint::detect_and_fix` on the same input.
    ///
    /// Structural comparison uses JSON normalization to handle the known
    /// `children: None` vs `children: Some([])` difference introduced by
    /// `EditorState::find_node_mut`'s `get_or_insert_with(Vec::new)` side
    /// effect (see `normalize_doc_json`).
    fn assert_parity(fixture_json: &str, label: &str) {
        // Reference path: detect_and_fix on a raw clone.
        let mut reference_doc = load_doc(fixture_json);
        let reference_report = op_design_lint::detect_and_fix(&mut reference_doc);

        // Adapter path: LintPreValidator through EditorState.
        let adapter_doc = load_doc(fixture_json);
        let mut sink = TestSink::from_doc(adapter_doc);
        let result = LintPreValidator.run_pre_validation_fixes(&mut sink);

        assert_eq!(
            reference_report.total as usize, result.total,
            "{label}: applied counts must match (reference={}, adapter={})",
            reference_report.total, result.total
        );

        // Compare via normalized JSON to handle children: None vs Some([]).
        let ref_json = serde_json::to_value(&reference_doc).expect("serialize reference");
        let adapter_json = serde_json::to_value(&sink.state().doc).expect("serialize adapter");
        let ref_norm = normalize_doc_json(ref_json);
        let adapter_norm = normalize_doc_json(adapter_json);
        assert_eq!(
            ref_norm, adapter_norm,
            "{label}: documents must be structurally identical after both fix paths"
        );
    }

    /// invisible-container: exercises `FixProperty::Stroke` (Warning)
    /// and verifies Info-severity dark-on-dark is NOT applied.
    #[test]
    fn parity_invisible_container() {
        assert_parity(
            include_str!("../../op-design-lint/tests/fixtures/docs/invisible-container.json"),
            "invisible-container",
        );
    }

    /// text-explicit-height: exercises `FixProperty::Height` → `SetHeightFitContent`.
    #[test]
    fn parity_text_explicit_height() {
        assert_parity(
            include_str!("../../op-design-lint/tests/fixtures/docs/text-explicit-height.json"),
            "text-explicit-height",
        );
    }

    /// stacked-horizontal-padding: all Info severity — zero fixes applied;
    /// doc remains unchanged under both paths.
    #[test]
    fn parity_stacked_horizontal_padding() {
        assert_parity(
            include_str!(
                "../../op-design-lint/tests/fixtures/docs/stacked-horizontal-padding.json"
            ),
            "stacked-horizontal-padding",
        );
    }

    /// unexpected-rotation: exercises `FixProperty::Rotation` → `SetNodeRotation`.
    #[test]
    fn parity_unexpected_rotation() {
        assert_parity(
            include_str!("../../op-design-lint/tests/fixtures/docs/unexpected-rotation.json"),
            "unexpected-rotation",
        );
    }

    /// empty-path: exercises `FixProperty::Remove` → `DeleteNode`.
    #[test]
    fn parity_empty_path() {
        assert_parity(
            include_str!("../../op-design-lint/tests/fixtures/docs/empty-path.json"),
            "empty-path",
        );
    }

    /// invisible-container-with-var: exercises `$color-border` design-token
    /// reference path through `SetNodeStrokeHex`. Regression guard caught
    /// by stop-time review — `cmd_set_node_stroke_hex` previously rejected
    /// `$`-prefixed strings via `parse_hex_rgb`, dropping the color silently
    /// while `SetNodeStrokeWidth` still applied (so `any_applied=true` and
    /// `applied_count` overreported). The op-editor-core fix relaxed both
    /// `cmd_set_node_stroke_hex` + `cmd_set_node_fill_hex` to passthrough
    /// `$ref` strings (the underlying `color: String` field accepts them).
    #[test]
    fn parity_invisible_container_with_var() {
        assert_parity(
            include_str!(
                "../../op-design-lint/tests/fixtures/docs/invisible-container-with-var.json"
            ),
            "invisible-container-with-var",
        );
    }
}
