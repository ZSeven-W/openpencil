//! Tool-executing loops for builtin Anthropic and OpenAI-compatible providers.
//! Canvas tool calls stream from the model, execute through the injected
//! [`ChatToolExecutor`], and ride the next request as correlated results until
//! the model stops or the turn cap is reached. Production uses the UI-thread
//! bridge; loopback tests keep this transport layer deterministic.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatToolDef, ChatToolExecutor, ChatToolResult, FinalizeReport,
    QualitySummary, StopReason, UnfilledScreensReport,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;

#[cfg(test)]
use crate::chat_agent_context::screenshot_image_base64;
use crate::chat_agent_context::{
    elide_inline_screenshots, prepare_screenshot_for_context, PreparedScreenshot,
    OMITTED_SCREENSHOT_TEXT, TEXT_ONLY_SCREENSHOT_TEXT,
};
use crate::chat_builtin_http::{
    map_anthropic_stop_reason, map_openai_stop_reason, BuiltinHttpError,
};

#[path = "chat_agent_loop_retry.rs"]
mod retry;
use retry::{
    is_correctable_write_failure, is_write_level, CorrectiveWriteRetry, CORRECTIVE_WRITE_EXHAUSTED,
    CORRECTIVE_WRITE_PROGRESS,
};

#[path = "chat_agent_loop_blockers.rs"]
mod blockers;
use blockers::{blocker_nudge_if_owed, report_blockers_if_any};

/// Everything one agent-loop run needs. `max_turns` is the TS `maxTurns`
/// cap — `MAX_TOOL_TURNS = 20` for plain chat, `DESIGN_LOOP_MAX_TURNS = 28`
/// for the gated design-generation loop (`chat_builtin_http.rs`; the two
/// callers pick per `finalize_on_exit`). This caps only ORDINARY turns —
/// a dedicated promise-delivery fill round (see `FILL_BUDGET_MAX_ROUNDS_PER_SCREEN`
/// below) is deliberately exempt from it. Tests shrink `max_turns` further.
pub struct AgentLoopConfig {
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
    pub history: Vec<(ChatHistoryRole, String)>,
    pub user_prompt: String,
    pub max_output_tokens: u32,
    pub tools: Vec<ChatToolDef>,
    pub executor: Arc<dyn ChatToolExecutor>,
    pub max_turns: usize,
    /// Opt-in structural backstop at design-loop exit. Plain chat must keep
    /// this false because finalization mutates the live document.
    pub finalize_on_exit: bool,
    /// Disable MiniMax / GLM hidden reasoning for structured design output;
    /// otherwise it can consume the whole output allowance before tool JSON.
    pub disable_thinking: bool,
    /// Dial policy inherited from the provider that spawned this loop —
    /// browser-originated endpoints resolve + pin per request.
    pub dial_policy: crate::provider_dial::EndpointDialPolicy,
}

impl AgentLoopConfig {
    fn level_for(&self, tool: &str) -> String {
        self.tools
            .iter()
            .find(|t| t.name == tool)
            .map(|t| t.level.clone())
            .unwrap_or_else(|| "read".to_string())
    }
}

// The stateful tool-call accumulation (one fully-accumulated call, the
// transcript tool-card envelope, per-wire collectors) moved to
// `op_ai::chat_tool_sse` so the mobile FFI design loop shares one
// implementation — same migration `op_ai::chat_sse`'s plain parsers made.
// Re-exported unchanged so existing `chat_agent_loop::` paths still resolve.
pub use op_ai::chat_tool_sse::{normalized_args, tool_card_envelope, PendingToolCall};

/// Run one tool call through the executor on a blocking thread (the
/// executor blocks on the UI ack; never block the runtime directly).
async fn execute_tool(
    executor: &Arc<dyn ChatToolExecutor>,
    name: &str,
    args_json: &str,
) -> ChatToolResult {
    let executor = executor.clone();
    let name = name.to_string();
    let args = normalized_args(args_json);
    tokio::task::spawn_blocking(move || executor.execute(&name, &args))
        .await
        .unwrap_or_else(|e| ChatToolResult {
            content: json!({ "success": false, "error": format!("tool executor panicked: {e}") })
                .to_string(),
            is_error: true,
        })
}

/// Run the deterministic structural-quality backstop once, at loop end
/// (Track-1 Step 4). Forwards to the executor's `finalize` (which the desktop
/// host bridges to `op_orchestrator::apply_loop_finalize` against the live
/// `EditorState`). Runs on a blocking thread — the host round-trip blocks until
/// the UI thread acks — mirroring [`execute_tool`]. The default executor
/// `finalize` is a no-op, so this is inert for scripted / read-only executors.
///
/// `enabled` is `cfg.finalize_on_exit`: a no-op early-return when this loop run
/// is a regular chat turn (the shared agent loop also serves plain builtin
/// chat), so only the gated design-generation loop mutates the document here.
///
/// Returns the promise-delivery invariant's committed/unfilled report AFTER
/// finalize ran and any still-unfilled screens got canvas-marked — empty on
/// the default no-op and on the common path where nothing was left unfilled.
async fn run_loop_finalize(executor: &Arc<dyn ChatToolExecutor>, enabled: bool) -> FinalizeReport {
    if !enabled {
        return FinalizeReport::default();
    }
    let executor = executor.clone();
    tokio::task::spawn_blocking(move || executor.finalize())
        .await
        .unwrap_or_default()
}

/// Cheap, read-only promise-delivery probe — same detector [`run_loop_finalize`]
/// runs, but never mutates the document or marks the canvas. Called whenever
/// the loop is deciding whether a dedicated fill round is owed, so a
/// still-eligible screen gets one more shot instead of immediately being
/// branded "(unfilled)". Gated by the same `enabled` flag as
/// [`run_loop_finalize`] — a plain chat turn never touches the document.
async fn check_unfilled(
    executor: &Arc<dyn ChatToolExecutor>,
    enabled: bool,
) -> UnfilledScreensReport {
    if !enabled {
        return UnfilledScreensReport::default();
    }
    let executor = executor.clone();
    tokio::task::spawn_blocking(move || executor.check_unfilled_screens())
        .await
        .unwrap_or_default()
}

/// Per-screen dedicated fill-round budget. "预算只防失控，绝不截断已承诺的
/// 工作" (budget only guards against a runaway retry avalanche; it must
/// never itself be the reason a promised screen ships empty): a fill round
/// spent nudging the model about a still-unfilled COMMITTED screen is exempt
/// from the ordinary `max_turns` cap — see the two loops' `turn >= turn_cap`
/// gates below, which keep issuing dedicated rounds past the cap as long as
/// some committed screen is still eligible. This constant is the ONLY thing
/// that stops that from running forever: the SAME screen failing across
/// `FILL_BUDGET_MAX_ROUNDS_PER_SCREEN` dedicated attempts is accepted as a
/// real failure (reported honestly, tier 3) rather than retried indefinitely
/// — 2 rounds mirrors `geometry_echo`'s "detect, retry, then accept"
/// discipline stretched by one extra attempt, since a wholly blank screen is
/// a starker failure than a layout nit.
const FILL_BUDGET_MAX_ROUNDS_PER_SCREEN: usize = 2;

/// Which of `unfilled` still has dedicated fill-round budget left, per
/// `attempts` (screen name -> dedicated rounds already spent on it this run).
fn eligible_for_fill_round(unfilled: &[String], attempts: &HashMap<String, usize>) -> Vec<String> {
    unfilled
        .iter()
        .filter(|name| {
            attempts.get(name.as_str()).copied().unwrap_or(0) < FILL_BUDGET_MAX_ROUNDS_PER_SCREEN
        })
        .cloned()
        .collect()
}

/// Record that a dedicated fill round was just spent on each of `names`.
fn spend_fill_round(attempts: &mut HashMap<String, usize>, names: &[String]) {
    for name in names {
        *attempts.entry(name.clone()).or_insert(0) += 1;
    }
}

/// Post-exhaustion "salvage" budget — a SEPARATE pool from
/// [`FILL_BUDGET_MAX_ROUNDS_PER_SCREEN`]'s ("两个预算，职责分离": the
/// ordinary `max_turns` cap guards against the model rambling on forever;
/// this pool guards "承诺必达" specifically once that ordinary budget is
/// exhausted — running out of turns must never itself be why a committed
/// screen ships empty). Every committed-but-unfilled screen still gets AT
/// MOST one dedicated salvage round each (never retried a second time here —
/// unlike the richer 2-round budget under ordinary turns), and the whole run
/// never spends more than [`SALVAGE_MAX_ROUNDS`] dedicated rounds total —
/// belt-and-suspenders against an avalanche, even though bundling every
/// still-eligible screen into ONE contract message (see the `turn >=
/// turn_cap` gates below) means this converges in exactly one round for the
/// common case of "some screens got missed."
const SALVAGE_MAX_ROUNDS: usize = 3;

/// Which of `unfilled` has not yet been salvaged this run.
fn salvage_eligible(
    unfilled: &[String],
    salvaged: &std::collections::HashSet<String>,
) -> Vec<String> {
    unfilled
        .iter()
        .filter(|name| !salvaged.contains(name.as_str()))
        .cloned()
        .collect()
}

/// Tier-2 nudge text — states the FULL commitment, not just what's still
/// missing, so the model sees an explicit broken promise instead of a
/// generic "fill it now" ("把承诺变成模型可见的契约" — turn the promise into
/// a contract the model can see). `committed` is every screen the run
/// scaffolded (filled or not, from [`UnfilledScreensReport::committed`]);
/// `still_empty` is the fill-budget-eligible subset this round is asking the
/// model to act on.
fn contract_nudge_text(committed: &[String], still_empty: &[String]) -> String {
    let commit_clause = if committed.len() > 1 {
        format!(
            "You committed {} screens ({}); ",
            committed.len(),
            committed.join("/")
        )
    } else {
        String::new()
    };
    let (subject, pronoun) = if still_empty.len() == 1 {
        ("is", "it")
    } else {
        ("are", "them")
    };
    format!(
        "{commit_clause}{} {subject} still empty. Complete {pronoun} before finishing.",
        still_empty.join(", ")
    )
}

/// Tier-3 unconditional honest report — appended to the transcript right
/// before `Done` whenever [`run_loop_finalize`] still finds unfilled screens
/// (every dedicated fill round the screen was eligible for ran out, or the
/// executor never had a live document to check). Wording mirrors the classic
/// path's `Progress::UnfilledScreens` line (`op-host-desktop::design_session`'s
/// `apply_progress` / `op-host-services::web_chat_standard`'s
/// `progress_label`) so a user sees the same sentence shape on either path.
async fn report_unfilled_if_any(tx: &mpsc::Sender<ChatDelta>, names: &[String]) {
    if names.is_empty() {
        return;
    }
    let text = format!(
        "\n\n• {} screen(s) left unfilled: {}",
        names.len(),
        names.join(", ")
    );
    let _ = tx.send(ChatDelta::TextDelta(text)).await;
}

/// Shared finalize + honest-report tail for every loop exit tier (turn-cap
/// salvage-exhausted, salvage-nothing-eligible, and the ordinary
/// voluntary-model-stop path) — replaces what used to be three near-identical
/// `run_loop_finalize` + `report_unfilled_if_any` call sites per provider
/// loop, and folds the unresolved-blocker report into the SAME tail so every
/// exit unconditionally surfaces both: a run that spent its whole turn
/// budget with blockers still open must never look, in the transcript, like
/// a run that had nothing wrong with it.
async fn finalize_and_report(
    tx: &mpsc::Sender<ChatDelta>,
    executor: &Arc<dyn ChatToolExecutor>,
    enabled: bool,
    zero_write: Option<&str>,
) -> UnfilledScreensReport {
    let finalized = run_loop_finalize(executor, enabled).await;
    report_unfilled_if_any(tx, &finalized.screens.unfilled).await;
    let blocker_report = blockers::check_blockers(executor, enabled).await;
    report_blockers_if_any(tx, &blocker_report).await;
    // Zero-write honesty (the empty-canvas postmortem): a design run that
    // never landed a write must never read, in the transcript, like a clean
    // success — and the stop/finish reason rides along so the NEXT such
    // field failure is diagnosable from the transcript alone (a truncated
    // reasoning budget, a voluntary stop, and a spent turn cap all look
    // identical without it).
    if let Some(stop_detail) = zero_write {
        let _ = tx
            .send(ChatDelta::TextDelta(format!(
                "\n\n• The run ended without applying any design write — \
                 the canvas is unchanged (stop reason: {stop_detail})."
            )))
            .await;
    }
    // The positive half of the same tail: what the deterministic passes
    // checked and repaired. Emitted LAST so it reads as the closing receipt
    // over whatever problem lines precede it, and its leftover count is the
    // sum of exactly those lines — never a separate, softer number.
    let remaining = finalized.screens.unfilled.len()
        + blocker_report.blockers.len()
        + usize::from(zero_write.is_some());
    report_quality_credential(tx, &finalized.quality, remaining).await;
    finalized.screens
}

/// One dedicated corrective round for a design run that has not applied a
/// single successful write-level tool call ("zero-write") — the postmortem
/// shape where a starved or distracted model reads the canvas, narrates,
/// and stops, leaving the canvas empty while the transcript looks clean.
/// Like the fill/salvage/blocker pools this is a SEPARATE budget: the
/// nudge round does not count against `max_turns`, and one round is enough
/// — a model that ignores an explicit "the canvas is EMPTY, build now"
/// contract will not do better on a second copy of it.
const ZERO_WRITE_MAX_ROUNDS: usize = 1;

/// The zero-write contract nudge. Script-first on purpose: this fires on
/// exactly the weak-model runs where the build protocol needs restating.
const ZERO_WRITE_NUDGE: &str = "You have not created any nodes — the canvas is EMPTY and \
nothing you described exists yet. Build the design NOW by calling batch_design with a \
`script` argument containing a JavaScript program of I(parent, node) calls. Do not stop \
again before the design is on the canvas.";

/// Whether the zero-write corrective round is owed at a voluntary stop.
fn zero_write_round_owed(finalize_on_exit: bool, wrote: bool, rounds_used: usize) -> bool {
    finalize_on_exit && !wrote && rounds_used < ZERO_WRITE_MAX_ROUNDS
}

/// The `zero_write` argument for [`finalize_and_report`]: `Some(stop
/// detail)` when this design run never applied a successful write.
fn zero_write_detail(finalize_on_exit: bool, wrote: bool, stop_detail: &str) -> Option<String> {
    (finalize_on_exit && !wrote).then(|| stop_detail.to_string())
}

/// Append the user-visible quality credential. Silent when the executor
/// reported no checks at all (plain chat turn, no live document) — see
/// `crate::quality_credential`'s honesty contract.
async fn report_quality_credential(
    tx: &mpsc::Sender<ChatDelta>,
    quality: &QualitySummary,
    remaining: usize,
) {
    if let Some(line) =
        crate::quality_credential::quality_credential_line_with_records(quality, Some(remaining))
    {
        let _ = tx.send(ChatDelta::TextDelta(line)).await;
    }
}

/// Self-diagnostic signal for the finalize-lifecycle invariant (0718-1-k3-1
/// postmortem) — a `run_anthropic_agent_loop` / `run_openai_agent_loop`
/// outer wrapper's backstop just ran [`run_loop_finalize`] on an `Err` exit
/// the inner loop's own paths never reached. Embedded directly in the
/// transcript (not a `tracing`/log call) on purpose: the ONLY forensic
/// trace available for the 0718-1-k3-1 incident afterward was the chat
/// transcript itself — no session log survived — so this is the greppable
/// answer to "did finalize actually run, and from where" the next time a
/// file turns up unfinalized. `enabled` mirrors [`run_loop_finalize`]'s own
/// gate — a plain (non-design) chat turn never touches the document, so it
/// never emits this either.
async fn emit_finalize_diagnostic(tx: &mpsc::Sender<ChatDelta>, enabled: bool, source: &str) {
    if !enabled {
        return;
    }
    let _ = tx
        .send(ChatDelta::TextDelta(format!(
            "\n\n• finalize ran (source={source})"
        )))
        .await;
}

// ---------------------------------------------------------------------------
// Shared SSE pump
// ---------------------------------------------------------------------------

/// Stateful per-event handler. `handle` returns the deltas to forward
/// to the chat channel for one SSE `data:` payload.
trait SseCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta>;
}

// The shared collectors live in `op_ai::chat_tool_sse` (see the re-export
// above); these impls adapt them onto the local pump trait. The bodies call
// the INHERENT `handle` (inherent methods win over trait methods in
// resolution), so no recursion.
impl SseCollector for op_ai::chat_tool_sse::OpenAiToolCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
        op_ai::chat_tool_sse::OpenAiToolCollector::handle(self, data)
    }
}

impl SseCollector for op_ai::chat_tool_sse::AnthropicToolCollector {
    fn handle(&mut self, data: &str) -> Vec<ChatDelta> {
        op_ai::chat_tool_sse::AnthropicToolCollector::handle(self, data)
    }
}

/// Drain `resp`'s SSE body through `collector`, forwarding returned
/// deltas to `tx`. Line/event framing mirrors
/// `chat_builtin_http::pump_sse_response` (multi-line `data:`
/// accumulation, trailing-buffer flush) but hands events to a
/// stateful collector instead of a pure parse fn — tool-call
/// accumulation spans many events.
async fn pump_sse<C: SseCollector>(
    resp: reqwest::Response,
    tx: &mpsc::Sender<ChatDelta>,
    collector: &mut C,
) -> Result<(), BuiltinHttpError> {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut event_data = String::new();

    while let Some(chunk) = stream.next().await {
        if tx.is_closed() {
            return Ok(());
        }
        let bytes = chunk.map_err(|e| BuiltinHttpError::SseStream {
            message: e.to_string(),
        })?;
        buf.extend_from_slice(&bytes);
        while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl_pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                dispatch_event(&mut event_data, tx, collector).await;
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(data.trim_start());
            }
        }
    }

    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data:") {
            if !event_data.is_empty() {
                event_data.push('\n');
            }
            event_data.push_str(data.trim_start());
        }
    }
    dispatch_event(&mut event_data, tx, collector).await;
    Ok(())
}

async fn dispatch_event<C: SseCollector>(
    event_data: &mut String,
    tx: &mpsc::Sender<ChatDelta>,
    collector: &mut C,
) {
    let data = event_data.trim();
    if data.is_empty() {
        event_data.clear();
        return;
    }
    let deltas = collector.handle(data);
    event_data.clear();
    for delta in deltas {
        if tx.send(delta).await.is_err() {
            return;
        }
    }
}

mod anthropic;
mod openai;

pub use anthropic::*;
pub use openai::*;

#[cfg(test)]
#[path = "chat_agent_loop_tests.rs"]
mod tests;

// Finalize-lifecycle invariant regression tests (0718-1-k3-1) — split out
// as a sibling rather than growing `chat_agent_loop_tests.rs` past its
// existing 800-line-cap debt further; reuses that file's scripted-executor
// + loopback-SSE test infra via `pub(super)`.
#[cfg(test)]
#[path = "chat_agent_loop_finalize_tests.rs"]
mod finalize_tests;

// Unresolved-blocker completion-gate tests — same split rationale as
// `finalize_tests` above, reusing `chat_agent_loop_tests.rs`'s scripted-
// executor + loopback-SSE infra via `pub(super)`.
#[cfg(test)]
#[path = "chat_agent_loop_blockers_tests.rs"]
mod blockers_tests;

// Quality-credential tests for the same finalize tail — same split
// rationale and shared infra as `blockers_tests` above.
#[cfg(test)]
#[path = "chat_agent_loop_quality_tests.rs"]
mod quality_tests;

#[cfg(test)]
#[path = "chat_agent_loop_retry_tests.rs"]
mod retry_tests;

// Zero-write completion-gate tests (the empty-canvas postmortem) — same
// split rationale and shared scripted-executor + loopback-SSE infra as the
// blocker tests above.
#[cfg(test)]
#[path = "chat_agent_loop_zero_write_tests.rs"]
mod zero_write_tests;

#[cfg(test)]
#[path = "chat_agent_loop_wire_tests.rs"]
mod wire_tests;
