use std::time::Duration;

use futures::StreamExt;
use jian_ops_schema::DesignMdSpec;
use op_ai::design_md::{
    clean_ai_design_md_result, design_md_system_prompt_with_extra_rules, truncate_chars,
    DESIGN_MD_MAX_TREE_CHARS, DESIGN_MD_MAX_VAR_CHARS,
};
use op_editor_core::EditorState;
use op_orchestrator::{AbortFlag, CallRequest, LlmChunk, LlmClient};

pub use crate::design_md_llm_error::DesignMdError;

/// Orchestrator-specific additions to the shared design.md system prompt
/// (`op_ai::design_md::DESIGN_MD_SYSTEM_PROMPT`): the generated brief
/// must capture the current canvas for reuse and route follow-on named
/// pages to sibling screens instead of appending below.
const DESIGN_MD_EXTRA_RULES: &[&str] = &[
    "- Capture the current canvas style for future screens; do not redesign the requested next screen inside design.md.",
    "- State that follow-on named app pages should be generated as a separate sibling/root screen beside the existing screen, not appended below it.",
];

pub(crate) fn design_md_system_prompt() -> String {
    design_md_system_prompt_with_extra_rules(DESIGN_MD_EXTRA_RULES)
}
pub(crate) const DESIGN_MD_TIMEOUT: Duration = Duration::from_secs(90);
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
) -> Result<DesignMdSpec, DesignMdError> {
    let req = CallRequest {
        system_prompt: design_md_system_prompt(),
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
            // `op_orchestrator::LlmError` is not owned by this pass; carry
            // its message verbatim.
            Err(err) => return Err(DesignMdError::Llm(err.message)),
        }
    }

    parse_design_md_text(&out)
}

/// Clean and parse the same design.md text accepted by the normal generation
/// path. Keeping this as one helper makes screenshot enrichment byte-identical
/// to the existing design.md extraction behaviour.
pub(crate) fn parse_design_md_text(raw: &str) -> Result<DesignMdSpec, DesignMdError> {
    let markdown = clean_ai_design_md_result(raw);
    if markdown.is_empty() {
        return Err(DesignMdError::EmptyOutput);
    }
    let spec = op_editor_core::parse_design_md(&markdown);
    if !spec.raw.trim_start().starts_with("# Design System:") {
        return Err(DesignMdError::NotADesignSystemDocument);
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use jian_ops_schema::node::PenNode;
    use op_editor_core::EditorState;
    use op_orchestrator::AbortFlag;

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
    fn system_prompt_keeps_orchestrator_rules_before_the_output_rules() {
        let prompt = design_md_system_prompt();
        let roles = prompt
            .find("- Explain functional roles for every design element.")
            .expect("shared rule");
        let capture = prompt
            .find("- Capture the current canvas style for future screens")
            .expect("continuity rule");
        let sibling = prompt
            .find("- State that follow-on named app pages")
            .expect("sibling-screen rule");
        let output_only = prompt
            .find("- Output ONLY the markdown document")
            .expect("output rule");
        assert!(roles < capture && capture < sibling && sibling < output_only);
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

    #[tokio::test]
    async fn llm_markdown_is_cleaned_parsed_and_returned_as_design_md() {
        let state = state_with_home_frame();
        let abort = AbortFlag::new();

        let spec = generate_design_md_spec(
            &crate::test_support::ScriptedLlm,
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
