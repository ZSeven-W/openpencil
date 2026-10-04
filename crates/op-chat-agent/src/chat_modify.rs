//! Shared scoped-modification contract for desktop, web, and mobile hosts.
//! A bounded structured response edits captured existing board ids; it never
//! enters the whole-design agent loop or its global finalize passes.

use crate::chat_canvas_tools::UiChatToolExecutor;
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, ChatToolExecutor, StopReason};
use op_editor_core::{EditorState, PenNodeExt};
use std::sync::{mpsc::Sender, Arc};
type PenNode = jian_ops_schema::node::PenNode;
fn is_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
}

/// Internal UI apply command, never advertised as a model tool.
pub const APPLY_MODIFICATION_OP: &str = "__apply_design_modification";

// ---------------------------------------------------------------------------
// Modification plan — port of generateDesignModification's inputs
// ---------------------------------------------------------------------------

/// TS `buildVariableContext` (design-generator.ts:43-72). `None` when
/// the document has no variables. BTreeMap iteration is sorted where
/// TS uses insertion order — content is identical, ordering may not be.
pub fn build_variable_context(state: &EditorState) -> Option<String> {
    let vars = state.doc.variables.as_ref().filter(|v| !v.is_empty())?;
    let mut lines: Vec<String> = vec![
        "DOCUMENT VARIABLES (use \"$name\" to reference, e.g. fill color \"$color-1\"):".into(),
    ];
    for (name, def) in vars {
        let kind = variable_kind_label(&def.kind);
        match &def.value {
            jian_ops_schema::variable::VariableValue::Themed(values) => {
                let default_val = values
                    .first()
                    .map(|v| scalar_display(&v.value))
                    .unwrap_or_else(|| "?".into());
                lines.push(format!("  - {name} ({kind}): {default_val} [themed]"));
            }
            jian_ops_schema::variable::VariableValue::Scalar(value) => {
                lines.push(format!("  - {name} ({kind}): {}", scalar_display(value)));
            }
        }
    }
    if let Some(themes) = state.doc.themes.as_ref().filter(|t| !t.is_empty()) {
        let summary = themes
            .iter()
            .map(|(axis, values)| format!("{axis}: [{}]", values.join(", ")))
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(format!("Themes: {summary}"));
    }
    Some(lines.join("\n"))
}

pub(super) fn variable_kind_label(kind: &jian_ops_schema::variable::VariableKind) -> &'static str {
    use jian_ops_schema::variable::VariableKind;
    match kind {
        VariableKind::Color => "color",
        VariableKind::Number => "number",
        VariableKind::String => "string",
        VariableKind::Boolean => "boolean",
    }
}

/// JS template-literal rendering of a variable scalar.
pub(super) fn scalar_display(value: &jian_ops_schema::variable::VariableScalar) -> String {
    use jian_ops_schema::variable::VariableScalar;
    match value {
        VariableScalar::Bool(b) => b.to_string(),
        VariableScalar::Str(s) => s.clone(),
        VariableScalar::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
    }
}

/// Pre-built `generateDesignModification` request inputs.
pub struct ModifyPlan {
    /// `CONTEXT NODES + INSTRUCTION (+ variable context)` user message.
    pub user_message: String,
    /// Maintenance skills (+ design-md style policy) system prompt.
    pub system_prompt: String,
    /// Immutable write scope captured when the turn starts.
    pub target_frame_ids: Vec<String>,
}

pub(super) fn strip_base64_data_uris(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) if s.starts_with("data:") && s.contains(";base64,") => {
            *s = "<image>".to_string();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_base64_data_uris(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                strip_base64_data_uris(item);
            }
        }
        _ => {}
    }
}

pub(super) struct ModifyNodeParse {
    pub(super) nodes: Vec<crate::chat_canvas_tools::DesignModificationOp>,
    pub(super) diagnostic: Option<String>,
}

pub(super) fn parse_modify_response(full_response: &str) -> ModifyNodeParse {
    if full_response.trim().is_empty() {
        return ModifyNodeParse {
            nodes: Vec::new(),
            diagnostic: Some("the model returned no text".into()),
        };
    }
    if full_response.len() > op_mcp::script_runner::MAX_SCRIPT_BYTES {
        return ModifyNodeParse {
            nodes: Vec::new(),
            diagnostic: Some("modification response exceeds the source size limit".into()),
        };
    }

    let script = op_mcp::script_runner::run_modification_script_to_program(full_response);
    let nodes: Vec<crate::chat_canvas_tools::DesignModificationOp> = script
        .as_ref()
        .ok()
        .map(|program| {
            op_mcp::parse_program_objects(program)
                .into_iter()
                .map(|(parent, mut node)| {
                    op_orchestrator::parse::normalize_generated_node_json(&mut node);
                    (parent, node)
                })
                .collect()
        })
        .unwrap_or_default();
    if !nodes.is_empty() {
        // Script execution proves syntax, not node validity. Validate every
        // subtree before any host write so malformed nested icons enter the
        // bounded feedback retry instead of failing halfway through apply.
        if let Some((index, error)) = nodes.iter().enumerate().find_map(|(index, (_, node))| {
            op_mcp::validate_replacement_node_data(node)
                .err()
                .map(|error| (index, error))
        }) {
            return ModifyNodeParse {
                nodes: Vec::new(),
                diagnostic: Some(format!("modification node {index} is invalid: {error}")),
            };
        }
        return ModifyNodeParse {
            nodes,
            diagnostic: None,
        };
    }

    let Some(json_response) = standalone_json_response(full_response) else {
        return ModifyNodeParse {
            nodes: Vec::new(),
            diagnostic: Some(
                "response was not valid modification JavaScript; response was not valid node JSON"
                    .into(),
            ),
        };
    };
    match op_orchestrator::parse::parse_nodes(json_response) {
        Ok(nodes) if !nodes.is_empty() => ModifyNodeParse {
            nodes: nodes
                .into_iter()
                .map(|node| {
                    (
                        "null".to_string(),
                        serde_json::to_value(node).unwrap_or(serde_json::Value::Null),
                    )
                })
                .collect(),
            diagnostic: None,
        },
        parsed => {
            let script_detail = match script {
                Ok(_) => "response contained no I(...) operations".to_string(),
                Err(_) => "response was not valid modification JavaScript".to_string(),
            };
            let node_detail = match parsed {
                Ok(_) => "node response contained no nodes".to_string(),
                Err(_) => "response was not valid node JSON".to_string(),
            };
            ModifyNodeParse {
                nodes: Vec::new(),
                diagnostic: Some(format!("{script_detail}; {node_detail}")),
            }
        }
    }
}

/// Only an entire JSON / JSONL payload may use the legacy JSON route. Never
/// scan object literals from a rejected, capped, throwing or no-op program.
fn standalone_json_response(text: &str) -> Option<&str> {
    let mut text = text.trim();
    if let Some(fenced) = text.strip_prefix("```") {
        let (language, body) = fenced.split_once('\n')?;
        if !matches!(language.trim(), "" | "json" | "jsonl") {
            return None;
        }
        text = body.trim().strip_suffix("```")?.trim();
    }
    let values = serde_json::Deserializer::from_str(text)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (!values.is_empty() && values.iter().all(|v| v.is_object() || v.is_array())).then_some(text)
}

pub fn parse_modify_nodes(
    full_response: &str,
) -> Vec<crate::chat_canvas_tools::DesignModificationOp> {
    parse_modify_response(full_response).nodes
}

/// Build a modification plan for explicitly selected Frames. Direct mutation
/// is never inferred from the last canvas node: when selection is empty,
/// stale, or includes a non-Frame, the caller must degrade to a non-modifying
/// route.
pub(super) fn selected_frame_ids(state: &EditorState) -> Option<Vec<String>> {
    let children = state.active_children();
    if state.selection.set.is_empty() {
        return None;
    }
    let mut ids = Vec::with_capacity(state.selection.set.len());
    for id in &state.selection.set {
        let node = op_editor_core::walkers::find_node(children, id)?;
        if !is_frame(node) {
            return None;
        }
        ids.push(node.id_str().to_string());
    }
    Some(ids)
}

/// Resolve normal-mode page references before selection-only professional
/// behavior. The returned ids are captured once and enforced at apply time.
pub fn modify_target_frame_ids(state: &EditorState, instruction: &str) -> Option<Vec<String>> {
    use crate::workspace_edit::{resolve_workspace_edit_scope, WorkspaceEditScope};
    match resolve_workspace_edit_scope(state, instruction) {
        WorkspaceEditScope::Target(target) => Some(vec![target.board_id]),
        WorkspaceEditScope::NeedsTarget => None,
        WorkspaceEditScope::NotApplicable => selected_frame_ids(state),
    }
}

pub fn build_modify_plan(state: &EditorState, instruction: &str) -> Option<ModifyPlan> {
    build_modify_plan_for_route(state, instruction, op_editor_core::LaunchRoute::Auto)
}

/// A pinned template refinement carries an explicit selected-board scope;
/// ordinary follow-ups must still resolve their own narrower target.
pub fn build_modify_plan_for_route(
    state: &EditorState,
    instruction: &str,
    route: op_editor_core::LaunchRoute,
) -> Option<ModifyPlan> {
    let target_frame_ids = if route.forces_in_place_refine() {
        selected_frame_ids(state)?
    } else {
        modify_target_frame_ids(state, instruction)?
    };
    build_modify_plan_for_targets(state, instruction, target_frame_ids)
}

/// Build from an already captured scope (for example a consumed reader edit).
pub fn build_modify_plan_for_targets(
    state: &EditorState,
    instruction: &str,
    target_frame_ids: Vec<String>,
) -> Option<ModifyPlan> {
    let children = state.active_children();
    if target_frame_ids.is_empty() {
        return None;
    }
    let targets = target_frame_ids
        .iter()
        .map(|id| op_editor_core::walkers::find_node(children, &op_editor_core::NodeId::new(id)))
        .collect::<Option<Vec<_>>>()?;
    if targets.iter().any(|node| !is_frame(node)) {
        return None;
    }

    let mut context = serde_json::to_value(&targets).ok()?;
    strip_base64_data_uris(&mut context);
    let context_json = serde_json::to_string(&context).ok()?;
    let mut user_message = format!("CONTEXT NODES:\n{context_json}\n\nINSTRUCTION:\n{instruction}");
    if let Some(var_context) = build_variable_context(state) {
        user_message.push_str("\n\n");
        user_message.push_str(&var_context);
    }

    // Maintenance-phase skills (TS resolveSkills('maintenance', …)).
    let has_variables = state
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| !vars.is_empty());
    let mut options = op_ai_skills::ResolveOptions::default();
    options
        .flags
        .insert("hasVariables".to_string(), has_variables);
    options
        .flags
        .insert("hasDesignMd".to_string(), state.doc.design_md.is_some());
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Maintenance, instruction, &options);
    let mut system_prompt = ctx
        .skills
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(spec) = state.doc.design_md.as_ref() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&op_orchestrator::build_design_md_style_policy(spec));
    }

    Some(ModifyPlan {
        user_message,
        system_prompt,
        target_frame_ids,
    })
}

/// TS `ai-chat-handlers.ts:721-722` — the modification progress step.
pub const MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;
pub(super) const MODIFY_RETRY_REMINDER: &str =
    "\n\nCRITICAL: Respond with ONLY I(...) JavaScript statements -- never prose, explanations, or numbered/bulleted lists. If you truly cannot make the change, return an empty program.";

pub(super) struct ModifyTurnParse {
    nodes: Vec<crate::chat_canvas_tools::DesignModificationOp>,
    parse_diagnostic: Option<String>,
    stream_error: Option<String>,
    retryable_completion: bool,
}

pub(super) fn applied_modify_nodes_json(
    nodes: &[crate::chat_canvas_tools::DesignModificationOp],
) -> Option<String> {
    let node_values = nodes
        .iter()
        .map(|(_, node)| node.clone())
        .collect::<Vec<_>>();
    if node_values.is_empty() {
        return None;
    }
    serde_json::to_string_pretty(&node_values).ok()
}

fn stream_and_parse_modify_turn_inner(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> ModifyTurnParse {
    let mut full_response = String::new();
    let mut stream_error: Option<String> = None;
    let mut retryable_completion = false;
    let stream = match cancel {
        Some(cancel) => provider.send_cancellable(request, cancel),
        None => provider.send(request),
    };
    for delta in stream {
        match delta {
            ChatDelta::TextDelta(s) => full_response.push_str(&s),
            // TS: thinking chunks are ignored for modification — the
            // caller already shows progress.
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Error(msg) => {
                stream_error = Some(msg);
                break;
            }
            ChatDelta::Done { stop_reason } => {
                retryable_completion = stop_reason == StopReason::EndTurn;
                break;
            }
        }
    }

    // TS order: parse first; a stream error only surfaces when no
    // nodes could be extracted (design-generator.ts:158-165).
    let parsed = parse_modify_response(&full_response);
    ModifyTurnParse {
        nodes: parsed.nodes,
        parse_diagnostic: parsed.diagnostic,
        stream_error,
        retryable_completion,
    }
}

pub(super) fn bounded_feedback(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut bounded = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        bounded.push_str("...");
    }
    bounded
}

pub(super) fn with_modify_retry_feedback(
    mut request: ChatRequest,
    failed: &ModifyTurnParse,
) -> ChatRequest {
    request.system_prompt.push_str(MODIFY_RETRY_REMINDER);
    let diagnostic = failed
        .stream_error
        .as_deref()
        .or(failed.parse_diagnostic.as_deref())
        .unwrap_or("no applicable nodes were extracted");
    request.user_message.push_str(
        "\n\nRETRY FEEDBACK:\nThe previous response produced no applicable edit. Rewrite the requested modification as valid I(parent, node) JavaScript.\nParser feedback: ",
    );
    request
        .user_message
        .push_str(&bounded_feedback(diagnostic, 500));
    request
}

/// The DESIGN_MODIFY route — port of `generateDesignModification` +
/// `extractAndApplyDesignModification` + the handler glue
/// (`ai-chat-handlers.ts:708-741`, `design-generator.ts:95-173`).
pub fn run_modify_turn(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    chat_tx: &Sender<ChatDelta>,
    executor: &UiChatToolExecutor,
    target_frame_ids: Vec<String>,
) {
    run_modify_turn_inner(provider, request, chat_tx, executor, target_frame_ids, None);
}

/// Cancellable direct/standard modify route. The UI-side ChatSession owns the
/// same flag and sets it on Stop/New Chat/drop.
pub fn run_modify_turn_cancellable(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    chat_tx: &Sender<ChatDelta>,
    executor: &UiChatToolExecutor,
    target_frame_ids: Vec<String>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) {
    run_modify_turn_inner(
        provider,
        request,
        chat_tx,
        executor,
        target_frame_ids,
        Some(cancel),
    );
}

fn run_modify_turn_inner(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    chat_tx: &Sender<ChatDelta>,
    executor: &UiChatToolExecutor,
    target_frame_ids: Vec<String>,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
) {
    if chat_tx
        .send(ChatDelta::TextDelta(MODIFY_STEP.to_string()))
        .is_err()
    {
        return;
    }

    let canceled = || {
        cancel
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
    };
    let mut parsed = stream_and_parse_modify_turn_inner(provider, request.clone(), cancel.clone());
    if canceled() {
        return;
    }
    let mut retried = false;
    if parsed.nodes.is_empty() && parsed.stream_error.is_none() && parsed.retryable_completion {
        retried = true;
        let retry_request = with_modify_retry_feedback(request, &parsed);
        parsed = stream_and_parse_modify_turn_inner(provider, retry_request, cancel.clone());
        if canceled() {
            return;
        }
    }

    let ModifyTurnParse {
        nodes,
        parse_diagnostic: _,
        stream_error,
        retryable_completion: _,
    } = parsed;
    if !nodes.is_empty() {
        let args = serde_json::json!({
            "nodes": &nodes,
            "targetFrameIds": target_frame_ids,
        });
        let result = executor.execute(APPLY_MODIFICATION_OP, &args.to_string());
        if canceled() {
            return;
        }
        let acknowledgement = serde_json::from_str::<serde_json::Value>(&result.content).ok();
        let applied = acknowledgement
            .as_ref()
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0);
        let confirmed = !result.is_error
            && acknowledgement
                .as_ref()
                .and_then(|v| v.get("success").and_then(serde_json::Value::as_bool))
                == Some(true);
        if !confirmed || applied == 0 {
            let _ = chat_tx.send(ChatDelta::Error("No changes were applied. The page may have changed or editing may be unavailable. Select the page and retry.".into()));
            let _ = chat_tx.send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            });
            return;
        }
        {
            if let Some(json) = applied_modify_nodes_json(&nodes) {
                if chat_tx
                    .send(ChatDelta::TextDelta(format!("\n```json\n{json}\n```")))
                    .is_err()
                {
                    return;
                }
            }
            // TS `ai-chat-handlers.ts:830-831`.
            let _ = chat_tx.send(ChatDelta::TextDelta("\n\n<!-- APPLIED -->".to_string()));
        }
        let _ = chat_tx.send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
        return;
    }

    let message = if let Some(err) = stream_error {
        err
    } else if retried {
        "The model did not return an applicable edit after one automatic retry. Name the element and the exact change, or retry the previous instruction."
            .to_string()
    } else {
        "The model did not return an applicable edit. Name the element and the exact change, or retry the previous instruction."
            .to_string()
    };
    let _ = chat_tx.send(ChatDelta::Error(message));
    let _ = chat_tx.send(ChatDelta::Done {
        stop_reason: StopReason::Aborted,
    });
}

#[cfg(test)]
#[path = "chat_modify_tests.rs"]
mod tests;
