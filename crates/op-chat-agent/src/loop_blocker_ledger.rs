//! Unresolved-blocker detection for agent-loop completion gating.
//!
//! [`detect_blockers`] is a LIVE recompute against the current `EditorState`
//! — deliberately NOT an accumulating ledger populated incrementally by
//! `design_agent_tools.rs`'s per-`batch_design` diagnostics. It mirrors the
//! discipline `op_orchestrator::unfilled_screens::detect_unfilled_screens`
//! already established for `ChatToolExecutor::check_unfilled_screens`: run
//! the same scans fresh every time, straight off the live document, so an
//! issue a later batch already fixed simply stops appearing — nothing to
//! prune, nothing that can go stale.
//!
//! Scope (MVP, conservative): only failure modes with an unambiguous
//! structural signal count as blockers —
//!
//! - **structure** — duplicate status bars / broken rings / duplicate
//!   header-icon rows (`design_agent_tools::scan_duplicate_root_issues` /
//!   `scan_ring_issues` / `scan_header_icon_row_issues` — the same scans
//!   whose hits ride into `batch_design`'s `structureIssues` field).
//! - **empty-shell** — a scaffolded section never filled
//!   (`design_agent_tools::scan_empty_shells`, `shellsRemaining` per-batch).
//! - **nav** — an unbound primary-mobile-screen nav tab
//!   (`op_orchestrator::nav_issues::scan_nav_issues`, `navIssues` per-batch).
//!
//! `layoutIssues` (`op_orchestrator::geometry_validation::geometry_diagnostics`)
//! is deliberately EXCLUDED from this MVP: its ~15 detectors (overflow /
//! collapse / jam / starvation / spill / …) all return the same flat
//! `Vec<String>` with no structured category field, so classifying "which of
//! these are overflow/collapse" would mean pattern-matching free-form
//! message text — exactly the name/string-heuristic fragility this codebase
//! has been burned by before. `layoutIssues` stays advisory (it still rides
//! in the per-batch tool result, unaffected by this module) until
//! `geometry_validation` grows a real category enum on its diagnostics.

use op_ai::chat_provider::{BlockerEntry, BlockerReport};
use op_editor_core::EditorState;

use crate::design_agent_tools::{
    scan_duplicate_root_issues, scan_empty_shells, scan_header_icon_row_issues, scan_ring_issues,
};

/// Recompute every unresolved blocker against the CURRENT active page of
/// `state` — same scope `design_agent_tools`'s per-batch diagnostics use.
/// Read-only: never mutates the document.
pub fn detect_blockers(state: &EditorState) -> BlockerReport {
    let children = state.active_children();
    let mut blockers = Vec::new();
    for detail in scan_duplicate_root_issues(children) {
        blockers.push(BlockerEntry {
            category: "structure".to_string(),
            detail,
        });
    }
    for detail in scan_ring_issues(children) {
        blockers.push(BlockerEntry {
            category: "structure".to_string(),
            detail,
        });
    }
    for detail in scan_header_icon_row_issues(children) {
        blockers.push(BlockerEntry {
            category: "structure".to_string(),
            detail,
        });
    }
    for detail in scan_empty_shells(children) {
        blockers.push(BlockerEntry {
            category: "empty-shell".to_string(),
            detail,
        });
    }
    for detail in op_orchestrator::nav_issues::scan_nav_issues(state) {
        blockers.push(BlockerEntry {
            category: "nav".to_string(),
            detail,
        });
    }
    BlockerReport { blockers }
}

#[cfg(test)]
#[path = "loop_blocker_ledger_tests.rs"]
mod tests;
