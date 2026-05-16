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
            CliName::Copilot => "gh-copilot",
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
                ChatProviderKind::Subprocess(cli)
                | ChatProviderKind::HttpServer(cli) => cli.default_binary().into(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub system_prompt: String,
    pub user_message: String,
    pub max_output_tokens: u32,
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
}
