//! Headless smoke runner for `op-orchestrator`.
//!
//! Drives one design turn against an API or production CLI provider without
//! the desktop UI / `DesignSession` actor model — single-threaded
//! `block_on(Orchestrator::run)` against an inline `DocSink`, with every
//! progress event + every applied `EditorCommand` dumped to stderr.
//!
//! ## Usage
//!
//! ```sh
//! export OPENPENCIL_ANTHROPIC_API_KEY=sk-ant-...   # or ANTHROPIC_API_KEY
//! cargo run -p op-smoke -- "design a login screen"
//! ```
//!
//! Optional env overrides:
//! - `OPENPENCIL_ORCHESTRATOR_MODEL` — default `claude-sonnet-4-6`.
//! - `OPENPENCIL_LLM_PROVIDER` — `anthropic` (default), `openai-compat`,
//!   or `antigravity` / `agy`. The Antigravity arm reuses the production
//!   generation-only subprocess transport and the CLI's existing login.
//! - `OPENPENCIL_SMOKE_VALIDATION=1` — opt into the production lint
//!   pre-validator and post-generation validation stage. The default remains
//!   skipped so existing smoke traces are unchanged.
//!
//! ## What this verifies vs the desktop GUI smoke
//!
//! - LLM client construction (API credentials or the production CLI bridge).
//! - `Orchestrator::run` reaching the network (200 OK / 401 / 429 etc.
//!   surfaces as a `LlmError` in the streamed events).
//! - Planner → scaffold → subtask → cleanup transitions
//!   (`Progress::*` enum, every variant rendered to stderr).
//! - `EditorCommand` applied to the in-memory state, including
//!   `InsertSubtree` ID-remapping.
//! - Terminal `RunSummary` (subtask outcomes + total node count) or
//!   `OrchestratorError`.
//!
//! What this does NOT verify (run the desktop binary for those):
//! - Canvas rendering / paint correctness.
//! - chat panel rendering of progress lines / streaming bubble.
//! - Cross-session abort (mid-turn switch to chat — covered by
//!   `chat_session::launch_if_pending` host tests).
//! - Pre-validation fixes by default — smoke uses `SkippedPreValidator`
//!   unless `OPENPENCIL_SMOKE_VALIDATION=1` explicitly opts into the
//!   production `LintPreValidator`.

use std::sync::Arc;

mod audit_rubric;
mod best_of;
mod llm_clients;
mod loop_mode;
mod loop_seed;
mod modify_mode;

use agent::provider::anthropic::AnthropicProvider;
use agent::provider::openai_compat::{OpenAiCompatConfig, OpenAiCompatProvider};
use op_ai::chat_provider::{ChatProvider, CliName};
use op_editor_core::{EditorCommand, EditorState};
use op_host_services::chat_provider_llm::ChatProviderLlmClient;
use op_host_services::chat_subprocess::SubprocessProvider;
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, LlmClient, Orchestrator, PreValidator, Progress,
    SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient, ValidationProviders,
};

use crate::llm_clients::{DirectOpenAiClient, SmokeLlmClient};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SmokeProviderKind {
    Anthropic,
    OpenAiCompat,
    Antigravity,
}

impl SmokeProviderKind {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" | "openai-compat" => Some(Self::OpenAiCompat),
            "antigravity" | "agy" => Some(Self::Antigravity),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAiCompat => "openai-compat",
            Self::Antigravity => "antigravity",
        }
    }
}

fn truthy_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on"
        )
    })
}

fn antigravity_llm(model: &str) -> Box<dyn LlmClient> {
    let provider = SubprocessProvider::for_cli_generation(CliName::Antigravity)
        .expect("Antigravity has a production subprocess transport");
    let provider: Arc<dyn ChatProvider> = Arc::new(provider);
    Box::new(ChatProviderLlmClient::new(provider).with_model(Some(model.to_string())))
}

/// Inline `DocSink` — owns the canonical state directly, no channel hop.
/// Every `apply` echoes the command kind + result so the smoke trace
/// shows the orchestrator's mutations linearly.
struct InlineDocSink {
    state: EditorState,
}

impl DocSink for InlineDocSink {
    fn state(&self) -> &EditorState {
        &self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        let label = describe_cmd(&cmd);
        let applied = self.state.apply(cmd);
        eprintln!("[CMD] {label} → applied={applied}");
        applied
    }

    fn begin_undo_batch(&mut self) {
        eprintln!("[UNDO] begin");
    }

    fn end_undo_batch(&mut self) {
        eprintln!("[UNDO] end");
    }
}

/// One-line label for an `EditorCommand` variant. We don't dump the full
/// payload (often kilobytes of node JSON) — just the variant + its key
/// identifying field so the trace stays readable.
fn describe_cmd(cmd: &EditorCommand) -> String {
    match cmd {
        EditorCommand::InsertSubtree {
            nodes, parent_id, ..
        } => {
            format!("InsertSubtree(parent={parent_id:?}, nodes={})", nodes.len())
        }
        EditorCommand::UpdateNode { node_id, .. } => format!("UpdateNode({node_id:?})"),
        EditorCommand::DeleteNode { node_id, .. } => format!("DeleteNode({node_id:?})"),
        EditorCommand::MoveNode { node_id, .. } => format!("MoveNode({node_id:?})"),
        EditorCommand::SetNodeLayoutProp {
            node_id, property, ..
        } => format!("SetNodeLayoutProp({node_id:?}, prop={property:?})"),
        EditorCommand::SetNodeStrokeHex { node_id, hex } => {
            format!("SetNodeStrokeHex({node_id:?}, {hex})")
        }
        EditorCommand::SetNodeStrokeWidth { node_id, .. } => {
            format!("SetNodeStrokeWidth({node_id:?})")
        }
        EditorCommand::SetNodeFillHex { node_id, hex } => {
            format!("SetNodeFillHex({node_id:?}, {hex})")
        }
        EditorCommand::RemoveNodeEffect { node_id, index } => {
            format!("RemoveNodeEffect({node_id:?}, [{index}])")
        }
        other => {
            let dbg = format!("{other:?}");
            // Truncate the Debug output so massive payloads don't blow up the trace.
            if dbg.len() > 120 {
                format!("{}...", dbg.chars().take(117).collect::<String>())
            } else {
                dbg
            }
        }
    }
}

/// Honor `OPENPENCIL_SMOKE_LIBRARY=<path>` by merging a harvested component
/// library (`.lib.op`) into `state` before generation. Unset ⇒ no-op (`Ok`),
/// preserving today's byte-for-byte behavior. On a load/parse error returns
/// `Err(ExitCode)` so the caller aborts — a benchmark that asked for a library
/// must not silently run without it. Shared by both smoke modes (orchestrator +
/// loop) so the library is available whichever path runs.
fn maybe_merge_smoke_library(state: &mut EditorState) -> Result<(), std::process::ExitCode> {
    let path = match std::env::var("OPENPENCIL_SMOKE_LIBRARY") {
        Ok(p) if !p.is_empty() => p,
        _ => return Ok(()),
    };
    match op_pen_loader::merge_library_into_state(state, &path) {
        Ok(report) => {
            eprintln!(
                "[SMOKE] library merged from {path}: +{} master(s), {} component(s) total, \
                 +{} variable(s), +{} theme axis(es)",
                report.masters_added,
                report.component_count,
                report.variables_added,
                report.themes_added
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("[SMOKE] library load failed ({path}): {e}");
            Err(std::process::ExitCode::from(5))
        }
    }
}

/// Map the chat `OPENPENCIL_SMOKE_*` thinking knobs onto a `ThinkingMode`.
/// Default `Disabled` for ab-v9 parity with the orchestrator DIRECT path
/// (reasoning models burn their output budget on `<think>` otherwise);
/// `OPENPENCIL_SMOKE_KEEP_THINKING=1` keeps reasoning on.
fn loop_thinking_mode() -> op_ai::chat_provider::ThinkingMode {
    use op_ai::chat_provider::ThinkingMode;
    if std::env::var("OPENPENCIL_SMOKE_KEEP_THINKING").is_ok() {
        ThinkingMode::Enabled
    } else {
        ThinkingMode::Disabled
    }
}

/// Headless agentic tool-loop branch (`OPENPENCIL_SMOKE_LOOP=1`).
///
/// Reads the openai-compat `OPENPENCIL_LLM_*` env (the ab-v9 wire), runs the
/// production builtin design loop against a live `EditorState`, then dumps
/// `state.doc` to `OPENPENCIL_SMOKE_OUT` using the SAME serialize path as the
/// orchestrator mode (`serde_json::to_string_pretty` → `std::fs::write`).
async fn run_loop_mode(prompt: String) -> std::process::ExitCode {
    let model =
        std::env::var("OPENPENCIL_ORCHESTRATOR_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    let key = match std::env::var("OPENPENCIL_LLM_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        Some(k) => k,
        None => {
            eprintln!(
                "error: OPENPENCIL_LLM_API_KEY is not set (loop mode needs an openai-compat key)"
            );
            return std::process::ExitCode::from(3);
        }
    };
    let base_url = match std::env::var("OPENPENCIL_LLM_BASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
    {
        Some(u) => u,
        None => {
            eprintln!("error: OPENPENCIL_LLM_BASE_URL is not set (e.g. https://api.openai.com/v1)");
            return std::process::ExitCode::from(3);
        }
    };
    let dump = std::env::var("OPENPENCIL_SMOKE_DUMP").is_ok();
    let max_tokens: u32 = std::env::var("OPENPENCIL_SMOKE_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8192);
    let thinking = loop_thinking_mode();
    // `OPENPENCIL_SMOKE_LOOP_SEED=1` (only meaningful with OPENPENCIL_SMOKE_LOOP=1)
    // arms the minimal-seed path: a page-root + named section stubs are applied
    // before the SAME agentic loop runs to fill them. Unset ⇒ pure loop (the
    // existing behaviour, byte-for-byte unchanged).
    let seed = std::env::var("OPENPENCIL_SMOKE_LOOP_SEED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on"))
        .unwrap_or(false);

    eprintln!("[SMOKE] mode=loop seed={seed} model={model} base_url={base_url}");
    eprintln!("[SMOKE] prompt={prompt:?} thinking={thinking:?} max_tokens={max_tokens}");

    // Protocol base + prompt-matched domain depth — mirrors the desktop
    // design-loop launch so headless A/B runs measure the same supply.
    let system_prompt = op_ai_skills::design_agent_system_prompt_with_skills(&prompt);
    // `OPENPENCIL_SMOKE_LIBRARY` is honored here too: the path is threaded into
    // `run_loop`, which merges the harvested library into the live `EditorState`
    // before the agentic loop runs (so its `batch_design` ref nodes can target
    // the loaded masters). Unset ⇒ no merge, byte-for-byte unchanged.
    let library_path = std::env::var("OPENPENCIL_SMOKE_LIBRARY")
        .ok()
        .filter(|p| !p.is_empty());
    let started = std::time::Instant::now();

    // The loop's `provider.send()` returns a blocking iterator driven by the
    // crate's global `shared_runtime()`; run the drain off the async worker.
    let result = tokio::task::spawn_blocking(move || {
        loop_mode::run_loop(
            base_url,
            key,
            model,
            system_prompt,
            prompt,
            thinking,
            op_ai::chat_provider::EffortLevel::Low,
            max_tokens,
            dump,
            library_path,
            seed,
        )
    })
    .await;
    let elapsed = started.elapsed();

    let state = match result {
        Ok(Ok(state)) => state,
        Ok(Err(e)) => {
            eprintln!("[LOOP] setup error: {e}");
            return std::process::ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("[LOOP] loop task panicked: {e}");
            return std::process::ExitCode::from(1);
        }
    };

    let guard = state.lock().expect("EditorState mutex poisoned");
    let total_nodes = guard.active_children().len();
    eprintln!("[LOOP] finished in {elapsed:?}; top-level node(s)={total_nodes}");

    // Dump via the IDENTICAL serialize path the orchestrator mode uses.
    let save_failed = match std::env::var("OPENPENCIL_SMOKE_OUT") {
        Ok(out_path) if !out_path.is_empty() => match serde_json::to_string_pretty(&guard.doc) {
            Ok(json) => match std::fs::write(&out_path, json) {
                Ok(()) => {
                    eprintln!("[SMOKE] saved doc → {out_path}");
                    false
                }
                Err(e) => {
                    eprintln!("[SMOKE] save failed ({out_path}): {e}");
                    true
                }
            },
            Err(e) => {
                eprintln!("[SMOKE] serialize failed: {e}");
                true
            }
        },
        _ => false,
    };

    if save_failed {
        std::process::ExitCode::from(4)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
    let prompt = match std::env::args().nth(1) {
        Some(p) if !p.is_empty() => p,
        _ => {
            eprintln!(
                "usage: op-smoke <prompt>\n\n\
                 providers:\n\
                   anthropic (default): OPENPENCIL_ANTHROPIC_API_KEY=...\n\
                   openai-compat: OPENPENCIL_LLM_BASE_URL=... OPENPENCIL_LLM_API_KEY=...\n\
                   antigravity/agy: uses the logged-in agy CLI\n\n\
                 common:\n\
                   OPENPENCIL_ORCHESTRATOR_MODEL=<model>\n\
                   OPENPENCIL_SMOKE_OUT=<result.op>\n\
                   OPENPENCIL_SMOKE_VALIDATION=1"
            );
            return std::process::ExitCode::from(2);
        }
    };

    if let Some(code) = modify_mode::run_if_requested(prompt.clone()).await {
        return code;
    }

    // `OPENPENCIL_SMOKE_LOOP=1` is a SEPARATE branch: instead of the
    // `Orchestrator`, it runs the headless Pencil-style agentic tool-loop
    // (the model calls `batch_design` / `get_screenshot` / … over a real SSE
    // stream; the host applies each call to a live `EditorState`). Without
    // this flag op-smoke behaves exactly as before. The branch returns early
    // so the orchestrator path below is byte-for-byte unchanged.
    if std::env::var("OPENPENCIL_SMOKE_LOOP")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on"))
        .unwrap_or(false)
    {
        return run_loop_mode(prompt).await;
    }

    let provider_kind_raw =
        std::env::var("OPENPENCIL_LLM_PROVIDER").unwrap_or_else(|_| "anthropic".into());
    let Some(provider_kind) = SmokeProviderKind::parse(&provider_kind_raw) else {
        eprintln!(
            "error: unknown OPENPENCIL_LLM_PROVIDER={provider_kind_raw:?} \
             (want anthropic|openai-compat|antigravity|agy)"
        );
        return std::process::ExitCode::from(3);
    };
    let model =
        std::env::var("OPENPENCIL_ORCHESTRATOR_MODEL").unwrap_or_else(|_| match provider_kind {
            SmokeProviderKind::Anthropic => "claude-sonnet-4-6".into(),
            SmokeProviderKind::OpenAiCompat => "gpt-4o-mini".into(),
            SmokeProviderKind::Antigravity => "gemini-3.6-flash-high".into(),
        });

    eprintln!("[SMOKE] provider={} model={model}", provider_kind.label());
    eprintln!("[SMOKE] prompt={prompt:?}");

    // `OPENPENCIL_SMOKE_DIRECT=1` swaps the QueryEngine path for a direct
    // openai-compat client that can send MiniMax `thinking:{type:disabled}`
    // (the vendored agent QueryEngine can't) — needed to validate M3 headless.
    let direct = std::env::var("OPENPENCIL_SMOKE_DIRECT").is_ok();
    let llm: Box<dyn LlmClient> = match provider_kind {
        SmokeProviderKind::Anthropic => {
            let key = std::env::var("OPENPENCIL_ANTHROPIC_API_KEY")
                .ok()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .filter(|k| !k.is_empty());
            let Some(key) = key else {
                eprintln!(
                    "error: neither OPENPENCIL_ANTHROPIC_API_KEY nor ANTHROPIC_API_KEY is set"
                );
                return std::process::ExitCode::from(3);
            };
            Box::new(SmokeLlmClient {
                provider: Arc::new(AnthropicProvider::new(key)),
                default_model: model.clone(),
            })
        }
        SmokeProviderKind::OpenAiCompat => {
            let key = std::env::var("OPENPENCIL_LLM_API_KEY")
                .ok()
                .filter(|k| !k.is_empty());
            let Some(key) = key else {
                eprintln!("error: OPENPENCIL_LLM_API_KEY is not set");
                return std::process::ExitCode::from(3);
            };
            let base_url = std::env::var("OPENPENCIL_LLM_BASE_URL")
                .ok()
                .filter(|u| !u.is_empty());
            let Some(base_url) = base_url else {
                eprintln!(
                    "error: OPENPENCIL_LLM_BASE_URL is not set (e.g. https://api.openai.com/v1)"
                );
                return std::process::ExitCode::from(3);
            };
            eprintln!("[SMOKE] base_url={base_url} direct={direct}");
            if direct {
                Box::new(DirectOpenAiClient {
                    base_url,
                    api_key: key,
                    default_model: model.clone(),
                })
            } else {
                Box::new(SmokeLlmClient {
                    provider: Arc::new(OpenAiCompatProvider::new(OpenAiCompatConfig::new(
                        key, base_url,
                    ))),
                    default_model: model.clone(),
                })
            }
        }
        SmokeProviderKind::Antigravity => antigravity_llm(&model),
    };

    // `OPENPENCIL_SMOKE_STARTER=1` seeds the fresh-canvas starter frame so a
    // smoke run exercises the TS `replaceEmptyFrame` reuse path (the desktop
    // GUI always carries this single empty starter); default stays the empty
    // `new()` doc so existing traces are unaffected.
    let seed_starter = std::env::var("OPENPENCIL_SMOKE_STARTER")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut sink = InlineDocSink {
        state: if seed_starter {
            EditorState::starter()
        } else {
            EditorState::new()
        },
    };
    // `OPENPENCIL_SMOKE_LIBRARY=<path-to-.lib.op>` loads a harvested component
    // library into the doc BEFORE generation so the generator can instantiate
    // its reusable masters (the AVAILABLE COMPONENTS manifest path). Unset =
    // today's behavior, byte-for-byte. A load error aborts the run loudly — a
    // benchmark that asked for a library must not silently run without it.
    if let Err(code) = maybe_merge_smoke_library(&mut sink.state) {
        return code;
    }

    // `OPENPENCIL_SMOKE_AUDIT=<path.op>` — self-loop quality gate: load the
    // doc, run the REAL-layout geometry diagnostics (the same detector family
    // the per-batch feedback uses), print a JSON report, exit. Zero LLM calls.
    // Exit code 0 = structurally clean, 1 = issues found. This is the
    // machine-checkable leg of the develop→generate→render→audit loop.
    if let Ok(audit_path) = std::env::var("OPENPENCIL_SMOKE_AUDIT") {
        let text = match std::fs::read_to_string(&audit_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[AUDIT] read {audit_path}: {e}");
                return std::process::ExitCode::from(3);
            }
        };
        // Load through the compat loader (not raw serde) so a `.op`
        // saved with the deduplicated `images` table audits with its
        // refs resolved instead of dangling `op-image:` strings.
        let doc: jian_ops_schema::PenDocument = match jian_ops_schema::load_str(&text) {
            Ok(loaded) => loaded.value,
            Err(e) => {
                eprintln!("[AUDIT] parse {audit_path}: {e}");
                return std::process::ExitCode::from(3);
            }
        };
        let state = op_editor_core::EditorState::from_document(doc);
        let issues = op_orchestrator::geometry_validation::geometry_diagnostics(&state);
        let roots: Vec<String> = state
            .active_children()
            .iter()
            .map(|n| {
                use op_editor_core::PenNodeExt;
                format!(
                    "{} ({})",
                    n.base().name.as_deref().unwrap_or("?"),
                    n.id_str()
                )
            })
            .collect();
        let report = serde_json::json!({
            "file": audit_path,
            "roots": roots,
            "issueCount": issues.len(),
            "issues": issues,
            // Chrome completeness / node-vocabulary / density metrics — the
            // dimensions raw geometry issues miss (see ab-g3 07-04 lesson).
            // Informational: exit code stays keyed on geometry issues alone.
            // `None`: this standalone-audit branch loads an existing `.op`
            // straight off disk and never drives the orchestrator, so there
            // is no `RunSummary` to report subtask-completeness against —
            // the rubric's `completeness` section is omitted here (never a
            // fabricated 0/0), same as `rubric_report`'s own doc contract.
            "rubric": audit_rubric::rubric_report(&state, None),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        return if issues.is_empty() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::from(1)
        };
    }

    // `OPENPENCIL_SMOKE_PROGRAM=<path>` runs a Pencil-style batch_design DSL
    // PROGRAM (a `binding=I(parent,{...})` tree-builder) against the doc and
    // saves it, bypassing the orchestrator entirely. This is the experiment
    // harness for "can a weak model emit a structurally-stable PROGRAM
    // (parent-by-reference) instead of fragile flat JSONL?". postProcess stays
    // OFF so the saved doc is the RAW structure the program builds — no cleanup
    // post-pass — which is the whole point: the program needs no repair pass.
    if let Ok(program_path) = std::env::var("OPENPENCIL_SMOKE_PROGRAM") {
        let program = match std::fs::read_to_string(&program_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[PROGRAM] read {program_path}: {e}");
                return std::process::ExitCode::from(3);
            }
        };
        let tool = op_mcp::batch_design_snapshot(&sink.state);
        let mut args: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        args.insert("operations".into(), program);
        let applied = match op_mcp::McpTool::call(&tool, &args) {
            op_mcp::ToolOutcome::OkJsonWithCommand(json, cmd) => {
                eprintln!("[PROGRAM] result envelope: {json}");
                let ok = sink.state.apply(cmd);
                eprintln!("[PROGRAM] apply -> {ok}");
                ok
            }
            op_mcp::ToolOutcome::OkJson(json) => {
                eprintln!("[PROGRAM] no command produced: {json}");
                false
            }
            other => {
                eprintln!("[PROGRAM] unexpected outcome: {other:?}");
                false
            }
        };
        let code = match std::env::var("OPENPENCIL_SMOKE_OUT") {
            Ok(out) if !out.is_empty() => match serde_json::to_string_pretty(&sink.state.doc) {
                Ok(j) => match std::fs::write(&out, j) {
                    Ok(()) => {
                        eprintln!("[PROGRAM] saved doc -> {out}");
                        if applied {
                            std::process::ExitCode::SUCCESS
                        } else {
                            std::process::ExitCode::from(1)
                        }
                    }
                    Err(e) => {
                        eprintln!("[PROGRAM] save failed ({out}): {e}");
                        std::process::ExitCode::from(4)
                    }
                },
                Err(e) => {
                    eprintln!("[PROGRAM] serialize failed: {e}");
                    std::process::ExitCode::from(4)
                }
            },
            _ => {
                if applied {
                    std::process::ExitCode::SUCCESS
                } else {
                    std::process::ExitCode::from(1)
                }
            }
        };
        return code;
    }

    let validation_enabled =
        truthy_env_value(std::env::var("OPENPENCIL_SMOKE_VALIDATION").ok().as_deref());
    let request = DesignRequest {
        prompt,
        model: Some(model),
        provider: None,
        design_md: sink.state.doc.design_md.clone(),
        continuation_context: None,
        append_context: None,
        concurrency: std::env::var("OPENPENCIL_SMOKE_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1),
        validation_enabled,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    };
    let abort = AbortFlag::new();
    // Preserve the historical skipped validator unless the caller explicitly
    // requests the production lint + validation path.
    let skipped_pre_validator = SkippedPreValidator;
    let lint_pre_validator = op_host_services::pre_validator::LintPreValidator;
    let pre_validator: &dyn PreValidator = if validation_enabled {
        &lint_pre_validator
    } else {
        &skipped_pre_validator
    };
    let screenshot = SkippedScreenshotProvider;
    let vision = SkippedVisionLlmClient;
    let providers = ValidationProviders {
        pre_validator,
        screenshot: &screenshot,
        vision: &vision,
        system_prompt: String::new(),
    };

    // `OPENPENCIL_SMOKE_BEST_OF=N` (2..=4): N independent generations,
    // geometry-scored, best one saved. The plain single-run path below
    // stays byte-identical when the knob is unset.
    let best_of_n = best_of::parse_best_of_count();
    if best_of_n > 1 {
        return best_of::run_best_of(
            best_of_n,
            &request,
            &sink.state,
            llm.as_ref(),
            &abort,
            &providers,
        )
        .await;
    }

    let mut on_progress = |p: Progress| {
        eprintln!("[PROGRESS] {p:?}");
    };

    let started = std::time::Instant::now();
    let result = Orchestrator::new()
        .run(
            request,
            &mut sink,
            llm.as_ref(),
            &mut on_progress,
            &abort,
            &providers,
        )
        .await;
    let elapsed = started.elapsed();

    // Persist the produced PenDocument when OPENPENCIL_SMOKE_OUT is set,
    // so the render / screenshot step can pick it up. Canonical
    // serde_json mirrors `persistence::save_to_path`. Saved regardless of
    // Ok/Err so a partial doc stays inspectable on failure.
    // A requested save that FAILS forces a non-zero exit below — a
    // benchmark driver must not read "success" when no `.op` was written.
    let save_failed = match std::env::var("OPENPENCIL_SMOKE_OUT") {
        Ok(out_path) if !out_path.is_empty() => match serde_json::to_string_pretty(&sink.state.doc)
        {
            Ok(json) => match std::fs::write(&out_path, json) {
                Ok(()) => {
                    eprintln!("[SMOKE] saved doc → {out_path}");
                    false
                }
                Err(e) => {
                    eprintln!("[SMOKE] save failed ({out_path}): {e}");
                    true
                }
            },
            Err(e) => {
                eprintln!("[SMOKE] serialize failed: {e}");
                true
            }
        },
        _ => false,
    };

    match result {
        Ok(summary) => {
            eprintln!("[FINAL] Ok in {elapsed:?}");
            eprintln!("  root_frame_id = {:?}", summary.root_frame_id);
            eprintln!("  total_nodes   = {}", summary.total_nodes);
            eprintln!("  subtasks      = {}", summary.subtasks.len());
            for s in &summary.subtasks {
                eprintln!(
                    "    - {}: {} node(s){}",
                    s.id,
                    s.node_count,
                    s.error
                        .as_deref()
                        .map(|e| format!(" [error: {e}]"))
                        .unwrap_or_default()
                );
            }
            if save_failed {
                eprintln!("[FINAL] generation OK but OPENPENCIL_SMOKE_OUT write failed");
                std::process::ExitCode::from(4)
            } else {
                std::process::ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("[FINAL] Err in {elapsed:?}: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn provider_parser_accepts_antigravity_aliases() {
        assert_eq!(
            SmokeProviderKind::parse("antigravity"),
            Some(SmokeProviderKind::Antigravity)
        );
        assert_eq!(
            SmokeProviderKind::parse("AGY"),
            Some(SmokeProviderKind::Antigravity)
        );
        assert_eq!(
            SmokeProviderKind::parse("openai"),
            Some(SmokeProviderKind::OpenAiCompat)
        );
        assert_eq!(SmokeProviderKind::parse("unknown"), None);
    }

    #[test]
    fn validation_switch_is_explicit_and_default_off() {
        assert!(!truthy_env_value(None));
        assert!(!truthy_env_value(Some("0")));
        assert!(!truthy_env_value(Some("false")));
        assert!(truthy_env_value(Some("1")));
        assert!(truthy_env_value(Some("true")));
        assert!(truthy_env_value(Some("on")));
    }

    #[test]
    fn antigravity_llm_constructs_without_api_credentials() {
        let _ = antigravity_llm("gemini-3.6-flash-high");
    }

    #[test]
    fn debug_trace_truncation_is_unicode_safe() {
        let command = EditorCommand::Batch {
            commands: vec![EditorCommand::SetNodeName {
                node_id: op_editor_core::NodeId::new("n1"),
                name: "收藏地图头部与筛选".repeat(20),
            }],
        };
        let label = describe_cmd(&command);
        assert!(label.ends_with("..."));
        assert_eq!(label.chars().count(), 120);
        assert!(label.contains("收藏地图"));
    }
}
