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
//! Like TS, a turn REQUIRES the MCP server to be running ("without it
//! they just call Terminal/Skill tools that don't work here",
//! agent.ts:497-512): the provider refuses with the TS error message
//! when it is stopped. The dedicated ACP system prompt rides
//! `_meta.systemPrompt` on `session/new` (agent.ts:576-580), ported
//! verbatim below.
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

use op_acp::{
    connect_acp_agent, session_update_to_delta, AcpAgentConfig, ConnectionType, McpHttpServer,
    NewSessionOptions,
};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
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

/// Build the `session/new` options for one ACP turn against the live
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
    /// ACP turns require it (TS parity — see module docs).
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

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        // TS parity: ACP agents require the OpenPencil MCP server —
        // refuse the turn up front when it is stopped (agent.ts:506).
        let Some(mcp_port) = self.live_mcp_port else {
            return mcp_not_running_turn();
        };
        let config = self.config.clone();
        // ACP `session/prompt` carries plain text. A local agent can
        // read temp-file path lines; a remote (WebSocket) agent
        // cannot, so for remote agents the attachments are omitted
        // with an honest note rather than passing meaningless local
        // paths. The thinking / effort knobs ride in-band either way
        // (ACP exposes no dedicated reasoning channel).
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
        let session_options = session_options_for_port(mcp_port);
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            run_acp_turn(config, session_options, prompt, guard, tx).await;
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Connect, open a session (advertising the canvas MCP endpoint +
/// ACP system prompt), and drive one prompt turn — streaming
/// `session/update` notifications into `tx` as they arrive and
/// emitting a terminal `Done` once `session/prompt` returns.
async fn run_acp_turn(
    config: AcpAgentConfig,
    session_options: NewSessionOptions,
    prompt: String,
    // Held for the turn so staged attachment temp files survive until
    // the agent has read them; dropped (and cleaned up) on return.
    _guard: Option<TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
) {
    let mut conn = match connect_acp_agent(&config).await {
        Ok(c) => c,
        Err(e) => {
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
            return;
        }
    };
    let session = match conn.new_session_with(&session_options).await {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(ChatDelta::Error(format!("acp session: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
            return;
        }
    };

    // Run the prompt turn while concurrently streaming notifications.
    let prompt_fut = conn.prompt(&session, &prompt);
    tokio::pin!(prompt_fut);
    let mut notes_open = true;
    let result = loop {
        tokio::select! {
            biased;
            res = &mut prompt_fut => break res,
            note = notes.recv(), if notes_open => match note {
                Some(note) => {
                    if let Some(delta) = session_update_to_delta(&note) {
                        if tx.send(delta).await.is_err() {
                            return; // chat panel went away
                        }
                    }
                }
                None => notes_open = false,
            }
        }
    };
    // Flush any notifications buffered before the turn resolved.
    while let Ok(note) = notes.try_recv() {
        if let Some(delta) = session_update_to_delta(&note) {
            let _ = tx.send(delta).await;
        }
    }
    match result {
        Ok(()) => {
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                })
                .await;
        }
        Err(e) => {
            let _ = tx.send(ChatDelta::Error(format!("acp prompt: {e}"))).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                })
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_acp::ConnectionType;
    use std::sync::Arc;

    fn config() -> AcpAgentConfig {
        AcpAgentConfig {
            id: "acp-1".into(),
            display_name: "Test Agent".into(),
            connection_type: ConnectionType::Local,
            command: Some("test-acp-agent".into()),
            args: vec![],
            env: std::collections::BTreeMap::new(),
            url: None,
            enabled: true,
        }
    }

    #[test]
    fn provider_label_names_the_agent() {
        let p = AcpProvider::new(config(), Some(3100));
        assert_eq!(p.provider_label(), "ACP: Test Agent");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        let _: Arc<dyn ChatProvider> = Arc::new(AcpProvider::new(config(), Some(3100)));
    }

    #[test]
    fn send_without_live_mcp_refuses_with_ts_error() {
        // TS parity (agent.ts:506-511): ACP turns hard-require the
        // running MCP server; the refusal is the exact TS message.
        let p = AcpProvider::new(config(), None);
        let deltas: Vec<ChatDelta> = p.send(ChatRequest::default()).collect();
        assert_eq!(deltas.len(), 2);
        match &deltas[0] {
            ChatDelta::Error(msg) => {
                assert!(msg.starts_with("MCP server is not running."), "got: {msg}");
                assert!(msg.contains("Settings → MCP"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(matches!(
            deltas[1],
            ChatDelta::Done {
                stop_reason: StopReason::Aborted
            }
        ));
    }

    #[test]
    fn session_options_advertise_openpencil_mcp_and_acp_prompt() {
        let options = session_options_for_port(4123);
        assert_eq!(options.mcp_servers.len(), 1);
        assert_eq!(options.mcp_servers[0].name, "openpencil");
        assert_eq!(options.mcp_servers[0].url, "http://127.0.0.1:4123/mcp");
        let prompt = options.system_prompt_meta.expect("ACP system prompt set");
        assert_eq!(prompt, ACP_SYSTEM_PROMPT);
    }

    #[test]
    fn acp_system_prompt_matches_ts_text_anchors() {
        // Spot-check the verbatim port of agent.ts:529-574 — first
        // line, MCP tool naming rule, pipeline phases, JSON rules tail.
        assert!(ACP_SYSTEM_PROMPT.starts_with(
            "You are an AI design assistant integrated inside the OpenPencil vector design tool."
        ));
        assert!(ACP_SYSTEM_PROMPT.contains("- DO use the `mcp__openpencil__*` tools"));
        assert!(ACP_SYSTEM_PROMPT.contains("2. **Build skeleton**: Call `design_skeleton`"));
        assert!(ACP_SYSTEM_PROMPT.contains("4. **Refine**: ALWAYS call `design_refine`"));
        assert!(ACP_SYSTEM_PROMPT.contains("Mobile top rhythm"));
        assert!(ACP_SYSTEM_PROMPT.contains("favorite/heart"));
        assert!(ACP_SYSTEM_PROMPT.contains("signature moment"));
        assert!(ACP_SYSTEM_PROMPT.ends_with("Every key has a value; every value has a key."));
    }
}
