//! Screen-group worker transcript routing for classic orchestrator sessions.

use op_editor_core::{ChatActivityStatus, ChatCompletion, ChatMessage, ChatRole, Locale};
use op_orchestrator::{Progress, RunSummary, WorkerEvent};

use super::{
    append_completion_narration, append_narration, apply_progress, count_u32, friendly_quota_error,
    subtask_failure_detail, update_activity, upsert_activity,
};

/// Route global progress to the primary message and worker-scoped progress to
/// one stable assistant bubble per screen group. Group zero adopts the primary
/// bubble; every later group appends a metadata-tagged worker bubble.
pub(super) fn apply_progress_to_transcript(
    messages: &mut Vec<ChatMessage>,
    progress: &[Progress],
    locale: Locale,
) -> bool {
    let mut changed = false;
    for event in progress {
        match event {
            Progress::WorkerScoped(worker) => {
                changed |= apply_worker_progress(messages, worker, locale);
            }
            event => {
                if let Some(index) = unscoped_design_message_index(messages, event) {
                    changed |=
                        apply_progress(&mut messages[index], std::slice::from_ref(event), locale);
                }
            }
        }
    }
    changed
}

fn apply_worker_progress(
    messages: &mut Vec<ChatMessage>,
    worker: &WorkerEvent,
    locale: Locale,
) -> bool {
    // Nested envelopes are legal because the inner event is boxed. Prefer the
    // innermost scope when an adapter re-wraps an event.
    if let Progress::WorkerScoped(inner) = worker.event.as_ref() {
        return apply_worker_progress(messages, inner, locale);
    }

    let Some(primary) = primary_design_message_index(messages) else {
        return false;
    };
    if worker.group_idx == 0 {
        let message = &mut messages[primary];
        let mut changed = stamp_worker_identity(message, worker);
        if message.design_worker_screen.as_deref() != Some(worker.screen.as_str()) {
            message.design_worker_screen = Some(worker.screen.clone());
            append_narration(
                message,
                &worker_started_narration(locale, &worker.screen, worker.group_idx),
            );
            changed = true;
        }
        changed |= apply_progress(message, std::slice::from_ref(worker.event.as_ref()), locale);
        return changed;
    }

    let group = count_u32(worker.group_idx);
    let current_start = current_design_turn_start(messages);
    let existing = messages
        .iter()
        .enumerate()
        .skip(current_start)
        .find(|(_, message)| message.design_worker_group == Some(group))
        .map(|(index, _)| index);
    let (index, mut changed) = match existing {
        Some(index) => (index, false),
        None => {
            let request_json = messages[primary].design_request_json_for_retry.clone();
            let mut message = ChatMessage::assistant_streaming();
            message.design_worker_group = Some(group);
            message.design_worker_screen = Some(worker.screen.clone());
            message.agent_name = Some(worker.identity.name.clone());
            message.agent_color = Some(worker.identity.color.clone());
            message.design_request_json_for_retry = request_json;
            message.content = worker_started_narration(locale, &worker.screen, worker.group_idx);
            messages.push(message);
            (messages.len() - 1, true)
        }
    };

    changed |= migrate_worker_activities(messages, primary, index, worker.event.as_ref());
    let message = &mut messages[index];
    changed |= stamp_worker_identity(message, worker);
    if message.design_worker_screen.as_deref() != Some(worker.screen.as_str()) {
        message.design_worker_screen = Some(worker.screen.clone());
        changed = true;
    }
    changed |= apply_progress(message, std::slice::from_ref(worker.event.as_ref()), locale);
    changed
}

fn stamp_worker_identity(message: &mut ChatMessage, worker: &WorkerEvent) -> bool {
    let mut changed = false;
    if message.agent_name.as_deref() != Some(worker.identity.name.as_str()) {
        message.agent_name = Some(worker.identity.name.clone());
        changed = true;
    }
    if message.agent_color.as_deref() != Some(worker.identity.color.as_str()) {
        message.agent_color = Some(worker.identity.color.clone());
        changed = true;
    }
    changed
}

fn migrate_worker_activities(
    messages: &mut [ChatMessage],
    primary: usize,
    worker: usize,
    event: &Progress,
) -> bool {
    let ids = worker_event_ids(event);
    if ids.is_empty() {
        return false;
    }
    let mut moved = Vec::new();
    for id in ids {
        if let Some(position) = messages[primary]
            .activities
            .iter()
            .position(|activity| activity.id == id)
        {
            moved.push(messages[primary].activities.remove(position));
        }
    }
    let mut changed = !moved.is_empty();
    for mut activity in moved {
        if messages[worker]
            .activities
            .iter()
            .any(|existing| existing.id == activity.id)
        {
            continue;
        }
        activity.content_offset = Some(count_u32(messages[worker].content.len()));
        messages[worker].activities.push(activity);
        changed = true;
    }
    changed
}

fn worker_event_ids(event: &Progress) -> Vec<&str> {
    match event {
        Progress::Planned { subtasks } => subtasks.iter().map(|(id, _)| id.as_str()).collect(),
        Progress::SubtaskStarted { id, .. }
        | Progress::SubtaskDone { id, .. }
        | Progress::SubtaskFailed { id, .. }
        | Progress::SubtaskSkills { id, .. }
        | Progress::SubtaskRetry { id, .. }
        | Progress::GeometryEcho { id, .. }
        | Progress::SubtaskNodes { id, .. } => vec![id.as_str()],
        Progress::WorkerScoped(worker) => worker_event_ids(worker.event.as_ref()),
        _ => Vec::new(),
    }
}

fn current_design_turn_start(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .rposition(|message| message.role == ChatRole::User)
        .map_or(0, |index| index + 1)
}

fn primary_design_message_index(messages: &[ChatMessage]) -> Option<usize> {
    messages
        .iter()
        .enumerate()
        .skip(current_design_turn_start(messages))
        .find(|(_, message)| {
            message.role == ChatRole::Assistant
                && message.design_worker_group.is_none()
                // The CLI-standard router parks a companion ChatSession beside
                // the real DesignSession. At terminal time that chat channel
                // can disconnect first and clear `streaming` before the design
                // pump drains ValidationDone + RunSummary. Once typed design
                // activities exist they are the durable ownership marker; the
                // launch-time request marker also covers a fast run whose first
                // progress batch arrives after that disconnect.
                && (message.streaming
                    || !message.activities.is_empty()
                    || message.design_request_json_for_retry.is_some())
        })
        .map(|(index, _)| index)
}

/// Unscoped events normally belong to the primary design bubble. A manual
/// single-subtask retry is the exception: it deliberately reuses the completed
/// message that owns the failed row, including a non-primary worker bubble.
/// The launcher marks only that message streaming, so route id-bearing retry
/// progress back to the activity owner before falling back to the primary.
fn unscoped_design_message_index(messages: &[ChatMessage], event: &Progress) -> Option<usize> {
    let ids = worker_event_ids(event);
    if !ids.is_empty() {
        let mut owners = messages
            .iter()
            .enumerate()
            .filter(|(_, message)| {
                message.role == ChatRole::Assistant
                    && message.streaming
                    && message
                        .activities
                        .iter()
                        .any(|activity| ids.contains(&activity.id.as_str()))
            })
            .map(|(index, _)| index);
        if let Some(index) = owners.next() {
            // Activity ids are normally run-unique. Be conservative if a
            // legacy transcript contains two simultaneously-streaming rows
            // with the same id instead of updating an arbitrary bubble.
            if owners.next().is_none() {
                return Some(index);
            }
        }
    }
    primary_design_message_index(messages)
}

fn current_design_message_indices(messages: &[ChatMessage]) -> Vec<usize> {
    let current: Vec<_> = messages
        .iter()
        .enumerate()
        .skip(current_design_turn_start(messages))
        .filter(|(_, message)| {
            message.role == ChatRole::Assistant
                && (message.streaming
                    || message.design_worker_group.is_some()
                    || !message.activities.is_empty()
                    || message.design_request_json_for_retry.is_some())
        })
        .map(|(index, _)| index)
        .collect();
    if !current.is_empty() {
        return current;
    }

    // Manual retry can reopen a failed activity in an older turn. When the
    // latest user turn has no streaming bubble, fall back to the globally
    // streaming design/activity bubble so its disconnect can close that row.
    messages
        .iter()
        .enumerate()
        .filter(|(_, message)| {
            message.role == ChatRole::Assistant
                && message.streaming
                && (message.design_worker_group.is_some() || !message.activities.is_empty())
        })
        .map(|(index, _)| index)
        .collect()
}

fn current_owned_design_message_indices(messages: &[ChatMessage]) -> Vec<usize> {
    // Do not use `design_request_json_for_retry` here: CLI-standard stashes it
    // before classifying the turn, so ordinary Chat and Modify bubbles have it
    // while their deliberately-unused DesignSession disconnects.
    let current: Vec<_> = messages
        .iter()
        .enumerate()
        .skip(current_design_turn_start(messages))
        .filter(|(_, message)| {
            message.role == ChatRole::Assistant
                && (message.design_worker_group.is_some() || !message.activities.is_empty())
        })
        .map(|(index, _)| index)
        .collect();
    if !current.is_empty() {
        return current;
    }

    messages
        .iter()
        .enumerate()
        .filter(|(_, message)| {
            message.role == ChatRole::Assistant
                && message.streaming
                && (message.design_worker_group.is_some() || !message.activities.is_empty())
        })
        .map(|(index, _)| index)
        .collect()
}

pub(super) fn finish_design_success(
    messages: &mut [ChatMessage],
    summary: &RunSummary,
    locale: Locale,
) -> bool {
    let indices = current_design_message_indices(messages);
    let Some(primary) = indices
        .iter()
        .copied()
        .find(|&index| messages[index].design_worker_group.is_none())
    else {
        return false;
    };
    let request_json = messages[primary].design_request_json_for_retry.clone();
    let mut stopped_workers = Vec::new();
    let mut unreported_active = 0usize;

    // A concurrent run can return a partial summary after cancellation. Only
    // an explicit successful outcome may turn an active row Done; a row that
    // never produced an outcome was stopped and must remain visibly failed.
    for &index in &indices {
        let is_worker = messages[index].design_worker_group.is_some();
        for activity in &mut messages[index].activities {
            if !matches!(
                activity.status,
                ChatActivityStatus::Pending | ChatActivityStatus::Running
            ) {
                continue;
            }
            // Double-underscore rows are pipeline stages (planning, cleanup,
            // validation), not screen subtasks and therefore never appear in
            // `RunSummary::subtasks`. A successful terminal summary closes
            // those stage rows normally.
            if activity.id.starts_with("__") {
                activity.status = ChatActivityStatus::Done;
                continue;
            }
            match summary
                .subtasks
                .iter()
                .find(|outcome| outcome.id == activity.id)
            {
                Some(outcome) if outcome.error.is_none() => {
                    activity.status = ChatActivityStatus::Done;
                }
                Some(_) => {
                    activity.status = ChatActivityStatus::Error;
                }
                None => {
                    activity.status = ChatActivityStatus::Error;
                    activity.detail = Some(
                        op_i18n::translate(locale, "ai.designProgress.detail.noResult").into(),
                    );
                    unreported_active += 1;
                    if is_worker && !stopped_workers.contains(&index) {
                        stopped_workers.push(index);
                    }
                }
            }
        }
    }

    for outcome in &summary.subtasks {
        let target = indices
            .iter()
            .copied()
            .find(|&index| {
                messages[index]
                    .activities
                    .iter()
                    .any(|activity| activity.id == outcome.id)
            })
            .unwrap_or(primary);
        if let Some(error) = outcome.error.as_deref() {
            let detail = Some(subtask_failure_detail(locale, error));
            if messages[target]
                .activities
                .iter()
                .any(|activity| activity.id == outcome.id)
            {
                update_activity(
                    &mut messages[target],
                    &outcome.id,
                    ChatActivityStatus::Error,
                    detail,
                );
            } else {
                let title = outcome
                    .subtask
                    .as_ref()
                    .map(|subtask| subtask.label.as_str())
                    .unwrap_or(outcome.id.as_str());
                upsert_activity(
                    &mut messages[target],
                    &outcome.id,
                    title,
                    ChatActivityStatus::Error,
                    detail,
                );
            }
        }
        if let Some(subtask) = &outcome.subtask {
            if let Ok(subtask_json) = serde_json::to_string(subtask) {
                let message = &mut messages[target];
                if message.design_request_json_for_retry.is_none() {
                    message.design_request_json_for_retry = request_json.clone();
                }
                if !message
                    .failed_subtasks
                    .iter()
                    .any(|pending| pending.subtask_id == outcome.id)
                {
                    message
                        .failed_subtasks
                        .push(op_editor_core::PendingSubtaskRetry {
                            subtask_id: outcome.id.clone(),
                            subtask_json,
                        });
                }
            }
        }
    }

    let ok = summary
        .subtasks
        .iter()
        .filter(|outcome| outcome.error.is_none())
        .count();
    let failed = summary.subtasks.len() - ok + unreported_active;
    messages[primary].completion = Some(ChatCompletion {
        succeeded: count_u32(ok),
        failed: count_u32(failed),
        nodes: count_u32(summary.total_nodes),
    });
    append_completion_narration(&mut messages[primary], ok, failed, locale);

    for &index in &indices {
        if messages[index].design_worker_group.is_some() {
            let terminal = if stopped_workers.contains(&index) {
                worker_stopped_narration(locale, messages[index].design_worker_screen.as_deref())
            } else {
                let has_error = messages[index]
                    .activities
                    .iter()
                    .any(|activity| activity.status == ChatActivityStatus::Error);
                worker_finished_narration(
                    locale,
                    messages[index].design_worker_screen.as_deref(),
                    has_error,
                )
            };
            append_narration(&mut messages[index], &terminal);
        }
        messages[index].streaming = false;
    }
    true
}

pub(super) fn finish_design_error(messages: &mut [ChatMessage], raw: &str, locale: Locale) -> bool {
    let indices = current_design_message_indices(messages);
    let Some(primary) = indices
        .iter()
        .copied()
        .find(|&index| messages[index].design_worker_group.is_none())
    else {
        return false;
    };
    let detail = subtask_failure_detail(locale, raw);
    for &index in &indices {
        mark_active_activities_error(&mut messages[index], &detail);
        if messages[index].design_worker_group.is_some() {
            let terminal =
                worker_stopped_narration(locale, messages[index].design_worker_screen.as_deref());
            append_narration(&mut messages[index], &terminal);
        }
        messages[index].streaming = false;
    }
    let primary_message = &mut messages[primary];
    primary_message.content = match friendly_quota_error(raw) {
        Some(friendly) => {
            primary_message.thinking.push_str("\n\n");
            primary_message.thinking.push_str(raw);
            friendly
        }
        None => format!("error: {raw}"),
    };
    true
}

pub(super) fn finish_disconnected_design_messages(
    messages: &mut [ChatMessage],
    locale: Locale,
) -> bool {
    // Unlike an explicit Done payload, a bare disconnect can come from the
    // unused DesignSession parked beside an ordinary CLI chat request. Require
    // durable design ownership here so that channel cannot terminate the real
    // plain-chat bubble.
    let indices = current_owned_design_message_indices(messages);
    let detail = op_i18n::translate(locale, "ai.designProgress.detail.connectionClosed");
    for &index in &indices {
        let had_active_activity = messages[index].activities.iter().any(|activity| {
            matches!(
                activity.status,
                ChatActivityStatus::Pending | ChatActivityStatus::Running
            )
        });
        mark_active_activities_error(&mut messages[index], detail);
        // A manual retry closes its channel after sending SubtaskDone/Failed
        // instead of a whole-turn summary. In that normal path the row is
        // already terminal, so merely stop streaming; only an abrupt
        // disconnect with unfinished work should narrate that the worker
        // stopped.
        if had_active_activity && messages[index].design_worker_group.is_some() {
            let terminal =
                worker_stopped_narration(locale, messages[index].design_worker_screen.as_deref());
            append_narration(&mut messages[index], &terminal);
        }
        messages[index].streaming = false;
    }
    !indices.is_empty()
}

/// Finalize an explicitly-stopped design turn. Unlike the normal completion
/// helpers this cannot depend on `message.streaming`: `ChatState::stop_streaming`
/// clears those flags synchronously before the host drains the Stop request.
pub(super) fn stop_design_messages(messages: &mut [ChatMessage], locale: Locale) -> bool {
    let current_start = current_design_turn_start(messages);
    let mut indices: Vec<_> = messages
        .iter()
        .enumerate()
        .skip(current_start)
        .filter(|(_, message)| {
            message.role == ChatRole::Assistant
                && message.activities.iter().any(|activity| {
                    matches!(
                        activity.status,
                        ChatActivityStatus::Pending | ChatActivityStatus::Running
                    )
                })
        })
        .map(|(index, _)| index)
        .collect();
    if indices.is_empty() {
        // A manual retry may target a failed row in an older turn. Its
        // streaming bit is already gone too, but the Running row still
        // identifies the exact bubble that the explicit Stop must close.
        indices = messages
            .iter()
            .enumerate()
            .filter(|(_, message)| {
                message.role == ChatRole::Assistant
                    && message.activities.iter().any(|activity| {
                        matches!(
                            activity.status,
                            ChatActivityStatus::Pending | ChatActivityStatus::Running
                        )
                    })
            })
            .map(|(index, _)| index)
            .collect();
    }

    let mut changed = false;
    let detail = op_i18n::translate(locale, "ai.designProgress.detail.stoppedByUser");
    for index in indices {
        let had_active = messages[index].activities.iter().any(|activity| {
            matches!(
                activity.status,
                ChatActivityStatus::Pending | ChatActivityStatus::Running
            )
        });
        changed |= mark_active_activities_error(&mut messages[index], detail);
        if messages[index].design_worker_group.is_some() && had_active {
            let terminal =
                worker_stopped_narration(locale, messages[index].design_worker_screen.as_deref());
            changed |= append_narration(&mut messages[index], &terminal);
        }
        if messages[index].streaming {
            messages[index].streaming = false;
            changed = true;
        }
    }
    changed
}

fn mark_active_activities_error(message: &mut ChatMessage, detail: &str) -> bool {
    let mut changed = false;
    for activity in &mut message.activities {
        if matches!(
            activity.status,
            ChatActivityStatus::Pending | ChatActivityStatus::Running
        ) {
            let next_detail = Some(detail.to_owned());
            changed |=
                activity.status != ChatActivityStatus::Error || activity.detail != next_detail;
            activity.status = ChatActivityStatus::Error;
            activity.detail = next_detail;
        }
    }
    changed
}

fn worker_started_narration(locale: Locale, screen: &str, group_idx: usize) -> String {
    let screen = if screen.trim().is_empty() {
        format!("Screen {}", group_idx.saturating_add(1))
    } else {
        screen.trim().to_string()
    };
    match locale {
        Locale::ZhCn => format!("正在设计 **{screen}**…"),
        Locale::ZhTw => format!("正在設計 **{screen}**…"),
        _ => format!("Designing **{screen}**…"),
    }
}

fn worker_finished_narration(locale: Locale, screen: Option<&str>, has_error: bool) -> String {
    let screen = screen
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("screen");
    match (locale, has_error) {
        (Locale::ZhCn, false) => format!("**{screen}** 已完成。"),
        (Locale::ZhCn, true) => format!("**{screen}** 已结束；失败区块已展开并标明具体原因。"),
        (Locale::ZhTw, false) => format!("**{screen}** 已完成。"),
        (Locale::ZhTw, true) => format!("**{screen}** 已結束；失敗區塊已展開並標明具體原因。"),
        (_, false) => format!("Finished **{screen}**."),
        (_, true) => {
            format!("Finished **{screen}**; failed sections are expanded with their reasons.")
        }
    }
}

fn worker_stopped_narration(locale: Locale, screen: Option<&str>) -> String {
    let screen = screen
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("screen");
    match locale {
        Locale::ZhCn => format!("**{screen}** 的设计已停止。"),
        Locale::ZhTw => format!("**{screen}** 的設計已停止。"),
        _ => format!("Stopped designing **{screen}**."),
    }
}
