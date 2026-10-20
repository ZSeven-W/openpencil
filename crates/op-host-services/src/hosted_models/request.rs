use serde::Deserialize;

/// A Hosted request has exactly one routing authority: an opaque session route.
/// Strict deserialization rejects duplicate and extra credential/target fields.
#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostedRequest {
    pub hosted_route_id: String,
    pub user: Option<String>,
    pub skills: Option<Vec<String>>,
    pub max_output_tokens: Option<u32>,
    pub thinking: Option<String>,
    pub effort: Option<String>,
    pub history: Option<serde_json::Value>,
    pub attachments: Option<serde_json::Value>,
    pub document: Option<serde_json::Value>,
    #[serde(rename = "editorMeta")]
    pub editor_meta: Option<serde_json::Value>,
    #[serde(rename = "selectedIds")]
    pub selected_ids: Option<serde_json::Value>,
    #[serde(rename = "activePageId")]
    pub active_page_id: Option<serde_json::Value>,
    pub agent_team_size: Option<u32>,
}

pub(crate) fn route_id(body: &str) -> Option<String> {
    if body.len() > 16 * 1024 * 1024 {
        return None;
    }
    let request: HostedRequest = serde_json::from_str(body).ok()?;
    let id = request.hosted_route_id;
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
    {
        return None;
    }
    Some(id)
}
