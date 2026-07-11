//! Claude Code IPC bridge — adapts `anthropic_agent_sdk::query` to
//! the shell-core [`ChatProvider`] trait.
//!
//! The vendored `vendor/anthropic-agent-sdk` crate carries
//! the full Claude Code CLI subprocess transport: spawn the `claude`
//! binary with `--print --verbose --output-format stream-json --`,
//! parse the line-delimited stream-JSON envelope, surface
//! `Message::Assistant` / `Result` / `System` / `User` /
//! `StreamEvent` shapes. This module is the thin mapping layer that
//! collapses those into the OP chat panel's `ChatDelta` vocabulary.
//!
//! Streaming granularity (TS parity, `chat.ts:353-400`): every turn
//! runs with `--include-partial-messages`, and `content_block_delta`
//! stream events forward token-level `text_delta` / `thinking_delta`
//! fragments as they arrive. The whole-message `Assistant` blocks
//! that follow are then only mined for `ToolUse` cards (text /
//! thinking already streamed); if a CLI build ignores the flag the
//! whole blocks still render as the coarse fallback.
//!
//! Image turns (GAP #34) follow the TS guided Read-tool flow
//! (`chat.ts:268-310`): attachments spill to temp files, the prompt
//! gains one "First, use the Read tool…" line per image, and the
//! system prompt is run through `stripNoToolsRestriction`. Divergence
//! from TS (documented): TS suppresses intermediate text on image
//! turns via a result-only flow (`maxTurns: 3`, only the final result
//! is emitted) — Rust streams every turn uniformly and renders
//! Claude's own tool calls (including the image Read) as transcript
//! cards, which the TS chat path (tools disabled) never has.
//!
//! Why a separate adapter instead of inlining into chat_subprocess.rs:
//! the SDK owns ~30 fields of CLI options (system prompts, MCP
//! servers, allowed tools, sandbox config, ...) that the bridge
//! eventually wires through. Keeping each provider in its own file
//! gives that surface room to grow without busting the 800-line cap.

use std::sync::{Arc, Mutex, OnceLock};

use anthropic_agent_sdk::{
    types::{identifiers::SessionId, ContentBlock, Message},
    ClaudeAgentOptions, StreamExt,
};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason, ThinkingMode};
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// Process-wide chat-session resume slot. Each chat turn spawns a
/// fresh `claude --print` subprocess; the slot carries the previous
/// turn's `session_id` forward so the SDK resumes it (`--resume`) and
/// follow-ups see the full conversation. New Chat clears the slot
/// (`reset_claude_chat_session`) so a fresh transcript starts a fresh
/// CLI session. Single-window desktop app — one slot is enough.
fn chat_resume_slot() -> &'static Mutex<Option<SessionId>> {
    static SLOT: OnceLock<Mutex<Option<SessionId>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Forget the resumable Claude Code chat session. Called when the
/// user starts a New Chat so stale context can't leak into it.
pub fn reset_claude_chat_session() {
    if let Ok(mut slot) = chat_resume_slot().lock() {
        *slot = None;
    }
}

/// `ChatProvider` impl that drives Claude Code via the
/// `anthropic-agent-sdk` Rust client. Each send spawns a fresh
/// `claude --print` subprocess; chat-tracked providers
/// ([`ClaudeCodeProvider::for_chat`]) thread the previous turn's
/// `session_id` through `options.resume` so the CLI reloads the
/// conversation — follow-ups carry full multi-turn context.
pub struct ClaudeCodeProvider {
    /// Optional CLI-options bundle the SDK forwards to `claude`.
    /// Cloned per-`send`; `None` falls through to the SDK's defaults.
    options: Option<ClaudeAgentOptions>,
    /// When true, this provider participates in the process-wide chat
    /// resume slot: it resumes the stored session and stores the
    /// session id each turn reports. The design orchestrator path
    /// constructs untracked providers so its sub-requests never join
    /// (or pollute) the user's chat session.
    track_chat_session: bool,
    label: String,
}

impl ClaudeCodeProvider {
    /// Build a Claude Code provider with no extra options (SDK
    /// defaults). The CLI is discovered via the SDK's own `find_cli`
    /// (PATH + npm-global / yarn / Linux package locations) — no
    /// binary path needed up front.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            options: None,
            track_chat_session: false,
            label: "Claude Code".into(),
        }
    }

    /// Build a chat-panel provider that resumes (and records) the
    /// process-wide chat session, so consecutive chat turns share one
    /// Claude Code conversation.
    pub fn for_chat() -> Self {
        Self {
            options: None,
            track_chat_session: true,
            label: "Claude Code".into(),
        }
    }

    /// Build a Claude Code provider with a pre-configured
    /// `ClaudeAgentOptions` (system prompt, model selection, tool
    /// allowlist, MCP servers, sandbox config, etc.). The settings
    /// modal calls this when the user has tuned options away from
    /// defaults.
    #[allow(dead_code)]
    pub fn with_options(options: ClaudeAgentOptions) -> Self {
        Self {
            options: Some(options),
            track_chat_session: false,
            label: "Claude Code".into(),
        }
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatProvider for ClaudeCodeProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let resume = if self.track_chat_session {
            chat_resume_slot().lock().ok().and_then(|s| s.clone())
        } else {
            None
        };
        // Image turns (TS chat.ts:293-297): the system prompt's
        // "NEVER use tools"-style restrictions are stripped so Claude
        // Code will use its Read tool to view the spilled image files.
        let mut request = request;
        if !request.attachments.is_empty() {
            request.system_prompt =
                crate::chat_attachment::strip_no_tools_restriction(&request.system_prompt);
        }
        let options = Some(effective_options(self.options.as_ref(), &request, resume));
        let track_session = self.track_chat_session;
        // Claude Code's `query` String API can't carry image content
        // blocks, so attachments spill to temp files and the prompt
        // carries the TS guided Read-tool flow (chat.ts:268-282): one
        // "First, use the Read tool…" line per image. The guard keeps
        // the temp files alive for the turn and removes them when the
        // worker task ends. A staging failure aborts the turn with an
        // error rather than silently dropping the attachments.
        let (prompt, guard) = match crate::chat_attachment::claude_image_prompt(
            &request.user_message,
            &request.attachments,
        ) {
            Ok(pair) => pair,
            Err(e) => return crate::chat_attachment::attachment_error_turn(e),
        };
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let _guard = guard;
            let stream = match anthropic_agent_sdk::query(prompt, options).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("claude query: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            let mut emitted_done = false;
            let mut state = StreamState::default();
            let mut out: Vec<ChatDelta> = Vec::new();
            'msgs: while let Some(msg_result) = stream.next().await {
                // Drop-out the moment the chat panel goes away.
                if tx.is_closed() {
                    break;
                }
                let msg = match msg_result {
                    Ok(m) => {
                        // Chat-tracked turns record the CLI session id
                        // so the NEXT turn resumes this conversation.
                        if track_session {
                            if let Message::Result { session_id, .. } = &m {
                                if let Ok(mut slot) = chat_resume_slot().lock() {
                                    *slot = Some(session_id.clone());
                                }
                            }
                        }
                        m
                    }
                    Err(e) => {
                        let _ = tx
                            .send(ChatDelta::Error(format!("claude stream: {e}")))
                            .await;
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        emitted_done = true;
                        break;
                    }
                };
                out.clear();
                let terminal = map_message(msg, &mut state, &mut out);
                for delta in out.drain(..) {
                    if tx.send(delta).await.is_err() {
                        break 'msgs;
                    }
                }
                if let Some(reason) = terminal {
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: reason,
                        })
                        .await;
                    emitted_done = true;
                    break;
                }
            }
            if !emitted_done {
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            }
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Build the per-turn SDK options bundle from the configured base:
///
/// - the thinking knob maps onto the SDK's numeric thinking-token
///   budget (`Adaptive` leaves whatever the base already carried);
/// - the selected model lands on `options.model`, which the SDK
///   forwards to the CLI as `--model <id>` (TS parity: `chat.ts`
///   spreads `model` into `query` options only when one is present —
///   no selection keeps the CLI default);
/// - the per-turn system prompt lands on `options.system_prompt`
///   (TS parity: `chat.ts` passes `systemPrompt: body.system`);
/// - `resume` threads the previous chat turn's session id through so
///   the CLI reloads the conversation — history rides the resumed
///   session, not the prompt (no transcript digest needed).
fn effective_options(
    base: Option<&ClaudeAgentOptions>,
    request: &ChatRequest,
    resume: Option<SessionId>,
) -> ClaudeAgentOptions {
    let mut options = base.cloned().unwrap_or_default();
    // Token-level streaming (TS parity: `chat.ts:361`
    // `includePartialMessages: true`) — the CLI emits
    // `content_block_delta` stream events the mapper below forwards
    // as they arrive instead of waiting for whole assistant messages.
    options.include_partial_messages = true;
    match request.thinking {
        ThinkingMode::Enabled => {
            options.max_thinking_tokens = Some(request.effort.budget_tokens());
        }
        ThinkingMode::Disabled => {
            options.max_thinking_tokens = Some(0);
        }
        ThinkingMode::Adaptive => {}
    }
    if let Some(model) = request.model_id() {
        options.model = Some(model.to_string());
    }
    if !request.system_prompt.trim().is_empty() {
        options.system_prompt = Some(request.system_prompt.clone().into());
    }
    if let Some(session) = resume {
        options.resume = Some(session);
    }
    // GUI-launch environment repair: a Dock-launched app misses the
    // shell-rc exports a terminal launch has — without ANTHROPIC_* the
    // CLI silently flips from the API key to the subscription OAuth
    // credential, which the API rejects for custom-system-prompt
    // requests with `403 Request not allowed`. Explicit process env
    // still wins (insert only when absent from options.env).
    //
    // PATH deliberately does NOT ride here: the SDK's dangerous-env
    // blocklist (`DANGEROUS_ENV_VARS` in transport/subprocess.rs)
    // rejects any request whose options.env carries PATH — the whole
    // query dies with "Invalid configuration" (measured 2026-07-11).
    // The GUI PATH repair is process-wide instead
    // (`chat_spawn::repair_gui_process_path` at desktop startup), which
    // the SDK's `env::vars()` baseline inherits naturally.
    for name in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
    ] {
        if let Some(value) = crate::chat_spawn::env_var_with_login_shell(name) {
            options.env.entry(name.to_string()).or_insert(value);
        }
    }
    options
}

/// Per-message streaming dedupe state. With
/// `--include-partial-messages` the CLI streams `content_block_delta`
/// events for a message, then ships the same content again as a whole
/// `Assistant` message. The flags record whether token deltas already
/// painted the current message's text / thinking so the whole-message
/// blocks don't re-emit them. Reset after every `Assistant` message —
/// agentic turns interleave (deltas → assistant → tool → deltas →
/// assistant …) and each cycle dedupes independently.
#[derive(Default)]
struct StreamState {
    text_streamed: bool,
    thinking_streamed: bool,
}

/// Map one SDK `Message` into `ChatDelta`s pushed onto `out`.
/// Returns `Some(reason)` when this message is the turn-terminating
/// `Result` (caller emits the terminal `Done` and breaks). Pure —
/// unit-testable without a channel or runtime.
fn map_message(
    msg: Message,
    state: &mut StreamState,
    out: &mut Vec<ChatDelta>,
) -> Option<StopReason> {
    match msg {
        // Token-level partial-message events (TS parity,
        // `chat.ts:379-390`): forward `text_delta` / `thinking_delta`
        // fragments the moment they arrive.
        Message::StreamEvent { event, .. } => {
            if event.get("type").and_then(|v| v.as_str()) == Some("content_block_delta") {
                let delta = event.get("delta");
                match delta.and_then(|d| d.get("type")).and_then(|v| v.as_str()) {
                    Some("text_delta") => {
                        if let Some(text) =
                            delta.and_then(|d| d.get("text")).and_then(|v| v.as_str())
                        {
                            out.push(ChatDelta::TextDelta(text.to_string()));
                            state.text_streamed = true;
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(thinking) = delta
                            .and_then(|d| d.get("thinking"))
                            .and_then(|v| v.as_str())
                        {
                            out.push(ChatDelta::Thinking(thinking.to_string()));
                            state.thinking_streamed = true;
                        }
                    }
                    // input_json_delta / signature_delta etc. — tool
                    // args render from the whole ToolUse block below.
                    _ => {}
                }
            }
            None
        }
        Message::Assistant { message, .. } => {
            // Claude Code groups multiple blocks (text + tool_use +
            // thinking) into one assistant message. Text / thinking
            // that already streamed token-level is skipped; ToolUse
            // always surfaces (stream events never carry the full
            // args). When the CLI ignored `--include-partial-messages`
            // the flags stay false and the whole blocks render as the
            // coarse fallback.
            for block in &message.content {
                match block {
                    ContentBlock::Text { text } => {
                        if !state.text_streamed {
                            out.push(ChatDelta::TextDelta(text.clone()));
                        }
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        if !state.thinking_streamed {
                            out.push(ChatDelta::Thinking(thinking.clone()));
                        }
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        out.push(ChatDelta::ToolUse {
                            name: name.clone(),
                            args: input.to_string(),
                        });
                    }
                    ContentBlock::ToolResult { .. } => {
                        // Tool results are part of the conversation
                        // history the CLI already shows the model;
                        // the chat widget doesn't render them
                        // separately today.
                    }
                }
            }
            *state = StreamState::default();
            None
        }
        Message::Result {
            subtype,
            is_error,
            result,
            errors,
            ..
        } => {
            let failed = is_error || subtype != "success";
            if failed {
                // TS parity (`chat.ts:391-400`): surface the error
                // text — joined `errors`, else the result body, else
                // the subtype.
                let joined = errors.join("; ");
                let content = if !joined.is_empty() {
                    joined
                } else if let Some(res) = result.filter(|r| !r.is_empty()) {
                    res
                } else {
                    format!("Query ended with: {subtype}")
                };
                out.push(ChatDelta::Error(content));
            }
            let reason = if is_error {
                StopReason::Aborted
            } else {
                map_result_subtype(&subtype)
            };
            Some(reason)
        }
        Message::System { .. } | Message::User { .. } | Message::Unknown => {
            // Init / context events plus any message type the SDK
            // doesn't model (e.g. `rate_limit_event`) — the chat
            // widget doesn't surface them today; silent.
            None
        }
    }
}

fn map_result_subtype(s: &str) -> StopReason {
    match s {
        "success" => StopReason::EndTurn,
        "error_max_turns" => StopReason::MaxTokens,
        "error_during_execution" | "error" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

// Keep the import surface alive so callers from main.rs can
// reach the SDK without re-importing.
#[allow(unused_imports)]
pub use anthropic_agent_sdk::{ClaudeAgentOptions as ClaudeOptions, ClaudeSDKClient};

/// Returned by `ClaudeCodeProvider::new` / `with_options` chain
/// for the rare smoke test where we want to confirm the wiring
/// compiles end-to-end without a live `claude` binary on PATH.
#[doc(hidden)]
#[allow(dead_code)]
pub fn _smoke_sdk_arc_send() -> Arc<dyn ChatProvider> {
    Arc::new(ClaudeCodeProvider::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_result_subtype_table() {
        assert!(matches!(map_result_subtype("success"), StopReason::EndTurn));
        assert!(matches!(
            map_result_subtype("error_max_turns"),
            StopReason::MaxTokens
        ));
        assert!(matches!(
            map_result_subtype("error_during_execution"),
            StopReason::Aborted
        ));
        assert!(matches!(map_result_subtype("error"), StopReason::Aborted));
        // Unknown subtypes default to EndTurn so a future SDK addition
        // doesn't break the chat widget.
        assert!(matches!(map_result_subtype("rocket"), StopReason::EndTurn));
    }

    #[test]
    fn provider_label_is_human_readable() {
        let p = ClaudeCodeProvider::new();
        assert_eq!(p.provider_label(), "Claude Code");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        // Type-system check: ClaudeCodeProvider satisfies the
        // ChatProvider trait bounds (Send + Sync) so it can live
        // behind an `Arc<dyn ChatProvider>` in the widget host.
        let _: Arc<dyn ChatProvider> = Arc::new(ClaudeCodeProvider::new());
    }

    #[test]
    fn effective_options_sets_selected_model() {
        let req = ChatRequest {
            model: Some("claude-sonnet-4-6".into()),
            ..Default::default()
        };
        let options = effective_options(None, &req, None);
        assert_eq!(options.model.as_deref(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn effective_options_without_model_keeps_base_default() {
        // No selection → leave the configured base untouched so the
        // CLI keeps its own default model.
        let req = ChatRequest::default();
        let options = effective_options(None, &req, None);
        assert!(options.model.is_none());

        // A base bundle with a model survives a turn without one.
        let base = ClaudeAgentOptions {
            model: Some("opus".into()),
            ..Default::default()
        };
        let options = effective_options(Some(&base), &req, None);
        assert_eq!(options.model.as_deref(), Some("opus"));
    }

    #[test]
    fn effective_options_ignores_blank_model() {
        // A blank id is "unset" — never forward an empty --model.
        let req = ChatRequest {
            model: Some("   ".into()),
            ..Default::default()
        };
        let options = effective_options(None, &req, None);
        assert!(options.model.is_none());
    }

    #[test]
    fn effective_options_threads_system_prompt_and_resume() {
        use anthropic_agent_sdk::types::options::SystemPrompt;
        let req = ChatRequest {
            system_prompt: "You are a design assistant.".into(),
            ..Default::default()
        };
        let session = SessionId::new("sess-123");
        let options = effective_options(None, &req, Some(session.clone()));
        assert!(
            matches!(
                options.system_prompt,
                Some(SystemPrompt::String(ref s)) if s == "You are a design assistant."
            ),
            "per-turn system prompt must ride options.system_prompt (TS parity: systemPrompt)"
        );
        assert_eq!(
            options.resume.as_ref().map(SessionId::as_str),
            Some("sess-123"),
            "resume must carry the previous chat turn's session id"
        );
    }

    #[test]
    fn effective_options_blank_system_prompt_keeps_sdk_default() {
        let req = ChatRequest {
            system_prompt: "   ".into(),
            ..Default::default()
        };
        let options = effective_options(None, &req, None);
        assert!(options.system_prompt.is_none());
        assert!(options.resume.is_none());
    }

    /// Deserialize a wire-shaped JSON message into the SDK enum —
    /// mirrors how the real subprocess transport produces them.
    fn msg(json: serde_json::Value) -> Message {
        serde_json::from_value(json).expect("valid SDK message json")
    }

    fn stream_text_event(text: &str) -> Message {
        msg(serde_json::json!({
            "type": "stream_event",
            "uuid": "u1",
            "session_id": "s1",
            "event": {
                "type": "content_block_delta",
                "index": 0,
                "delta": { "type": "text_delta", "text": text }
            }
        }))
    }

    fn assistant_message(blocks: serde_json::Value) -> Message {
        msg(serde_json::json!({
            "type": "assistant",
            "message": { "model": "claude-test", "content": blocks }
        }))
    }

    #[test]
    fn effective_options_enables_partial_messages() {
        // TS parity: chat.ts:361 `includePartialMessages: true` — the
        // flag is what makes the CLI emit token-level stream events.
        let options = effective_options(None, &ChatRequest::default(), None);
        assert!(options.include_partial_messages);
    }

    #[test]
    fn stream_event_forwards_token_level_text_and_thinking() {
        let mut state = StreamState::default();
        let mut out = Vec::new();
        assert!(map_message(stream_text_event("He"), &mut state, &mut out).is_none());
        assert!(map_message(stream_text_event("llo"), &mut state, &mut out).is_none());
        let thinking = msg(serde_json::json!({
            "type": "stream_event",
            "uuid": "u2",
            "session_id": "s1",
            "event": {
                "type": "content_block_delta",
                "index": 0,
                "delta": { "type": "thinking_delta", "thinking": "hmm" }
            }
        }));
        assert!(map_message(thinking, &mut state, &mut out).is_none());
        assert_eq!(
            out,
            vec![
                ChatDelta::TextDelta("He".into()),
                ChatDelta::TextDelta("llo".into()),
                ChatDelta::Thinking("hmm".into()),
            ]
        );
    }

    #[test]
    fn assistant_after_stream_deltas_skips_duplicates_keeps_tool_use() {
        let mut state = StreamState::default();
        let mut out = Vec::new();
        let _ = map_message(stream_text_event("Hello"), &mut state, &mut out);
        let assistant = assistant_message(serde_json::json!([
            { "type": "text", "text": "Hello" },
            { "type": "tool_use", "id": "t1", "name": "Read", "input": { "path": "a.png" } }
        ]));
        let _ = map_message(assistant, &mut state, &mut out);
        // The whole-message text must NOT re-emit (already streamed
        // token-level); the tool call still surfaces as a card.
        assert_eq!(
            out,
            vec![
                ChatDelta::TextDelta("Hello".into()),
                ChatDelta::ToolUse {
                    name: "Read".into(),
                    args: r#"{"path":"a.png"}"#.into()
                },
            ]
        );
    }

    #[test]
    fn assistant_without_stream_deltas_falls_back_to_whole_blocks() {
        // CLI builds that ignore --include-partial-messages still
        // render: no stream deltas seen → whole blocks emit.
        let mut state = StreamState::default();
        let mut out = Vec::new();
        let assistant = assistant_message(serde_json::json!([
            { "type": "thinking", "thinking": "mull", "signature": "sig" },
            { "type": "text", "text": "whole" }
        ]));
        let _ = map_message(assistant, &mut state, &mut out);
        assert_eq!(
            out,
            vec![
                ChatDelta::Thinking("mull".into()),
                ChatDelta::TextDelta("whole".into()),
            ]
        );
    }

    #[test]
    fn dedupe_state_resets_per_assistant_message() {
        // Agentic turns interleave deltas → assistant → deltas →
        // assistant; each message cycle dedupes independently.
        let mut state = StreamState::default();
        let mut out = Vec::new();
        let _ = map_message(stream_text_event("first"), &mut state, &mut out);
        let _ = map_message(
            assistant_message(serde_json::json!([{ "type": "text", "text": "first" }])),
            &mut state,
            &mut out,
        );
        // Second message arrives whole-block only.
        let _ = map_message(
            assistant_message(serde_json::json!([{ "type": "text", "text": "second" }])),
            &mut state,
            &mut out,
        );
        assert_eq!(
            out,
            vec![
                ChatDelta::TextDelta("first".into()),
                ChatDelta::TextDelta("second".into()),
            ]
        );
    }

    #[test]
    fn error_result_emits_error_text_then_terminates() {
        // TS parity (chat.ts:391-400): errors.join('; ') || result ||
        // "Query ended with: <subtype>".
        let mut state = StreamState::default();
        let mut out = Vec::new();
        let result = msg(serde_json::json!({
            "type": "result",
            "subtype": "error_during_execution",
            "duration_ms": 1,
            "duration_api_ms": 1,
            "is_error": true,
            "num_turns": 1,
            "session_id": "s1",
            "errors": ["boom", "bang"]
        }));
        let reason = map_message(result, &mut state, &mut out);
        assert!(matches!(reason, Some(StopReason::Aborted)));
        assert_eq!(out, vec![ChatDelta::Error("boom; bang".into())]);

        // Subtype fallback when errors + result are empty.
        out.clear();
        let result = msg(serde_json::json!({
            "type": "result",
            "subtype": "error_max_turns",
            "duration_ms": 1,
            "duration_api_ms": 1,
            "is_error": false,
            "num_turns": 1,
            "session_id": "s1"
        }));
        let reason = map_message(result, &mut state, &mut out);
        assert!(matches!(reason, Some(StopReason::MaxTokens)));
        assert_eq!(
            out,
            vec![ChatDelta::Error("Query ended with: error_max_turns".into())]
        );
    }

    #[test]
    fn success_result_terminates_without_error() {
        let mut state = StreamState::default();
        let mut out = Vec::new();
        let result = msg(serde_json::json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1,
            "duration_api_ms": 1,
            "is_error": false,
            "num_turns": 1,
            "session_id": "s1",
            "result": "fine"
        }));
        let reason = map_message(result, &mut state, &mut out);
        assert!(matches!(reason, Some(StopReason::EndTurn)));
        assert!(out.is_empty());
    }

    #[test]
    fn effective_options_maps_thinking_budget() {
        use op_ai::chat_provider::EffortLevel;
        let req = ChatRequest {
            thinking: ThinkingMode::Enabled,
            effort: EffortLevel::High,
            ..Default::default()
        };
        let options = effective_options(None, &req, None);
        assert_eq!(options.max_thinking_tokens, Some(24_000));

        let req = ChatRequest {
            thinking: ThinkingMode::Disabled,
            ..Default::default()
        };
        let options = effective_options(None, &req, None);
        assert_eq!(options.max_thinking_tokens, Some(0));
    }
}

#[cfg(test)]
mod effective_options_env_tests {
    use super::*;
    use op_ai::chat_provider::{ChatRequest, EffortLevel, ThinkingMode};

    #[test]
    fn options_env_never_carries_path() {
        // The agent SDK's DANGEROUS_ENV_VARS blocklist rejects any request
        // whose env map carries PATH — injecting it kills EVERY claude query
        // with "Invalid configuration" (regression measured 2026-07-11).
        let request = ChatRequest {
            system_prompt: "design something".into(),
            user_message: "hi".into(),
            history: vec![],
            max_output_tokens: 1024,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
            attachments: vec![],
            model: None,
        };
        let options = effective_options(None, &request, None);
        assert!(
            !options.env.contains_key("PATH"),
            "PATH must never ride options.env: {:?}",
            options.env.keys().collect::<Vec<_>>()
        );
    }
}
