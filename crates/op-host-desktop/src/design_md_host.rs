//! Design-MD panel host logic — drains the panel's import / export
//! requests, which need the native file dialog the widget layer
//! cannot reach.
//!
//! Split out of `main.rs` to keep that file under the repo's
//! 800-line-per-file cap.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode};
use op_ai::design_md::{
    clean_ai_design_md_result, truncate_chars, DESIGN_MD_MAX_TREE_CHARS, DESIGN_MD_MAX_VAR_CHARS,
    DESIGN_MD_SYSTEM_PROMPT,
};
use op_editor_core::EditorState;

use crate::chat_session::{provider_for_selected_model, selected_cli_model_id};
use crate::design_md_error::DesignMdError;
use crate::DesktopApp;

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

impl DesktopApp {
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
        let Some(provider) = self.design_md_provider_for_auto_generate() else {
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

    fn design_md_provider_for_auto_generate(&mut self) -> Option<Box<dyn ChatProvider>> {
        #[cfg(test)]
        if let Some(provider) = self.design_md_test_provider.take() {
            return Some(provider);
        }
        provider_for_selected_model(&self.host)
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
