//! Browser-side Design-MD action drain.
//!
//! The widget layer mirrors native and only raises
//! `editor_ui.design_md_request`; this module consumes that request from the
//! CanvasKit event loop and performs the web host work.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use op_editor_core::{DesignMdRequest, EditorState};
use wasm_bindgen::JsValue;

use crate::repaint_ctx::RepaintContext;
use crate::web_ai_transport::{post_ai_stream_to, AiEvent, AiStreamHandle};

type InnerRc<C> = Rc<RefCell<C>>;
type EventQueue = Rc<RefCell<VecDeque<AiEvent>>>;

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

struct ActiveDesignMd {
    handle: AiStreamHandle,
    generation: u64,
}

thread_local! {
    static ACTIVE_DESIGN_MD: RefCell<Option<ActiveDesignMd>> = const { RefCell::new(None) };
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(1) };
}

pub(crate) fn drain_design_md_action<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let request = inner
        .borrow_mut()
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .design_md_request
        .take();
    match request {
        Some(DesignMdRequest::Import) => import_design_md(inner),
        Some(DesignMdRequest::AutoGenerate) => auto_generate_design_md(inner),
        Some(DesignMdRequest::Export) => export_design_md(inner),
        None => {}
    }
}

fn import_design_md<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let inner = inner.clone();
    crate::dom_io::open_file_picker(
        ".md,.markdown",
        Box::new(move |file| {
            let name = file.name();
            let inner2 = inner.clone();
            crate::dom_io::read_file(
                file,
                crate::dom_io::ReadMode::Text,
                Box::new(move |value| match value.as_string() {
                    Some(markdown) => apply_imported_design_md(&inner2, &markdown),
                    None => console_error(&format!(
                        "[design-md-import] {name}: file read produced no text"
                    )),
                }),
            );
        }),
    );
}

fn apply_imported_design_md<C: RepaintContext + 'static>(inner: &InnerRc<C>, markdown: &str) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let spec = op_editor_core::parse_design_md(markdown);
    let snap = b.host().editor_state().snapshot_for_history();
    let state = b.host_mut().editor_state_mut();
    state.doc.design_md = Some(spec);
    state.editor_ui.design_md_scroll.offset = 0.0;
    state.history_push_past(snap);
    b.host_mut().mark_editor_state_dirty();
    let _ = b.repaint();
}

fn export_design_md<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let raw = inner
        .borrow()
        .host()
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .map(|spec| spec.raw.clone());
    let Some(raw) = raw else {
        return;
    };
    if let Err(e) = crate::web_clipboard::download_bytes(
        "design.md",
        "text/markdown;charset=utf-8",
        raw.as_bytes(),
    ) {
        web_sys::console::error_1(&e);
    }
}

fn auto_generate_design_md<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    if abort_active_design_md() {
        set_generating(inner, false);
        return;
    }
    let body_json = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let Some(body) = build_auto_generate_body(b.host().editor_state()) else {
            return;
        };
        b.host_mut()
            .editor_state_mut()
            .editor_ui
            .design_md_generating = true;
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
        body
    };
    launch_generation(inner, body_json);
}

fn launch_generation<C: RepaintContext + 'static>(inner: &InnerRc<C>, body_json: String) {
    let generation = NEXT_GENERATION.with(|g| {
        let v = g.get();
        g.set(v + 1);
        v
    });
    let queue: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let queue_cb = queue.clone();
    let on_event: Rc<dyn Fn(AiEvent)> = Rc::new(move |evt| {
        queue_cb.borrow_mut().push_back(evt);
    });
    let base = crate::daemon_base::daemon_base();
    match post_ai_stream_to(&base, "/api/ai/stream", body_json, on_event) {
        Ok(handle) => {
            ACTIVE_DESIGN_MD.with(|slot| {
                *slot.borrow_mut() = Some(ActiveDesignMd { handle, generation });
            });
            start_generation_pump(inner.clone(), queue, generation);
        }
        Err(e) => {
            console_error(&format!("[design-md-generate] request failed: {e:?}"));
            set_generating(inner, false);
        }
    }
}

fn start_generation_pump<C: RepaintContext + 'static>(
    inner: InnerRc<C>,
    queue: EventQueue,
    generation: u64,
) {
    let markdown = Rc::new(RefCell::new(String::new()));
    let tick: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let still_active = ACTIVE_DESIGN_MD.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|active| active.generation == generation)
        });
        if !still_active {
            return false;
        }
        let mut terminal = None;
        loop {
            let evt = queue.borrow_mut().pop_front();
            let Some(evt) = evt else { break };
            match evt {
                AiEvent::Delta(text) => markdown.borrow_mut().push_str(&text),
                AiEvent::Thinking(_) => {}
                AiEvent::Done => {
                    terminal = Some(Ok(()));
                    break;
                }
                AiEvent::Error(error) => {
                    terminal = Some(Err(error));
                    break;
                }
            }
        }
        let Some(outcome) = terminal else {
            return true;
        };
        ACTIVE_DESIGN_MD.with(|slot| {
            let mut active = slot.borrow_mut();
            if active
                .as_ref()
                .is_some_and(|item| item.generation == generation)
            {
                *active = None;
            }
        });
        match outcome {
            Ok(()) => apply_generated_design_md(&inner, &markdown.borrow()),
            Err(error) => {
                console_error(&format!("[design-md-generate] {error}"));
                set_generating(&inner, false);
            }
        }
        false
    });
    crate::raf_pump::start(tick);
}

fn apply_generated_design_md<C: RepaintContext + 'static>(inner: &InnerRc<C>, markdown: &str) {
    let markdown = clean_ai_design_md_result(markdown);
    if markdown.is_empty() {
        console_error("[design-md-generate] returned empty output");
        set_generating(inner, false);
        return;
    }
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let spec = op_editor_core::parse_design_md(&markdown);
    let snap = b.host().editor_state().snapshot_for_history();
    let state = b.host_mut().editor_state_mut();
    state.doc.design_md = Some(spec);
    state.editor_ui.design_md_scroll.offset = 0.0;
    state.editor_ui.design_md_generating = false;
    state.history_push_past(snap);
    b.host_mut().mark_editor_state_dirty();
    let _ = b.repaint();
}

fn set_generating<C: RepaintContext + 'static>(inner: &InnerRc<C>, generating: bool) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    if b.host().editor_state().editor_ui.design_md_generating == generating {
        return;
    }
    b.host_mut()
        .editor_state_mut()
        .editor_ui
        .design_md_generating = generating;
    b.host_mut().mark_editor_state_dirty();
    let _ = b.repaint();
}

fn abort_active_design_md() -> bool {
    let active = ACTIVE_DESIGN_MD.with(|slot| slot.borrow_mut().take());
    if let Some(active) = active {
        active.handle.abort();
        true
    } else {
        false
    }
}

fn build_auto_generate_body(state: &EditorState) -> Option<String> {
    if state.active_children().is_empty() {
        return None;
    }
    state.chat.selected_model_entry()?;
    let (model, credential) = crate::web_ai_credentials::selected_target(state);
    let user = build_design_md_user_message(state);
    let body = serde_json::json!({
        "model": model,
        "skills": [],
        "user": user,
        "max_output_tokens": 8192u32,
        "thinking": "disabled",
        "effort": "high",
        "credential": credential,
    });
    serde_json::to_string(&body).ok()
}

fn build_design_md_user_message(state: &EditorState) -> String {
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
    let user_prompt = format!(
        "Analyze this PenNode design tree and generate a comprehensive design.md.\n\n\
         Project: {project}\n\n\
         Design tree JSON for the active page:\n{tree}\n\n\
         Design variables JSON:\n{vars}"
    );
    format!("{DESIGN_MD_SYSTEM_PROMPT}\n\n---\n\n{user_prompt}")
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        out.push_str("\n... [truncated]");
    }
    out
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

fn console_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(msg));
}

#[cfg(test)]
mod tests {
    use super::build_auto_generate_body;
    use op_editor_core::{BuiltinAgentKind, EditorState};

    #[test]
    fn design_md_proxy_body_carries_the_selected_request_scoped_credential() {
        let mut state = EditorState::starter();
        let selected_id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Private",
            "sk-design-md",
            "private-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        state.rebuild_chat_models();
        state.chat.selected_model = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.builtin_provider_id.as_deref() == Some(selected_id.as_str()))
            .unwrap();

        let body: serde_json::Value =
            serde_json::from_str(&build_auto_generate_body(&state).expect("request body")).unwrap();

        assert_eq!(body["model"], "private-model");
        assert_eq!(body["credential"]["api_key"], "sk-design-md");
    }
}
