//! Multi-root screen-group scaffold insertion — split out of `run.rs` to
//! keep it under the repo's 800-line-per-file cap. Child module of `run`
//! (`mod run_screen_groups;` inside `run.rs`), so `super::` reaches `run`'s
//! private helpers (`next_root_insert_position`) the same way every other
//! private-parent-item access works in this codebase's split-file
//! convention (e.g. `op-host-native/src/preview/app_mode.rs`).

use crate::agent_identity::AgentIdentity;
use crate::plan::OrchestratorPlan;
use crate::scaffold::{build_screen_group_scaffold, ScaffoldError};
use crate::screen_groups::ScreenGroup;
use crate::subagent::{apply_command_with_reveal, reveal_now_millis};
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// Failure of the multi-root screen-group scaffold insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScreenGroupScaffoldError {
    /// A per-group root template failed to build (implementation bug).
    Scaffold(ScaffoldError),
    /// The document rejected one of the `InsertSubtree` commands.
    InsertRejected,
    /// The post-insert root diff did not yield one new root per group.
    RootCountMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for ScreenGroupScaffoldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scaffold(error) => write!(f, "{error}"),
            Self::InsertRejected => {
                write!(f, "screen-group scaffold insert rejected by document")
            }
            Self::RootCountMismatch { expected, actual } => write!(
                f,
                "expected {expected} screen-group scaffold roots, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ScreenGroupScaffoldError {}

impl From<ScaffoldError> for ScreenGroupScaffoldError {
    fn from(error: ScaffoldError) -> Self {
        Self::Scaffold(error)
    }
}

/// Insert one scaffold root PER screen group (multiscreen-fanout-break fix,
/// item A) and reassign every subtask's `parent_frame_id` from
/// `plan_normalize::normalize`'s placeholder id to the REAL post-insert root
/// id — mirroring exactly how the single-root path already resolves its one
/// `rid` by diffing `active_children()` before/after insertion, just for N
/// roots instead of one. Groups still execute strictly SEQUENTIALLY
/// afterward (the caller's existing per-subtask loop; this function only
/// builds canvas, it does not run any sub-agent).
///
/// The first root lands at `next_root_insert_position` (beside whatever is
/// already on the canvas — the SAME "follow-on screen" convention the
/// single-root path uses); each subsequent group's root lands
/// `SCREEN_GROUP_GAP` to the right of the previous one.
///
/// `identities` is either EMPTY (the sequential path — this function tags
/// NO frame at all, so every reveal falls through to the host-confirmed
/// `cursor_agent`, the single source of truth for a single-agent run — see
/// `run.rs`'s `group_identities` doc for why, dual-cursor-identity fix
/// 2026-07-17) or exactly `groups.len()` DISTINCT identities (a genuinely
/// concurrent run, D-lite's three-piece visibility fix: N different root
/// badges is what lets the canvas resolve N different agent cursors). Any
/// other length tags nothing — the caller (`run.rs`) is the only caller and
/// always passes one of these two shapes.
///
/// Returns `Err` on scaffold-template bugs or a remap-count mismatch — the
/// caller rolls back + ends the undo batch on either, same as every other
/// scaffold-insert failure path in `run()`.
pub(crate) fn insert_screen_group_roots(
    plan: &mut OrchestratorPlan,
    groups: &[ScreenGroup],
    is_mobile: bool,
    sink: &mut dyn DocSink,
    scaffold_root_ids_before: &[String],
    agent_indicator_epoch: Option<u64>,
    identities: &[AgentIdentity],
) -> Result<(Vec<String>, Vec<usize>), ScreenGroupScaffoldError> {
    // Multi-root inserts intentionally bypass the editor's single-root
    // replacement heuristic. Consume the same sole empty starter explicitly
    // after all roots land, rather than exporting it as an extra blank screen.
    let starter = super::detect_reusable_empty_frame(sink.state());
    let (insert_x, insert_y) = if starter.is_some() {
        (super::DEFAULT_ROOT_X, super::DEFAULT_ROOT_Y)
    } else {
        super::next_root_insert_position(sink.state(), plan.root_frame.width)
    };
    let (cmds, _placeholder_ids, baselines) =
        build_screen_group_scaffold(plan, groups, is_mobile, insert_x, insert_y)?;

    for cmd in cmds {
        if !apply_command_with_reveal(sink, cmd, agent_indicator_epoch, reveal_now_millis()) {
            return Err(ScreenGroupScaffoldError::InsertRejected);
        }
    }

    // Every group gets a fresh root in one batched insert, so the
    // remap discovery is the single-root path's diff trick generalized to N:
    // every top-level child NOT present before insertion is a new root, in
    // insertion order.
    let new_root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .filter(|n| !scaffold_root_ids_before.iter().any(|old| old == n.id_str()))
        .map(|n| n.id_str().to_string())
        .collect();
    if new_root_ids.len() != groups.len() {
        return Err(ScreenGroupScaffoldError::RootCountMismatch {
            expected: groups.len(),
            actual: new_root_ids.len(),
        });
    }
    if let Some(id) = starter {
        if !sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(id),
            page_id: None,
        }) {
            return Err(ScreenGroupScaffoldError::InsertRejected);
        }
    }

    for (group, real_id) in groups.iter().zip(new_root_ids.iter()) {
        for &idx in &group.indices {
            if let Some(subtask) = plan.subtasks.get_mut(idx) {
                subtask.parent_frame_id = Some(real_id.clone());
            }
        }
    }

    // Freeze the normalized planning screen list into real route markers
    // before any fan-out prompt is built. The prompt inventory reads the same
    // merged candidates and allocator, so generated cross-screen actions can
    // never reference a virtual route that cleanup later assigns differently.
    crate::wire_screen_navigation::ensure_planned_screen_routes(sink, plan);

    // Only the genuinely-concurrent shape (`identities.len() ==
    // new_root_ids.len()`) tags anything — an empty slice (the sequential
    // path) deliberately leaves every root untagged; see this function's doc.
    if let Some(epoch) = agent_indicator_epoch {
        if identities.len() == new_root_ids.len() {
            for (id, identity) in new_root_ids.iter().zip(identities.iter()) {
                op_editor_core::agent_indicators::add_frame(
                    epoch,
                    id,
                    &identity.color,
                    &identity.name,
                );
            }
        }
    }

    Ok((new_root_ids, baselines))
}
