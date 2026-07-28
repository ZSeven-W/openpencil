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

use op_editor_core::{ChatActivity, ChatActivityStatus, ChatMessage, Locale};
use op_editor_host_core::design::{DesignCmdAck, DesignCmdOp};
// Re-export so `crate::design_session::DesignSession` (the DesktopApp
// field type in main.rs) resolves with zero churn.
pub use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;
use op_orchestrator::Progress;

use op_host_services::design_session::fit_design_viewport_to_content;

#[path = "design_session_workers.rs"]
mod workers;

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
    let mut any_applied = false;
    let mut any_rejected = false;
    for req in reqs {
        let applied = match req.op {
            DesignCmdOp::Apply(cmd) => {
                if !host.gate_collaboration_action(
                    op_editor_core::CollabGateAction::Document(
                        op_editor_core::CollabDocumentMutation::Unsupported(
                            op_editor_core::CollabUnsupportedFeature::BulkWrite,
                        ),
                    ),
                    op_editor_core::CollabEditSource::Ai,
                ) {
                    any_rejected = true;
                    let snapshot = op_editor_core::request_snapshot::narrowed_snapshot(
                        host.editor_state_mut(),
                    );
                    let _ = req.ack.send(DesignCmdAck {
                        applied: false,
                        new_state: snapshot,
                    });
                    continue;
                }
                let state = host.editor_state_mut();
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
        // Narrowed: drops chat/codegen/theme_presets, which grow with
        // session length and are read by nothing in the worker's mirror
        // (see op_editor_core::request_snapshot's consumer audit).
        let snapshot = op_editor_core::request_snapshot::narrowed_snapshot(host.editor_state_mut());
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
    any_applied || any_rejected
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
        changed |=
            workers::apply_progress_to_transcript(&mut chat.messages, &poll.progress, locale);
        if changed {
            let _ = crate::design_loop_indicator::ensure_design_session_transcript_identity(
                host.editor_state_mut(),
                running_tab,
            );
        }
    }
    if let Some(summary) = &poll.summary {
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
        changed |= match summary {
            Ok(summary) => workers::finish_design_success(&mut chat.messages, summary, locale),
            Err(error) => {
                workers::finish_design_error(&mut chat.messages, &error.to_string(), locale)
            }
        };
    } else if poll.finished {
        // A disconnected worker without a terminal `Done` must not leave any
        // worker persona (or the primary bubble) animating forever.
        let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
        changed |= workers::finish_disconnected_design_messages(&mut chat.messages, locale);
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if poll.finished {
        *current = None;
    }
    changed
}

/// Finalize transcript rows for an explicit Stop before the design session is
/// dropped. The widget already cleared every `streaming` bit, so worker
/// metadata and active activities — not streaming state — identify the turn.
pub(crate) fn stop_design_transcript(
    host: &mut WidgetHostNative,
    running_tab: Option<usize>,
) -> bool {
    let locale = host.editor_state().editor_ui.locale;
    let chat = host.editor_state_mut().chat.run_tab_mut(running_tab);
    let changed = workers::stop_design_messages(&mut chat.messages, locale);
    if changed {
        host.mark_editor_state_dirty();
    }
    changed
}

/// Drain a manual subtask-retry click (`chat.pending_subtask_retry`, raised
/// by `ChatState::begin_subtask_retry` when the user clicks a failed row's
/// "Retry" icon) and launch a [`op_host_services::design_session::
/// start_subtask_retry`] worker. Returns true when state changed (a turn
/// launched OR an inline error was written) — mirrors
/// `codegen_session::launch_codegen_if_pending`'s shape.
///
/// A live `current_design` blocks a new launch; the flag is left SET in
/// that case (not cleared) so the SAME click retries on a later frame once
/// the in-flight turn ends, rather than silently dropping it.
pub fn launch_subtask_retry_if_pending(
    host: &mut WidgetHostNative,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if current_design.is_some() {
        return false;
    }
    let Some((msg_idx, subtask_id)) = host.editor_state().chat.pending_subtask_retry.clone() else {
        return false;
    };
    host.editor_state_mut().chat.pending_subtask_retry = None;

    let Some(msg) = host.editor_state().chat.messages.get(msg_idx) else {
        return true;
    };
    let Some(request_json) = msg.design_request_json_for_retry.clone() else {
        write_inline_error(
            host,
            msg_idx,
            "error: nothing to retry — this turn's original request was not retained.",
        );
        return true;
    };
    let Some(entry) = msg
        .failed_subtasks
        .iter()
        .find(|p| p.subtask_id == subtask_id)
        .cloned()
    else {
        // `ChatState::begin_subtask_retry` already gates on this — a
        // defensive no-op, not a user-visible error (nothing was promised).
        return true;
    };
    let request: op_orchestrator::DesignRequest = match serde_json::from_str(&request_json) {
        Ok(r) => r,
        Err(e) => {
            write_inline_error(
                host,
                msg_idx,
                &format!("error: could not restore the original request for retry: {e}"),
            );
            return true;
        }
    };
    let subtask: op_orchestrator::plan::Subtask = match serde_json::from_str(&entry.subtask_json) {
        Ok(s) => s,
        Err(e) => {
            write_inline_error(
                host,
                msg_idx,
                &format!("error: could not restore the failed section's spec for retry: {e}"),
            );
            return true;
        }
    };
    // Whatever provider is CURRENTLY selected — not frozen from the
    // original turn. The user may have switched specifically because the
    // first provider kept failing; `ChatProviderLlmClient` adapts any
    // `ChatProvider` (CLI subprocess or builtin API-key) identically.
    let Some(provider) = crate::chat_session::provider_for_selected_model(host) else {
        write_inline_error(
            host,
            msg_idx,
            "error: no model configured to retry with — pick an agent via the model chip.",
        );
        return true;
    };
    let provider_arc: std::sync::Arc<dyn op_ai::chat_provider::ChatProvider> =
        std::sync::Arc::from(provider);
    let llm = op_host_services::chat_provider_llm::ChatProviderLlmClient::new(provider_arc)
        .with_model(crate::chat_session::selected_cli_model_id(host));
    let initial_state =
        op_editor_core::request_snapshot::narrowed_snapshot(host.editor_state_mut());
    *current_design = Some(op_host_services::design_session::start_subtask_retry(
        llm,
        request,
        subtask,
        initial_state,
    ));
    // Reuse the exact message that owns the failed activity row. This also
    // makes a non-primary worker bubble the explicit target for the retry's
    // unscoped SubtaskStarted/Done/Failed events until its channel closes.
    if let Some(message) = host.editor_state_mut().chat.messages.get_mut(msg_idx) {
        message.streaming = true;
    }
    host.mark_editor_state_dirty();
    true
}

fn write_inline_error(host: &mut WidgetHostNative, msg_idx: usize, text: &str) {
    if let Some(msg) = host.editor_state_mut().chat.messages.get_mut(msg_idx) {
        msg.content.push_str("\n\n");
        msg.content.push_str(text);
    }
    host.mark_editor_state_dirty();
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
            // Not translated — same "diagnostic confirmation line" treatment
            // as the D-lite lines above; this is the geometry_echo in-loop
            // self-correction step announcing itself (keeps the subtask's
            // row Running while it retries against a real layout finding).
            Progress::GeometryEcho { id, issue_count } => update_activity(
                msg,
                id,
                ChatActivityStatus::Running,
                Some(format!("Fixing {issue_count} layout issue(s)…")),
            ),
            // Not translated — this is a diagnostic confirmation line (D-lite
            // "three-piece" visibility fix) rather than a narrated sentence,
            // same treatment as the raw subtask id/error text already
            // embedded in other arms above.
            Progress::ConcurrentGroupsStarted {
                group_count,
                workers,
            } => append_narration(
                msg,
                &format!("• {group_count} screen groups · {workers} workers"),
            ),
            Progress::ScreenGroupsSequential {
                group_count,
                requested_workers,
            } => append_narration(
                msg,
                &format!(
                    "• {group_count} screen groups · sequential (parallel setting: {requested_workers})"
                ),
            ),
            // Top-level routing normally consumes this envelope before
            // `apply_progress` sees it. Recursively unwrapping here keeps the
            // adapter safe for direct callers and nested worker envelopes.
            Progress::WorkerScoped(worker) => apply_progress(
                msg,
                std::slice::from_ref(worker.event.as_ref()),
                locale,
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
            // "承诺-交付" honest report — not translated, same diagnostic
            // confirmation-line treatment as GeometryEcho above. The canvas
            // itself already carries the " (unfilled)" name suffix
            // (`unfilled_screens::mark_unfilled_screens`); this line is the
            // transcript-side half so the user sees it without having to
            // scroll the layer panel.
            Progress::UnfilledScreens { names } => append_narration(
                msg,
                &format!(
                    "• {} screen(s) left unfilled: {}",
                    names.len(),
                    names.join(", ")
                ),
            ),
            // Quality credential — same untranslated diagnostic-line
            // treatment as the two reports above. `remaining` is `None`:
            // the promise-delivery check has not run yet at this point, and
            // an assumed "no issues left" would be a claim, not a fact.
            Progress::QualityChecked { checks, repairs } => {
                match op_host_services::quality_credential::quality_credential_line(
                    &op_ai::chat_provider::QualitySummary {
                        checks: checks.clone(),
                        repairs: repairs.clone(),
                    },
                    None,
                ) {
                    Some(line) => append_narration(msg, line.trim_start()),
                    None => false,
                }
            }
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

#[cfg(test)]
#[path = "design_session_worker_tests.rs"]
mod worker_tests;

#[cfg(test)]
#[path = "design_session_terminal_tests.rs"]
mod terminal_tests;

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
