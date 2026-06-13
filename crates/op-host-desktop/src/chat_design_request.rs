use op_editor_core::EditorState;
use op_orchestrator::{AppendContext, DesignRequest};

pub(crate) fn build_design_request(
    prompt: String,
    state: &EditorState,
    append_context: Option<AppendContext>,
) -> DesignRequest {
    DesignRequest {
        prompt,
        // The chosen chat agent decides its own model; the orchestrator
        // only passes through `req.model` when it explicitly overrides per
        // sub-call, which it does not today.
        model: None,
        provider: None,
        design_md: state.doc.design_md.clone(),
        // Detected by `chat_intent::detect_append_intent` when the
        // prompt asks to extend the existing page (GAP #33). TS wires
        // this from the agent tool executor (agent-tool-executor.ts:234);
        // the shell's design pipeline is that path's equivalent.
        append_context,
        concurrency: state.chat.agent_team_size,
        validation_enabled: true,
        visual_ref_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::EditorState;

    #[test]
    fn built_in_design_requests_enable_validation() {
        let mut state = EditorState::new();
        state.chat.agent_team_size = 4;

        let req = build_design_request("draw a mobile settings screen".into(), &state, None);

        assert!(req.validation_enabled);
        assert!(!req.visual_ref_enabled);
        assert_eq!(req.concurrency, 4);
        assert_eq!(req.model, None);
        assert!(req.append_context.is_none());
    }

    #[test]
    fn append_context_rides_the_request() {
        let state = EditorState::new();
        let ctx = AppendContext {
            target_parent_id: "content-root".into(),
            target_width: 390.0,
            existing_section_labels: vec!["Hero".into()],
            is_mobile: true,
        };

        let req = build_design_request("continue the page".into(), &state, Some(ctx));

        let ctx = req.append_context.expect("append context attached");
        assert_eq!(ctx.target_parent_id, "content-root");
        assert_eq!(ctx.existing_section_labels, vec!["Hero".to_string()]);
    }
}
