//! Desktop GUI pumps for the background design turn — the host-coupled
//! half of the design session.
//!
//! The worker spawn + viewport-fit math live in
//! [`op_host_services::design_session`]; this residual keeps the two UI-loop
//! pumps (`pump_commands` / `pump_progress`, which take `&mut
//! WidgetHostNative` — orphan rule) plus the progress-line renderer they
//! fold into the chat transcript.
//!
//! - UI event loop drains pending `DesignCmdReq` each frame via
//!   [`pump_commands`] — applies on the real state, replies ack.
//! - UI event loop also drains `DesignDelta` via [`pump_progress`] and
//!   renders progress into the trailing chat bubble.

use op_editor_host_core::design::{DesignCmdAck, DesignCmdOp};
// Re-export so `crate::design_session::DesignSession` (the DesktopApp
// field type in main.rs) resolves with zero churn.
pub use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;
use op_orchestrator::Progress;

use op_host_services::design_session::fit_design_viewport_to_content;

/// Drain every pending apply request from the in-flight design
/// session and execute it against the real `EditorState`. Each
/// request gets an ack containing a fresh state snapshot so the
/// worker's mirror reflects ID-remapping. Returns true when at least
/// one command applied (caller should mark redraw dirty).
pub fn pump_commands(
    host: &mut WidgetHostNative,
    current: &mut Option<DesignSession>,
    viewport_width: f32,
    viewport_height: f32,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let reqs = session.drain_cmd_requests();
    if reqs.is_empty() {
        return false;
    }
    let state = host.editor_state_mut();
    let mut any_applied = false;
    for req in reqs {
        let applied = match req.op {
            DesignCmdOp::Apply(cmd) => {
                let applied = state.apply(cmd);
                if applied {
                    fit_design_viewport_to_content(state, viewport_width, viewport_height);
                }
                applied
            }
            // TODO(host): wire into op-editor-core history batch mode
            // once available. Today undo-batch boundaries are no-ops so
            // each `EditorCommand::InsertSubtree` is its own undo step —
            // functionally correct, just finer-grained than ideal.
            DesignCmdOp::BeginUndoBatch | DesignCmdOp::EndUndoBatch => true,
        };
        let snapshot = state.clone();
        let ack = DesignCmdAck {
            applied,
            new_state: snapshot,
        };
        // If the ack fails to send, the worker already dropped its
        // receiver (e.g. turn aborted) — nothing to do here.
        let _ = req.ack.send(ack);
        if applied {
            any_applied = true;
        }
    }
    if any_applied {
        host.mark_editor_state_dirty();
    }
    any_applied
}

/// Drain every pending progress delta and fold it into the trailing
/// assistant message. Clears `current` once the terminal `Done`
/// arrives. Returns true when the transcript changed.
pub fn pump_progress(host: &mut WidgetHostNative, current: &mut Option<DesignSession>) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll_progress();
    let mut changed = false;
    if !poll.progress.is_empty() {
        let appended = render_progress(&poll.progress);
        let chat = &mut host.editor_state_mut().chat;
        if let Some(msg) = chat.messages.last_mut() {
            msg.thinking.push_str(&appended);
            msg.thinking_collapsed = false;
            changed = true;
        }
    }
    if let Some(summary) = &poll.summary {
        let chat = &mut host.editor_state_mut().chat;
        if let Some(msg) = chat.messages.last_mut() {
            match summary {
                Ok(s) => {
                    let ok = s.subtasks.iter().filter(|o| o.error.is_none()).count();
                    let failed = s.subtasks.len() - ok;
                    msg.content.push_str(&format!(
                        "\n\nDone — {} subtask(s) succeeded, {} failed, {} node(s) total.",
                        ok, failed, s.total_nodes,
                    ));
                }
                Err(e) => {
                    msg.content = format!("error: {e}");
                }
            }
            msg.streaming = false;
            changed = true;
        }
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if poll.finished {
        *current = None;
    }
    changed
}

/// Render a list of `Progress` events into a human-readable line block
/// the chat transcript can append. Matches the spirit of TS
/// `apps/web/src/services/ai/visual-ref-orchestrator.ts` step labels.
fn render_progress(progress: &[Progress]) -> String {
    let mut out = String::new();
    for p in progress {
        out.push('\n');
        out.push_str(&progress_label(p));
    }
    out
}

fn progress_label(p: &Progress) -> String {
    match p {
        Progress::Planning => "• Planning…".into(),
        Progress::Planned { subtasks } => {
            // Full task checklist upfront (TS parity) — one row per planned
            // section so the user sees the whole plan immediately.
            let rows: String = subtasks
                .iter()
                .map(|(_, label)| format!("\n  ☐ {label}"))
                .collect();
            format!("• Plan — {} sections:{}", subtasks.len(), rows)
        }
        Progress::ScaffoldDone => "• Scaffold ready".into(),
        Progress::SubtaskStarted { id, label } => format!("• Subtask `{id}` — {label}"),
        Progress::SubtaskDone { id, node_count } => {
            format!("• Subtask `{id}` done ({node_count} nodes)")
        }
        Progress::SubtaskFailed { id, error } => format!("• Subtask `{id}` failed: {error}"),
        Progress::CleanupDone => "• Cleanup done".into(),
        Progress::ValidationStarted => "• Validation started".into(),
        Progress::ValidationPreCheckDone { applied, .. } => {
            format!("• Pre-validation applied {applied} fix(es)")
        }
        Progress::ValidationRoundStarted { round } => {
            format!("• Vision round {round} started")
        }
        Progress::ValidationRoundDone {
            round,
            applied,
            quality_score,
        } => {
            format!("• Vision round {round} done — {applied} fix(es), quality {quality_score}/100")
        }
        Progress::ValidationDone { total_applied } => {
            format!("• Validation done — {total_applied} fix(es) total")
        }
        Progress::VisualRefStarted => "• Visual-ref pipeline started".into(),
        Progress::VisualRefDesignSystem { var_count } => {
            format!("• Design system ready — {var_count} variable(s) seeded")
        }
        Progress::VisualRefHtmlGenerated { byte_len } => {
            format!("• Visual-ref HTML generated ({byte_len} bytes)")
        }
        Progress::VisualRefScreenshotReady { skipped } => {
            if *skipped {
                "• Visual-ref screenshot skipped".into()
            } else {
                "• Visual-ref screenshot captured".into()
            }
        }
        Progress::VisualRefFallback { reason } => format!("• Visual-ref fallback: {reason}"),
    }
}

#[cfg(test)]
#[path = "design_session_tests.rs"]
mod tests;
