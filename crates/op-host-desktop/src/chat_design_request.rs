use op_editor_core::EditorState;
use op_orchestrator::DesignRequest;

pub(crate) fn build_design_request(prompt: String, state: &EditorState) -> DesignRequest {
    DesignRequest {
        prompt,
        // The chosen chat agent decides its own model; the orchestrator
        // only passes through `req.model` when it explicitly overrides per
        // sub-call, which it does not today.
        model: None,
        provider: None,
        design_md: state.doc.design_md.clone(),
        append_context: None,
        concurrency: 1,
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
        let state = EditorState::new();

        let req = build_design_request("draw a mobile settings screen".into(), &state);

        assert!(req.validation_enabled);
        assert!(!req.visual_ref_enabled);
        assert_eq!(req.concurrency, 1);
        assert_eq!(req.model, None);
    }
}
