//! MCP tool `run_design_agent` — run the App's built-in design-agent loop
//! headlessly against the active document and land the finished design.
//!
//! This is the daemon-side equivalent of the desktop design generation
//! (`chat_session_launch_design.rs`): the same skills-assembled system
//! prompt (`op_ai_skills::design_agent_system_prompt_with_skills`), the
//! same tool-executing loop (`ConfiguredBuiltinProvider::send` →
//! `run_{anthropic,openai}_agent_loop`, `DESIGN_LOOP_MAX_TURNS`), the same
//! 15-tool surface including real offscreen `get_screenshot`, and the same
//! loop-end structural backstop (`finalize_on_exit`). The provider comes
//! from the daemon's persisted agent settings
//! (`EditorState.editor_ui.agent_settings.builtin_agents`), never from the
//! request.
//!
//! ## How the mutation reaches the host
//!
//! MCP tools are snapshots: the loop runs against a CLONE of the document
//! (the executor pattern proven by op-smoke's headless loop mode), and the
//! finished clone lands as ONE `EditorCommand::Batch`: delete the original
//! top-level frames, insert the authored roots id-stably, and carry the
//! variables/themes the loop set. A pre-flight failure, timeout, or empty
//! result therefore leaves the live document byte-identical.
//!
//! ## Blocking + timeout
//!
//! The call BLOCKS until the loop drains or the `timeout_seconds` budget
//! elapses. The provider loop runs on the chat runtime and is drained from
//! a dedicated thread, so a deadline can cancel the stream
//! (`send_cancellable`) without wedging the MCP connection thread.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, ChatToolExecutor, ChatToolResult, EffortLevel,
    FinalizeReport, StopReason, ThinkingMode, UnfilledScreensReport,
};
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use super::design_agent_run_error::DesignAgentRunError;
use crate::chat_builtin_http::{ConfiguredBuiltinProvider, DESIGN_LOOP_MAX_OUTPUT_TOKENS};
use crate::design_agent_tools::{
    design_tool_defs, execute_agent_tool_with_root_seed_guard, RootSeedGuard,
};

/// Default and ceiling wall-clock budgets. A full design loop with
/// screenshots takes minutes; the ceiling keeps a stuck provider from
/// pinning a connection thread for an hour.
const DEFAULT_TIMEOUT_SECONDS: u64 = 600;
const MAX_TIMEOUT_SECONDS: u64 = 1800;
/// `max_turns` ceiling — matches the loop's own hard bound territory.
const MAX_TURNS_CEILING: usize = 64;
/// Reject adversarial multi-megabyte briefs before prompt assembly.
const MAX_BRIEF_BYTES: usize = 32 * 1024;

/// Headless [`ChatToolExecutor`] owning the cloned `EditorState` behind a
/// mutex — the op-smoke loop-mode executor, rehomed so the daemon shares
/// one apply seam with the desktop (`execute_agent_tool_with_root_seed_guard`
/// → `state.apply(EditorCommand)`).
pub(crate) struct HeadlessDesignExecutor {
    state: Arc<Mutex<EditorState>>,
    root_seed_guard: Mutex<RootSeedGuard>,
    finalize_report: Mutex<Option<FinalizeReport>>,
}

impl HeadlessDesignExecutor {
    pub(crate) fn new(state: Arc<Mutex<EditorState>>, brief: &str) -> Self {
        Self {
            state,
            root_seed_guard: Mutex::new(RootSeedGuard::from_prompt(brief)),
            finalize_report: Mutex::new(None),
        }
    }

    fn take_finalize_report(&self) -> Option<FinalizeReport> {
        self.finalize_report
            .lock()
            .expect("finalize report mutex poisoned")
            .take()
    }
}

impl ChatToolExecutor for HeadlessDesignExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        // Sub-agent launches are desktop-only glue (sub_agent_session.rs);
        // the op_mcp stub would silently drop the sub-designers' work. Tell
        // the model honestly so it designs inline — the same contract the
        // headless smoke loop ships.
        if name == "spawn_agents" {
            let envelope = serde_json::json!({
                "success": false,
                "error": "spawn_agents is unavailable in the headless design loop — \
                          design the layout directly with batch_design instead of spawning sub-agents",
            });
            return ChatToolResult {
                content: envelope.to_string(),
                is_error: true,
            };
        }
        let mut state = self
            .state
            .lock()
            .expect("EditorState mutex poisoned by a panicking tool call");
        let mut root_seed_guard = self
            .root_seed_guard
            .lock()
            .expect("RootSeedGuard mutex poisoned by a panicking tool call");
        let (result, _mutated) = execute_agent_tool_with_root_seed_guard(
            &mut state,
            name,
            args_json,
            None,
            Some(&mut *root_seed_guard),
        );
        result
    }

    fn finalize(&self) -> FinalizeReport {
        let mut state = self
            .state
            .lock()
            .expect("EditorState mutex poisoned before finalize");
        let quality = op_orchestrator::apply_loop_finalize_counted(&mut state);
        let unfilled =
            op_orchestrator::unfilled_screens::finalize_and_mark_unfilled_screens(&mut state);
        let committed = op_orchestrator::unfilled_screens::list_screen_candidates(&state)
            .into_iter()
            .map(|hit| hit.name)
            .collect::<Vec<_>>();
        let report = FinalizeReport {
            screens: UnfilledScreensReport {
                committed,
                unfilled,
            },
            quality: crate::quality_credential::quality_summary_from_repairs(&quality),
        };
        *self
            .finalize_report
            .lock()
            .expect("finalize report mutex poisoned") = Some(report.clone());
        report
    }

    fn check_unfilled_screens(&self) -> UnfilledScreensReport {
        let state = self
            .state
            .lock()
            .expect("EditorState mutex poisoned before check_unfilled_screens");
        let unfilled = op_orchestrator::unfilled_screens::detect_unfilled_screens(&state)
            .into_iter()
            .map(|hit| hit.name)
            .collect();
        let committed = op_orchestrator::unfilled_screens::list_screen_candidates(&state)
            .into_iter()
            .map(|hit| hit.name)
            .collect();
        UnfilledScreensReport {
            committed,
            unfilled,
        }
    }
}

/// Transport seam so tests drive the tool without a network provider: the
/// production driver wraps a wired [`ConfiguredBuiltinProvider`]; tests
/// script deltas and mutate the clone through the shared executor.
pub(crate) trait DesignLoopDriver: Send {
    fn run(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send>;
}

struct BuiltinLoopDriver {
    provider: ConfiguredBuiltinProvider,
}

impl DesignLoopDriver for BuiltinLoopDriver {
    fn run(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.provider.send_cancellable(request, cancel)
    }
}

/// Factory seam: builds the driver AFTER the tool wires the executor, so
/// tests can capture the executor while production attaches it to the
/// provider's canvas-tool loop.
type DriverFactory =
    Box<dyn Fn(Arc<HeadlessDesignExecutor>) -> Box<dyn DesignLoopDriver> + Send + Sync>;

/// The `run_design_agent` tool: a snapshot of the document at registration
/// time. `driver_factory` is `None` in production (the builtin provider is
/// resolved per call from the snapshot's persisted agent settings); tests
/// inject a scripted driver.
pub struct RunDesignAgentTool {
    state: EditorState,
    driver_factory: Option<DriverFactory>,
}

/// Snapshot constructor used by the registry.
pub fn run_design_agent_snapshot(doc: &EditorState) -> RunDesignAgentTool {
    RunDesignAgentTool {
        state: doc.clone(),
        driver_factory: None,
    }
}

#[cfg(test)]
impl RunDesignAgentTool {
    /// Test-only constructor with an injected loop driver (never network).
    pub(crate) fn for_test(doc: &EditorState, driver_factory: DriverFactory) -> RunDesignAgentTool {
        RunDesignAgentTool {
            state: doc.clone(),
            driver_factory: Some(driver_factory),
        }
    }
}

struct ParsedArgs {
    brief: String,
    timeout: Duration,
    max_turns: Option<usize>,
    provider_id: Option<String>,
    model: Option<String>,
}

fn parse_args(args: &BTreeMap<String, String>) -> Result<ParsedArgs, String> {
    let brief = args
        .get("brief")
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    if brief.is_empty() {
        return Err(DesignAgentRunError::EmptyBrief.to_string());
    }
    if brief.len() > MAX_BRIEF_BYTES {
        return Err(format!(
            "brief is too large ({} bytes; maximum {MAX_BRIEF_BYTES})",
            brief.len()
        ));
    }
    let timeout_seconds = match args.get("timeout_seconds").map(|value| value.trim()) {
        None | Some("") => DEFAULT_TIMEOUT_SECONDS,
        Some(raw) => {
            let parsed = raw
                .parse::<u64>()
                .map_err(|_| format!("invalid timeout_seconds: {raw}"))?;
            if parsed == 0 {
                return Err("timeout_seconds must be greater than zero".to_string());
            }
            parsed.min(MAX_TIMEOUT_SECONDS)
        }
    };
    let max_turns = match args.get("max_turns").map(|value| value.trim()) {
        None | Some("") => None,
        Some(raw) => {
            let parsed = raw
                .parse::<usize>()
                .map_err(|_| format!("invalid max_turns: {raw}"))?;
            if parsed == 0 {
                return Err("max_turns must be greater than zero".to_string());
            }
            Some(parsed.min(MAX_TURNS_CEILING))
        }
    };
    let optional = |key: &str| {
        args.get(key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    Ok(ParsedArgs {
        brief,
        timeout: Duration::from_secs(timeout_seconds),
        max_turns,
        provider_id: optional("provider_id"),
        model: optional("model"),
    })
}

/// Resolve the builtin provider from persisted agent settings. Operator-owned
/// daemon configuration keeps the `Trusted` dial policy — the same trust tier
/// the desktop grants its own saved settings.
fn resolve_provider(
    state: &EditorState,
    provider_id: Option<&str>,
    model: Option<&str>,
    max_turns: Option<usize>,
    executor: Arc<HeadlessDesignExecutor>,
) -> Result<ConfiguredBuiltinProvider, DesignAgentRunError> {
    let agents = &state.editor_ui.agent_settings.builtin_agents;
    let config = match provider_id {
        Some(id) => agents
            .iter()
            .find(|agent| agent.id == id)
            .ok_or_else(|| DesignAgentRunError::UnknownProvider(id.to_string()))?,
        None => agents
            .iter()
            .find(|agent| agent.ready())
            .ok_or(DesignAgentRunError::NoConfiguredProvider)?,
    };
    if !config.ready() {
        return Err(DesignAgentRunError::ProviderNotReady(config.id.clone()));
    }
    let provider = match model {
        Some(model) => ConfiguredBuiltinProvider::from_builtin_agent_with_model(config, model)
            .ok_or_else(|| DesignAgentRunError::ModelNotSaved {
                provider: config.id.clone(),
                model: model.to_string(),
            })?,
        None => ConfiguredBuiltinProvider::from_builtin_agent(config)
            .ok_or_else(|| DesignAgentRunError::ProviderNotReady(config.id.clone()))?,
    };
    let provider = provider
        .with_canvas_tools(design_tool_defs(), executor)
        .with_loop_finalize();
    Ok(match max_turns {
        Some(turns) => provider.with_max_turns(turns),
        None => provider,
    })
}

struct DrainStats {
    tool_calls: usize,
    text_chars: usize,
    stop_reason: Option<StopReason>,
    last_error: Option<String>,
}

/// Drain the loop's delta stream on a dedicated thread so the deadline can
/// cancel a stalled provider without wedging this connection thread.
fn drain_with_deadline(
    driver: Box<dyn DesignLoopDriver>,
    request: ChatRequest,
    deadline: Instant,
) -> Result<DrainStats, DesignAgentRunError> {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<ChatDelta>();
    let thread_cancel = cancel.clone();
    std::thread::Builder::new()
        .name("op-design-agent-run".into())
        .spawn(move || {
            for delta in driver.run(request, thread_cancel) {
                if tx.send(delta).is_err() {
                    break;
                }
            }
        })
        .expect("spawn op-design-agent-run drain thread");

    let mut stats = DrainStats {
        tool_calls: 0,
        text_chars: 0,
        stop_reason: None,
        last_error: None,
    };
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            cancel.store(true, Ordering::Release);
            return Err(DesignAgentRunError::Timeout {
                seconds: 0, // caller rewrites with the configured budget
            });
        }
        match rx.recv_timeout(remaining) {
            Ok(ChatDelta::TextDelta(text)) => stats.text_chars += text.len(),
            Ok(ChatDelta::Thinking(_)) => {}
            Ok(ChatDelta::ToolUse { .. }) => stats.tool_calls += 1,
            Ok(ChatDelta::Error(message)) => stats.last_error = Some(message),
            Ok(ChatDelta::Done { stop_reason }) => stats.stop_reason = Some(stop_reason),
            // Stream closed: the loop ended (Done normally precedes close).
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(stats),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancel.store(true, Ordering::Release);
                return Err(DesignAgentRunError::Timeout { seconds: 0 });
            }
        }
    }
}

/// Build the landing batch: replace the original top-level frames with the
/// clone's authored roots (id-stable), then carry variables/themes when the
/// loop changed them.
fn landing_batch(original: &EditorState, finished: &EditorState) -> Option<EditorCommand> {
    let authored = finished.active_children().to_vec();
    if authored.is_empty() {
        return None;
    }
    let mut commands = Vec::new();
    for root in original.active_children() {
        commands.push(EditorCommand::DeleteNode {
            node_id: NodeId::new(root.base().id.clone()),
            page_id: None,
        });
    }
    commands.push(EditorCommand::InsertAuthoredSubtreePreservingRoots {
        nodes: authored,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    if finished.doc.variables != original.doc.variables {
        commands.push(EditorCommand::SetVariables {
            variables: finished.doc.variables.clone().unwrap_or_default(),
            replace: true,
        });
    }
    if finished.doc.themes != original.doc.themes {
        commands.push(EditorCommand::SetThemes {
            themes: finished.doc.themes.clone().unwrap_or_default(),
            replace: true,
        });
    }
    Some(EditorCommand::Batch { commands })
}

impl McpTool for RunDesignAgentTool {
    fn name(&self) -> &str {
        "run_design_agent"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let parsed = match parse_args(args) {
            Ok(parsed) => parsed,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };

        let clone = Arc::new(Mutex::new(self.state.clone()));
        let executor = Arc::new(HeadlessDesignExecutor::new(clone.clone(), &parsed.brief));
        let driver: Box<dyn DesignLoopDriver> = match &self.driver_factory {
            Some(factory) => factory(executor.clone()),
            None => {
                match resolve_provider(
                    &self.state,
                    parsed.provider_id.as_deref(),
                    parsed.model.as_deref(),
                    parsed.max_turns,
                    executor.clone(),
                ) {
                    Ok(provider) => Box::new(BuiltinLoopDriver { provider }),
                    Err(error) => {
                        return ToolOutcome::Err(ToolErrorCode::InvalidArgument, error.to_string())
                    }
                }
            }
        };

        let request = ChatRequest {
            system_prompt: op_ai_skills::design_agent_system_prompt_with_skills(&parsed.brief),
            user_message: parsed.brief.clone(),
            history: Vec::new(),
            max_output_tokens: DESIGN_LOOP_MAX_OUTPUT_TOKENS,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::default(),
            attachments: Vec::new(),
            model: None,
        };

        let deadline = Instant::now() + parsed.timeout;
        let stats = match drain_with_deadline(driver, request, deadline) {
            Ok(stats) => stats,
            Err(DesignAgentRunError::Timeout { .. }) => {
                let error = DesignAgentRunError::Timeout {
                    seconds: parsed.timeout.as_secs(),
                };
                return ToolOutcome::Err(ToolErrorCode::ToolFailed, error.to_string());
            }
            Err(error) => return ToolOutcome::Err(ToolErrorCode::ToolFailed, error.to_string()),
        };

        let finalize = executor.take_finalize_report();
        let finished = clone
            .lock()
            .expect("EditorState mutex poisoned after the loop drained")
            .clone();
        let Some(command) = landing_batch(&self.state, &finished) else {
            let message = match stats.last_error {
                Some(error) => DesignAgentRunError::Loop(error).to_string(),
                None => DesignAgentRunError::EmptyResult.to_string(),
            };
            return ToolOutcome::Err(ToolErrorCode::ToolFailed, message);
        };

        let landed_roots = finished.active_children().len();
        let finalize_json = finalize.as_ref().map(|report| {
            serde_json::json!({
                "committedScreens": report.screens.committed,
                "unfilledScreens": report.screens.unfilled,
                "qualityChecks": report.quality.checks,
                "qualityRepairs": report
                    .quality
                    .repairs
                    .iter()
                    .map(|(key, count)| serde_json::json!({ "check": key, "repairs": count }))
                    .collect::<Vec<_>>(),
                "qualityNotes": report.quality.notes,
            })
        });
        let json = serde_json::json!({
            "toolCalls": stats.tool_calls,
            "textChars": stats.text_chars,
            "stopReason": stats.stop_reason.map(|reason| format!("{reason:?}")),
            "landedRoots": landed_roots,
            "finalize": finalize_json,
            "loopError": stats.last_error,
        })
        .to_string();
        ToolOutcome::OkJsonWithCommand(json, command)
    }
}
