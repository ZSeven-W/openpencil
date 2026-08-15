use op_codegen::ai::types::PendingRequest;
use op_editor_core::AgentProvider;

/// Build the JSON request body for the proxy. Skill names are forwarded; the
/// daemon composes the expanded system prompt used by the desktop host.
pub(super) fn build_body_json(
    req: &PendingRequest,
    provider: Option<AgentProvider>,
    builtin_provider_id: Option<&str>,
    model: &str,
    credential: Option<&serde_json::Value>,
) -> String {
    let skills_json = req
        .skills
        .iter()
        .map(|skill| serde_json::Value::String((*skill).to_string()))
        .collect::<Vec<_>>();
    let body = serde_json::json!({
        "provider": provider.map(AgentProvider::wire_id),
        "builtinProviderId": builtin_provider_id,
        "model": model,
        "skills": skills_json,
        "user": req.user_message,
        "max_output_tokens": req.max_output_tokens,
        "thinking": req.thinking.as_str(),
        "effort": req.effort.as_str(),
        "credential": credential,
    });
    serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string())
}
