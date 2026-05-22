//! Append-to-document mode — `apply_append_context_to_plan`.
//!
//! Faithful port of `apps/web/src/services/ai/orchestrator-append.ts`.
//! Mutates an [`OrchestratorPlan`] in-place when an [`AppendContext`] is
//! present; returns an [`AppendPlanResult`] that `run.rs` uses to skip
//! root-frame insertion and status-bar scaffold (Task B2).

#![allow(dead_code)] // callers land in Task B2

use crate::plan::OrchestratorPlan;
use crate::types::AppendContext;

/// Result of applying append context to a plan.
///
/// Port of the three-field return in `orchestrator-append.ts`.
/// `plan` is mutated in-place; these flags tell `run.rs` what to skip.
pub(crate) struct AppendPlanResult {
    pub skip_root_insertion: bool,
    pub skip_status_bar: bool,
}

/// Returns `true` when `id + " " + label` (lowercased) matches any of the
/// status-bar keyword alternatives.
///
/// Port of `STATUS_BAR_SUBTASK_RE = /(status\s*bar|status_bar|status-bar|system chrome|系统栏|状态栏)/i`.
/// We avoid the `regex` crate: the only non-literal in the original is `\s*`
/// between "status" and "bar".  We cover that with two literals:
/// `"status bar"` (single space) and `"statusbar"` (zero spaces).
fn is_status_bar_subtask(id: &str, label: &str) -> bool {
    let haystack = format!("{id} {label}").to_lowercase();
    haystack.contains("status bar")
        || haystack.contains("statusbar")
        || haystack.contains("status_bar")
        || haystack.contains("status-bar")
        || haystack.contains("system chrome")
        || haystack.contains("系统栏")
        || haystack.contains("状态栏")
}

/// Mutates `plan` according to `append` and returns skip-flags for `run.rs`.
///
/// - `None` → returns `{ false, false }`, plan unmutated.
/// - `Some(a)`:
///   - `plan.root_frame.id` ← `a.target_parent_id`
///   - `plan.root_frame.width` ← `a.target_width`
///   - status-bar subtasks filtered out
///   - every remaining subtask's `existing_section_labels` ← `Some(a.existing_section_labels.clone())`
///   - returns `{ skip_root_insertion: true, skip_status_bar: true }`
///
/// Port of `applyAppendContextToPlan` in `orchestrator-append.ts`.
pub(crate) fn apply_append_context_to_plan(
    plan: &mut OrchestratorPlan,
    append: Option<&AppendContext>,
) -> AppendPlanResult {
    let Some(a) = append else {
        return AppendPlanResult {
            skip_root_insertion: false,
            skip_status_bar: false,
        };
    };

    plan.root_frame.id = a.target_parent_id.clone();
    plan.root_frame.width = a.target_width;
    plan.subtasks
        .retain(|st| !is_status_bar_subtask(&st.id, &st.label));
    for st in &mut plan.subtasks {
        st.existing_section_labels = Some(a.existing_section_labels.clone());
    }

    AppendPlanResult {
        skip_root_insertion: true,
        skip_status_bar: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

    fn make_plan(subtask_ids_labels: &[(&str, &str)]) -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "original-root".into(),
                name: "Design".into(),
                width: 1200.0,
                height: 800.0,
                layout: Some("vertical".into()),
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: subtask_ids_labels
                .iter()
                .map(|(id, label)| Subtask {
                    id: id.to_string(),
                    label: label.to_string(),
                    region: Region {
                        width: 1200.0,
                        height: 400.0,
                    },
                    id_prefix: id.to_string(),
                    parent_frame_id: None,
                    elements: None,
                    screen: None,
                    generated_root_id: None,
                    existing_section_labels: None,
                })
                .collect(),
            style_guide_name: None,
        }
    }

    fn make_ctx(id: &str, width: f64, labels: &[&str]) -> AppendContext {
        AppendContext {
            target_parent_id: id.into(),
            target_width: width,
            existing_section_labels: labels.iter().map(|s| s.to_string()).collect(),
            is_mobile: false,
        }
    }

    // ── None case ─────────────────────────────────────────────────────────────

    /// None → returns { false, false }, plan unmutated.
    #[test]
    fn none_returns_false_flags_and_no_mutation() {
        let mut plan = make_plan(&[("hero", "Hero"), ("features", "Features")]);
        let original_id = plan.root_frame.id.clone();
        let original_width = plan.root_frame.width;
        let original_subtask_count = plan.subtasks.len();

        let result = apply_append_context_to_plan(&mut plan, None);

        assert!(!result.skip_root_insertion);
        assert!(!result.skip_status_bar);
        assert_eq!(plan.root_frame.id, original_id, "root id must be unmutated");
        assert_eq!(
            plan.root_frame.width, original_width,
            "root width must be unmutated"
        );
        assert_eq!(
            plan.subtasks.len(),
            original_subtask_count,
            "subtask count must be unmutated"
        );
        assert!(
            plan.subtasks
                .iter()
                .all(|st| st.existing_section_labels.is_none()),
            "existing_section_labels must remain None"
        );
    }

    // ── Some case: basic mutation ──────────────────────────────────────────────

    /// Some(ctx) → returns { true, true }, root_frame repointed, labels propagated.
    #[test]
    fn some_returns_true_flags_and_mutates_plan() {
        let mut plan = make_plan(&[("hero", "Hero"), ("pricing", "Pricing")]);
        let ctx = make_ctx("existing-frame-abc", 390.0, &["Hero", "About"]);

        let result = apply_append_context_to_plan(&mut plan, Some(&ctx));

        assert!(result.skip_root_insertion);
        assert!(result.skip_status_bar);
        assert_eq!(plan.root_frame.id, "existing-frame-abc");
        assert_eq!(plan.root_frame.width, 390.0);
    }

    /// Each remaining subtask gets existing_section_labels = Some(ctx labels).
    #[test]
    fn some_propagates_existing_section_labels_to_all_remaining_subtasks() {
        let mut plan = make_plan(&[("hero", "Hero"), ("pricing", "Pricing")]);
        let ctx = make_ctx("frame-1", 1200.0, &["Hero", "Pricing"]);

        apply_append_context_to_plan(&mut plan, Some(&ctx));

        for st in &plan.subtasks {
            let labels = st.existing_section_labels.as_ref().expect("labels set");
            assert_eq!(labels, &vec!["Hero", "Pricing"]);
        }
    }

    // ── Status-bar filter ─────────────────────────────────────────────────────

    /// "status-bar" in id is matched and filtered.
    #[test]
    fn status_bar_subtask_filtered_by_id_hyphen() {
        let mut plan = make_plan(&[("status-bar", "Top Area"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    /// "Status Bar" in label (case-insensitive) is matched and filtered.
    #[test]
    fn status_bar_subtask_filtered_by_label_case_insensitive() {
        let mut plan = make_plan(&[("chrome", "Status Bar"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    /// CJK 状态栏 in label is matched.
    #[test]
    fn status_bar_subtask_filtered_by_cjk_zhuangtailan() {
        let mut plan = make_plan(&[("chrome", "状态栏"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    /// CJK 系统栏 in label is matched.
    #[test]
    fn status_bar_subtask_filtered_by_cjk_xitonglan() {
        let mut plan = make_plan(&[("chrome", "系统栏"), ("content", "Content")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "content");
    }

    /// "system chrome" in label is matched.
    #[test]
    fn status_bar_subtask_filtered_by_system_chrome() {
        let mut plan = make_plan(&[("top-bar", "System Chrome"), ("footer", "Footer")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "footer");
    }

    /// "statusbar" (zero space) in label is matched (covers `\s*` zero-space case).
    #[test]
    fn status_bar_subtask_filtered_by_statusbar_no_space() {
        let mut plan = make_plan(&[("s", "statusbar"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    /// "status_bar" (underscore) in id is matched.
    #[test]
    fn status_bar_subtask_filtered_by_underscore() {
        let mut plan = make_plan(&[("status_bar", "Chrome"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].id, "hero");
    }

    /// "Status Mention" should NOT match (no false positive).
    #[test]
    fn status_mention_is_not_filtered() {
        let mut plan = make_plan(&[("status-mention", "Status Mention"), ("hero", "Hero")]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        // Neither subtask matches the status-bar pattern
        assert_eq!(plan.subtasks.len(), 2);
    }

    /// Multiple status-bar subtasks all get filtered.
    #[test]
    fn multiple_status_bar_subtasks_all_filtered() {
        let mut plan = make_plan(&[
            ("status-bar", "Status Bar"),
            ("hero", "Hero"),
            ("system-chrome", "System Chrome"),
            ("footer", "Footer"),
        ]);
        let ctx = make_ctx("frame-1", 390.0, &[]);
        apply_append_context_to_plan(&mut plan, Some(&ctx));
        assert_eq!(plan.subtasks.len(), 2);
        let ids: Vec<_> = plan.subtasks.iter().map(|st| st.id.as_str()).collect();
        assert!(ids.contains(&"hero"));
        assert!(ids.contains(&"footer"));
    }

    // ── Full integration: all mutations together ───────────────────────────────

    /// Full append scenario: root repointed, status-bar filtered, labels propagated.
    #[test]
    fn full_append_scenario() {
        let mut plan = make_plan(&[
            ("status-bar", "Status Bar"),
            ("hero", "Hero"),
            ("pricing", "Pricing"),
        ]);
        let ctx = make_ctx("target-frame-xyz", 390.0, &["Nav", "Hero"]);

        let result = apply_append_context_to_plan(&mut plan, Some(&ctx));

        assert!(result.skip_root_insertion);
        assert!(result.skip_status_bar);
        assert_eq!(plan.root_frame.id, "target-frame-xyz");
        assert_eq!(plan.root_frame.width, 390.0);
        // status-bar filtered out; 2 remaining
        assert_eq!(plan.subtasks.len(), 2);
        let ids: Vec<_> = plan.subtasks.iter().map(|st| st.id.as_str()).collect();
        assert!(ids.contains(&"hero"));
        assert!(ids.contains(&"pricing"));
        // labels propagated to all remaining
        for st in &plan.subtasks {
            let labels = st.existing_section_labels.as_ref().unwrap();
            assert_eq!(labels, &vec!["Nav", "Hero"]);
        }
    }
}
