//! Unresolved-blocker completion gate — shared between the Anthropic and
//! OpenAI-compatible agent loops in `chat_agent_loop.rs` (split out as a
//! sibling module the same way `chat_agent_loop_retry.rs` splits out the
//! corrective-write retry policy, so neither loop function duplicates this
//! logic). See `op_host_services::loop_blocker_ledger`'s module doc for what
//! counts as a blocker (structure / empty-shell / nav) and why this is a
//! live recompute against the document rather than an accumulating ledger.

use std::sync::Arc;

use op_ai::chat_provider::{BlockerReport, ChatDelta, ChatToolExecutor};
use tokio::sync::mpsc;

/// Per-run budget for the unresolved-blocker corrective round — a SEPARATE
/// pool from `FILL_BUDGET_MAX_ROUNDS_PER_SCREEN` / `SALVAGE_MAX_ROUNDS`
/// (blockers and unfilled screens are different failure modes; see those
/// constants' doc comments in `chat_agent_loop.rs` for "budget guards
/// runaway retry, never truncates promised work"). Blockers are a
/// narrower, already-detected class — the model already saw each one as a
/// per-batch tool-result hint (`structureIssues` / `shellsRemaining` /
/// `navIssues`) — so a flat per-run cap (not a per-item pool like the fill
/// budget) is enough to give the model a real chance to react without room
/// for an avalanche.
pub(super) const BLOCKER_NUDGE_MAX_ROUNDS: usize = 2;

/// Cheap, read-only unresolved-blocker scan. Gated the same way
/// `check_unfilled`/`run_loop_finalize` are: `enabled` mirrors
/// `cfg.finalize_on_exit`, so a plain (non-design) chat turn never touches
/// the document.
pub(super) async fn check_blockers(
    executor: &Arc<dyn ChatToolExecutor>,
    enabled: bool,
) -> BlockerReport {
    if !enabled {
        return BlockerReport::default();
    }
    let executor = executor.clone();
    tokio::task::spawn_blocking(move || executor.check_blockers())
        .await
        .unwrap_or_default()
}

/// Corrective-message contract text for a still-blocked loop completion —
/// mirrors `contract_nudge_text`'s "state the full commitment, not just
/// what's missing" shape, but for structural blockers instead of unfilled
/// screens.
pub(super) fn blocker_nudge_text(report: &BlockerReport) -> String {
    let lines: Vec<String> = report
        .blockers
        .iter()
        .map(|b| format!("- [{}] {}", b.category, b.detail))
        .collect();
    format!(
        "The design still has {} unresolved blocker(s) that must be fixed before finishing:\n{}",
        lines.len(),
        lines.join("\n")
    )
}

/// Decide whether this `calls.is_empty()` model-stop is owed one more
/// corrective round for unresolved blockers — mirrors
/// `eligible_for_fill_round`'s per-screen budget check, but against a flat
/// per-run round counter instead of a per-screen map (blockers aren't keyed
/// by screen name the way unfilled screens are). Spends a round from
/// `rounds_used` when it returns `Some`; the caller pushes the text as a
/// user message and `continue`s WITHOUT counting it against the ordinary
/// turn budget, exactly like the fill round.
pub(super) async fn blocker_nudge_if_owed(
    executor: &Arc<dyn ChatToolExecutor>,
    enabled: bool,
    rounds_used: &mut usize,
) -> Option<String> {
    if !enabled || *rounds_used >= BLOCKER_NUDGE_MAX_ROUNDS {
        return None;
    }
    let report = check_blockers(executor, enabled).await;
    if !report.has_blockers() {
        return None;
    }
    *rounds_used += 1;
    Some(blocker_nudge_text(&report))
}

/// Tier-3 unconditional honest report for blockers — appended right
/// alongside `report_unfilled_if_any`'s screen line whenever the loop is
/// about to send `Done` with blockers still unresolved (round budget
/// spent, or this exit tier never spent a round at all — e.g. the
/// turn-cap/salvage tiers, which only nudge for unfilled screens, not
/// blockers). Never silently succeeds: any blocker still present at ANY
/// exit tier surfaces here.
pub(super) async fn report_blockers_if_any(tx: &mpsc::Sender<ChatDelta>, report: &BlockerReport) {
    if !report.has_blockers() {
        return;
    }
    let text = format!(
        "\n\n• {} unresolved blocker(s): {}",
        report.blockers.len(),
        report
            .blockers
            .iter()
            .map(|b| b.detail.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    let _ = tx.send(ChatDelta::TextDelta(text)).await;
}
