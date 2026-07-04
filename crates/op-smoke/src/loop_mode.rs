//! Headless agentic tool-loop mode for the smoke runner
//! (`OPENPENCIL_SMOKE_LOOP=1`).
//!
//! The DEFAULT smoke path (no `OPENPENCIL_SMOKE_LOOP`) drives the
//! `Orchestrator` and is byte-for-byte unchanged — see `main.rs`. This
//! module is a SEPARATE branch that exercises Pencil's *agentic loop*
//! instead: the model is handed the 14 design tools, calls them
//! (`batch_design` / `get_screenshot` / …) over a real
//! `/chat/completions` SSE stream, the host executes each call against a
//! live `EditorState`, and the result rides a follow-up request — looping
//! until the model stops calling tools or the production turn cap
//! (`MAX_TOOL_TURNS = 20`) is reached.
//!
//! ## What is reused vs. new
//!
//! Nothing about tool *semantics* is reimplemented here. The loop and the
//! tool-apply path are the production code:
//!
//! - **Loop transport** — `ConfiguredBuiltinProvider::send()`
//!   (`op_host_services::chat_builtin_http`) which internally builds an
//!   `AgentLoopConfig { max_turns: MAX_TOOL_TURNS, … }` and runs
//!   `run_openai_agent_loop` for the `OpenAiCompat` kind, spawned on the
//!   crate's global `shared_runtime()`. `send()` returns a *blocking*
//!   iterator; draining it to exhaustion blocks the caller until the loop
//!   ends.
//! - **Tool apply** — [`HeadlessExecutor::execute`] calls
//!   `op_host_services::design_agent_tools::execute_agent_tool(&mut state,
//!   name, args)`, the exact transport-free apply seam the desktop
//!   `drain_tool_requests` runs (`state.apply(EditorCommand)` under the
//!   hood). This covers ALL 14 design tools including `get_screenshot`
//!   (real offscreen raster PNG) — so the loop's Step-1 image-block path
//!   is exercised end-to-end.
//! - **finalize** — [`HeadlessExecutor::finalize`] calls
//!   `op_orchestrator::apply_loop_finalize(&mut state)` (the same Class-A
//!   structural backstop the desktop host runs at loop end). Gated by
//!   `with_loop_finalize()` so it only fires for this design loop.
//!
//! The single genuine desktop entanglement is `spawn_agents`: the actual
//! sub-loop launch is desktop-only glue (`sub_agent_session.rs`,
//! post-pump). Calling `execute_agent_tool("spawn_agents", …)` here would
//! hit the `op_mcp` stub that only returns `{spawned: N}` while silently
//! dropping the sub-designers' work. Rather than fake that, the headless
//! executor returns an HONEST structured error (see
//! [`HeadlessExecutor::execute`]) so the model is told sub-agents are
//! unavailable and proceeds to call `batch_design` itself — the
//! single-agent loop that is sufficient to benchmark.
//!
//! ## The `.op` dump
//!
//! The loop mutates the SAME `EditorState.doc` (`PenDocument`) the
//! orchestrator mode does, so the dump uses the IDENTICAL serialize path
//! (`serde_json::to_string_pretty(&state.doc)` → `std::fs::write`).

use std::sync::{Arc, Mutex};

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, ChatToolExecutor, ChatToolResult, EffortLevel,
    ThinkingMode,
};
use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState};
use op_host_services::chat_builtin_http::ConfiguredBuiltinProvider;
use op_host_services::design_agent_tools::{
    design_tool_defs, design_tool_level, execute_agent_tool_with_root_seed_guard, RootSeedGuard,
};

/// Headless [`ChatToolExecutor`] owning the live `EditorState` behind a
/// `Mutex`. Each `execute` applies one design tool to the state via the
/// production transport-free apply path; `finalize` runs the loop-end
/// structural backstop. Shared with the smoke driver via `Arc` so the
/// dump step can serialize the same mutated state.
pub struct HeadlessExecutor {
    state: Arc<Mutex<EditorState>>,
    root_seed_guard: Mutex<RootSeedGuard>,
    /// Echo each tool call + truncated result to stderr when
    /// `OPENPENCIL_SMOKE_DUMP` is set, so ab-v9 logs capture the run.
    dump: bool,
}

impl HeadlessExecutor {
    #[cfg(test)]
    pub fn new(state: Arc<Mutex<EditorState>>, dump: bool) -> Self {
        Self {
            state,
            root_seed_guard: Mutex::new(RootSeedGuard::disabled()),
            dump,
        }
    }

    pub fn with_prompt(state: Arc<Mutex<EditorState>>, dump: bool, prompt: &str) -> Self {
        Self {
            state,
            root_seed_guard: Mutex::new(RootSeedGuard::from_prompt(prompt)),
            dump,
        }
    }
}

impl ChatToolExecutor for HeadlessExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        // `spawn_agents` cannot run headlessly: the actual sub-loop launch is
        // desktop-only glue (sub_agent_session.rs, post-pump). `execute_agent_tool`
        // would hit the op_mcp stub that returns `{spawned:N}` while dropping the
        // sub-designers' work. Return an honest error instead of faking a result —
        // the model is told to design inline (batch_design) rather than fan out.
        if name == "spawn_agents" {
            let envelope = serde_json::json!({
                "success": false,
                "error": "spawn_agents is unavailable in the headless smoke loop — \
                          design the layout directly with batch_design instead of spawning sub-agents",
            });
            if self.dump {
                eprintln!("[LOOP] tool=spawn_agents → DEFERRED (headless): sub-agent launch is desktop-only");
            }
            return ChatToolResult {
                content: envelope.to_string(),
                is_error: true,
            };
        }

        let (result, mutated) = {
            let mut state = self
                .state
                .lock()
                .expect("EditorState mutex poisoned by a panicking tool call");
            let mut root_seed_guard = self
                .root_seed_guard
                .lock()
                .expect("RootSeedGuard mutex poisoned by a panicking tool call");
            execute_agent_tool_with_root_seed_guard(
                &mut state,
                name,
                args_json,
                None,
                Some(&mut *root_seed_guard),
            )
        };
        if self.dump {
            let level = design_tool_level(name).unwrap_or("?");
            // get_screenshot results carry a multi-KB base64 PNG — don't spew it.
            let preview: String = if name == "get_screenshot" {
                format!("<png base64, {} bytes>", result.content.len())
            } else {
                result.content.chars().take(240).collect()
            };
            eprintln!(
                "[LOOP] tool={name} level={level} mutated={mutated} is_error={} result={preview}",
                result.is_error
            );
        }
        result
    }

    fn finalize(&self) {
        let mut state = self
            .state
            .lock()
            .expect("EditorState mutex poisoned before finalize");
        op_orchestrator::apply_loop_finalize(&mut state);
        if self.dump {
            eprintln!("[LOOP] finalize: apply_loop_finalize ran (Class-A structural backstop)");
        }
    }
}

/// Build the design-tool builtin provider from the `OPENPENCIL_LLM_*` env.
///
/// Always the `OpenAiCompat` wire (ab-v9's path) — the loop runs
/// `run_openai_agent_loop` for this kind. `base_url` / `api_key` / `model`
/// come from the same env the orchestrator mode reads. The 14 design tool
/// defs + the headless executor are attached so the loop advertises tools
/// and runs each call against `state`; `with_loop_finalize` arms the
/// loop-end backstop.
fn build_design_provider(
    base_url: String,
    api_key: String,
    model: String,
    executor: Arc<HeadlessExecutor>,
) -> Result<ConfiguredBuiltinProvider, String> {
    let config = BuiltinAgentConfig {
        id: "smoke-loop".into(),
        preset: BuiltinAgentPresetKey::Custom,
        display_name: "smoke-loop".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key,
        model,
        base_url,
        enabled: true,
    };
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&config)
        .ok_or_else(|| "builtin provider not ready (need non-empty api_key + model)".to_string())?;
    Ok(provider
        .with_canvas_tools(design_tool_defs(), executor)
        .with_loop_finalize())
}

/// Run the headless agentic design loop to completion and return the
/// shared `EditorState` so the caller can dump `state.doc`.
///
/// Drives `provider.send(req)` — a blocking iterator fed by the agent loop
/// running on the crate's global `shared_runtime()`. We drain it to
/// exhaustion; the executor mutates `state` in place from the loop's
/// blocking tool calls (both share the `Arc<Mutex<EditorState>>`), so on
/// return `state` carries the finalized design (`finalize()` already ran,
/// gated by `with_loop_finalize`).
///
/// `OPENPENCIL_SMOKE_DUMP` streams per-delta text / tool-call / thinking
/// lines to stderr so ab-v9 logs capture the run.
///
/// `seed = true` (the `OPENPENCIL_SMOKE_LOOP_SEED=1` path) applies a MINIMAL
/// scaffold seed to the `EditorState` BEFORE the loop runs — a page-root frame
/// plus a few empty named section stubs, derived from the orchestrator's
/// heuristic fallback planner (see [`crate::loop_seed`]). The system prompt is
/// augmented to tell the model the scaffold exists and to fill the sections.
/// `seed = false` is byte-for-byte the original pure-loop path.
#[allow(clippy::too_many_arguments)]
pub fn run_loop(
    base_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
    user_prompt: String,
    thinking: ThinkingMode,
    effort: EffortLevel,
    max_output_tokens: u32,
    dump: bool,
    library_path: Option<String>,
    seed: bool,
) -> Result<Arc<Mutex<EditorState>>, String> {
    // `OPENPENCIL_SMOKE_STARTER=1` seeds the fresh-canvas starter frame so the
    // loop exercises the same `replaceEmptyFrame` reuse path the desktop GUI
    // carries; default stays the empty `new()` doc (matches orchestrator mode).
    let seed_starter = std::env::var("OPENPENCIL_SMOKE_STARTER")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut initial = if seed_starter {
        EditorState::starter()
    } else {
        EditorState::new()
    };
    // Merge a harvested component library (`OPENPENCIL_SMOKE_LIBRARY`) into the
    // doc before the loop runs so its `batch_design` ref nodes can target the
    // loaded masters. `None` ⇒ no merge, byte-for-byte unchanged.
    if let Some(path) = library_path.as_deref() {
        let report = op_pen_loader::merge_library_into_state(&mut initial, path)
            .map_err(|e| format!("library load failed: {e}"))?;
        eprintln!(
            "[LOOP] library merged from {path}: +{} master(s), {} component(s) total, \
             +{} variable(s), +{} theme axis(es)",
            report.masters_added,
            report.component_count,
            report.variables_added,
            report.themes_added
        );
    }
    // Minimal scaffold seed (`OPENPENCIL_SMOKE_LOOP_SEED=1`): apply a page-root
    // + named empty section stubs to the live state BEFORE the loop runs, so a
    // weak model has structure to fill instead of an empty canvas it collapses
    // on. `seed = false` ⇒ this whole block is skipped (pure-loop, unchanged).
    let mut system_prompt = system_prompt;
    if seed {
        let cmd = crate::loop_seed::build_seed_command(&user_prompt)?;
        let applied = initial.apply(cmd);
        if !applied {
            return Err("minimal scaffold seed failed to apply to EditorState".into());
        }
        system_prompt.push_str(&crate::loop_seed::seed_system_prompt_suffix(&user_prompt));
        eprintln!(
            "[LOOP][seed] minimal scaffold applied: {} top-level node(s), \
             system prompt augmented to fill seeded sections",
            initial.active_children().len()
        );
    }

    let state = Arc::new(Mutex::new(initial));

    let executor = Arc::new(HeadlessExecutor::with_prompt(
        state.clone(),
        dump,
        &user_prompt,
    ));
    let provider = build_design_provider(base_url, api_key, model, executor)?;

    let request = ChatRequest {
        system_prompt,
        user_message: user_prompt,
        history: Vec::new(),
        max_output_tokens: max_output_tokens.max(1),
        thinking,
        effort,
        attachments: Vec::new(),
        model: None,
    };

    eprintln!(
        "[LOOP] starting agentic design loop (max_turns={})",
        op_host_services::chat_canvas_tools::MAX_TOOL_TURNS
    );

    // `send()` blocks the iterating thread until the loop ends. Drain every
    // delta; the executor mutates `state` in place via the shared Arc.
    let mut text_chars = 0usize;
    let mut tool_calls = 0usize;
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(t) => {
                text_chars += t.len();
                if dump {
                    eprint!("{t}");
                }
            }
            ChatDelta::Thinking(t) => {
                if dump {
                    eprintln!(
                        "[LOOP][thinking] {}",
                        t.chars().take(160).collect::<String>()
                    );
                }
            }
            ChatDelta::ToolUse { name, args } => {
                tool_calls += 1;
                let preview: String = args.chars().take(200).collect();
                eprintln!("[LOOP] → tool_use #{tool_calls}: {name} args={preview}");
            }
            ChatDelta::Done { stop_reason } => {
                eprintln!("\n[LOOP] done: stop_reason={stop_reason:?}");
            }
            ChatDelta::Error(e) => {
                eprintln!("\n[LOOP] error delta: {e}");
            }
        }
    }
    eprintln!("[LOOP] loop drained: {tool_calls} tool call(s), {text_chars} text char(s)");

    Ok(state)
}

#[cfg(test)]
#[path = "loop_mode_tests.rs"]
mod tests;
