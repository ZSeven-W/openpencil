use std::time::Duration;

use futures::StreamExt;
use jian_ops_schema::DesignMdSpec;
use op_editor_core::EditorState;
use op_orchestrator::{AbortFlag, CallRequest, LlmChunk, LlmClient};

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
- Capture the current canvas style for future screens; do not redesign the requested next screen inside design.md.
- State that follow-on named app pages should be generated as a separate sibling/root screen beside the existing screen, not appended below it.
- Output ONLY the markdown document, starting with "# Design System:".
- NO preamble, NO commentary, NO tool calls, NO code fences around the output.
- Do NOT use <tool_call> tags or any tool invocations. Just output the markdown text directly."##;

const DESIGN_MD_MAX_TREE_CHARS: usize = 24_000;
const DESIGN_MD_MAX_VAR_CHARS: usize = 6_000;
const DESIGN_MD_TIMEOUT: Duration = Duration::from_secs(90);
const DESIGN_MD_NO_TEXT_TIMEOUT: Duration = Duration::from_secs(25);
const DESIGN_MD_FIRST_TEXT_TIMEOUT: Duration = Duration::from_secs(45);

pub fn build_design_md_user_prompt(state: &EditorState, user_request: &str) -> String {
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
        "Analyze this existing PenNode canvas and generate design.md for style reuse.\n\n\
         Follow-on user request that needs this design system:\n{user_request}\n\n\
         Continuity requirement:\n\
         - Extract the current app framework, palette, typography, component rhythm, radius, and spacing.\n\
         - Future named app pages must be separate sibling/root screens beside the existing screen, not as content appended below it.\n\
         - Preserve the existing header/search/navigation/card framework while changing page-specific content.\n\n\
         Project: {project}\n\n\
         Design tree JSON for the active page:\n{tree}\n\n\
         Design variables JSON:\n{vars}"
    )
}

pub async fn generate_design_md_spec(
    llm: &dyn LlmClient,
    state: &EditorState,
    user_request: &str,
    model: Option<String>,
    provider: Option<String>,
    abort: &AbortFlag,
) -> Result<DesignMdSpec, String> {
    let req = CallRequest {
        system_prompt: DESIGN_MD_SYSTEM_PROMPT.to_string(),
        user_prompt: build_design_md_user_prompt(state, user_request),
        model,
        provider,
        timeout: DESIGN_MD_TIMEOUT,
        abort: abort.clone(),
        no_text_timeout: Some(DESIGN_MD_NO_TEXT_TIMEOUT),
        first_text_timeout: Some(DESIGN_MD_FIRST_TEXT_TIMEOUT),
    };

    let mut stream = llm.call(req);
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(LlmChunk::Text(text)) => out.push_str(&text),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(err) => return Err(err.message),
        }
    }

    let markdown = clean_ai_design_md_result(&out);
    if markdown.is_empty() {
        return Err("design.md generation returned empty output".into());
    }
    let spec = op_editor_core::parse_design_md(&markdown);
    if !spec.raw.trim_start().starts_with("# Design System:") {
        return Err("design.md generation did not return a Design System document".into());
    }
    Ok(spec)
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

#[cfg(test)]
mod tests {
    use futures::stream;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::EditorState;
    use op_orchestrator::{AbortFlag, CallRequest, LlmChunk, LlmClient, LlmError};

    use super::*;

    fn frame(id: &str, name: &str) -> PenNode {
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": id,
            "name": name,
            "x": 0.0,
            "y": 0.0,
            "width": 375.0,
            "height": 812.0,
            "children": []
        }))
        .expect("valid frame")
    }

    fn state_with_home_frame() -> EditorState {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(frame("home", "Food App Home"));
        state
    }

    #[test]
    fn design_md_prompt_describes_current_canvas_and_follow_on_rule() {
        let state = state_with_home_frame();
        let prompt = build_design_md_user_prompt(&state, "继续画出发现页");

        assert!(prompt.contains("Food App Home"));
        assert!(prompt.contains("继续画出发现页"));
        assert!(prompt.contains("sibling/root screen"));
        assert!(prompt.contains("not as content appended below"));
    }

    struct ScriptedLlm;

    impl LlmClient for ScriptedLlm {
        fn call(
            &self,
            _req: CallRequest,
        ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
            Box::pin(stream::iter(vec![Ok(LlmChunk::Text(
                "```markdown\n# Design System: Food App\n\n## 1. Visual Theme & Atmosphere\nWarm, compact mobile ordering UI.\n\n## 2. Color Palette & Roles\n- **Flame Orange** (#FF5A1F) — Primary action\n\n## 5. Layout Principles\nUse a sibling/root screen beside the existing app page.\n```"
                    .to_string(),
            ))]))
        }
    }

    #[tokio::test]
    async fn llm_markdown_is_cleaned_parsed_and_returned_as_design_md() {
        let state = state_with_home_frame();
        let abort = AbortFlag::new();

        let spec = generate_design_md_spec(
            &ScriptedLlm,
            &state,
            "继续画出发现页",
            Some("model-a".into()),
            Some("provider-a".into()),
            &abort,
        )
        .await
        .expect("parsed design.md");

        assert_eq!(spec.project_name.as_deref(), Some("Food App"));
        assert!(
            spec.raw.starts_with("# Design System: Food App"),
            "raw markdown is kept for prompt injection"
        );
        assert!(spec
            .color_palette
            .as_ref()
            .is_some_and(|colors| colors.iter().any(|c| c.hex == "#FF5A1F")));
    }
}
