//! Dashboard sequential path — S3b-3 Task C3.
//!
//! Extracted from `run.rs` to keep it under the 800-line ceiling.
//! Called only when `should_use_dashboard_columns` returns `true` for a
//! sequential (non-concurrent) request.  The concurrent path never uses
//! the dashboard treatment (spec §2).

use crate::cleanup::{descendant_count, run_cleanup_passes};
use crate::dashboard_columns::{
    extract_sidebar_surface_color, is_sidebar_subtask, normalize_dashboard_main_subtasks,
    reorder_dashboard_main_children,
};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::OrchestratorPlan;
use crate::retry::is_non_retryable;
use crate::scaffold_dashboard::build_scaffold_dashboard;
use crate::subagent::{reveal_now_millis, run_subtask_with_reveal_at};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, LlmClient, OrchestratorError, Progress, RunSummary,
    SubtaskOutcome, ValidationProviders,
};
use crate::validation::run_post_generation_validation;
use crate::variables::{rollback, VarSnapshot};
use jian_ops_schema::node::PenNode;
use op_ai_skills::style_guide::style_guide_registry;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// Run the dashboard sequential path.
///
/// Entry conditions (checked by caller):
/// - `effective_concurrency <= 1` (sequential).
/// - `should_use_dashboard_columns` returned `true`.
/// - The undo batch is open and `var_snapshot` was taken.
/// - Variable seed commands have been applied.
/// - `scaffold_root_index` is the active-children count BEFORE inserting
///   the scaffold (so we can find the live root after InsertSubtree).
///
/// On `Err`, the undo batch is closed and variables are rolled back.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_dashboard_path(
    mut plan: OrchestratorPlan,
    request: DesignRequest,
    scaffold_root_index: usize,
    sink: &mut dyn DocSink,
    llm: &dyn LlmClient,
    var_snapshot: &VarSnapshot,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
    host_epoch: Option<u64>,
) -> Result<RunSummary, OrchestratorError> {
    let planned_root_id = plan.root_frame.id.clone();

    // Relabel "Main Content" subtask → "Top Bar", clamp its height.
    // Port of TS `normalizeDashboardMainSubtasks` call
    // (`orchestrator.ts:733-735`).
    normalize_dashboard_main_subtasks(&mut plan);

    // Resolve sidebar surface color (spec §4.4 precedence chain).
    let style_guide_content: Option<String> = plan.style_guide_name.as_deref().and_then(|name| {
        style_guide_registry()
            .iter()
            .find(|g| g.name == name)
            .map(|g| g.content.clone())
    });
    let sidebar_fill =
        extract_sidebar_surface_color(style_guide_content.as_deref(), request.design_md.as_ref())
            .or_else(|| plan.root_frame.first_solid_hex())
            .unwrap_or_else(|| "#0F172A".to_string());

    let (scaffold_cmds, logical_sidebar_id, logical_main_id, scaffold_baseline_count) =
        match build_scaffold_dashboard(&mut plan, &sidebar_fill) {
            Ok(r) => r,
            Err(e) => {
                rollback(sink, var_snapshot);
                sink.end_undo_batch();
                return Err(OrchestratorError::Internal(e));
            }
        };

    for cmd in scaffold_cmds {
        if !sink.apply(cmd) {
            rollback(sink, var_snapshot);
            sink.end_undo_batch();
            return Err(OrchestratorError::Internal(
                "dashboard scaffold insert rejected by document".into(),
            ));
        }
    }

    // Get the live root id (the child at `scaffold_root_index`).
    let Some(root_id) = sink
        .state()
        .active_children()
        .get(scaffold_root_index)
        .map(|n| n.id_str().to_string())
    else {
        rollback(sink, var_snapshot);
        sink.end_undo_batch();
        return Err(OrchestratorError::Internal(format!(
            "dashboard scaffold root `{planned_root_id}` was not inserted"
        )));
    };

    // -- Logical-to-live id resolution --
    //
    // `InsertSubtree` remaps every node id.  The logical ids
    // (`logical_sidebar_id`, `logical_main_id`, slot ids written by
    // `assign_dashboard_main_parents` into `plan.subtasks[*].parent_frame_id`)
    // are all stale after the apply.
    //
    // Strategy: walk the live root's subtree and build a `name → live_id` map.
    // Every synthesized frame has a distinct name (Sidebar / Main Content /
    // "{label} Slot"), so name-matching is unambiguous.
    let name_to_live: std::collections::HashMap<String, String> = sink
        .state()
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .map(collect_name_id_map)
        .unwrap_or_default();

    let live_sidebar_id = name_to_live
        .get("Sidebar")
        .cloned()
        .unwrap_or_else(|| logical_sidebar_id.clone());
    let live_main_id = name_to_live
        .get("Main Content")
        .cloned()
        .unwrap_or_else(|| logical_main_id.clone());

    // Update each subtask's parent_frame_id to the live id.
    // - Sidebar subtasks → live_sidebar_id.
    // - Others → their slot live id (looked up by slot name).
    for subtask in &mut plan.subtasks {
        if is_sidebar_subtask(subtask) {
            subtask.parent_frame_id = Some(live_sidebar_id.clone());
        } else {
            // Slot name = "{subtask.label} Slot" (from `make_slot_frame`).
            let slot_name = format!("{} Slot", subtask.label);
            let slot_live_id = name_to_live
                .get(&slot_name)
                .cloned()
                .unwrap_or_else(|| live_main_id.clone());
            subtask.parent_frame_id = Some(slot_live_id.clone());
            // Set generated_root_id = slot live id, used by
            // `reorder_dashboard_main_children` as fallback when
            // parent_frame_id resolution fails (port of TS
            // `subtask.generatedRootId` fallback at `orchestrator.ts:541`).
            subtask.generated_root_id = Some(slot_live_id);
        }
    }

    on_progress(Progress::ScaffoldDone);

    // -- 阶段 3 (dashboard): 顺序 sub-agent 循环 (same 3-attempt ladder) --
    let tier = resolve_model_profile(request.model.as_deref().unwrap_or("")).tier;
    let mut outcomes: Vec<SubtaskOutcome> = Vec::new();
    let mut aborted_mid = false;
    let mut zero_node_failure = false;
    for subtask in &plan.subtasks {
        if abort.is_set() {
            aborted_mid = true;
            break;
        }
        on_progress(Progress::SubtaskStarted {
            id: subtask.id.clone(),
            label: subtask.label.clone(),
        });

        let outcome1 = run_subtask_with_reveal_at(
            subtask,
            &plan,
            &request,
            llm,
            sink,
            abort,
            false,
            false,
            host_epoch,
            reveal_now_millis(),
        )
        .await;
        let non_retryable = outcome1
            .error
            .as_deref()
            .map(is_non_retryable)
            .unwrap_or(false);
        let retryable = |o: &SubtaskOutcome| {
            o.error.is_some() && o.node_count == 0 && !abort.is_set() && !non_retryable
        };
        let outcome2 = if retryable(&outcome1) {
            Some(
                run_subtask_with_reveal_at(
                    subtask,
                    &plan,
                    &request,
                    llm,
                    sink,
                    abort,
                    tier == ModelTier::Basic,
                    false,
                    host_epoch,
                    reveal_now_millis(),
                )
                .await,
            )
        } else {
            None
        };
        let outcome_after2 = outcome2.as_ref().unwrap_or(&outcome1);
        let outcome3 = if retryable(outcome_after2) {
            Some(
                run_subtask_with_reveal_at(
                    subtask,
                    &plan,
                    &request,
                    llm,
                    sink,
                    abort,
                    true,
                    true,
                    host_epoch,
                    reveal_now_millis(),
                )
                .await,
            )
        } else {
            None
        };
        let outcome = outcome3.unwrap_or_else(|| outcome2.unwrap_or(outcome1));

        let zero = outcome.node_count == 0;
        let node_count = outcome.node_count;
        let err_msg = outcome.error.clone();
        outcomes.push(outcome);

        if abort.is_set() {
            aborted_mid = true;
            if zero {
                on_progress(Progress::SubtaskFailed {
                    id: subtask.id.clone(),
                    error: err_msg.unwrap_or_else(|| "aborted".into()),
                });
            } else {
                on_progress(Progress::SubtaskDone {
                    id: subtask.id.clone(),
                    node_count,
                });
            }
            break;
        }
        if zero {
            on_progress(Progress::SubtaskFailed {
                id: subtask.id.clone(),
                error: err_msg.unwrap_or_default(),
            });
            zero_node_failure = true;
            break;
        }
        on_progress(Progress::SubtaskDone {
            id: subtask.id.clone(),
            node_count,
        });
    }

    // -- Post-loop: reorder main children to plan order --
    let reorder_cmds = reorder_dashboard_main_children(&plan, &live_main_id, sink.state());
    for cmd in reorder_cmds {
        sink.apply(cmd);
    }

    // -- 阶段 4 (dashboard): 清理 —— height-fit on BOTH columns --
    run_cleanup_passes(sink, &plan, &[&live_sidebar_id, &live_main_id]);
    on_progress(Progress::CleanupDone);

    // -- 阶段 4.5: 收尾 --
    let zero_content = descendant_count(sink.state(), &root_id) <= scaffold_baseline_count;
    if zero_content {
        if zero_node_failure {
            sink.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(root_id.clone()),
                page_id: None,
            });
        }
        rollback(sink, var_snapshot);
    }
    sink.end_undo_batch();

    if zero_content {
        return Err(if aborted_mid {
            OrchestratorError::Aborted
        } else {
            OrchestratorError::NoContent
        });
    }

    // -- 阶段 5 (dashboard):视觉校验 (S3c D1) --
    // Port of `orchestrator.ts:1247-1292`.
    // 守卫: request.validation_enabled && !abort.is_set().
    if request.validation_enabled && !abort.is_set() {
        let _ = run_post_generation_validation(
            sink,
            providers.pre_validator,
            providers.screenshot,
            providers.vision,
            &providers.system_prompt,
            &request,
            on_progress,
            abort,
        );
    }

    let total_nodes = outcomes.iter().map(|o| o.node_count).sum();
    Ok(RunSummary {
        root_frame_id: root_id,
        subtasks: outcomes,
        total_nodes,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Builds a `name → live_id` map by recursively walking the live node tree.
///
/// Used to resolve logical scaffold ids (remapped by `InsertSubtree`) to their
/// live counterparts.  Every synthesized frame has a distinct name, so
/// name-matching is unambiguous within the dashboard scaffold subtree.
pub(crate) fn collect_name_id_map(node: &PenNode) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    collect_name_id_map_rec(node, &mut map);
    map
}

fn collect_name_id_map_rec(node: &PenNode, map: &mut std::collections::HashMap<String, String>) {
    if let Some(name) = node.base().name.as_deref() {
        map.insert(name.to_string(), node.id_str().to_string());
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_name_id_map_rec(child, map);
        }
    }
}
