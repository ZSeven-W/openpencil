//! Desktop GUI pumps for the background design turn — the host-coupled
//! half of the design session.
//!
//! The worker spawn + viewport-fit math live in
//! [`op_host_services::design_session`]; this residual keeps the two UI-loop
//! pumps (`pump_commands` / `pump_progress`, which take `&mut
//! WidgetHostNative` — orphan rule) plus the typed progress adapter they
//! fold into the chat transcript.
//!
//! - UI event loop drains pending `DesignCmdReq` each frame via
//!   [`pump_commands`] — applies on the real state, replies ack.
//! - UI event loop also drains `DesignDelta` via [`pump_progress`] and
//!   renders typed activity in the trailing assistant message.

use op_editor_core::{ChatActivity, ChatActivityStatus, ChatCompletion, ChatMessage, Locale};
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
/// `running_tab` binds the activities + summary to the chat tab this
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
    let locale = host.editor_state().editor_ui.locale;
    let poll = session.poll_progress();
    let mut changed = false;
    if !poll.progress.is_empty() {
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
        if let Some(msg) = chat.messages.last_mut() {
            changed |= apply_progress(msg, &poll.progress, locale);
        }
        if changed {
            let _ = crate::design_loop_indicator::ensure_design_session_transcript_identity(
                host.editor_state_mut(),
                running_tab,
            );
        }
    }
    if let Some(summary) = &poll.summary {
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
        if let Some(msg) = chat.messages.last_mut() {
            match summary {
                Ok(s) => {
                    let ok = s.subtasks.iter().filter(|o| o.error.is_none()).count();
                    let failed = s.subtasks.len() - ok;
                    for activity in &mut msg.activities {
                        if matches!(
                            activity.status,
                            ChatActivityStatus::Pending | ChatActivityStatus::Running
                        ) {
                            activity.status = ChatActivityStatus::Done;
                        }
                    }
                    msg.completion = Some(ChatCompletion {
                        succeeded: count_u32(ok),
                        failed: count_u32(failed),
                        nodes: count_u32(s.total_nodes),
                    });
                    changed |= append_completion_narration(msg, ok, failed, locale);
                }
                Err(e) => {
                    for activity in &mut msg.activities {
                        if matches!(
                            activity.status,
                            ChatActivityStatus::Pending | ChatActivityStatus::Running
                        ) {
                            activity.status = ChatActivityStatus::Error;
                        }
                    }
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

/// Apply typed orchestrator progress to the provider-neutral transcript
/// model. Internal scheduling data (skills, token budgets, dropped context)
/// deliberately remains out of the user-facing message.
fn apply_progress(msg: &mut ChatMessage, progress: &[Progress], locale: Locale) -> bool {
    let mut changed = false;
    for event in progress {
        changed |= match event {
            Progress::Planning => {
                let mut event_changed = append_narration(
                    msg,
                    op_i18n::translate(locale, "ai.designProgress.narration.planning"),
                );
                event_changed |= upsert_activity(
                    msg,
                    "__planning",
                    op_i18n::translate(locale, "ai.designProgress.activity.planning"),
                    ChatActivityStatus::Running,
                    None,
                );
                event_changed
            }
            Progress::Planned { subtasks } => {
                let mut event_changed = remove_activity(msg, "__planning");
                event_changed |= append_narration(msg, &planned_narration(locale, subtasks.len()));
                for (id, label) in subtasks {
                    event_changed |=
                        upsert_activity(msg, id, label, ChatActivityStatus::Pending, None);
                }
                event_changed
            }
            Progress::ScaffoldDone | Progress::SubtaskSkills { .. } => false,
            Progress::SubtaskStarted { id, label } => {
                upsert_activity(msg, id, label, ChatActivityStatus::Running, None)
            }
            Progress::SubtaskDone { id, node_count } => update_activity(
                msg,
                id,
                ChatActivityStatus::Done,
                Some(element_count(locale, *node_count)),
            ),
            Progress::SubtaskFailed { id, .. } => update_activity(
                msg,
                id,
                ChatActivityStatus::Error,
                Some(op_i18n::translate(locale, "ai.designProgress.detail.needsAttention").into()),
            ),
            Progress::SubtaskRetry { id, attempt, .. } => update_activity(
                msg,
                id,
                ChatActivityStatus::Running,
                Some(
                    op_i18n::translate(locale, "ai.designProgress.detail.retrying")
                        .replace("{{attempt}}", &attempt.to_string()),
                ),
            ),
            Progress::SubtaskNodes { id, nodes_so_far } => update_activity(
                msg,
                id,
                ChatActivityStatus::Running,
                Some(element_count(locale, *nodes_so_far)),
            ),
            Progress::CleanupDone => {
                let mut event_changed = append_narration(
                    msg,
                    op_i18n::translate(locale, "ai.designProgress.narration.polishing"),
                );
                event_changed |= upsert_activity(
                    msg,
                    "__polish",
                    op_i18n::translate(locale, "ai.designProgress.activity.polishing"),
                    ChatActivityStatus::Done,
                    None,
                );
                event_changed
            }
            Progress::ValidationStarted => {
                let mut event_changed = append_narration(
                    msg,
                    op_i18n::translate(locale, "ai.designProgress.narration.checking"),
                );
                event_changed |= upsert_activity(
                    msg,
                    "__validation",
                    op_i18n::translate(locale, "ai.designProgress.activity.checking"),
                    ChatActivityStatus::Running,
                    None,
                );
                event_changed
            }
            Progress::ValidationPreCheckDone { .. }
            | Progress::ValidationRoundStarted { .. }
            | Progress::ValidationRoundDone { .. } => update_activity(
                msg,
                "__validation",
                ChatActivityStatus::Running,
                Some(op_i18n::translate(locale, "ai.designProgress.detail.refining").into()),
            ),
            Progress::ValidationDone { .. } => {
                update_activity(msg, "__validation", ChatActivityStatus::Done, None)
            }
            Progress::VisualRefStarted => {
                let mut event_changed = append_narration(
                    msg,
                    op_i18n::translate(locale, "ai.designProgress.narration.visualReference"),
                );
                event_changed |= upsert_activity(
                    msg,
                    "__visual_ref",
                    op_i18n::translate(locale, "ai.designProgress.activity.visualReference"),
                    ChatActivityStatus::Running,
                    None,
                );
                event_changed
            }
            Progress::VisualRefDesignSystem { .. }
            | Progress::VisualRefHtmlGenerated { .. }
            | Progress::VisualRefScreenshotReady { .. } => {
                update_activity(msg, "__visual_ref", ChatActivityStatus::Running, None)
            }
            Progress::VisualRefFallback { .. } => update_activity(
                msg,
                "__visual_ref",
                ChatActivityStatus::Done,
                Some(op_i18n::translate(locale, "ai.designProgress.detail.standardPath").into()),
            ),
        };
    }
    changed
}

fn upsert_activity(
    msg: &mut ChatMessage,
    id: &str,
    title: &str,
    status: ChatActivityStatus,
    detail: Option<String>,
) -> bool {
    let content_offset = Some(count_u32(msg.content.len()));
    if let Some(activity) = msg.activities.iter_mut().find(|item| item.id == id) {
        let next = ChatActivity {
            id: id.to_string(),
            title: title.to_string(),
            detail,
            status,
            content_offset: activity.content_offset.or(content_offset),
        };
        if *activity == next {
            false
        } else {
            *activity = next;
            true
        }
    } else {
        msg.activities.push(ChatActivity {
            id: id.to_string(),
            title: title.to_string(),
            detail,
            status,
            content_offset,
        });
        true
    }
}

fn update_activity(
    msg: &mut ChatMessage,
    id: &str,
    status: ChatActivityStatus,
    detail: Option<String>,
) -> bool {
    if let Some(activity) = msg.activities.iter_mut().find(|item| item.id == id) {
        let changed = activity.status != status || activity.detail != detail;
        activity.status = status;
        activity.detail = detail;
        changed
    } else {
        upsert_activity(msg, id, id, status, detail)
    }
}

fn remove_activity(msg: &mut ChatMessage, id: &str) -> bool {
    let before = msg.activities.len();
    msg.activities.retain(|activity| activity.id != id);
    msg.activities.len() != before
}

fn element_count(locale: Locale, count: usize) -> String {
    let key = if count == 1 {
        "ai.designProgress.detail.elementOne"
    } else {
        "ai.designProgress.detail.elementMany"
    };
    op_i18n::translate(locale, key).replace("{{count}}", &count.to_string())
}

fn planned_narration(locale: Locale, count: usize) -> String {
    let key = if count == 1 {
        "ai.designProgress.narration.plannedOne"
    } else {
        "ai.designProgress.narration.plannedMany"
    };
    op_i18n::translate(locale, key).replace("{{count}}", &count.to_string())
}

fn append_narration(msg: &mut ChatMessage, text: &str) -> bool {
    if text.is_empty() || msg.content.contains(text) {
        return false;
    }
    if !msg.content.trim().is_empty() {
        msg.content.push_str("\n\n");
    }
    msg.content.push_str(text);
    true
}

fn append_completion_narration(
    msg: &mut ChatMessage,
    succeeded: usize,
    failed: usize,
    locale: Locale,
) -> bool {
    let text = if failed == 0 {
        let key = if succeeded == 0 {
            "ai.designProgress.completion.empty"
        } else if succeeded == 1 {
            "ai.designProgress.completion.one"
        } else {
            "ai.designProgress.completion.many"
        };
        op_i18n::translate(locale, key).replace("{{count}}", &succeeded.to_string())
    } else {
        op_i18n::translate(locale, "ai.designProgress.completion.issues")
            .replace("{{completed}}", &succeeded.to_string())
            .replace("{{failed}}", &failed.to_string())
    };
    append_narration(msg, &text)
}

fn count_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
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
