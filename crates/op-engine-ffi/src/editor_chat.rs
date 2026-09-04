//! Mobile-editor chat pump — drains `chat.pending_send` and runs the turn.
//!
//! Mobile shells share the desktop chat UI (`begin_send` pushes the user
//! message plus a streaming assistant bubble and raises `pending_send`),
//! but until this pump existed NOTHING on iOS / Android drained that flag:
//! the assistant bubble sat on "Thinking…" forever. This is the mobile
//! counterpart of `op-host-desktop`'s `chat_session::{launch_if_pending,
//! pump}`, scoped to what a phone can actually run:
//!
//! - Built-in (API-key) provider selections stream a plain chat turn over
//!   HTTPS (`editor_chat_turn.rs`).
//! - Built-in selections whose message reads as a DESIGN request run the
//!   REAL desktop design agent loop (`op_chat_agent::chat_agent_loop`):
//!   the shared design-agent system prompt, the shared design toolset
//!   (script-mode `batch_design` included), corrective budgets, and
//!   per-frame tool execution against the live document — real nodes land
//!   in the open `.op` file, desktop-style.
//! - Every other selection (external CLI agents, ACP servers — none of
//!   which exist on a phone) writes an honest error into the assistant
//!   bubble instead of hanging.
//! - Provider failures always land as `error: …` transcript text: the
//!   worker terminates every turn with a `Done` delta and the pump clears
//!   the bubble's streaming flag.
//!
//! Worker tasks run on a dedicated Tokio runtime and only own a value
//! snapshot plus an `mpsc::Sender`; deltas land on the engine thread during
//! `op_frame`, mirroring `editor_model_discovery`.

use std::sync::mpsc;
use std::sync::OnceLock;

use std::sync::Arc;

use op_ai::chat_history::{trim_chat_history, DEFAULT_MAX_CHARS, DEFAULT_MAX_MESSAGES};
use op_ai::chat_provider::{ChatDelta, EffortLevel, CHECK_BLOCKERS_OP, LOOP_FINALIZE_OP};
use op_editor_core::{ChatRole, EditorState};
use op_editor_host_core::chat::{
    apply_poll_to_message_with, chat_history_from_transcript, chat_tool_channel, ChatSession,
    ChatToolRequest,
};
use op_host_native::WidgetHostNative;
use tokio::runtime::{Builder, Runtime};
use tokio::task::AbortHandle;

use op_ai::chat_provider::StopReason;
use op_ai::chat_sse::provider_endpoint;
use op_chat_agent::backoff::DESIGN_LOOP_MAX_TURNS;
use op_chat_agent::chat_agent_loop::{
    run_anthropic_agent_loop, run_openai_agent_loop, AgentLoopConfig,
};
use op_chat_agent::provider_dial::EndpointDialPolicy;
use op_editor_core::BuiltinAgentKind;

use crate::editor_chat_design::{
    clear_fresh_starter_frame_for_design, design_intent, design_turn_disable_thinking,
    mobile_design_system_prompt, reconcile_starter_ghost, root_seed_hints,
    DESIGN_LOOP_MAX_OUTPUT_TOKENS,
};
use crate::editor_chat_design_tools::{attach_tool_result_to_transcript, mobile_design_tool_defs};
use crate::editor_chat_turn::{run_builtin_turn, BuiltinChatTurn};
use crate::lifecycle::Session;

/// Engine-thread repoll cadence while a turn streams (~30 fps), matching
/// the desktop winit loop's wake rate during a chat turn.
const STREAM_POLL_INTERVAL_MS: u64 = 33;

/// Desktop parity: plain chat turns cap the reply budget at 4096 tokens.
const CHAT_MAX_OUTPUT_TOKENS: u32 = 4096;

struct ChatTurnJob {
    session: ChatSession,
    /// Tab this turn is bound to. Deltas keep landing there even after the
    /// user switches tabs mid-stream (desktop `running_tab` parity);
    /// `run_tab_mut` falls back to the active tab if the index went stale.
    running_tab: usize,
    abort: Option<AbortHandle>,
    /// The agent-indicator epoch a design turn opened (frame glows +
    /// entrance reveals). `None` for plain chat turns.
    indicator_epoch: Option<u64>,
    /// At least one design tool changed the live document. Kept separately
    /// from `viewport_fitted`: variable/style writes may precede the first
    /// sized root, but final completion must still fit any resulting content.
    document_mutated: bool,
}

#[derive(Default)]
struct ToolExecutionOutcome {
    state_changed: bool,
    document_mutated: bool,
}

/// Runtime-only chat jobs stay out of serializable editor state. Dropping
/// the engine aborts the in-flight request so a torn-down editor does not
/// keep a credential-bearing stream open until the network timeout.
#[derive(Default)]
pub(crate) struct MobileChatHost {
    turn: Option<ChatTurnJob>,
}

impl Drop for MobileChatHost {
    fn drop(&mut self) {
        self.drop_turn();
    }
}

impl MobileChatHost {
    pub(crate) fn has_background_work(&self, host: &WidgetHostNative) -> bool {
        self.turn.is_some() || host.editor_state().chat.pending_send.is_some()
    }

    fn drop_turn(&mut self) {
        if let Some(job) = self.turn.take() {
            if let Some(abort) = job.abort {
                abort.abort();
            }
        }
    }

    /// Drain widget-raised chat flags, launch a newly requested turn, and
    /// fold streamed deltas into the transcript. Returns the next
    /// engine-thread poll deadline while a turn is in flight.
    pub(crate) fn pump(
        &mut self,
        host: &mut WidgetHostNative,
        now_ms: u64,
        viewport_size: (f32, f32),
    ) -> Option<u64> {
        let mut changed = self.drain_new_chat_and_stop(host);
        changed |= self.launch_if_pending(host);
        changed |= self.poll_into(host, viewport_size);
        if changed {
            host.mark_editor_state_dirty();
        }
        self.turn
            .as_ref()
            .map(|_| now_ms.saturating_add(STREAM_POLL_INTERVAL_MS))
    }

    /// New Chat / Stop only need the worker dropped host-side — the widget
    /// layer already opened the fresh tab / cleared the streaming flags.
    fn drain_new_chat_and_stop(&mut self, host: &mut WidgetHostNative) -> bool {
        let state = host.editor_state_mut();
        let new_chat = std::mem::take(&mut state.chat.pending_new_chat);
        let stop = std::mem::take(&mut state.chat.pending_stop_chat);
        if new_chat || stop {
            self.retire_turn(host);
        }
        new_chat || stop
    }

    /// Tear down the in-flight turn with the design-loop lifecycle honored:
    /// the finalize-lifecycle invariant (desktop
    /// `finalize_design_session_if_needed` parity — a loop that died before
    /// its finalize op still gets one structural pass), the indicator epoch
    /// ended, the "1/1 designing…" header cleared, and the starter ghost
    /// reconciled. Returns true when editor state changed.
    fn retire_turn(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(mut job) = self.turn.take() else {
            return false;
        };
        if let Some(abort) = job.abort.take() {
            abort.abort();
        }
        let mut changed = false;
        if job.session.is_design_loop() {
            eprintln!(
                "openpencil-mobile: design turn retired (finalized={})",
                job.session.loop_finalized()
            );
            if !job.session.loop_finalized() && allow_ai_bulk_write(host) {
                let state = host.editor_state_mut();
                op_orchestrator::apply_loop_finalize(state);
                op_orchestrator::unfilled_screens::finalize_and_mark_unfilled_screens(state);
                changed = true;
            }
            if let Some(epoch) = job.indicator_epoch {
                op_editor_core::agent_indicators::end_if_epoch(epoch);
            }
            let state = host.editor_state_mut();
            state.chat.agents_running = (0, 0);
            changed |= reconcile_starter_ghost(state, false);
        }
        changed
    }

    /// Drain `chat.pending_send` (raised by `ChatState::begin_send`) and
    /// route it: built-in providers launch a streaming turn, anything else
    /// gets the honest mobile-unavailable error.
    fn launch_if_pending(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(user_text) = host.editor_state_mut().chat.pending_send.take() else {
            return false;
        };
        // This turn consumes the staged attachments. The mobile transport is
        // text-only today: the images already ride the user bubble via
        // `begin_send`, they just do not reach the provider.
        host.editor_state_mut().chat.pending_attachments.clear();
        let running_tab = host.editor_state().chat.active_index();
        // A send fired mid-turn replaces the in-flight turn (desktop
        // parity) — the old worker drains harmlessly once its receiver
        // drops.
        self.retire_turn(host);
        match prepare_builtin_turn(host.editor_state(), &user_text) {
            Some(mut turn) => {
                // Design requests run the REAL desktop agent tool loop
                // (`op_chat_agent::chat_agent_loop` — corrective budgets,
                // retry ladder, loop finalize) with the shared design-agent
                // system prompt and the shared design toolset, script-mode
                // batch_design included.
                let design = design_intent(&user_text);
                let launched = if design {
                    let state = host.editor_state_mut();
                    // Clear the starter BEFORE building the prompt so the
                    // consistency brief sees what the model will see.
                    clear_fresh_starter_frame_for_design(state);
                    turn.system_prompt = mobile_design_system_prompt(state, &user_text);
                    turn.max_output_tokens = DESIGN_LOOP_MAX_OUTPUT_TOKENS;
                    let hints = root_seed_hints(state, &user_text);
                    // Chat header shows "1/1 designing…" while the loop runs.
                    state.chat.agents_running = (1, 1);
                    start_design_turn(turn, running_tab, hints)
                } else {
                    start_turn(turn, running_tab)
                };
                match launched {
                    Ok(job) => self.turn = Some(job),
                    Err(message) => {
                        if design {
                            let state = host.editor_state_mut();
                            state.chat.agents_running = (0, 0);
                            // The starter frame was already cleared for this
                            // turn; drop its ghost so a failed launch does not
                            // paint a phantom frame forever.
                            reconcile_starter_ghost(state, false);
                        }
                        write_turn_error(host, running_tab, message);
                    }
                }
            }
            None => {
                let label = selected_provider_label(host.editor_state());
                write_turn_error(
                    host,
                    running_tab,
                    format!(
                        "error: {label} chat is not available on this device — \
                         external CLI and ACP agents require the desktop app. \
                         Configure an API-key provider in Settings → Agents \
                         and pick one of its models via the model chip."
                    ),
                );
            }
        }
        true
    }

    /// Fold everything the worker streamed since the last frame into the
    /// bound tab's trailing assistant bubble, then execute any pending
    /// canvas tool calls against the live editor state (design loop).
    /// Errors land as `error: …` content and `finished` clears the bubble's
    /// streaming flag — the two halves of "never stuck at Thinking…".
    fn poll_into(&mut self, host: &mut WidgetHostNative, viewport_size: (f32, f32)) -> bool {
        let Some(job) = self.turn.as_mut() else {
            return false;
        };
        // Snapshot tool requests BEFORE polling deltas: the worker always
        // publishes ToolUse before forwarding its request, so every captured
        // request's card is already visible to the following poll (desktop
        // pump ordering).
        let tool_requests = job.session.drain_tool_requests();
        let poll = job.session.poll();
        let is_design = job.session.is_design_loop();
        let running_tab = job.running_tab;
        let mut changed = false;
        if !poll.is_idle() {
            let messages = &mut host
                .editor_state_mut()
                .chat
                .run_tab_mut(Some(running_tab))
                .messages;
            if let Some(index) = messages
                .iter()
                .rposition(|message| message.role == ChatRole::Assistant)
            {
                apply_poll_to_message_with(&mut messages[index], &poll, is_design);
                changed = true;
            }
        }
        let tool_execution =
            execute_tool_requests(host, &mut job.session, tool_requests, running_tab);
        changed |= tool_execution.state_changed;
        job.document_mutated |= tool_execution.document_mutated;
        if is_design && tool_execution.document_mutated && !job.session.viewport_fitted() {
            // Desktop parity: as soon as the first real design write lands,
            // stop framing the retired starter document and center the new
            // output. Variable/style-only writes can precede the first root;
            // do not consume the one-shot fit until an actual sized artboard
            // exists. The completion branch performs the final-size fit.
            let has_sized_root = host.editor_state().active_children().iter().any(|node| {
                op_editor_core::PenNodeExt::width_px(node).is_some()
                    && op_editor_core::PenNodeExt::height_px(node).is_some()
            });
            if has_sized_root {
                host.mark_editor_state_dirty();
                host.fit_content_to_viewport(viewport_size.0, viewport_size.1);
                job.session.mark_viewport_fitted();
            }
        }
        if is_design {
            changed |= reconcile_starter_ghost(host.editor_state_mut(), !poll.finished);
        }
        let fit_completed_design = is_design && job.document_mutated;
        if poll.finished {
            changed |= self.retire_turn(host);
            // Natural completion is the one safe final-fit point: every
            // pending tool request has been applied and retire_turn has run
            // the fallback finalize (if needed) plus ghost reconciliation.
            // Stop / New Chat / replacement also retire turns, but do not
            // reach this branch and therefore never yank the user's camera.
            if fit_completed_design {
                let before = host.editor_state().viewport;
                host.mark_editor_state_dirty();
                host.fit_content_to_viewport(viewport_size.0, viewport_size.1);
                changed |= host.editor_state().viewport != before;
            }
        }
        changed
    }
}

/// AI/MCP bulk document writes are disabled while a collaboration session
/// is bound (desktop pump parity).
fn allow_ai_bulk_write(host: &mut WidgetHostNative) -> bool {
    host.gate_collaboration_action(
        op_editor_core::CollabGateAction::Document(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::BulkWrite,
            ),
        ),
        op_editor_core::CollabEditSource::Ai,
    )
}

fn collaboration_ai_rejection() -> op_ai::chat_provider::ChatToolResult {
    op_ai::chat_provider::ChatToolResult {
        content: serde_json::json!({
            "success": false,
            "error": "AI document mutation is unavailable during collaboration"
        })
        .to_string(),
        is_error: true,
    }
}

/// Execute one pump cycle's captured canvas tool requests against the live
/// `EditorState` — the mobile port of the desktop pump's
/// `execute_tool_requests` (`op-host-desktop::chat_session`), including the
/// reserved loop-finalize / blocker-scan interceptions backed by the real
/// `op_orchestrator` passes.
fn execute_tool_requests(
    host: &mut WidgetHostNative,
    session: &mut ChatSession,
    requests: Vec<ChatToolRequest>,
    running_tab: usize,
) -> ToolExecutionOutcome {
    let mut outcome = ToolExecutionOutcome::default();
    for req in requests {
        // Reserved loop-finalize op: run the deterministic structural
        // backstop (or its read-only `checkOnly` probe) over the assembled
        // live document and ack the committed/unfilled/quality report.
        if req.name == LOOP_FINALIZE_OP {
            let check_only = serde_json::from_str::<serde_json::Value>(&req.args_json)
                .ok()
                .and_then(|v| v.get("checkOnly").and_then(|b| b.as_bool()))
                .unwrap_or(false);
            if !check_only && !allow_ai_bulk_write(host) {
                let result = collaboration_ai_rejection();
                let _ = req.ack.send(result);
                continue;
            }
            let state = host.editor_state_mut();
            let mut quality = op_orchestrator::RepairSummary::default();
            let unfilled = if check_only {
                op_orchestrator::unfilled_screens::detect_unfilled_screens(state)
                    .into_iter()
                    .map(|hit| hit.name)
                    .collect::<Vec<_>>()
            } else {
                quality = op_orchestrator::apply_loop_finalize_counted(state);
                let names =
                    op_orchestrator::unfilled_screens::finalize_and_mark_unfilled_screens(state);
                session.mark_loop_finalized();
                outcome.state_changed = true;
                names
            };
            let committed = op_orchestrator::unfilled_screens::list_screen_candidates(state)
                .into_iter()
                .map(|hit| hit.name)
                .collect::<Vec<_>>();
            let _ = req.ack.send(op_ai::chat_provider::ChatToolResult {
                content: serde_json::json!({
                    "success": true,
                    "committed": committed,
                    "unfilled": unfilled,
                    "quality": {
                        "checks": quality
                            .checked()
                            .into_iter()
                            .map(|c| c.key())
                            .collect::<Vec<_>>(),
                        "repairs": quality
                            .repaired()
                            .into_iter()
                            .map(|(check, count)| {
                                serde_json::json!({ "check": check.key(), "count": count })
                            })
                            .collect::<Vec<_>>(),
                        "records": quality
                            .records()
                            .iter()
                            .map(op_orchestrator::RepairRecord::line)
                            .collect::<Vec<_>>(),
                        "notes": quality.notes(),
                    },
                })
                .to_string(),
                is_error: false,
            });
            continue;
        }
        // Reserved unresolved-blocker scan: always read-only.
        if req.name == CHECK_BLOCKERS_OP {
            let report = op_chat_agent::loop_blocker_ledger::detect_blockers(host.editor_state());
            let blockers: Vec<serde_json::Value> = report
                .blockers
                .iter()
                .map(|b| serde_json::json!({ "category": b.category, "detail": b.detail }))
                .collect();
            let _ = req.ack.send(op_ai::chat_provider::ChatToolResult {
                content: serde_json::json!({ "success": true, "blockers": blockers }).to_string(),
                is_error: false,
            });
            continue;
        }
        if !allow_ai_bulk_write(host) {
            let result = collaboration_ai_rejection();
            let state = host.editor_state_mut();
            outcome.state_changed |= attach_tool_result_to_transcript(
                state.chat.run_tab_mut(Some(running_tab)),
                &req.name,
                &result,
            );
            let _ = req.ack.send(result);
            continue;
        }
        // `spawn_agents` is not advertised on mobile (no sub-agent session
        // host); refuse honestly if a model calls it anyway.
        if req.name == "spawn_agents" {
            let result = op_ai::chat_provider::ChatToolResult {
                content: serde_json::json!({
                    "success": true,
                    "data": {
                        "spawned": 0,
                        "agentIds": [],
                        "note": "sub-agents are not available on this device — design every screen yourself"
                    }
                })
                .to_string(),
                is_error: false,
            };
            let state = host.editor_state_mut();
            outcome.state_changed |= attach_tool_result_to_transcript(
                state.chat.run_tab_mut(Some(running_tab)),
                &req.name,
                &result,
            );
            let _ = req.ack.send(result);
            continue;
        }
        let state = host.editor_state_mut();
        let (result, mutated) =
            op_chat_agent::design_agent_tools::execute_agent_tool(state, &req.name, &req.args_json);
        // Same diagnosability rationale as the turn-start line: one line per
        // executed design tool names the call, its outcome, and whether the
        // document changed.
        eprintln!(
            "openpencil-mobile: design tool {} -> {} (mutated={})",
            req.name,
            if result.is_error { "ERROR" } else { "ok" },
            mutated
        );
        if mutated {
            outcome.state_changed = true;
            outcome.document_mutated = true;
        }
        if attach_tool_result_to_transcript(
            state.chat.run_tab_mut(Some(running_tab)),
            &req.name,
            &result,
        ) {
            outcome.state_changed = true;
        }
        let _ = req.ack.send(result);
    }
    outcome
}

impl Session {
    pub(crate) fn pump_editor_chat(&mut self, now_ms: u64) -> Option<u64> {
        let viewport_size = self.editor_viewport();
        let Session { editor, chat, .. } = self;
        editor
            .as_mut()
            .and_then(|host| chat.pump(host, now_ms, viewport_size))
    }
}

/// Spawn the worker task for one prepared turn and park its session.
fn start_turn(turn: BuiltinChatTurn, running_tab: usize) -> Result<ChatTurnJob, String> {
    let runtime = chat_runtime().map_err(|error| format!("error: {error}"))?;
    let (tx, rx) = mpsc::channel::<ChatDelta>();
    let task = runtime.spawn(run_builtin_turn(turn, tx));
    Ok(ChatTurnJob {
        session: ChatSession::from_channels(rx, None),
        running_tab,
        abort: Some(task.abort_handle()),
        indicator_epoch: None,
        document_mutated: false,
    })
}

/// Spawn the REAL desktop design agent loop
/// (`op_chat_agent::chat_agent_loop`) for one prepared turn: the
/// tool-request channel bridges the worker to the engine thread, whose pump
/// executes each call against the live document (`execute_tool_requests`),
/// including the loop's finalize / blocker probes.
fn start_design_turn(
    turn: BuiltinChatTurn,
    running_tab: usize,
    root_seed_hints: (bool, bool),
) -> Result<ChatTurnJob, String> {
    let runtime = chat_runtime().map_err(|error| format!("error: {error}"))?;
    let (executor, tool_rx) = chat_tool_channel();
    let (std_tx, std_rx) = mpsc::channel::<ChatDelta>();
    let disable_thinking = design_turn_disable_thinking(Some(&turn.model));
    let url = match turn.kind {
        BuiltinAgentKind::Anthropic => provider_endpoint(&turn.base_url, "/v1/messages"),
        BuiltinAgentKind::OpenAiCompat => provider_endpoint(&turn.base_url, "/chat/completions"),
    };
    let kind = turn.kind;
    let cfg = AgentLoopConfig {
        url,
        api_key: turn.api_key,
        model: turn.model,
        system_prompt: turn.system_prompt,
        history: turn.history,
        user_prompt: turn.prompt,
        max_output_tokens: turn.max_output_tokens,
        tools: mobile_design_tool_defs(),
        executor: Arc::new(executor),
        max_turns: DESIGN_LOOP_MAX_TURNS,
        finalize_on_exit: true,
        disable_thinking,
        // Operator-entered API-key settings — the same trust level as the
        // desktop settings modal.
        dial_policy: EndpointDialPolicy::Trusted,
    };
    // Low-frequency lifecycle line: release mobile builds emit no logs at
    // all, which turned the first field failure of this path into an
    // hours-long blind diagnosis. One line per design turn is cheap and
    // makes "did the loop even start, on which wire" answerable from the
    // device console.
    eprintln!(
        "openpencil-mobile: design turn start (model={}, wire={:?}, disable_thinking={})",
        cfg.model, kind, disable_thinking
    );
    // Indicator epoch BEFORE the worker can apply its first batch (desktop
    // parity): badges, frame glows, and entrance reveals adopt it.
    let epoch = op_editor_core::agent_indicators::begin_with_root_seed_hint(
        root_seed_hints.0,
        root_seed_hints.1,
    );
    let task = runtime.spawn(async move {
        // The shared loop streams into a tokio channel; forward into the
        // std receiver `ChatSession` polls (std send never blocks).
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatDelta>(64);
        let forward = tokio::spawn(async move {
            while let Some(delta) = rx.recv().await {
                if std_tx.send(delta).is_err() {
                    break;
                }
            }
        });
        let outcome = match kind {
            BuiltinAgentKind::Anthropic => run_anthropic_agent_loop(cfg, &tx).await,
            BuiltinAgentKind::OpenAiCompat => run_openai_agent_loop(cfg, &tx).await,
        };
        // Terminal-Done contract (desktop `send_inner` parity): every exit
        // retires the bubble instead of leaving it on "Thinking…".
        match outcome {
            Ok(true) => {}
            Ok(false) => {
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            }
            Err(error) => {
                let _ = tx.send(ChatDelta::Error(error.to_string())).await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    })
                    .await;
            }
        }
        drop(tx);
        let _ = forward.await;
    });
    Ok(ChatTurnJob {
        session: ChatSession::from_channels(std_rx, Some(tool_rx)).into_design_loop(),
        running_tab,
        abort: Some(task.abort_handle()),
        indicator_epoch: Some(epoch),
        document_mutated: false,
    })
}

/// Resolve the selected chat model into a runnable built-in turn snapshot.
/// `None` when the selection is not a ready built-in (API-key) entry.
fn prepare_builtin_turn(state: &EditorState, user_text: &str) -> Option<BuiltinChatTurn> {
    let entry = state.chat.selected_model_entry()?;
    let id = entry.builtin_provider_id.as_deref()?;
    let selected_model = entry.builtin_model_id()?;
    let config = state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id && agent.ready())?;
    if !config.has_model(selected_model) {
        return None;
    }
    let base_url = if config.base_url.trim().is_empty() {
        config.kind.default_base_url().to_string()
    } else {
        config.base_url.trim().to_string()
    };
    // Per-turn knobs the chat panel carries. The BuiltIn wire has no
    // separate effort channel, so a non-default effort rides in-band as a
    // leading directive (desktop parity).
    let mut prompt = user_text.to_string();
    let effort = state.chat.effort_level;
    if effort != EffortLevel::Low {
        prompt = format!("Apply {} reasoning effort.\n\n{prompt}", effort.as_str());
    }
    // Prior transcript turns, trimmed by the shared sliding-window policy.
    // `begin_send` already pushed this turn's user message + empty
    // assistant bubble; the transcript mapper excludes both.
    let history = trim_chat_history(
        &chat_history_from_transcript(&state.chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    Some(BuiltinChatTurn {
        kind: config.kind,
        api_key: config.api_key.trim().to_string(),
        model: selected_model.to_string(),
        base_url,
        // Plain turns carry no system prompt (the desktop's context-rich
        // chat prompt builder lives in op-host-services, which mobile
        // deliberately does not link). Design turns swap in the shared
        // design-agent prompt at the launch site.
        system_prompt: String::new(),
        history,
        prompt,
        max_output_tokens: CHAT_MAX_OUTPUT_TOKENS,
    })
}

/// Display name for the honest-error path, mirroring the desktop's
/// `selected_provider_label`.
fn selected_provider_label(state: &EditorState) -> String {
    if let Some(entry) = state.chat.selected_model_entry() {
        if let Some(id) = entry.builtin_provider_id.as_deref() {
            if let Some(agent) = state
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|agent| agent.id == id)
            {
                return agent.display_name.clone();
            }
        }
        if let Some(id) = entry.acp_agent_id() {
            if let Some(agent) = state
                .editor_ui
                .agent_settings
                .acp_agents
                .iter()
                .find(|agent| agent.id == id)
            {
                return agent.display_name.clone();
            }
        }
    }
    op_editor_core::AgentProvider::ALL
        .get(state.editor_ui.chat_selected_agent)
        .map(|agent| agent.name().to_string())
        .unwrap_or_else(|| "This agent".into())
}

/// Land an aborted turn's message in the bound tab's assistant bubble and
/// stop its streaming animation (desktop honest-error parity).
fn write_turn_error(host: &mut WidgetHostNative, running_tab: usize, message: String) {
    let chat = host.editor_state_mut().chat.run_tab_mut(Some(running_tab));
    if let Some(msg) = chat
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
    {
        msg.content = message;
        msg.streaming = false;
    }
}

/// Dedicated single-worker runtime for chat turns. Kept separate from the
/// model-discovery runtime so a long streaming turn and a catalog refresh
/// never queue behind each other's blocking phases.
fn chat_runtime() -> Result<&'static Runtime, &'static str> {
    static RUNTIME: OnceLock<Result<Runtime, ()>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .thread_name("op-mobile-chat")
                .build()
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| "chat runtime is unavailable")
}

#[cfg(test)]
#[path = "editor_chat_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "editor_chat_design_tests.rs"]
mod design_tests;
