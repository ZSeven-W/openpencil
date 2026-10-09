//! App-shell restructure — a single deterministic structural post-pass that
//! turns a flat-vertical desktop dashboard whose FIRST child is a full-width
//! sidebar into a horizontal `[sidebar(fixed) | content-column(fill)]` shell.
//!
//! Weak models (glm-5.x etc.) routinely emit a "Sidebar Navigation" as the
//! first child of a vertical root at the *full* page width, so it renders as a
//! full-width top band instead of a narrow left column. The orchestrator's old
//! plan-level "dashboard bespoke scaffold" (that pre-built the two-column root)
//! was removed in 346bcaa8; this is its lightweight node-tree replacement — one
//! pass, no row/slot bin-packing, no placeholder-height estimation.
//!
//! Runs once over each assembled page-root in `cleanup::run_cleanup_passes` —
//! the whole-doc finalize point SHARED by the orchestrator (per-subtask role
//! passes already ran) and the agentic loop (`apply_loop_finalize` ran the
//! whole-doc role passes) — so the moved sections keep their resolved roles.
//!
//! Both broken shapes the orchestrator emits are handled: a VERTICAL root with
//! the sidebar stacked as a full-width band, and a HORIZONTAL root with the
//! sidebar AND every content section crammed into one row. Detection is
//! intentionally strict (see [`detect`]); the adversarial design review flagged
//! restaurant "Menu" pages, "Navy" hero sections, top-nav bars, mobile/tablet
//! roots, and multi-screen files as the false-positives to avoid, so the gate
//! leans on a STRONG sidebar-name token + a real dashboard-content gate (a short
//! numeric header height is rejected, but `fit_content` sidebars are allowed).

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

// Pass submodules: this file keeps the layout constants and the `pub(crate)`
// entry points; detection / restructure / eviction live in their own files and
// are re-imported here so the entry points (and the test module mounted below)
// see the same flat namespace as before.
#[path = "app_shell_detect.rs"]
mod app_shell_detect;
#[path = "app_shell_evict.rs"]
mod app_shell_evict;
#[path = "app_shell_restructure.rs"]
mod app_shell_restructure;
#[path = "app_shell_height_contract.rs"]
mod height_contract;

use app_shell_detect::*;
pub(crate) use app_shell_evict::*;
pub(crate) use app_shell_restructure::*;
pub(crate) use height_contract::{height_contract_diagnostics, repair_height_contract};

/// Fixed left-sidebar column width. Mirrors the surviving sizing constant in
/// `dashboard_columns.rs` for parity with the removed scaffold.
const SIDEBAR_WIDTH: f64 = 260.0;
/// Desktop floor — excludes phones/tablets. Mirrors
/// `cleanup_desktop_dashboard::DESKTOP_DASHBOARD_MIN_WIDTH`.
const DESKTOP_MIN_WIDTH: f64 = 900.0;
/// A real sidebar is full-height; a 64–96px full-width band is a header.
const MIN_SIDEBAR_HEIGHT: f64 = 200.0;
/// Outer gutter for the content column `[vertical, horizontal]`. Mirrors
/// Pencil's app-shell content `padding:[32,40]`.
const CONTENT_PADDING: [i64; 2] = [32, 40];

/// Restructure a flat-vertical desktop dashboard whose first child is a
/// full-width sidebar into a horizontal `[sidebar(fixed) | content(fill)]`
/// app-shell. Mutates the page-root wrapper in place via the same serialize →
/// mutate `Value` → deserialize round-trip the section passes use. Returns
/// `true` iff it restructured; no-op + `false` when the strict detection
/// criteria do not hold or the round-trip fails (the node is never dropped).
pub(crate) fn reshape_sidebar_to_app_shell(wrapper: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*wrapper) else {
        return false;
    };
    if !detect(&v) {
        return false;
    }
    if !restructure(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *wrapper = new_node;
            true
        }
        // Bad round-trip: leave the wrapper exactly as it was.
        Err(_) => false,
    }
}

// ── Value readers (mirror the tolerant accessors the sibling passes use) ──

#[cfg(test)]
#[path = "app_shell_tests.rs"]
mod tests;
