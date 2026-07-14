use super::*;

pub(super) fn validate_payload_fields(raw: &serde_json::Value) -> Result<(), String> {
    let Some(root) = raw.as_object() else {
        return Ok(());
    };
    validate_known_fields(
        root,
        &[
            "version",
            "theme",
            "locale",
            "mcp_port",
            "mcp_cli_enabled",
            "images_advanced_open",
            "openverse_oauth",
            "openverse_credential_owner",
            "auto_update_enabled",
            "experimental_features_enabled",
            "connected",
            "builtin_agents",
            "acp_agents",
            "image_gen_profiles",
            "active_image_gen_profile_id",
            "recent_files",
        ],
        "root",
    )?;
    validate_optional_object_fields(
        root.get("openverse_oauth"),
        &["client_id", "client_secret"],
        "Openverse OAuth",
    )?;
    validate_array_object_fields(
        root.get("builtin_agents"),
        &[
            "id",
            "preset",
            "display_name",
            "kind",
            "api_key",
            "model",
            "base_url",
            "enabled",
        ],
        "built-in agent",
    )?;
    validate_array_object_fields(
        root.get("acp_agents"),
        &[
            "id",
            "display_name",
            "connection_type",
            "command",
            "args",
            "env",
            "url",
            "enabled",
        ],
        "ACP agent",
    )?;
    validate_array_object_fields(
        root.get("image_gen_profiles"),
        &["id", "name", "provider", "api_key", "model", "base_url"],
        "image profile",
    )?;
    validate_array_object_fields(
        root.get("recent_files"),
        &["path", "modified_at"],
        "recent file",
    )
}

fn validate_optional_object_fields(
    value: Option<&serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if let Some(object) = value.and_then(serde_json::Value::as_object) {
        validate_known_fields(object, allowed, context)?;
    }
    Ok(())
}

fn validate_array_object_fields(
    value: Option<&serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if let Some(entries) = value.and_then(serde_json::Value::as_array) {
        for entry in entries {
            if let Some(object) = entry.as_object() {
                validate_known_fields(object, allowed, context)?;
            }
        }
    }
    Ok(())
}

fn validate_known_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if object
        .keys()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(format!("unknown settings field in {context}"));
    }
    Ok(())
}

pub(super) fn validate_lossless_payload(payload: &SettingsPayload) -> Result<(), String> {
    let lossy = || "settings file cannot be loaded losslessly".to_string();
    validate_credential_entries(payload)?;
    if payload
        .theme
        .as_deref()
        .is_some_and(|theme| !matches!(theme, "dark" | "light"))
    {
        return Err(lossy());
    }
    if payload.locale.as_deref().is_some_and(|saved| {
        str_to_locale(saved).is_none_or(|locale| locale_to_str(locale) != saved)
    }) {
        return Err(lossy());
    }
    if payload.mcp_port.is_some_and(|port| port < 1024)
        || payload
            .recent_files
            .as_ref()
            .is_some_and(|recent| recent.len() > RECENT_FILE_CAP)
    {
        return Err(lossy());
    }
    if payload.builtin_agents.as_ref().is_some_and(|agents| {
        agents.iter().any(|agent| {
            agent
                .preset
                .as_deref()
                .is_some_and(|preset| BuiltinAgentPresetKey::from_str(preset).is_none())
        })
    }) {
        return Err(lossy());
    }
    if payload
        .active_image_gen_profile_id
        .as_deref()
        .is_some_and(|active| {
            !payload
                .image_gen_profiles
                .as_ref()
                .is_some_and(|profiles| profiles.iter().any(|profile| profile.id == active))
        })
    {
        return Err(lossy());
    }
    if payload
        .image_gen_profiles
        .as_ref()
        .is_some_and(|profiles| !profiles.is_empty())
        && payload.active_image_gen_profile_id.is_none()
    {
        return Err(lossy());
    }
    if payload.openverse_oauth.as_ref().is_some_and(|oauth| {
        oauth.client_id.trim() != oauth.client_id
            || oauth.client_secret.trim() != oauth.client_secret
            || (oauth.client_id.is_empty() && oauth.client_secret.is_empty())
    }) {
        return Err(lossy());
    }
    Ok(())
}

fn validate_credential_entries(payload: &SettingsPayload) -> Result<(), String> {
    fn invalid() -> String {
        "unsupported settings credential entry".to_string()
    }
    fn lossy() -> String {
        "settings file cannot be loaded losslessly".to_string()
    }
    if let Some(agents) = payload.builtin_agents.as_ref() {
        for agent in agents {
            if !matches!(agent.kind.as_str(), "anthropic" | "openai-compat") {
                if builtin_agent_from_payload(agent.clone()).is_some() {
                    return Err(lossy());
                }
                return Err(invalid());
            }
            let parsed = builtin_agent_from_payload(agent.clone()).ok_or_else(invalid)?;
            let saved = agent
                .preset
                .as_deref()
                .and_then(BuiltinAgentPresetKey::from_str)
                .ok_or_else(lossy)?;
            if parsed.preset != saved {
                return Err(lossy());
            }
        }
    }
    if payload.acp_agents.as_ref().is_some_and(|agents| {
        agents
            .iter()
            .cloned()
            .any(|agent| acp_agent_from_payload(agent).is_none())
    }) {
        return Err(invalid());
    }
    if payload.image_gen_profiles.as_ref().is_some_and(|profiles| {
        profiles
            .iter()
            .cloned()
            .any(|profile| image_gen_profile_from_payload(profile).is_none())
    }) {
        return Err(invalid());
    }
    Ok(())
}
