//! Support types the smoke runner's `main` and its mode modules share:
//! provider selection, the inline `DocSink`, command tracing, the smoke
//! template-library merge, and the loop-mode thinking switch.
//!
//! Carved out of `main.rs` as pure code motion to keep that spine under the
//! repo's 800-line cap; every item keeps its name so `crate::<item>` paths
//! (re-exported from `main.rs`) stay stable.

use std::sync::Arc;

use op_ai::chat_provider::{ChatProvider, CliName};
use op_editor_core::{EditorCommand, EditorState};
use op_host_services::chat_provider_llm::ChatProviderLlmClient;
use op_host_services::chat_subprocess::SubprocessProvider;
use op_orchestrator::{DocSink, LlmClient};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SmokeProviderKind {
    Anthropic,
    OpenAiCompat,
    Antigravity,
}

impl SmokeProviderKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" | "openai-compat" => Some(Self::OpenAiCompat),
            "antigravity" | "agy" => Some(Self::Antigravity),
            _ => None,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAiCompat => "openai-compat",
            Self::Antigravity => "antigravity",
        }
    }
}

pub(crate) fn truthy_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on"
        )
    })
}

pub(crate) fn antigravity_llm(model: &str) -> Box<dyn LlmClient> {
    let provider = SubprocessProvider::for_cli_generation(CliName::Antigravity)
        .expect("Antigravity has a production subprocess transport");
    let provider: Arc<dyn ChatProvider> = Arc::new(provider);
    Box::new(ChatProviderLlmClient::new(provider).with_model(Some(model.to_string())))
}

/// Inline `DocSink` — owns the canonical state directly, no channel hop.
/// Every `apply` echoes the command kind + result so the smoke trace
/// shows the orchestrator's mutations linearly.
pub(crate) struct InlineDocSink {
    pub(crate) state: EditorState,
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
pub(crate) fn describe_cmd(cmd: &EditorCommand) -> String {
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
pub(crate) fn maybe_merge_smoke_library(
    state: &mut EditorState,
) -> Result<(), std::process::ExitCode> {
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
pub(crate) fn loop_thinking_mode() -> op_ai::chat_provider::ThinkingMode {
    use op_ai::chat_provider::ThinkingMode;
    if std::env::var("OPENPENCIL_SMOKE_KEEP_THINKING").is_ok() {
        ThinkingMode::Enabled
    } else {
        ThinkingMode::Disabled
    }
}
