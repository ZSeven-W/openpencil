use op_editor_core::EditorState;

use crate::ai_proxy::AiStreamRequest;

pub(super) fn selected_model_id(req: &AiStreamRequest, snapshot: &EditorState) -> Option<String> {
    let model = req.model.trim();
    if model.is_empty() || model == "default" {
        return None;
    }
    if let Some(structured) = model.strip_prefix("builtin:") {
        if let Some(builtin_id) = req.builtin_provider_id.as_deref() {
            let selected = structured
                .strip_prefix(builtin_id)?
                .strip_prefix(':')?
                .trim();
            return (!selected.is_empty()).then(|| selected.to_string());
        }

        // Rolling-upgrade compatibility for a tab loaded before the separate
        // provider id field existed. Match complete generated values rather
        // than splitting at `:`: provider ids and model ids may both contain
        // colons. Ambiguous joins are rejected by the provider resolver too.
        let mut selected = None;
        for agent in &snapshot.editor_ui.agent_settings.builtin_agents {
            if req
                .provider
                .is_some_and(|provider| provider != agent.kind.model_provider())
            {
                continue;
            }
            for saved_model in &agent.models {
                if format!("builtin:{}:{saved_model}", agent.id) != model {
                    continue;
                }
                if selected.is_some() {
                    return None;
                }
                selected = Some(saved_model.clone());
            }
        }
        return selected;
    }
    Some(model.to_string())
}
