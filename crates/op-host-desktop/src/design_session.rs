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
///
/// `running_tab` binds the progress lines + summary to the chat tab this
/// design turn started on (MT.3 session-per-tab), so switching the active tab
/// mid-run doesn't fold deltas into the wrong tab. `None` / out-of-range falls
/// back to the active tab.
pub fn pump_progress(
    host: &mut WidgetHostNative,
    current: &mut Option<DesignSession>,
    running_tab: Option<usize>,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll_progress();
    let mut changed = false;
    if !poll.progress.is_empty() {
        let appended = render_progress(&poll.progress);
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
        if let Some(msg) = chat.messages.last_mut() {
            msg.thinking.push_str(&appended);
            msg.thinking_collapsed = false;
            changed = true;
        }
    }
    if let Some(summary) = &poll.summary {
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
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
                    let raw = e.to_string();
                    msg.content = match friendly_quota_error(&raw) {
                        Some(friendly) => {
                            // Raw provider JSON stays available in the
                            // collapsible thinking block for debugging.
                            msg.thinking.push_str("\n\n");
                            msg.thinking.push_str(&raw);
                            friendly
                        }
                        None => format!("error: {raw}"),
                    };
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
        Progress::SubtaskSkills {
            id,
            included,
            dropped,
            budget_used,
            budget_max,
        } => format_subtask_skills(id, included, dropped, *budget_used, *budget_max),
        Progress::SubtaskRetry {
            attempt, reason, ..
        } => {
            format!("  ▸ retry #{attempt}: {reason}")
        }
        Progress::SubtaskNodes { id, nodes_so_far } => {
            format!("• Subtask `{id}` — {nodes_so_far} node(s) so far")
        }
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

/// Format a `SubtaskSkills` payload into the concise summary line plus
/// indented `▸ skills:` / `▸ dropped:` detail sub-lines (spec Component 5).
/// The Component-6 UI parser reads `  ▸ ` sub-lines back into checklist details.
fn format_subtask_skills(
    id: &str,
    included: &[op_orchestrator::SkillBrief],
    dropped: &[(String, String)],
    budget_used: u32,
    budget_max: u32,
) -> String {
    let mut out = format!(
        "• Subtask `{id}`  ·  {} skills · {budget_used}/{budget_max} tok · {} dropped",
        included.len(),
        dropped.len(),
    );
    if !included.is_empty() {
        let names: Vec<String> = included
            .iter()
            .map(|s| {
                if s.truncated {
                    format!("{} (truncated)", s.name)
                } else {
                    s.name.clone()
                }
            })
            .collect();
        out.push_str(&format!("\n  ▸ skills: {}", names.join(", ")));
    }
    if !dropped.is_empty() {
        let drops: Vec<String> = dropped.iter().map(|(n, r)| format!("{n} ({r})")).collect();
        out.push_str(&format!("\n  ▸ dropped: {}", drops.join(", ")));
    }
    out
}

#[cfg(test)]
#[path = "design_session_tests.rs"]
mod tests;

/// Render a provider quota-exhaustion error (HTTP 429 with an
/// `AccountQuotaExceeded`-style body) as one human sentence instead of
/// raw JSON. Extracts the reset timestamp when the provider names one
/// ("It will reset at 2026-07-10 16:59:53 +0800 CST."). `None` for
/// every other error so the raw message keeps rendering unchanged.
fn friendly_quota_error(raw: &str) -> Option<String> {
    let quota_shaped = raw.contains("AccountQuotaExceeded")
        || (raw.contains("429") && raw.to_ascii_lowercase().contains("quota"));
    if !quota_shaped {
        return None;
    }
    let reset = raw.find("reset at ").map(|i| {
        let tail = &raw[i + "reset at ".len()..];
        let end = tail
            .find(". ")
            .or_else(|| tail.find('"'))
            .unwrap_or_else(|| tail.find('.').unwrap_or(tail.len()));
        tail[..end].trim().to_string()
    });
    Some(match reset {
        Some(when) if !when.is_empty() => format!(
            "Model quota exhausted — the provider's usage window is used up. It resets at \
             {when}; generation will work again after that, or switch to another model for now."
        ),
        _ => "Model quota exhausted — the provider's usage window is used up. Wait for the \
              quota to reset, or switch to another model for now."
            .to_string(),
    })
}

#[cfg(test)]
mod quota_error_tests {
    use super::friendly_quota_error;

    #[test]
    fn ark_quota_json_renders_one_friendly_sentence_with_reset_time() {
        let raw = r#"orchestration failed: openai-compatible http 429 Too Many Requests: {"error":{"code":"AccountQuotaExceeded","message":"You have exceeded the 5-hour usage quota. It will reset at 2026-07-10 16:59:53 +0800 CST. We recommend upgrading your plan for more quota, or waiting for the reset. Request id: 0217","param":"","type":"TooManyRequests"}}"#;
        let friendly = friendly_quota_error(raw).expect("quota-shaped error");
        assert!(
            friendly.contains("2026-07-10 16:59:53 +0800 CST"),
            "{friendly}"
        );
        assert!(
            !friendly.contains('{'),
            "no raw JSON in the friendly line: {friendly}"
        );
    }

    #[test]
    fn non_quota_errors_pass_through() {
        assert!(friendly_quota_error("orchestration failed: http 500 internal").is_none());
        assert!(friendly_quota_error("parse error in subtask").is_none());
    }
}
