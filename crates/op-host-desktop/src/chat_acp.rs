//! ACP chat bridge — adapts an `op_acp` connection to the
//! [`ChatProvider`] trait.
//!
//! `ChatProviderKind::Acp` is the catch-all for third-party agents
//! OpenPencil ships no dedicated adapter for. An [`AcpProvider`] is
//! built from a persisted [`AcpAgentConfig`]; each `send` connects,
//! opens a session, drives one prompt turn, and streams the agent's
//! `session/update` notifications back as `ChatDelta`s.
//!
//! ## Canvas tool surface (TS parity)
//!
//! ACP agents get canvas tools the way the TS host shipped them
//! (`apps/web/server/api/ai/agent.ts:503-580`): `session/new` carries
//! the live in-process MCP server's HTTP endpoint in `mcpServers`, so
//! the agent connects to `http://127.0.0.1:<port>/mcp` itself and
//! sees the FULL first-party tool catalog (`mcp__openpencil__*`) —
//! not the 7-tool chat subset the builtin agent loop advertises
//! (`chat_canvas_tools.rs`; that subset exists for API-key wires that
//! cannot reach an MCP server). `session/update` `tool_call` /
//! `tool_call_update` notifications stay display-only
//! (`pen-acp/src/event-adapter.ts:17-18` — "the agent executes them
//! via MCP"); execution and permission-gating live in the MCP server.
//!
//! Like TS, a LOCAL turn requires the MCP server to be running
//! ("without it they just call Terminal/Skill tools that don't work
//! here", agent.ts:497-512): the provider refuses with the TS error
//! message when it is stopped. A remote agent never receives the
//! desktop's loopback URL — `127.0.0.1` would name the remote host,
//! not this OpenPencil process — and therefore also does not receive
//! the tool-specific system prompt.
//!
//! Documented divergences from TS:
//! - The MCP-running check happens BEFORE spawning the agent process
//!   (TS checks after obtaining its pooled connection, agent.ts:484 →
//!   :497) — the visible error is identical and a doomed turn never
//!   spawns a child here, where connections are per-turn.
//! - TS prompts with the last user message only (agent.ts:592-597);
//!   this shell additionally folds a compact history digest plus
//!   thinking/effort directives into the prompt (ACP sessions are
//!   per-turn, so the digest preserves cross-turn context).
//! - A `--live-mcp`-forced server (CLI launch) is not advertised:
//!   only the settings-reconciled server state is visible from
//!   `EditorState` (`agent_settings.mcp_server`), which tracks the
//!   user-enabled server's bound port.

use std::sync::{atomic::AtomicBool, Arc};

use op_acp::{
    connect_acp_agent, session_update_to_delta, AcpAgentConfig, AcpConnection, AcpSession,
    AcpStopReason, ConnectionType, McpHttpServer, NewSessionOptions, SessionConfigKind,
    SessionConfigOptionCategory, SessionConfigOptionValue, SessionConfigSelectOptions,
};
use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use tokio::sync::mpsc;

use op_host_services::chat_attachment::TempGuard;
use op_host_services::chat_runtime::{shared_runtime, BlockingRecvIter};

/// TS error surfaced when ACP is used with the MCP server stopped —
/// ported verbatim from `apps/web/server/api/ai/agent.ts:506-511`.
const MCP_NOT_RUNNING_ERROR: &str = "MCP server is not running. Open Settings → MCP and click \
     \"Start\" to enable ACP agents to access OpenPencil design tools.";

/// Dedicated ACP system prompt sent via `_meta.systemPrompt` —
/// verbatim port of `apps/web/server/api/ai/agent.ts:529-574`
/// (`acpSystemPrompt`). It overrides the agent's default so it drives
/// the canvas through MCP tools instead of CLI/skill workflows.
const ACP_SYSTEM_PROMPT: &str = r##"You are an AI design assistant integrated inside the OpenPencil vector design tool.
The user sees a live canvas; your job is to produce polished, visually refined UI designs on it.
You have direct access to OpenPencil's document via the "openpencil" MCP server.

## Tool Usage Rules
- NEVER use Bash/Terminal to run `op` CLI commands. The CLI is not available here.
- NEVER use the openpencil-skill or Skill tool. They are for a different context.
- DO use the `mcp__openpencil__*` tools to operate on the canvas.
- After finishing, provide a brief one-sentence summary of what was done.

## REQUIRED Workflow for Creating New Designs
Always follow this three-phase pipeline (it produces higher quality than ad-hoc insert calls):

1. **Load the design guide (ONCE)**: Call `get_design_prompt` to receive OpenPencil's design principles, node schema details, role system, color/typography tokens, and layout patterns. Read it carefully — it defines the canonical shapes and defaults.
2. **Build skeleton**: Call `design_skeleton` with a high-level description. This creates the structural frames (sections, layout containers) with correct auto-layout.
3. **Fill content**: Call `design_content` once per section from step 2, adding the concrete children (buttons, inputs, text, icons).
4. **Refine**: ALWAYS call `design_refine` on the root after all sections are populated. This is mandatory for AI chat output because it applies final polish, consistent spacing, role-based styling, icon cleanup, and layout safety.

Only fall back to `batch_design` or `insert_node` when the user explicitly asks for small/surgical edits rather than a new page.

## AI Chat Design Quality
- Mobile top rhythm: keep the status/header, title, and primary module close. On 375-430px screens, the gap from the header/title group to the first primary module (search, hero action, chart, or card) should usually be 20-32px, never a large empty band unless the request explicitly asks for a dramatic editorial hero.
- Product/card favorite/heart controls are functional `icon-button`s, not decorative badges. Place each favorite/heart fully inside its product card or product image with an 8-12px inset. Never use negative x/y, never straddle a card border, and never let the circle protrude into the section heading gap.
- Every new design needs a distinct visual concept before building: choose one concrete direction (for example editorial food magazine, glassy premium delivery, neo-brutal marketplace, calm bento dashboard, luxury concierge). Do not repeat the same predictable mobile stack of search + categories + orange promo + two cards unless that is genuinely the best fit for the prompt.
- Include one signature moment that gives the screen character: a crafted hero composition, editorial image treatment, distinctive category rail, playful but controlled card system, or refined data/offer module. Keep it purposeful and avoid decoration spam.

## Modifying Existing Designs
- Call `snapshot_layout` first to see the current tree.
- Use `update_node` for property changes, `move_node` for reparenting, `delete_node` to remove.
- Prefer one `batch_design` over many individual calls when making multiple related changes.

## Canonical Node Shapes (IMPORTANT)
The canvas will render nothing useful if you use the wrong `type` or shape. Use these:

- **Frame** (container with layout): `{"type": "frame", "name": "X", "width": 375, "height": 812, "layout": "vertical", "gap": 16, "padding": [24, 24, 24, 24], "fill": [{"type": "solid", "color": "#FFFFFF"}], "children": [...]}`
- **Text** (field is `content` NOT `text`): `{"type": "text", "name": "Title", "content": "Welcome", "fontSize": 24, "fontWeight": 700, "fill": [{"type": "solid", "color": "#111827"}]}`
- **Icon** (use `icon_font` NOT `icon`, field is `iconFontName` NOT `iconName`): `{"type": "icon_font", "name": "Lock Icon", "iconFontName": "lock", "width": 20, "height": 20, "fill": [{"type": "solid", "color": "#6B7280"}]}`. Common iconFontName values (Lucide): `mail`, `lock`, `eye`, `eye-off`, `chrome`, `apple`, `message-circle`, `x`, `arrow-right`, `search`, `heart`, `star`, `check`, `plus`, `bell`, `home`, `user`, `settings`.
- **Rectangle**: `{"type": "rectangle", "width": 100, "height": 100, "cornerRadius": 8, "fill": [{"type": "solid", "color": "#3B82F6"}]}`
- **Button** (frame + text child): use `"role": "cta-button"` on the frame so role resolution applies standard button styling.

## STRICT JSON Rules
When emitting node JSON inside tool arguments, produce strictly valid JSON:
- Every property MUST have BOTH a key and value. NEVER emit `": 50` or `: 50` with no key.
- Every key MUST be a double-quoted non-empty string.
- `fill` is ALWAYS an array: `"fill": [{"type": "solid", "color": "#hex"}]`.
- `stroke` is `{"thickness": 1, "fill": [{"type": "solid", "color": "#hex"}]}`. NEVER `{"thickness": 1, "color": "#hex"}`.
- NO trailing commas, NO comments, use straight `"` not smart quotes.
- Layout on frames: `"layout": "vertical" | "horizontal" | "none"`, `"gap": number`, `"padding": [top, right, bottom, left]`, `"alignItems": "start" | "center" | "end"`, `"justifyContent": "start" | "center" | "end" | "space-between"`.
- Width/height: number OR `"fill_container"` OR `"fit_content"`.
- Before calling the tool, mentally verify the JSON is valid. Every key has a value; every value has a key."##;

/// Build the `session/new` options for one local ACP turn against the live
/// MCP server: the `openpencil` HTTP endpoint (TS agent.ts:513-521)
/// plus the dedicated ACP system prompt via `_meta` (agent.ts:576-580).
fn session_options_for_port(port: u16) -> NewSessionOptions {
    NewSessionOptions {
        mcp_servers: vec![McpHttpServer {
            name: "openpencil".into(),
            url: format!("http://127.0.0.1:{port}/mcp"),
        }],
        system_prompt_meta: Some(ACP_SYSTEM_PROMPT.to_string()),
    }
}

/// Select the safe `session/new` payload for the transport. Returning `None`
/// means a local agent cannot access the canvas because the host MCP server is
/// stopped. Remote agents deliberately receive no desktop loopback endpoint.
fn session_options_for_connection(
    connection_type: ConnectionType,
    live_mcp_port: Option<u16>,
) -> Option<NewSessionOptions> {
    match connection_type {
        ConnectionType::Local => live_mcp_port.map(session_options_for_port),
        ConnectionType::Remote => Some(NewSessionOptions::default()),
    }
}

/// Resolve a local command through the same GUI-aware search path used by the
/// other CLI transports, and give npm shebangs the merged login-shell PATH.
/// Explicit per-agent PATH values remain authoritative.
fn prepare_connection_config(mut config: AcpAgentConfig) -> AcpAgentConfig {
    if config.connection_type != ConnectionType::Local {
        return config;
    }
    if let Some(command) = config.command.as_deref() {
        config.command = Some(op_host_services::chat_spawn::find_binary(command));
    }
    let has_explicit_path = config
        .env
        .keys()
        .any(|key| key.eq_ignore_ascii_case("PATH"));
    if !has_explicit_path {
        let path = op_host_services::chat_spawn::effective_path_env();
        if !path.is_empty() {
            config.env.insert("PATH".into(), path);
        }
    }
    config
}

const CANCEL_RESPONSE_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// Release and remove a per-turn ACP session before tearing down its transport.
/// Stable v1 gates both methods independently: close frees active resources,
/// then delete removes persisted state. Either remains a valid fallback alone,
/// and delete is still attempted if close reports an error.
async fn cleanup_turn_session(conn: &AcpConnection, session_id: &str) {
    if conn.supports_session_close() {
        let _ = conn.close_session_if_supported(session_id).await;
    }
    if conn.supports_session_delete() {
        let _ = conn.delete_session_if_supported(session_id).await;
    }
}

fn map_acp_stop_reason(reason: AcpStopReason) -> StopReason {
    match reason {
        AcpStopReason::EndTurn => StopReason::EndTurn,
        AcpStopReason::MaxTokens => StopReason::MaxTokens,
        AcpStopReason::Cancelled => StopReason::Aborted,
        AcpStopReason::MaxTurnRequests | AcpStopReason::Refusal => StopReason::Aborted,
        _ => StopReason::Aborted,
    }
}

fn select_value_matching(
    options: &SessionConfigSelectOptions,
    candidates: &[&str],
) -> Option<String> {
    let matches = |value: &str, name: &str| {
        candidates.iter().any(|candidate| {
            value.eq_ignore_ascii_case(candidate) || name.eq_ignore_ascii_case(candidate)
        })
    };
    match options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .iter()
            .find(|option| matches(option.value.0.as_ref(), &option.name))
            .map(|option| option.value.to_string()),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| group.options.iter())
            .find(|option| matches(option.value.0.as_ref(), &option.name))
            .map(|option| option.value.to_string()),
        _ => None,
    }
}

fn requested_config_value(
    session: &AcpSession,
    category: SessionConfigOptionCategory,
    candidates: &[&str],
) -> Option<(String, String)> {
    session.config_options.iter().find_map(|option| {
        if option.category.as_ref() != Some(&category) {
            return None;
        }
        let SessionConfigKind::Select(select) = &option.kind else {
            return None;
        };
        let value = select_value_matching(&select.options, candidates)?;
        (select.current_value.to_string() != value).then(|| (option.id.to_string(), value))
    })
}

async fn apply_session_preferences(
    conn: &AcpConnection,
    session: &mut AcpSession,
    model: Option<&str>,
    thinking: ThinkingMode,
    effort: EffortLevel,
) -> Result<(), op_acp::AcpError> {
    if let Some(model) = model {
        if let Some((config_id, value)) =
            requested_config_value(session, SessionConfigOptionCategory::Model, &[model])
        {
            session.config_options = conn
                .set_session_config_option(
                    &session.session_id,
                    &config_id,
                    SessionConfigOptionValue::value_id(value),
                )
                .await?;
        }
    }

    let thought_candidates: &[&str] = match thinking {
        ThinkingMode::Disabled => &["disabled", "off", "none"],
        ThinkingMode::Adaptive | ThinkingMode::Enabled => match effort {
            EffortLevel::Low => &["low", "minimal"],
            EffortLevel::Medium => &["medium", "normal"],
            EffortLevel::High => &["high"],
            EffortLevel::Max => &["max", "xhigh", "high"],
        },
    };
    if let Some((config_id, value)) = requested_config_value(
        session,
        SessionConfigOptionCategory::ThoughtLevel,
        thought_candidates,
    ) {
        session.config_options = conn
            .set_session_config_option(
                &session.session_id,
                &config_id,
                SessionConfigOptionValue::value_id(value),
            )
            .await?;
    }
    Ok(())
}

/// One-shot turn that reports the TS "MCP server is not running"
/// refusal (agent.ts:506-511) and ends.
fn mcp_not_running_turn() -> Box<dyn Iterator<Item = ChatDelta> + Send> {
    Box::new(
        vec![
            ChatDelta::Error(MCP_NOT_RUNNING_ERROR.to_string()),
            ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            },
        ]
        .into_iter(),
    )
}

/// `ChatProvider` backed by a third-party ACP agent.
pub struct AcpProvider {
    config: AcpAgentConfig,
    /// Bound port of the live in-process MCP server when it is
    /// running (`agent_settings.mcp_server`), `None` when stopped.
    /// Local ACP turns require it (TS parity — see module docs).
    live_mcp_port: Option<u16>,
    label: String,
}

impl AcpProvider {
    /// Build an ACP provider for a persisted agent config.
    /// `live_mcp_port` is the live MCP server's bound port when the
    /// server is running — the canvas tool surface advertised to the
    /// agent in `session/new`.
    #[allow(dead_code)]
    pub fn new(config: AcpAgentConfig, live_mcp_port: Option<u16>) -> Self {
        let label = format!("ACP: {}", config.display_name);
        Self {
            config,
            live_mcp_port,
            label,
        }
    }
}

impl ChatProvider for AcpProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

impl AcpProvider {
    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let config = prepare_connection_config(self.config.clone());
        // A loopback MCP URL is meaningful only to a local child. Never
        // disclose it to a remote WebSocket agent: 127.0.0.1 there points at
        // the remote host, not this desktop. Local agents retain the existing
        // hard requirement because canvas edits depend on this tool surface.
        let session_options =
            match session_options_for_connection(config.connection_type, self.live_mcp_port) {
                Some(options) => options,
                None => return mcp_not_running_turn(),
            };
        // ACP `session/prompt` carries plain text. A local agent can
        // read temp-file path lines; a remote (WebSocket) agent
        // cannot, so for remote agents the attachments are omitted
        // with an honest note rather than passing meaningless local
        // paths. The thinking / effort directive remains an in-band
        // fallback for agents that do not advertise a stable-v1
        // thought-level session config option.
        let (mut prompt, guard) = if config.connection_type == ConnectionType::Local {
            match op_host_services::chat_attachment::prompt_with_attachments(
                &request.user_message,
                &request.attachments,
            ) {
                Ok(pair) => pair,
                Err(e) => return op_host_services::chat_attachment::attachment_error_turn(e),
            }
        } else {
            let mut prompt = request.user_message.clone();
            if !request.attachments.is_empty() {
                prompt.push_str(&format!(
                    "\n\n[note: {} attachment(s) omitted — a remote ACP agent \
                     cannot read local files]",
                    request.attachments.len()
                ));
            }
            (prompt, None)
        };
        let mut directive = String::new();
        if let Some(d) = op_host_services::chat_attachment::thinking_directive(request.thinking) {
            directive.push_str(d);
        }
        if request.effort != EffortLevel::Low {
            if !directive.is_empty() {
                directive.push(' ');
            }
            directive.push_str(&format!(
                "Apply {} reasoning effort.",
                request.effort.as_str()
            ));
        }
        if !directive.is_empty() {
            prompt = format!("{directive}\n\n{prompt}");
        }
        // ACP `session/prompt` opens a fresh session per send — fold a
        // compact transcript digest in front so follow-up turns keep
        // their context (TS sends the last user message only,
        // agent.ts:592-597; the digest is the documented extra). The
        // per-turn chat system prompt is NOT folded in: the dedicated
        // ACP system prompt rides `_meta.systemPrompt` instead, like
        // TS (agent.ts:576-580).
        let digest = op_ai::chat_history::history_digest(
            &request.history,
            op_ai::chat_history::DEFAULT_DIGEST_CHARS,
        );
        if !digest.is_empty() {
            prompt = format!("{digest}\n\n{prompt}");
        }
        let requested_model = request.model_id().map(str::to_string);
        let thinking = request.thinking;
        let effort = request.effort;
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            run_acp_turn(
                AcpTurnRequest {
                    config,
                    session_options,
                    requested_model,
                    thinking,
                    effort,
                    prompt,
                    // Held for the turn so staged attachment temp files
                    // survive until the local agent has read them.
                    _guard: guard,
                },
                tx,
            )
            .await;
        });
        match cancel {
            Some(cancel) => Box::new(BlockingRecvIter::cooperative(rx, cancel)),
            None => Box::new(BlockingRecvIter::new(rx)),
        }
    }
}

struct AcpTurnRequest {
    config: AcpAgentConfig,
    session_options: NewSessionOptions,
    requested_model: Option<String>,
    thinking: ThinkingMode,
    effort: EffortLevel,
    prompt: String,
    _guard: Option<TempGuard>,
}

/// Connect, open a session (advertising the canvas MCP endpoint +
/// ACP system prompt for local agents), and drive one prompt turn —
/// streaming `session/update` notifications into `tx` as they arrive and
/// emitting a terminal `Done` once `session/prompt` returns.
async fn run_acp_turn(turn: AcpTurnRequest, tx: mpsc::Sender<ChatDelta>) {
    let AcpTurnRequest {
        config,
        session_options,
        requested_model,
        thinking,
        effort,
        prompt,
        _guard,
    } = turn;
    let connect = connect_acp_agent(&config);
    tokio::pin!(connect);
    let mut conn = match tokio::select! {
        result = &mut connect => Some(result),
        _ = tx.closed() => None,
    } {
        None => return,
        Some(Ok(connection)) => connection,
        Some(Err(e)) => {
            let _ = tx.send(ChatDelta::Error(format!("acp connect: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            return;
        }
    };
    let mut notes = match conn.take_notifications() {
        Some(n) => n,
        None => {
            let _ = tx
                .send(ChatDelta::Error(
                    "acp: notification channel unavailable".into(),
                ))
                .await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            conn.shutdown().await;
            return;
        }
    };
    let session_result = {
        let new_session = conn.new_session_with(&session_options);
        tokio::pin!(new_session);
        tokio::select! {
            result = &mut new_session => Some(result),
            _ = tx.closed() => None,
        }
    };
    let mut session = match session_result {
        None => {
            conn.shutdown().await;
            return;
        }
        Some(Ok(session)) => session,
        Some(Err(e)) => {
            let _ = tx.send(ChatDelta::Error(format!("acp session: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            conn.shutdown().await;
            return;
        }
    };

    let configure_result = {
        let configure = apply_session_preferences(
            &conn,
            &mut session,
            requested_model.as_deref(),
            thinking,
            effort,
        );
        tokio::pin!(configure);
        tokio::select! {
            result = &mut configure => Some(result),
            _ = tx.closed() => None,
        }
    };
    match configure_result {
        None => {
            let _ = conn.cancel_session(&session.session_id).await;
            cleanup_turn_session(&conn, &session.session_id).await;
            conn.shutdown().await;
            return;
        }
        Some(Ok(())) => {}
        Some(Err(e)) => {
            let _ = tx
                .send(ChatDelta::Error(format!("acp session config: {e}")))
                .await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            cleanup_turn_session(&conn, &session.session_id).await;
            conn.shutdown().await;
            return;
        }
    }

    enum TurnOutcome {
        Prompt(Result<AcpStopReason, op_acp::AcpError>),
        ReceiverDropped,
    }

    // Run the prompt turn while concurrently streaming notifications and
    // watching the consumer. Stop/New Chat drops the receiver, which now
    // sends the mandatory `session/cancel` notification before teardown.
    let outcome = {
        let prompt_fut = conn.prompt(&session.session_id, &prompt);
        tokio::pin!(prompt_fut);
        let mut notes_open = true;
        loop {
            tokio::select! {
                biased;
                _ = tx.closed() => {
                    let _ = conn.cancel_session(&session.session_id).await;
                    let _ = tokio::time::timeout(CANCEL_RESPONSE_GRACE, &mut prompt_fut).await;
                    break TurnOutcome::ReceiverDropped;
                }
                res = &mut prompt_fut => break TurnOutcome::Prompt(res),
                note = notes.recv(), if notes_open => match note {
                    Some(note) => {
                        if let Some(delta) = session_update_to_delta(&note) {
                            if tx.send(delta).await.is_err() {
                                let _ = conn.cancel_session(&session.session_id).await;
                                let _ = tokio::time::timeout(
                                    CANCEL_RESPONSE_GRACE,
                                    &mut prompt_fut,
                                )
                                .await;
                                break TurnOutcome::ReceiverDropped;
                            }
                        }
                    }
                    None => notes_open = false,
                }
            }
        }
    };

    if matches!(outcome, TurnOutcome::ReceiverDropped) {
        cleanup_turn_session(&conn, &session.session_id).await;
        conn.shutdown().await;
        return;
    }

    // Flush notifications buffered before the turn resolved.
    while let Ok(note) = notes.try_recv() {
        if let Some(delta) = session_update_to_delta(&note) {
            let _ = tx.send(delta).await;
        }
    }
    match outcome {
        TurnOutcome::Prompt(Ok(reason)) => {
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: map_acp_stop_reason(reason),
                })
                .await;
        }
        TurnOutcome::Prompt(Err(e)) => {
            let _ = tx.send(ChatDelta::Error(format!("acp prompt: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
        }
        TurnOutcome::ReceiverDropped => unreachable!("handled above"),
    }
    cleanup_turn_session(&conn, &session.session_id).await;
    conn.shutdown().await;
}

#[cfg(test)]
#[path = "chat_acp_tests.rs"]
mod tests;
