//! Design-MD panel host logic — drains the panel's import / export
//! requests, which need the native file dialog the widget layer
//! cannot reach.
//!
//! Split out of `main.rs` to keep that file under the repo's
//! 800-line-per-file cap.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode};
use op_editor_core::EditorState;

use crate::chat_session::{provider_for_selected_model, selected_cli_model_id};
use crate::DesktopApp;

const DESIGN_MD_SYSTEM_PROMPT: &str = r##"You are a Design Systems Lead. Analyze the provided PenNode design tree and generate a comprehensive design.md in the Google Stitch format.

OUTPUT FORMAT — a complete markdown document with these sections:

# Design System: [Project Name]

## 1. Visual Theme & Atmosphere
Describe the mood, density, and aesthetic philosophy using evocative adjectives.

## 2. Color Palette & Roles
For each color found in the design:
- **Descriptive Name** (#HEX) — Functional role (e.g. "Primary CTA", "Background", "Body text")

## 3. Typography Rules
- Font families used, weight hierarchy, size scale, line-height conventions.

## 4. Component Stylings
- **Buttons**: shape, colors, padding, states
- **Cards**: corners, shadows, internal padding
- **Inputs**: borders, backgrounds
- **Navigation**: layout, spacing

## 5. Layout Principles
- Grid system, whitespace strategy, spacing units, responsive breakpoints.

## 6. Design System Notes
- Key language/terms to use when generating new designs in this style.

RULES:
- Use descriptive natural language, NOT technical jargon (e.g. "subtly rounded corners" not "rounded-lg").
- Pair ALL colors with exact hex codes.
- Explain functional roles for every design element.
- Output ONLY the markdown document, starting with "# Design System:".
- NO preamble, NO commentary, NO tool calls, NO code fences around the output.
- Do NOT use <tool_call> tags or any tool invocations. Just output the markdown text directly."##;

const DESIGN_MD_MAX_TREE_CHARS: usize = 24_000;
const DESIGN_MD_MAX_VAR_CHARS: usize = 6_000;

pub(crate) struct DesignMdSession {
    rx: Receiver<Result<String, String>>,
}

impl DesignMdSession {
    fn start(provider: Box<dyn ChatProvider>, model: Option<String>, state: &EditorState) -> Self {
        let request = build_design_md_chat_request(state, model);
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("op-design-md".into())
            .spawn(move || {
                let _ = tx.send(run_design_md_provider_blocking(provider, request));
            })
            .expect("spawn op-design-md thread");
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

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        out.push_str("\n... [truncated]");
    }
    out
}

fn run_design_md_provider_blocking(
    provider: Box<dyn ChatProvider>,
    request: ChatRequest,
) -> Result<String, String> {
    let mut out = String::new();
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(text) => out.push_str(&text),
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Done { .. } => break,
            ChatDelta::Error(message) => return Err(message),
        }
    }
    let cleaned = clean_ai_design_md_result(&out);
    if cleaned.is_empty() {
        Err("design.md generation returned empty output".into())
    } else {
        Ok(cleaned)
    }
}

fn clean_ai_design_md_result(raw: &str) -> String {
    let mut text = strip_tool_call_blocks(raw.trim());
    text = strip_code_fence(text);
    if let Some(start) = text.find("# ") {
        text = text[start..].to_string();
    }
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("{\"name\"")
                && !trimmed.starts_with("{\"tool_use_id\"")
                && !trimmed.starts_with("{\"file_path\"")
                && trimmed != "<tool_call>"
                && trimmed != "</tool_call>"
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n\n", "\n\n\n")
        .trim()
        .to_string()
}

fn strip_code_fence(mut text: String) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return text;
    }
    text = trimmed.to_string();
    if let Some(idx) = text.find('\n') {
        text = text[idx + 1..].to_string();
    }
    if let Some(idx) = text.rfind("```") {
        text.truncate(idx);
    }
    text.trim().to_string()
}

fn strip_tool_call_blocks(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let Some(start) = rest.find("<tool_call>") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_start = &rest[start + "<tool_call>".len()..];
        let Some(end) = after_start.find("</tool_call>") else {
            break;
        };
        rest = &after_start[end + "</tool_call>".len()..];
    }
    out
}

impl DesktopApp {
    /// Run a queued Design-MD request — `design_md_request`, set by a
    /// panel click. A no-op when nothing is queued.
    pub(crate) fn drain_design_md_action(&mut self) -> bool {
        use op_editor_core::DesignMdRequest;
        let Some(request) = self
            .host
            .editor_state_mut()
            .editor_ui
            .design_md_request
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
        let spec = op_editor_core::parse_design_md(&markdown);
        // Snapshot first so the import is a single undo step.
        let snap = self.host.editor_state().snapshot_for_history();
        let state = self.host.editor_state_mut();
        state.doc.design_md = Some(spec);
        state.editor_ui.design_md_scroll.offset = 0.0;
        state.history_push_past(snap);
        self.host.mark_editor_state_dirty();
        true
    }

    /// Generate a fresh design.md from the open `.op` document using
    /// the selected chat-panel model. Replaces any existing brief only
    /// after the model returns markdown.
    fn auto_generate_design_md(&mut self) -> bool {
        if self.current_design_md.take().is_some() {
            self.host.editor_state_mut().editor_ui.design_md_generating = false;
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
        self.host.editor_state_mut().editor_ui.design_md_generating = true;
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
            Err(TryRecvError::Disconnected) => {
                Err("design.md generation worker vanished".to_string())
            }
        };
        self.current_design_md = None;
        self.host.editor_state_mut().editor_ui.design_md_generating = false;
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
        let spec = op_editor_core::parse_design_md(&markdown);
        let snap = self.host.editor_state().snapshot_for_history();
        let state = self.host.editor_state_mut();
        state.doc.design_md = Some(spec);
        state.editor_ui.design_md_scroll.offset = 0.0;
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
