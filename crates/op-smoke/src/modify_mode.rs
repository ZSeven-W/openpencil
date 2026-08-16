//! Fixed-input DESIGN_MODIFY mode for prompt-thumbnail generation.
//!
//! `OPENPENCIL_SMOKE_MODIFY_INPUT=<baseline.op>` takes a real document through
//! the same `build_modify_plan` -> `run_modify_turn` -> scoped host apply path
//! as the desktop. The baseline is never overwritten: output must use a
//! distinct `OPENPENCIL_SMOKE_OUT` path, and both files are SHA-256 addressed
//! in the terminal summary.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::Receiver;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, ChatToolResult, CliName, StopReason, ThinkingMode,
};
use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState, NodeId, PenNodeExt,
};
use op_host_services::chat_builtin_http::ConfiguredBuiltinProvider;
use op_host_services::chat_canvas_tools::{
    apply_design_modification, chat_tool_channel, parse_design_modification_ops_arg,
    parse_design_modification_target_frame_ids_arg, ChatToolRequest,
};
use op_host_services::chat_intent::{build_modify_plan, run_modify_turn, APPLY_MODIFICATION_OP};
use op_host_services::chat_subprocess::SubprocessProvider;
use sha2::{Digest as _, Sha256};

const MODIFY_INPUT_ENV: &str = "OPENPENCIL_SMOKE_MODIFY_INPUT";
const MODIFY_TARGET_ENV: &str = "OPENPENCIL_SMOKE_MODIFY_TARGET";
const KEEP_THINKING_ENV: &str = "OPENPENCIL_SMOKE_KEEP_THINKING";
const OUTPUT_ENV: &str = "OPENPENCIL_SMOKE_OUT";

pub async fn run_if_requested(instruction: String) -> Option<ExitCode> {
    let input = std::env::var(MODIFY_INPUT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let result =
        tokio::task::spawn_blocking(move || run_from_env(&instruction, Path::new(&input))).await;
    Some(match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            eprintln!("[MODIFY] error: {error}");
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("[MODIFY] worker panicked: {error}");
            ExitCode::from(1)
        }
    })
}

fn run_from_env(instruction: &str, input: &Path) -> Result<(), String> {
    let output = required_env_path(OUTPUT_ENV)?;
    reject_input_overwrite(input, &output)?;
    let provider_kind_raw =
        std::env::var("OPENPENCIL_LLM_PROVIDER").unwrap_or_else(|_| "openai-compat".into());
    let provider_kind = ModifyProviderKind::parse(&provider_kind_raw)?;
    let model = required_env("OPENPENCIL_ORCHESTRATOR_MODEL")?;
    let thinking = modify_thinking(
        std::env::var(KEEP_THINKING_ENV)
            .ok()
            .as_deref()
            .is_some_and(truthy),
    );
    let target = std::env::var(MODIFY_TARGET_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let input_bytes =
        std::fs::read(input).map_err(|error| format!("read {}: {error}", input.display()))?;
    let input_sha256 = sha256_hex(&input_bytes);
    let state = load_rewritable_baseline(input)?;
    let provider = build_provider(
        provider_kind,
        &model,
        std::env::var("OPENPENCIL_LLM_BASE_URL").ok(),
        std::env::var("OPENPENCIL_LLM_API_KEY").ok(),
    )?;
    let execution = run_loaded_modify(
        state,
        instruction,
        target.as_deref(),
        &model,
        thinking,
        provider.as_ref(),
    )?;

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create output directory {}: {error}", parent.display()))?;
    }
    op_host_services::doc_io::save_to_path(&execution.state, &output)
        .map_err(|error| format!("save {}: {error}", output.display()))?;

    let input_after =
        std::fs::read(input).map_err(|error| format!("re-read {}: {error}", input.display()))?;
    if sha256_hex(&input_after) != input_sha256 {
        return Err(format!(
            "immutable baseline changed during the run: {}",
            input.display()
        ));
    }
    let output_bytes =
        std::fs::read(&output).map_err(|error| format!("read {}: {error}", output.display()))?;
    let summary = serde_json::json!({
        "mode": "modify-input",
        "input": input,
        "output": output,
        "model": model,
        "thinking": thinking.as_str(),
        "targetFrameIds": execution.target_frame_ids,
        "appliedCount": execution.applied_count,
        "inputSha256": input_sha256,
        "instructionSha256": sha256_hex(instruction.as_bytes()),
        "outputSha256": sha256_hex(&output_bytes),
    });
    eprintln!(
        "[MODIFY] {}",
        serde_json::to_string(&summary).unwrap_or_else(|_| "{}".into())
    );
    Ok(())
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required"))
}

fn required_env_path(name: &str) -> Result<PathBuf, String> {
    required_env(name).map(PathBuf::from)
}

fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModifyProviderKind {
    OpenAiCompat,
    Antigravity,
}

impl ModifyProviderKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" | "openai-compat" => Ok(Self::OpenAiCompat),
            "antigravity" | "agy" => Ok(Self::Antigravity),
            _ => Err(format!(
                "unknown OPENPENCIL_LLM_PROVIDER={value:?} for modify-input mode \
                 (want openai-compat|antigravity|agy)"
            )),
        }
    }
}

fn modify_thinking(keep_thinking: bool) -> ThinkingMode {
    if keep_thinking {
        ThinkingMode::Adaptive
    } else {
        ThinkingMode::Disabled
    }
}

fn reject_input_overwrite(input: &Path, output: &Path) -> Result<(), String> {
    let input_canonical = std::fs::canonicalize(input)
        .map_err(|error| format!("resolve input {}: {error}", input.display()))?;
    if std::fs::canonicalize(output)
        .ok()
        .is_some_and(|candidate| candidate == input_canonical)
    {
        return Err(format!(
            "{OUTPUT_ENV} must not overwrite the immutable baseline {}",
            input.display()
        ));
    }
    Ok(())
}

fn load_rewritable_baseline(path: &Path) -> Result<EditorState, String> {
    let loaded =
        op_host_services::doc_io::load_editor_state_with_report(path, op_editor_core::Locale::EnUs)
            .map_err(|error| format!("load {}: {error}", path.display()))?;
    if loaded.report.rewrite_blocked_by_schema_warning {
        return Err(format!(
            "refusing to modify {} because its schema is newer or malformed",
            path.display()
        ));
    }
    Ok(loaded.state)
}

fn build_provider(
    kind: ModifyProviderKind,
    model: &str,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<Box<dyn ChatProvider>, String> {
    match kind {
        ModifyProviderKind::OpenAiCompat => {
            let base_url = required_provider_value("OPENPENCIL_LLM_BASE_URL", base_url)?;
            let api_key = required_provider_value("OPENPENCIL_LLM_API_KEY", api_key)?;
            build_openai_compat_provider(base_url, api_key, model.to_string())
                .map(|provider| Box::new(provider) as Box<dyn ChatProvider>)
        }
        ModifyProviderKind::Antigravity => {
            SubprocessProvider::for_cli_generation(CliName::Antigravity)
                .map(|provider| Box::new(provider) as Box<dyn ChatProvider>)
                .ok_or_else(|| "Antigravity has no production CLI transport".to_string())
        }
    }
}

fn required_provider_value(name: &str, value: Option<String>) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required"))
}

fn build_openai_compat_provider(
    base_url: String,
    api_key: String,
    model: String,
) -> Result<ConfiguredBuiltinProvider, String> {
    let config = BuiltinAgentConfig {
        id: "smoke-modify".into(),
        preset: BuiltinAgentPresetKey::Custom,
        display_name: "smoke-modify".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key,
        models: vec![model],
        base_url,
        enabled: true,
    };
    ConfiguredBuiltinProvider::from_builtin_agent(&config)
        .ok_or_else(|| "builtin modify provider is not ready".into())
}

struct ModifyExecution {
    state: EditorState,
    target_frame_ids: Vec<String>,
    applied_count: usize,
}

fn run_loaded_modify(
    mut state: EditorState,
    instruction: &str,
    requested_target: Option<&str>,
    model: &str,
    thinking: ThinkingMode,
    provider: &dyn ChatProvider,
) -> Result<ModifyExecution, String> {
    let target_id = select_target_frame(&mut state, requested_target)?;
    let plan = build_modify_plan(&state, instruction)
        .ok_or_else(|| "build_modify_plan rejected the selected Frame".to_string())?;
    let target_frame_ids = plan.target_frame_ids.clone();
    let request = ChatRequest {
        system_prompt: plan.system_prompt,
        user_message: plan.user_message,
        max_output_tokens: 8192,
        model: Some(model.to_string()),
        thinking,
        ..Default::default()
    };

    let (chat_tx, chat_rx) = std::sync::mpsc::channel::<ChatDelta>();
    let (executor, tool_rx) = chat_tool_channel();
    let expected_targets = target_frame_ids.clone();
    let host = std::thread::Builder::new()
        .name("op-smoke-modify-host".into())
        .spawn(move || serve_modify_apply(state, tool_rx, &expected_targets))
        .map_err(|error| format!("spawn modify host: {error}"))?;

    run_modify_turn(
        provider,
        request,
        &chat_tx,
        &executor,
        target_frame_ids.clone(),
    );
    drop(executor);
    drop(chat_tx);
    let host = host
        .join()
        .map_err(|_| "modify host thread panicked".to_string())?;
    let deltas = chat_rx.try_iter().collect::<Vec<_>>();
    echo_deltas(&deltas);

    if let Some(error) = host.error {
        return Err(error);
    }
    let provider_error = deltas.iter().find_map(|delta| match delta {
        ChatDelta::Error(message) => Some(message.clone()),
        _ => None,
    });
    if let Some(error) = provider_error {
        return Err(format!("provider returned no applicable edit: {error}"));
    }
    let ended = deltas.iter().any(|delta| {
        matches!(
            delta,
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            }
        )
    });
    let marked_applied = deltas.iter().any(
        |delta| matches!(delta, ChatDelta::TextDelta(text) if text.contains("<!-- APPLIED -->")),
    );
    if host.tool_calls != 1 || host.applied_count == 0 || !host.mutated || !ended || !marked_applied
    {
        return Err(format!(
            "modify turn did not apply exactly one scoped edit batch to {target_id} \
             (toolCalls={}, appliedCount={}, mutated={}, ended={}, markedApplied={})",
            host.tool_calls, host.applied_count, host.mutated, ended, marked_applied
        ));
    }
    Ok(ModifyExecution {
        state: host.state,
        target_frame_ids,
        applied_count: host.applied_count,
    })
}

fn select_target_frame(
    state: &mut EditorState,
    requested_target: Option<&str>,
) -> Result<String, String> {
    let target = if let Some(requested) = requested_target {
        let requested = requested.trim();
        let id = NodeId::new(requested);
        let node = op_editor_core::walkers::find_node(state.active_children(), &id)
            .ok_or_else(|| format!("{MODIFY_TARGET_ENV}={requested:?} was not found"))?;
        if !matches!(node, jian_ops_schema::node::PenNode::Frame(_)) {
            return Err(format!("{MODIFY_TARGET_ENV}={requested:?} is not a Frame"));
        }
        requested.to_string()
    } else {
        let roots = state
            .active_children()
            .iter()
            .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
            .map(|node| node.id_str().to_string())
            .collect::<Vec<_>>();
        if roots.len() != 1 {
            return Err(format!(
                "baseline must contain exactly one top-level Frame when {MODIFY_TARGET_ENV} \
                 is unset; found {}",
                roots.len()
            ));
        }
        roots[0].clone()
    };
    state.selection.set = vec![NodeId::new(&target)];
    state.selection.anchor = NodeId::new(&target);
    Ok(target)
}

struct ModifyHostResult {
    state: EditorState,
    tool_calls: usize,
    applied_count: usize,
    mutated: bool,
    error: Option<String>,
}

fn serve_modify_apply(
    state: EditorState,
    requests: Receiver<ChatToolRequest>,
    expected_targets: &[String],
) -> ModifyHostResult {
    let mut result = ModifyHostResult {
        state,
        tool_calls: 0,
        applied_count: 0,
        mutated: false,
        error: None,
    };
    for request in requests {
        result.tool_calls += 1;
        if request.name != APPLY_MODIFICATION_OP {
            let message = format!("unexpected modify host op {:?}", request.name);
            acknowledge_error(request, &message);
            result.error.get_or_insert(message);
            continue;
        }
        let nodes = parse_design_modification_ops_arg(&request.args_json);
        let targets = parse_design_modification_target_frame_ids_arg(&request.args_json);
        if targets != expected_targets {
            let message = format!(
                "modify scope changed in transit: expected {expected_targets:?}, got {targets:?}"
            );
            acknowledge_error(request, &message);
            result.error.get_or_insert(message);
            continue;
        }
        if nodes.is_empty() {
            let message = "modify host received an empty node batch".to_string();
            acknowledge_error(request, &message);
            result.error.get_or_insert(message);
            continue;
        }
        let (count, mutated) =
            apply_design_modification(&mut result.state, &nodes, expected_targets);
        result.applied_count += count;
        result.mutated |= mutated;
        let success = count > 0 && mutated;
        let content = serde_json::json!({
            "success": success,
            "count": count,
        })
        .to_string();
        let _ = request.ack.send(ChatToolResult {
            content,
            is_error: !success,
        });
    }
    result
}

fn acknowledge_error(request: ChatToolRequest, message: &str) {
    let _ = request.ack.send(ChatToolResult {
        content: serde_json::json!({
            "success": false,
            "error": message,
        })
        .to_string(),
        is_error: true,
    });
}

fn echo_deltas(deltas: &[ChatDelta]) {
    for delta in deltas {
        match delta {
            ChatDelta::TextDelta(text) => {
                let preview = text.chars().take(400).collect::<String>();
                eprintln!("[MODIFY] text: {preview}");
            }
            ChatDelta::Thinking(_) => eprintln!("[MODIFY] thinking"),
            ChatDelta::ToolUse { name, .. } => eprintln!("[MODIFY] tool: {name}"),
            ChatDelta::Done { stop_reason } => eprintln!("[MODIFY] done: {stop_reason:?}"),
            ChatDelta::Error(error) => eprintln!("[MODIFY] provider error: {error}"),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
#[path = "modify_mode_tests.rs"]
mod tests;
