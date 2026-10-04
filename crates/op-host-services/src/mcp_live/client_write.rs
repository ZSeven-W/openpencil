//! Canvas presence for MCP client writes — the shared producer half of
//! the `agent_indicators` registry for BOTH MCP request loops.
//!
//! `mcp_live` (desktop `--live-mcp`) and the serve-web daemon's `/mcp`
//! dispatch (`web_canvas_server::connection`) are separate request loops
//! over the same `EditorCommand` apply path. This module holds ONE
//! implementation of "which nodes did that write touch / create, and how
//! do they show up on canvas" so the two loops cannot drift:
//!
//! - nodes that appeared across the write (before/after id diff) get the
//!   full reveal treatment — staggered scale-pop entrance + the cursor's
//!   waypoint queue — via [`crate::design_agent_tools::register_new_node_reveals`];
//! - nodes an existing-node command merely TOUCHED get a reveal whose
//!   start is back-dated past the entrance windows — the reveal entry is
//!   what anchors the agent cursor and the current-element breathing
//!   border — so the cursor lands on the edited node WITHOUT replaying
//!   a pop that would read as "this node was just created";
//! - genuinely new top-level `Frame` roots get the `add_frame` glow +
//!   badge tag, the same rule `design_loop_indicator` applies to in-app
//!   design runs.
//!
//! The browser-facing half needs no change at all: the daemon relays the
//! process-global registry verbatim over `GET /api/mcp/indicators` and
//! the web host mirrors it into its own registry (`apply_remote`).

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{agent_indicators, EditorCommand, EditorState, NodeId};

/// Fallback client-facing label when `initialize` carried no
/// `clientInfo.name` (some minimal/older MCP clients omit it) — honest
/// about what it is ("some external MCP tool"), not a fabricated persona.
pub(crate) const MCP_CLIENT_FALLBACK_NAME: &str = "MCP Client";
/// Fixed neutral badge colour for every MCP-driven write — a deliberate
/// contrast with the in-app agent persona pool
/// (`assign_agent_identities_seeded`'s vivid per-agent colours): an
/// external MCP client isn't one of the app's own named agents.
pub(crate) const MCP_CLIENT_COLOR: &str = "#8A8F98";

/// How far before the write landed a "touched" node's reveal start is
/// parked. Two constraints in one number: it must sit past the entrance
/// windows (wireframe ~260ms + scale-pop ~180ms — the `op-editor-ui`
/// reveal paint constants this crate cannot import) or the edited node
/// would replay a creation pop, and it must stay inside
/// `agent_indicators::REVEAL_DURATION_MS` so the reveal entry still
/// counts as a live cursor waypoint and keeps the epoch's reveal queue
/// "draining" for the reuse check below. `pub(crate)` for tests
/// asserting the no-pop back-date.
pub(crate) const FOCUS_REVEAL_LOOKBACK_MS: u64 = 600;
// A focus reveal parked outside `REVEAL_DURATION_MS` is born expired —
// the first paint-side prune drops it and the cursor never anchors.
const _: () = assert!(FOCUS_REVEAL_LOOKBACK_MS < agent_indicators::REVEAL_DURATION_MS);

/// How long the daemon keeps a write's epoch `run_active` after the last
/// registration — the relay grace. The registry only reaches the browser
/// through the 400ms `GET /api/mcp/indicators` poll, and `apply_remote`
/// mirrors ONLY active runs: an epoch finished the instant its write
/// lands is one the browser never sees at all. Three poll ticks
/// guarantees the `active:true` observation lands the cursor + badges
/// before the first inactive snapshot drains them.
const RELAY_GRACE_MS: u64 = 1_200;

/// The epoch-reuse rule one request loop asks for when a write
/// registers. Both loops coalesce a burst of calls into ONE continuous
/// run — the difference is only in what "the previous run is still
/// live" means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EpochReuse {
    /// Reuse while the last epoch's reveal queue is still draining
    /// (`latest_reveal_end_ms`). The desktop pump finishes its epoch
    /// synchronously and the paint pass drains it in-process, so the
    /// drain window IS the burst window.
    WhileDraining,
    /// Reuse only while the last epoch is still `run_active` AND inside
    /// its relay grace. The daemon MUST NOT extend a finished epoch —
    /// `apply_remote` mirrors active runs only, so indicators registered
    /// after `finish_if_epoch` would never reach the browser — and a
    /// grace deadline that passed unpolled means the run should already
    /// have retired, so late writes start a fresh epoch rather than
    /// parking the cursor on stale waypoints.
    WhileRelayLive,
}

/// Per-request-loop bookkeeping for MCP client writes → canvas
/// indicators: the client-declared badge identity plus the epoch the
/// loop's last write minted (or reused). One session per request loop —
/// `McpLiveServer` wraps it in an `Arc<Mutex<_>>` shared with its
/// connection threads (they sniff `initialize` on their own threads);
/// `WebCanvasState` keeps it inline (the daemon's state mutex already
/// serializes every `/mcp` call).
#[derive(Default)]
pub(crate) struct ClientWriteIndicators {
    /// `(name, color)` from `initialize`'s `params.clientInfo.name` —
    /// always overwritten on the next initialize so the badge names the
    /// client ACTUALLY driving now, not whoever connected first.
    client_identity: Option<(String, String)>,
    /// The last epoch this loop minted or reused — `0` before the first
    /// write landed any indicators.
    last_epoch: u64,
    /// Daemon only: the wall-clock ms at which the live epoch may finish
    /// — re-armed by every registered write; the relay endpoint's own
    /// poll cadence retires it ([`Self::finish_expired_relay`]). Always
    /// `None` on the desktop, which finishes synchronously.
    relay_finish_at_ms: Option<u64>,
    /// Daemon only: whether an `active:true` body has been served for
    /// `last_epoch` since it was minted. The grace deadline alone must
    /// not retire an epoch the browser never observed — a backgrounded
    /// or stalled first poll would drop the write's whole presence.
    relay_seen_active: bool,
}

impl ClientWriteIndicators {
    /// Capture `params.clientInfo.name` off an `initialize` body — the
    /// ONLY message on this wire that carries a client-declared name.
    /// Always overwrite: a later initialize means a different tool
    /// connected, and the badge should name who is ACTUALLY driving now.
    /// A `None` (missing/empty name) also overwrites — keeping a prior
    /// client's badge on an anonymous reconnect is worse than the honest
    /// fallback.
    pub(crate) fn note_client_name(&mut self, name: Option<String>) {
        self.client_identity = name.map(|name| (name, MCP_CLIENT_COLOR.to_string()));
    }

    /// The badge identity to paint — the declared client name or the
    /// honest "an external MCP tool" fallback when no `initialize`
    /// named itself.
    fn tag(&self) -> (String, String) {
        self.client_identity.clone().unwrap_or_else(|| {
            (
                MCP_CLIENT_FALLBACK_NAME.to_string(),
                MCP_CLIENT_COLOR.to_string(),
            )
        })
    }

    /// Register canvas indicators for a write that JUST applied and
    /// repaired. Returns `Some(epoch)` when anything was registered —
    /// the caller then decides how the epoch ends (the desktop finishes
    /// synchronously; `EpochReuse::WhileRelayLive` re-arms the daemon's
    /// relay grace inside this call).
    ///
    /// - `ids_before` — the active-tree id set captured before the apply
    ///   (`None` when the command can't touch the tree): every id present
    ///   now but absent then gets the staggered reveal pop.
    /// - `touched_ids` — the ids the applied `EditorCommand`(s) directly
    ///   target (`command::affected_node_ids`): each gets cursor focus
    ///   WITHOUT the entrance replay.
    ///
    /// A no-op — `None`, no epoch minted — when the write neither
    /// created nor touched a node: reads, empty no-ops, collab-refused
    /// writes (which never reach apply), and non-document commands.
    pub(crate) fn register_applied_writes(
        &mut self,
        state: &EditorState,
        ids_before: Option<&HashSet<String>>,
        touched_ids: &[String],
        reuse: EpochReuse,
        now_ms: u64,
    ) -> Option<u64> {
        let new_ids: HashSet<String> = match ids_before {
            Some(before) => crate::design_agent_tools::collect_active_node_ids(state)
                .difference(before)
                .cloned()
                .collect(),
            None => HashSet::new(),
        };
        // Touched ids that are ALSO new keep their staggered pop — the
        // new-node treatment wins over focus-only. Ids absent from the
        // post-apply active tree (a `DeleteNode`/`ReplaceNode` target, a
        // node the write pushed onto a different page) are dropped too —
        // a badge the canvas cannot paint would still count toward the
        // burst smoother and the drain window.
        let mut touched: Vec<&str> = Vec::with_capacity(touched_ids.len());
        for id in touched_ids {
            if !new_ids.contains(id.as_str())
                && !touched.contains(&id.as_str())
                && op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id))
                    .is_some()
            {
                touched.push(id.as_str());
            }
        }
        if new_ids.is_empty() && touched.is_empty() {
            return None;
        }
        let epoch = self.resolve_epoch(reuse, now_ms)?;
        let (name, color) = self.tag();
        // Confirms the badge identity AND parks the cursor at the
        // viewport's idle anchor, so the first waypoint hop is a real
        // flight rather than a teleport.
        agent_indicators::confirm_cursor_agent(epoch, &color, &name);
        // Only a genuinely NEW top-level Frame is a fresh generation
        // root — mirrors `design_loop_indicator.rs::register_new_frames`
        // (Frame-only) and `run_screen_groups.rs::insert_screen_group_roots`
        // (new-vs-existing top-level diff), the two established places
        // this codebase already tags a root frame.
        for node in state.active_children() {
            if matches!(node, PenNode::Frame(_)) && new_ids.contains(node.id_str()) {
                agent_indicators::add_frame(epoch, node.id_str(), &color, &name);
            }
        }
        if let Some(before) = ids_before {
            crate::design_agent_tools::register_new_node_reveals(
                before,
                state,
                Some(epoch),
                now_ms,
            );
        }
        for id in &touched {
            agent_indicators::add_node(epoch, id, &color, &name);
        }
        // ONE back-dated reveal anchors the cursor waypoint on the write's
        // last target. A second back-dated entry counts as a newly-due
        // burst at the next paint (`smooth_overdue_reveal_burst` reschedules
        // every due reveal once more than `REVEAL_MAX_NEW_STARTS_PER_SNAPSHOT`
        // lands in one snapshot) and the edited nodes would replay the
        // entrance pop this path exists to avoid.
        if let Some(anchor) = touched.last() {
            agent_indicators::add_reveal(
                epoch,
                anchor,
                now_ms.saturating_sub(FOCUS_REVEAL_LOOKBACK_MS),
            );
        }
        if self.last_epoch != epoch {
            self.relay_seen_active = false;
        }
        self.last_epoch = epoch;
        if matches!(reuse, EpochReuse::WhileRelayLive) {
            self.arm_relay_finish(now_ms);
        }
        Some(epoch)
    }

    /// The reuse-while-live epoch resolution — see [`EpochReuse`]. A
    /// burst of writes inside the live window extends the SAME epoch so
    /// the canvas reads it as one continuous run instead of `begin()`'s
    /// bump-and-wipe clipping the previous write's tail animation.
    ///
    /// Returns `None` when a DIFFERENT producer owns the live run (an
    /// in-app design turn, an in-daemon chat design) — `begin()` clears
    /// every map unconditionally, so minting here would wipe that run's
    /// badges, reveals and pending root-seed contract mid-flight. The
    /// write's own presence is skipped instead; the next write lands once
    /// the registry is free or this session's epoch is reusable again.
    fn resolve_epoch(&mut self, reuse: EpochReuse, now_ms: u64) -> Option<u64> {
        if self.last_epoch != 0 {
            let reusable = match reuse {
                EpochReuse::WhileDraining => {
                    agent_indicators::latest_reveal_end_ms(self.last_epoch)
                        .is_some_and(|end_ms| now_ms < end_ms)
                }
                EpochReuse::WhileRelayLive => {
                    agent_indicators::active_epoch() == Some(self.last_epoch)
                        && self
                            .relay_finish_at_ms
                            .is_some_and(|deadline| now_ms < deadline)
                }
            };
            if reusable {
                return Some(self.last_epoch);
            }
        }
        if let Some(active) = agent_indicators::active_epoch() {
            if active != self.last_epoch {
                return None;
            }
            // Our own run is still live but outside its reuse window —
            // fall through and let `begin()` retire it cleanly.
        }
        Some(agent_indicators::begin())
    }

    /// Re-arm the relay grace after a write registered. The epoch stays
    /// `run_active` until the first poll past this deadline retires it —
    /// long enough for the browser's 400ms cadence to see `active:true`
    /// at least once. Each registration in a burst pushes the finish
    /// out, so the run ends once, after the LAST write's grace.
    fn arm_relay_finish(&mut self, now_ms: u64) {
        self.relay_finish_at_ms = Some(now_ms.saturating_add(RELAY_GRACE_MS));
    }

    /// Daemon only: record that the relay just served a body. Call AFTER
    /// [`Self::finish_expired_relay`] so an already-due epoch retires on
    /// this poll rather than riding one more cycle.
    pub(crate) fn note_relay_served(&mut self) {
        if agent_indicators::active_epoch() == Some(self.last_epoch) {
            self.relay_seen_active = true;
        }
    }

    /// Daemon only: retire the live write epoch once its relay grace has
    /// elapsed. Called from `GET /api/mcp/indicators` — the browser's own
    /// poll cadence is what finishes the run, so a daemon with no browser
    /// attached needs no timer thread at all (the next write simply
    /// starts a fresh epoch).
    ///
    /// The deadline alone is not enough: `apply_remote` mirrors only
    /// `run_active` runs, so an epoch that expired before its FIRST
    /// `active:true` body was served would vanish unobserved (a hidden
    /// tab's throttled poll easily misses 1.2s). Retire when the deadline
    /// passed AND at least one active body was served — or, for a run a
    /// browser never polls, after a hard cap of one extra grace so stale
    /// overlays can't park forever.
    pub(crate) fn finish_expired_relay(&mut self, now_ms: u64) {
        let Some(deadline) = self.relay_finish_at_ms else {
            return;
        };
        let due = now_ms >= deadline;
        let overdue = now_ms >= deadline.saturating_add(RELAY_GRACE_MS);
        if !(overdue || (due && self.relay_seen_active)) {
            return;
        }
        self.relay_finish_at_ms = None;
        self.relay_seen_active = false;
        // Epoch-guarded: a different producer's active run (an in-daemon
        // design turn, say) is never finished by an MCP deadline.
        if agent_indicators::active_epoch() == Some(self.last_epoch) {
            agent_indicators::finish_if_epoch(self.last_epoch);
        }
    }

    #[cfg(test)]
    pub(crate) fn last_epoch(&self) -> u64 {
        self.last_epoch
    }
}

/// Whether `cmd` can change the document at all — the same
/// non-mutating-command set the desktop pump uses for its layout-dirty
/// flag, shared here so both loops gate their id-diff snapshot the same
/// way (a read-only or UI-only command never justifies a tree walk).
pub(crate) fn command_mutates_document(cmd: &EditorCommand) -> bool {
    match cmd {
        EditorCommand::ClearSelection
        | EditorCommand::SetSelection { .. }
        | EditorCommand::SetSelectionSet { .. }
        | EditorCommand::ToggleNodeSelection { .. }
        | EditorCommand::SetViewport { .. }
        | EditorCommand::SetActiveTool { .. }
        | EditorCommand::CopySelected => false,
        EditorCommand::SetNodeFlag { flag, .. } => {
            !matches!(flag, op_editor_core::NodeFlag::Collapsed)
        }
        _ => true,
    }
}

/// Whether `cmd` can add, remove, or rewrite nodes anywhere — the gate
/// for the indicators' before/after id diff. A pure property writer
/// (`SetNodeFillHex`-class) provably leaves the id set unchanged, so it
/// never justifies a `collect_active_node_ids` walk; a `Batch` is
/// conservatively included since any sub-command can.
///
/// A command whose real effect is to change WHICH tree is active —
/// `SetActivePage` itself, a `Batch` containing one, `DeletePage` of the
/// live page, or an `Undo`/`Redo` restoring a different index — passes
/// this gate but produces a diff of the newly-active page's pre-existing
/// nodes: an all-new reveal sweep over content the write never touched.
/// The callers therefore compare [`active_page_identity`] before and
/// after the apply and veto the diff when it changed.
pub(crate) fn command_can_change_id_set(cmd: &EditorCommand) -> bool {
    command_mutates_document(cmd)
        && !matches!(
            cmd,
            EditorCommand::UpdateNode { .. }
                | EditorCommand::PatchNodeData { .. }
                | EditorCommand::MoveNode { .. }
                | EditorCommand::SetNodeFlag { .. }
                | EditorCommand::SetNodeFlip { .. }
                | EditorCommand::SetEllipseArc { .. }
                | EditorCommand::AddNodeEffect { .. }
                | EditorCommand::RemoveNodeEffect { .. }
                | EditorCommand::SetEffectParam { .. }
                | EditorCommand::SetEffectColor { .. }
                | EditorCommand::SetNodeRotation { .. }
                | EditorCommand::SetNodeText { .. }
                | EditorCommand::SetNodeCornerRadius { .. }
                | EditorCommand::SetNodeFontSize { .. }
                | EditorCommand::SetNodeFontWeight { .. }
                | EditorCommand::SetNodeStrokeHex { .. }
                | EditorCommand::SetNodeStrokeWidth { .. }
                | EditorCommand::SetNodeStrokeSideWidth { .. }
                | EditorCommand::SetNodeFillHex { .. }
                | EditorCommand::SetNodeName { .. }
                | EditorCommand::SetNodeLayoutProp { .. }
                | EditorCommand::ReplaceAllMatchingProperties { .. }
        )
}

/// The id of the page `active_page_index` currently resolves to, or
/// `None` for page-less documents. Compared across an apply so a write
/// that moved the active page can't read the newly-active tree as
/// freshly-created content — see [`command_can_change_id_set`].
pub(crate) fn active_page_identity(state: &EditorState) -> Option<String> {
    state.doc.pages.as_ref().and_then(|pages| {
        (!pages.is_empty()).then(|| {
            pages[state.ui.active_page_index.min(pages.len() - 1)]
                .id
                .clone()
        })
    })
}

#[cfg(test)]
#[path = "client_write_tests.rs"]
mod tests;
