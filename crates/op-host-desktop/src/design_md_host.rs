//! Design-MD panel host logic — drains the panel's import / export
//! requests, which need the native file dialog the widget layer
//! cannot reach.
//!
//! Split out of `main.rs` to keep that file under the repo's
//! 800-line-per-file cap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use op_ai::design_md::{
    clean_ai_design_md_result, truncate_chars, DESIGN_MD_MAX_TREE_CHARS, DESIGN_MD_MAX_VAR_CHARS,
    DESIGN_MD_SYSTEM_PROMPT,
};
use op_editor_core::EditorState;

use crate::chat_session::{provider_for_selected_model, selected_cli_model_id};
use crate::design_md_error::DesignMdError;
use crate::DesktopApp;

/// Hard response ceiling for extension-triggered design-system extraction.
/// The model stream is stopped before it can accumulate an arbitrarily large
/// response in the desktop process, and the cleaned document is checked again
/// before it crosses the MCP responder.
const MCP_DESIGN_MD_MAX_OUTPUT_BYTES: usize = 512 * 1024;
/// Cleaning may remove a small outer fence or preamble. Allow that bounded
/// envelope while keeping the raw accumulation effectively at the public cap.
const MCP_DESIGN_MD_MAX_RAW_BYTES: usize = MCP_DESIGN_MD_MAX_OUTPUT_BYTES + 256;

#[derive(Debug, Clone, PartialEq, Eq)]
enum McpDesignMdFailure {
    Provider,
    EmptyOutput,
    InvalidOutput,
    OutputTooLarge,
}

pub(crate) struct DesignMdSession {
    rx: Receiver<Result<String, DesignMdError>>,
}

impl DesignMdSession {
    fn start(provider: Box<dyn ChatProvider>, model: Option<String>, state: &EditorState) -> Self {
        let request = build_design_md_chat_request(state, model);
        let (tx, rx) = mpsc::channel();
        let worker_tx = tx.clone();
        let spawned = thread::Builder::new()
            .name("op-design-md".into())
            .spawn(move || {
                let _ = worker_tx.send(run_design_md_provider_blocking(provider, request));
            });
        if let Err(err) = spawned {
            // Deliver the failure through the normal poll path so the
            // generating flag clears instead of the app crashing.
            eprintln!("openpencil-desktop: spawn op-design-md thread failed: {err}");
            let _ = tx.send(Err(DesignMdError::WorkerSpawn(err.to_string())));
        }
        Self { rx }
    }
}

fn build_design_md_chat_request(state: &EditorState, model: Option<String>) -> ChatRequest {
    let user_prompt = build_design_md_user_prompt(state);
    ChatRequest {
        // CLI-backed providers do not all expose a system-prompt slot, so inline
        // the role prompt exactly like the design orchestrator adapter does.
        system_prompt: String::new(),
        user_message: format!("{DESIGN_MD_SYSTEM_PROMPT}\n\n---\n\n{user_prompt}"),
        history: Vec::new(),
        max_output_tokens: 8192,
        thinking: ThinkingMode::Disabled,
        effort: EffortLevel::High,
        attachments: vec![],
        model,
    }
}

fn build_mcp_design_md_chat_request(
    system_prompt: String,
    user_prompt: String,
    model: Option<String>,
) -> ChatRequest {
    ChatRequest {
        // Extension evidence is allowed only through the evidence-only
        // built-in provider, which honors the real system channel. Keep the
        // untrusted corpus exclusively in the user/data role instead of
        // duplicating trusted instructions into it.
        system_prompt,
        user_message: user_prompt,
        history: Vec::new(),
        max_output_tokens: 8192,
        thinking: ThinkingMode::Disabled,
        effort: EffortLevel::High,
        attachments: vec![],
        model,
    }
}

fn build_design_md_user_prompt(state: &EditorState) -> String {
    let project = state.doc.name.as_deref().unwrap_or("Untitled");
    let tree =
        serde_json::to_string_pretty(state.active_children()).unwrap_or_else(|_| "[]".to_string());
    let tree = truncate_chars(&tree, DESIGN_MD_MAX_TREE_CHARS);
    let vars = state
        .doc
        .variables
        .as_ref()
        .and_then(|vars| serde_json::to_string_pretty(vars).ok())
        .map(|json| truncate_chars(&json, DESIGN_MD_MAX_VAR_CHARS))
        .unwrap_or_else(|| "{}".to_string());

    format!(
        "Analyze this PenNode design tree and generate a comprehensive design.md.\n\n\
         Project: {project}\n\n\
         Design tree JSON for the active page:\n{tree}\n\n\
         Design variables JSON:\n{vars}"
    )
}

fn run_design_md_provider_blocking(
    provider: Box<dyn ChatProvider>,
    request: ChatRequest,
) -> Result<String, DesignMdError> {
    let mut out = String::new();
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(text) => out.push_str(&text),
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Done { .. } => break,
            // `ChatDelta::Error` carries a `String` from a trait this pass
            // does not own; store it verbatim.
            ChatDelta::Error(message) => return Err(DesignMdError::Provider(message)),
        }
    }
    let cleaned = clean_ai_design_md_result(&out);
    if cleaned.is_empty() {
        Err(DesignMdError::EmptyOutput)
    } else {
        Ok(cleaned)
    }
}

/// Run the extension MCP extraction turn without touching the document-bound
/// Design-MD session. This stricter path validates the public response
/// contract in addition to applying the shared model-output cleanup.
fn run_mcp_design_md_provider_blocking(
    provider: Box<dyn ChatProvider>,
    request: ChatRequest,
    cancel: Arc<AtomicBool>,
) -> Result<String, McpDesignMdFailure> {
    if cancel.load(Ordering::Acquire)
        || !provider.supports_cancellable_send()
        || !provider.supports_evidence_only_send()
    {
        return Err(McpDesignMdFailure::Provider);
    }
    let mut out = String::new();
    for delta in provider.send_cancellable(request, Arc::clone(&cancel)) {
        if cancel.load(Ordering::Acquire) {
            return Err(McpDesignMdFailure::Provider);
        }
        match delta {
            ChatDelta::TextDelta(text) => {
                if out.len().saturating_add(text.len()) > MCP_DESIGN_MD_MAX_RAW_BYTES {
                    return Err(McpDesignMdFailure::OutputTooLarge);
                }
                out.push_str(&text);
            }
            ChatDelta::Thinking(_) => {}
            ChatDelta::ToolUse { .. } => {
                return Err(McpDesignMdFailure::Provider);
            }
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            } => break,
            ChatDelta::Done { .. } => return Err(McpDesignMdFailure::Provider),
            ChatDelta::Error(_) => return Err(McpDesignMdFailure::Provider),
        }
    }
    if cancel.load(Ordering::Acquire) {
        return Err(McpDesignMdFailure::Provider);
    }
    let cleaned = clean_mcp_design_md_result(&out);
    if cleaned.is_empty() {
        return Err(McpDesignMdFailure::EmptyOutput);
    }
    if cleaned.len() > MCP_DESIGN_MD_MAX_OUTPUT_BYTES {
        return Err(McpDesignMdFailure::OutputTooLarge);
    }
    if !cleaned.starts_with("# Design System:")
        || cleaned.contains("http://")
        || cleaned.contains("https://")
        || cleaned.contains("data:")
    {
        return Err(McpDesignMdFailure::InvalidOutput);
    }
    let mut last_heading_position = 0;
    for required_heading in ["## Color System", "## Typography", "## Corner Radius"] {
        if cleaned
            .lines()
            .filter(|line| *line == required_heading)
            .count()
            != 1
        {
            return Err(McpDesignMdFailure::InvalidOutput);
        }
        let Some(position) = cleaned.find(required_heading) else {
            return Err(McpDesignMdFailure::InvalidOutput);
        };
        if position < last_heading_position {
            return Err(McpDesignMdFailure::InvalidOutput);
        }
        last_heading_position = position;
    }
    Ok(cleaned)
}

fn clean_mcp_design_md_result(raw: &str) -> String {
    let mut cleaned = clean_ai_design_md_result(raw);
    // The shared cleaner handles either a preamble or an outer fence. Some
    // models emit both (`Here you go: ```markdown ... ````); after the
    // preamble is removed that leaves only the closing fence. Strip it when
    // the original answer proves the design document was fence-wrapped.
    if cleaned.ends_with("\n```")
        && raw.trim_end().ends_with("```")
        && raw
            .find("# Design System:")
            .is_some_and(|heading| raw[..heading].contains("```"))
    {
        cleaned.truncate(cleaned.len() - "\n```".len());
        cleaned = cleaned.trim_end().to_string();
    }
    cleaned
}

impl DesktopApp {
    /// Hand a validated extension evidence request to the currently selected
    /// generation model. The responder travels with a detached worker, so the
    /// UI pump returns immediately and no document/session state is occupied.
    pub(crate) fn start_mcp_design_md_request(
        &mut self,
        pending: op_host_services::mcp_live::PendingDesignMdRequest,
    ) {
        use op_host_services::mcp_live::DesignMdResponseError;

        // The HTTP caller may have given up while the UI thread was busy.
        // Starting a paid provider turn after that deadline would produce a
        // result nobody can receive. Dropping the pending request also drops
        // its single-flight lease, so a fresh button press can try again.
        if pending.is_cancelled() {
            return;
        }
        let (system_prompt, user_prompt, responder) = pending.into_parts();
        let Some(provider) = self.mcp_design_md_provider_for_generation() else {
            let _ = responder.error(DesignMdResponseError::NoModel);
            return;
        };
        let request = build_mcp_design_md_chat_request(
            system_prompt,
            user_prompt,
            selected_cli_model_id(&self.host),
        );
        let cancel = responder.cancellation_flag();
        // Keep the single-use responder behind a shared slot. `spawn`
        // consumes its closure even when thread creation fails; this lets the
        // caller report `WorkerSpawn` without making the responder cloneable
        // (and therefore without creating a double-response capability).
        let responder = Arc::new(Mutex::new(Some(responder)));
        let worker_responder = Arc::clone(&responder);
        let spawned = thread::Builder::new()
            .name("op-mcp-design-md".into())
            .spawn(move || {
                let outcome = run_mcp_design_md_provider_blocking(provider, request, cancel);
                let Some(responder) = worker_responder
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .take()
                else {
                    return;
                };
                match outcome {
                    Ok(markdown) => {
                        let _ = responder.success(markdown);
                    }
                    Err(error) => {
                        let response_error = match error {
                            McpDesignMdFailure::Provider => {
                                eprintln!("openpencil-desktop: MCP design.md provider failed");
                                DesignMdResponseError::ProviderError
                            }
                            McpDesignMdFailure::EmptyOutput => DesignMdResponseError::EmptyOutput,
                            McpDesignMdFailure::InvalidOutput => {
                                DesignMdResponseError::InvalidOutput
                            }
                            McpDesignMdFailure::OutputTooLarge => {
                                DesignMdResponseError::OutputTooLarge
                            }
                        };
                        let _ = responder.error(response_error);
                    }
                }
            });
        if let Err(error) = spawned {
            eprintln!("openpencil-desktop: spawn MCP design.md thread failed: {error}");
            if let Some(responder) = responder
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take()
            {
                let _ = responder.error(DesignMdResponseError::WorkerSpawn);
            }
        }
    }

    /// Run a queued Design-MD request — `design_md_panel.request`, set by a
    /// panel click. A no-op when nothing is queued.
    pub(crate) fn drain_design_md_action(&mut self) -> bool {
        use op_editor_core::DesignMdRequest;
        let Some(request) = self
            .host
            .editor_state_mut()
            .editor_ui
            .design_md_panel
            .request
            .take()
        else {
            return false;
        };
        let locale = self.host.editor_state().editor_ui.locale;
        match request {
            DesignMdRequest::Import => self.import_design_md(locale),
            DesignMdRequest::AutoGenerate => self.auto_generate_design_md(),
            DesignMdRequest::Export => self.export_design_md(locale),
        }
    }

    /// Pick a `.md` file, parse it into a `DesignMdSpec`, and bind it
    /// to the open document (undoable).
    fn import_design_md(&mut self, locale: op_editor_core::Locale) -> bool {
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::Document(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::RootMetadata,
                ),
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return true;
        }
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "designMd.import"))
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file();
        let Some(path) = picked else {
            return false;
        };
        let markdown = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("openpencil-desktop: design.md import failed: {err}");
                return false;
            }
        };
        // The native picker yielded the event loop; re-check the live role
        // immediately before the document sink.
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::Document(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::RootMetadata,
                ),
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return true;
        }
        let spec = op_editor_core::parse_design_md(&markdown);
        // Snapshot first so the import is a single undo step.
        let snap = self.host.editor_state().snapshot_for_history();
        let state = self.host.editor_state_mut();
        state.doc.design_md = Some(spec);
        state.editor_ui.design_md_panel.scroll.offset = 0.0;
        state.history_push_past(snap);
        self.host.mark_editor_state_dirty();
        true
    }

    /// Generate a fresh design.md from the open `.op` document using
    /// the selected chat-panel model. Replaces any existing brief only
    /// after the model returns markdown.
    fn auto_generate_design_md(&mut self) -> bool {
        if self.current_design_md.take().is_some() {
            self.host
                .editor_state_mut()
                .editor_ui
                .design_md_panel
                .generating = false;
            self.host.mark_editor_state_dirty();
            return true;
        }
        if self.host.editor_state().active_children().is_empty() {
            return false;
        }
        let Some(provider) = self.design_md_provider_for_generation() else {
            eprintln!("openpencil-desktop: design.md auto-generate skipped: no model configured");
            return false;
        };
        let model = selected_cli_model_id(&self.host);
        let initial_state = self.host.editor_state().clone();
        self.current_design_md = Some(DesignMdSession::start(provider, model, &initial_state));
        self.host
            .editor_state_mut()
            .editor_ui
            .design_md_panel
            .generating = true;
        self.host.mark_editor_state_dirty();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_design_md_test_provider(&mut self, provider: Box<dyn ChatProvider>) {
        self.design_md_test_provider = Some(provider);
    }

    fn design_md_provider_for_generation(&mut self) -> Option<Box<dyn ChatProvider>> {
        #[cfg(test)]
        if let Some(provider) = self.design_md_test_provider.take() {
            return Some(provider);
        }
        provider_for_selected_model(&self.host)
    }

    /// Extension design extraction has a hard route deadline. Only transports
    /// that can abort their in-flight work may start here; returning `NoModel`
    /// lets the extension use its local deterministic fallback instead of
    /// leaving an uncancellable CLI/ACP turn paid and busy after timeout.
    fn mcp_design_md_provider_for_generation(&mut self) -> Option<Box<dyn ChatProvider>> {
        let provider = self.design_md_provider_for_generation()?;
        if !provider.supports_cancellable_send() || !provider.supports_evidence_only_send() {
            eprintln!(
                "openpencil-desktop: MCP design.md skipped unsafe provider: {}",
                provider.provider_label()
            );
            return None;
        }
        Some(provider)
    }

    pub(crate) fn poll_design_md_generation(&mut self) -> bool {
        let Some(session) = self.current_design_md.as_ref() else {
            return false;
        };
        let outcome = match session.rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(DesignMdError::WorkerVanished),
        };
        self.current_design_md = None;
        self.host
            .editor_state_mut()
            .editor_ui
            .design_md_panel
            .generating = false;
        match outcome {
            Ok(markdown) => {
                self.apply_generated_design_md(markdown);
            }
            Err(err) => {
                eprintln!("openpencil-desktop: design.md auto-generate failed: {err}");
                self.host.mark_editor_state_dirty();
            }
        }
        true
    }

    fn apply_generated_design_md(&mut self, markdown: String) {
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::Document(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::RootMetadata,
                ),
            ),
            op_editor_core::CollabEditSource::Ai,
        ) {
            return;
        }
        let spec = op_editor_core::parse_design_md(&markdown);
        let snap = self.host.editor_state().snapshot_for_history();
        let state = self.host.editor_state_mut();
        state.doc.design_md = Some(spec);
        state.editor_ui.design_md_panel.scroll.offset = 0.0;
        state.history_push_past(snap);
        self.host.mark_editor_state_dirty();
    }

    /// Write the open document's design.md to a `.md` file. The
    /// original markdown (`DesignMdSpec::raw`) round-trips verbatim.
    fn export_design_md(&mut self, locale: op_editor_core::Locale) -> bool {
        let Some(raw) = self
            .host
            .editor_state()
            .doc
            .design_md
            .as_ref()
            .map(|s| s.raw.clone())
        else {
            // Nothing to export — the panel's export button is only
            // meaningful once a brief exists.
            return false;
        };
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "designMd.export"))
            .add_filter("Markdown", &["md"])
            .set_file_name("design.md")
            .save_file();
        let Some(path) = picked else {
            return false;
        };
        if let Err(err) = std::fs::write(&path, raw) {
            eprintln!("openpencil-desktop: design.md export failed: {err}");
        }
        false
    }
}

#[cfg(test)]
#[path = "design_md_host_mcp_tests.rs"]
mod mcp_tests;
