//! Chat / agent provider abstraction. Four backend categories
//! mirror the architecture decision in
//! [[project_agent_runtime]]:
//!
//! - **BuiltIn** — `agent-rs` crate's `QueryEngine` runs in-process
//!   against the user's chosen `Provider` (Anthropic, OpenAI-compat,
//!   Ollama, ...). This is the OP-native agent.
//! - **Subprocess(CliName)** — spawn an external CLI binary
//!   (Claude Code / Gemini / Copilot) and pipe line-delimited JSON
//!   over its stdin / stdout.
//! - **HttpServer(CliName)** — spawn `codex serve` / `opencode serve`
//!   and hit its local HTTP/SSE endpoint with reqwest.
//! - **Acp** — Agent Client Protocol (ndJSON over stdio); the
//!   open extension point for third-party agents OP doesn't ship a
//!   dedicated adapter for.
//!
//! shell-core only carries the data shapes + the trait. Real
//! transports live in the future `pen-agent-cli` crate (desktop-
//! side) because they pull tokio / reqwest / process-spawn which
//! shell-core's wasm32 target can't accept. The `EchoProvider` test
//! double stays here so widget tests don't need a real backend.

/// Which external CLI a Subprocess / HttpServer / Acp provider
/// is bridging to. Subprocess + Acp paths spawn the binary and
/// talk over stdio; HttpServer spawns with a `serve` subcommand
/// then connects via HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliName {
    /// Anthropic's `claude` CLI (subprocess IPC).
    ClaudeCode,
    /// Google's `gemini` CLI (subprocess IPC).
    Gemini,
    /// GitHub Copilot CLI (subprocess IPC).
    Copilot,
    /// OpenAI Codex CLI (HTTP server mode).
    Codex,
    /// OpenCode AI's CLI (HTTP server mode).
    OpenCode,
}

impl CliName {
    pub fn label(self) -> &'static str {
        match self {
            CliName::ClaudeCode => "Claude Code",
            CliName::Gemini => "Gemini",
            CliName::Copilot => "GitHub Copilot",
            CliName::Codex => "Codex",
            CliName::OpenCode => "OpenCode",
        }
    }
    /// Default binary name on PATH. Users override via
    /// `ChatProviderConfig::binary` when a non-standard install
    /// location applies.
    pub fn default_binary(self) -> &'static str {
        match self {
            CliName::ClaudeCode => "claude",
            CliName::Gemini => "gemini",
            // The standalone `copilot` CLI (the `gh-copilot` gh
            // extension is retired; the official SDK + model
            // discovery both target `copilot`).
            CliName::Copilot => "copilot",
            CliName::Codex => "codex",
            CliName::OpenCode => "opencode",
        }
    }
    /// Which backend transport this CLI uses. Mirrors the table in
    /// [[project_agent_runtime]] memory:
    /// Claude/Gemini/Copilot = subprocess IPC; Codex/OpenCode = HTTP server.
    pub fn backend(self) -> ChatProviderKind {
        match self {
            CliName::ClaudeCode | CliName::Gemini | CliName::Copilot => {
                ChatProviderKind::Subprocess(self)
            }
            CliName::Codex | CliName::OpenCode => ChatProviderKind::HttpServer(self),
        }
    }
}

/// Provider backend category. The widget host dispatches each
/// chat message through the corresponding transport in
/// `pen-agent-cli`. Built-in keeps everything in-process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatProviderKind {
    /// `agent-rs` QueryEngine in-process (built-in agent).
    BuiltIn,
    /// Spawn the CLI binary + talk over stdio (line-delimited JSON).
    Subprocess(CliName),
    /// Spawn `<cli> serve` + talk over the resulting HTTP endpoint.
    HttpServer(CliName),
    /// Agent Client Protocol via ndJSON over stdio. The catch-all
    /// for third-party agents OP doesn't carry a dedicated adapter.
    Acp,
}

/// Persisted per-provider config the chat panel + agent-settings
/// modal own. Fields are interpreted per `kind`:
/// - `BuiltIn` — `api_key` + `endpoint` + `model` are passed to the
///   underlying `agent-rs` Provider (e.g. Anthropic).
/// - `Subprocess` / `HttpServer` / `Acp` — `binary` overrides
///   `CliName::default_binary()`; `endpoint` for HttpServer is the
///   bind URL (defaults to `127.0.0.1:0` so the OS picks a port).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatProviderConfig {
    pub kind: ChatProviderKind,
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
    pub binary: String,
}

impl ChatProviderConfig {
    /// Empty default — every string blank, kind = BuiltIn. The
    /// settings modal pre-fills user-facing inputs from this seed.
    pub fn new(kind: ChatProviderKind) -> Self {
        Self {
            kind,
            api_key: String::new(),
            endpoint: String::new(),
            model: String::new(),
            binary: match kind {
                ChatProviderKind::Subprocess(cli) | ChatProviderKind::HttpServer(cli) => {
                    cli.default_binary().into()
                }
                _ => String::new(),
            },
        }
    }
}

/// Streaming delta from a provider — text fragments, tool calls,
/// status events. Mirrors `agent-rs`'s `stream::Event` enum (which
/// is the cross-product source of truth); shell-core duplicates the
/// shape so widget code doesn't need agent-rs in its dep graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatDelta {
    TextDelta(String),
    Thinking(String),
    ToolUse { name: String, args: String },
    Done { stop_reason: StopReason },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    Aborted,
    MaxTokens,
    ToolUse,
}

/// Thinking / reasoning-budget control for a chat turn. `Adaptive`
/// lets the provider decide; `Disabled` suppresses extended thinking;
/// `Enabled` forces it. Mirrors the TS chat panel's thinking-mode
/// selector (`apps/web/.../ai/chat.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThinkingMode {
    #[default]
    Adaptive,
    Disabled,
    Enabled,
}

/// Reasoning-effort hint. Each provider maps it onto its own knob
/// (Claude's thinking-token budget, Codex's `--effort`, …); a
/// provider with no such knob ignores it. The default is `Low`, to
/// match TS `ai-runtime-config.ts::DEFAULT_THINKING_EFFORT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffortLevel {
    #[default]
    Low,
    Medium,
    High,
    Max,
}

impl ThinkingMode {
    /// Lowercase wire token (TS parity: `"adaptive"` / `"disabled"` /
    /// `"enabled"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ThinkingMode::Adaptive => "adaptive",
            ThinkingMode::Disabled => "disabled",
            ThinkingMode::Enabled => "enabled",
        }
    }
}

impl EffortLevel {
    /// Lowercase wire token (`"low"` / `"medium"` / `"high"` / `"max"`).
    pub fn as_str(self) -> &'static str {
        match self {
            EffortLevel::Low => "low",
            EffortLevel::Medium => "medium",
            EffortLevel::High => "high",
            EffortLevel::Max => "max",
        }
    }

    /// Extended-thinking token budget a provider with a numeric knob
    /// (Claude's `max_thinking_tokens`, …) should use for this effort.
    /// The range tracks TS `ai-runtime-config.ts` — Low is a modest
    /// budget, Max saturates a typical extended-thinking ceiling.
    pub fn budget_tokens(self) -> u32 {
        match self {
            EffortLevel::Low => 4096,
            EffortLevel::Medium => 10_000,
            EffortLevel::High => 24_000,
            EffortLevel::Max => 32_000,
        }
    }
}

/// A file the user attached to a chat turn — typically a pasted or
/// picked image. Mirrors TS `ChatAttachment` (`apps/web/.../ai`),
/// minus the UI-only `id` / `size` fields. `data` is the raw decoded
/// bytes; providers base64-encode (Claude image blocks) or spill to a
/// temp file (CLI subprocesses) as their wire format demands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatAttachment {
    /// Original file name, e.g. `screenshot.png`.
    pub name: String,
    /// MIME type, e.g. `image/png`.
    pub media_type: String,
    /// Raw file bytes (not base64).
    pub data: Vec<u8>,
}

impl ChatAttachment {
    /// True when this attachment is an image — the only kind every
    /// provider can ingest (as an image content block).
    pub fn is_image(&self) -> bool {
        self.media_type.starts_with("image/")
    }
}

/// Author of one prior chat turn carried in [`ChatRequest::history`].
/// Mirrors the TS chat wire's `role: 'user' | 'assistant'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatHistoryRole {
    User,
    Assistant,
}

impl ChatHistoryRole {
    /// Lowercase wire token (`"user"` / `"assistant"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ChatHistoryRole::User => "user",
            ChatHistoryRole::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatRequest {
    pub system_prompt: String,
    pub user_message: String,
    /// Prior conversation turns (oldest first), excluding
    /// `user_message` itself. Transports that speak a real messages
    /// wire (builtin Anthropic / OpenAI-compatible HTTP) send these
    /// as full `messages[]` entries; prompt-only CLI transports fold
    /// them into a compact digest (see `op_ai::chat_history`).
    /// Text-only by design — attachments stay current-turn-only.
    pub history: Vec<(ChatHistoryRole, String)>,
    pub max_output_tokens: u32,
    /// Thinking-mode control for this turn (default `Adaptive`).
    pub thinking: ThinkingMode,
    /// Reasoning-effort hint for this turn (default `Low`, TS parity).
    pub effort: EffortLevel,
    /// Files attached to this turn (images, …). Empty for a plain
    /// text turn. Each provider maps these onto its own wire format.
    pub attachments: Vec<ChatAttachment>,
    /// Model id the user picked in the chat model picker (e.g.
    /// `gpt-5.5`, `claude-sonnet-4-6`). Each transport forwards it on
    /// its own knob (`--model` for Codex, `-m` for Gemini, SDK
    /// `options.model` for Claude Code / Copilot). `None` keeps the
    /// provider's own default — TS parity: every provider in
    /// `apps/web/server/api/ai/chat.ts` only sets the model when one
    /// was supplied.
    pub model: Option<String>,
}

impl ChatRequest {
    /// The selected model id, trimmed. Returns `None` when unset or
    /// blank so transports never emit an empty model flag.
    pub fn model_id(&self) -> Option<&str> {
        let m = self.model.as_deref()?.trim();
        if m.is_empty() {
            None
        } else {
            Some(m)
        }
    }
}

/// Provider abstraction the widget host calls. Implementations live
/// in the future `pen-agent-cli` desktop crate (one per kind):
/// `BuiltInProvider` wraps `agent-rs`, `SubprocessProvider` /
/// `HttpServerProvider` / `AcpProvider` each own their transport.
/// shell-core only carries the trait + the test double.
pub trait ChatProvider: Send + Sync {
    fn provider_label(&self) -> &str;
    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send>;
}

/// One canvas tool exposed to a tool-capable chat transport (the
/// builtin Anthropic / OpenAI-compatible agent loop). Mirrors the TS
/// `ToolDef` in `apps/web/src/services/ai/agent-tools.ts` — name,
/// description, auth level, and the JSON Schema the wire carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatToolDef {
    pub name: String,
    pub description: String,
    /// TS `AuthLevel` token: `read` / `create` / `modify` / `delete`.
    /// Rides into the transcript tool-card envelope so the chat panel
    /// can pick its collapsed/expanded default per level.
    pub level: String,
    /// Pre-serialized JSON Schema object for the tool's arguments.
    pub input_schema_json: String,
}

/// Result of executing one chat tool call. `content` is the JSON the
/// model sees as the tool result (TS shape: `{"success":true,"data":…}`
/// or `{"success":false,"error":…}`); `is_error` marks transport-level
/// failure so Anthropic `tool_result` blocks can set `is_error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatToolResult {
    pub content: String,
    pub is_error: bool,
}

/// Executes one canvas tool call on behalf of a chat agent loop. The
/// desktop host implements this with a channel bridge that forwards
/// the call to the UI thread (mutations must run against the live
/// `EditorState`), mirroring how the design orchestrator's
/// `RemoteDocSink` forwards `EditorCommand`s. Called from a worker
/// thread — implementations may block until the host replies.
pub trait ChatToolExecutor: Send + Sync {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult;
}

/// Test double — replays a fixed delta script. Lets the chat widget
/// run unit tests without spinning up agent-rs / a CLI subprocess /
/// an HTTP server.
pub struct EchoProvider {
    pub script: Vec<ChatDelta>,
}

impl ChatProvider for EchoProvider {
    fn provider_label(&self) -> &str {
        "echo"
    }
    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(self.script.clone().into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_name_backend_table_matches_architecture_memo() {
        // project_agent_runtime memory:
        //  Subprocess IPC = Claude Code / Gemini / Copilot
        //  HTTP server   = Codex / OpenCode
        assert!(matches!(
            CliName::ClaudeCode.backend(),
            ChatProviderKind::Subprocess(_)
        ));
        assert!(matches!(
            CliName::Gemini.backend(),
            ChatProviderKind::Subprocess(_)
        ));
        assert!(matches!(
            CliName::Copilot.backend(),
            ChatProviderKind::Subprocess(_)
        ));
        assert!(matches!(
            CliName::Codex.backend(),
            ChatProviderKind::HttpServer(_)
        ));
        assert!(matches!(
            CliName::OpenCode.backend(),
            ChatProviderKind::HttpServer(_)
        ));
    }

    #[test]
    fn cli_default_binary_uses_expected_names() {
        assert_eq!(CliName::ClaudeCode.default_binary(), "claude");
        assert_eq!(CliName::Codex.default_binary(), "codex");
        assert_eq!(CliName::OpenCode.default_binary(), "opencode");
    }

    #[test]
    fn provider_config_new_seeds_binary_for_cli_kinds() {
        let cfg = ChatProviderConfig::new(ChatProviderKind::Subprocess(CliName::ClaudeCode));
        assert_eq!(cfg.binary, "claude");
        let cfg2 = ChatProviderConfig::new(ChatProviderKind::HttpServer(CliName::Codex));
        assert_eq!(cfg2.binary, "codex");
        // BuiltIn / Acp leave binary empty — built-in needs no
        // spawn target; Acp's binary is user-supplied per-instance.
        let cfg3 = ChatProviderConfig::new(ChatProviderKind::BuiltIn);
        assert!(cfg3.binary.is_empty());
        let cfg4 = ChatProviderConfig::new(ChatProviderKind::Acp);
        assert!(cfg4.binary.is_empty());
    }

    #[test]
    fn echo_provider_replays_script() {
        let p = EchoProvider {
            script: vec![
                ChatDelta::TextDelta("Hello".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        };
        let req = ChatRequest {
            system_prompt: String::new(),
            user_message: "hi".into(),
            max_output_tokens: 1024,
            ..Default::default()
        };
        let mut iter = p.send(req);
        match iter.next() {
            Some(ChatDelta::TextDelta(s)) => assert_eq!(s, "Hello"),
            _ => panic!(),
        }
        match iter.next() {
            Some(ChatDelta::Done { .. }) => {}
            _ => panic!(),
        }
    }

    #[test]
    fn cli_label_is_human_readable() {
        assert_eq!(CliName::ClaudeCode.label(), "Claude Code");
        assert_eq!(CliName::OpenCode.label(), "OpenCode");
    }

    #[test]
    fn chat_request_thinking_effort_defaults_and_wire_tokens() {
        // A defaulted request reasons adaptively at low effort —
        // matching TS `DEFAULT_THINKING_MODE` / `DEFAULT_THINKING_EFFORT`.
        let req = ChatRequest::default();
        assert_eq!(req.thinking, ThinkingMode::Adaptive);
        assert_eq!(req.effort, EffortLevel::Low);
        // Wire tokens match the TS chat-request vocabulary.
        assert_eq!(ThinkingMode::Adaptive.as_str(), "adaptive");
        assert_eq!(ThinkingMode::Disabled.as_str(), "disabled");
        assert_eq!(ThinkingMode::Enabled.as_str(), "enabled");
        assert_eq!(EffortLevel::Low.as_str(), "low");
        assert_eq!(EffortLevel::Medium.as_str(), "medium");
        assert_eq!(EffortLevel::High.as_str(), "high");
        assert_eq!(EffortLevel::Max.as_str(), "max");
    }

    #[test]
    fn effort_budget_tokens_climbs_with_level() {
        // A provider with a numeric thinking knob scales its budget
        // with effort; the table is monotonically increasing.
        assert_eq!(EffortLevel::Low.budget_tokens(), 4096);
        assert_eq!(EffortLevel::Medium.budget_tokens(), 10_000);
        assert_eq!(EffortLevel::High.budget_tokens(), 24_000);
        assert_eq!(EffortLevel::Max.budget_tokens(), 32_000);
        assert!(EffortLevel::Low.budget_tokens() < EffortLevel::Medium.budget_tokens());
        assert!(EffortLevel::High.budget_tokens() < EffortLevel::Max.budget_tokens());
    }

    #[test]
    fn chat_request_attachments_default_empty() {
        let req = ChatRequest::default();
        assert!(req.attachments.is_empty());
    }

    #[test]
    fn chat_request_model_defaults_to_none() {
        // No model selected = the CLI keeps its own default; no flag
        // is ever emitted for the unset case.
        let req = ChatRequest::default();
        assert!(req.model.is_none());
        assert!(req.model_id().is_none());
    }

    #[test]
    fn chat_request_model_id_trims_and_rejects_blank() {
        let mut req = ChatRequest {
            model: Some("  gpt-5.5  ".into()),
            ..Default::default()
        };
        assert_eq!(req.model_id(), Some("gpt-5.5"));
        // Blank / whitespace-only ids are treated as unset so a bare
        // `--model` flag can never reach a CLI.
        req.model = Some("   ".into());
        assert!(req.model_id().is_none());
        req.model = Some(String::new());
        assert!(req.model_id().is_none());
    }

    #[test]
    fn chat_request_history_defaults_empty_and_roles_have_wire_tokens() {
        // A defaulted request is single-shot — no prior turns. The
        // role tokens match the TS chat wire vocabulary.
        let req = ChatRequest::default();
        assert!(req.history.is_empty());
        assert_eq!(ChatHistoryRole::User.as_str(), "user");
        assert_eq!(ChatHistoryRole::Assistant.as_str(), "assistant");
    }

    #[test]
    fn chat_attachment_is_image_checks_media_type() {
        let png = ChatAttachment {
            name: "shot.png".into(),
            media_type: "image/png".into(),
            data: vec![1, 2, 3],
        };
        assert!(png.is_image());
        let txt = ChatAttachment {
            name: "notes.txt".into(),
            media_type: "text/plain".into(),
            data: vec![],
        };
        assert!(!txt.is_image());
    }
}
