//! Lossless validation for browser-owned settings snapshots.
//!
//! Serde's default forward-compatible behavior ignores unknown object fields,
//! while the settings applicators also normalize or filter several values.
//! That is convenient for native best-effort loading, but unsafe for a browser
//! store: a later edit would rewrite the snapshot and could silently delete a
//! newer client's data or secret. Validate the raw value before applying it so
//! incompatible same-version snapshots can stay untouched and read-only.

use super::*;

pub(super) fn settings_payload(value: &serde_json::Value) -> Result<SettingsPayload, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "browser settings must be an object".to_string())?;
    validate_known_fields(
        object,
        &[
            "version",
            "theme",
            "locale",
            "mcp_port",
            "mcp_cli_enabled",
            "images_advanced_open",
            "openverse_oauth",
            "auto_update_enabled",
            "experimental_features_enabled",
            "builtin_agents",
            "image_gen_profiles",
            "active_image_gen_profile_id",
            "recent_files",
        ],
        "browser settings",
    )?;
    validate_nested_fields(object)?;
    let payload: SettingsPayload = serde_json::from_value(value.clone())
        .map_err(|_| "invalid browser settings schema".to_string())?;
    if payload.version != SETTINGS_VERSION {
        return Err("unsupported browser settings version".into());
    }
    validate_general_semantics(&payload)?;
    validate_credential_semantics(
        payload.builtin_agents.as_deref().unwrap_or_default(),
        payload.image_gen_profiles.as_deref().unwrap_or_default(),
        payload.active_image_gen_profile_id.as_deref(),
        payload.openverse_oauth.as_ref(),
    )?;
    Ok(payload)
}

pub(super) fn credential_payload(value: &serde_json::Value) -> Result<CredentialPayload, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "browser credentials must be an object".to_string())?;
    validate_known_fields(
        object,
        &[
            "version",
            "builtin_agents",
            "image_gen_profiles",
            "active_image_gen_profile_id",
            "openverse_oauth",
        ],
        "browser credentials",
    )?;
    validate_nested_fields(object)?;
    let payload: CredentialPayload = serde_json::from_value(value.clone())
        .map_err(|_| "invalid browser credential schema".to_string())?;
    if payload.version != CREDENTIAL_PAYLOAD_VERSION {
        return Err("unsupported browser credential version".into());
    }
    validate_credential_semantics(
        &payload.builtin_agents,
        &payload.image_gen_profiles,
        payload.active_image_gen_profile_id.as_deref(),
        payload.openverse_oauth.as_ref(),
    )?;
    Ok(payload)
}

fn validate_nested_fields(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    validate_optional_object_fields(
        object.get("openverse_oauth"),
        &["client_id", "client_secret"],
        "Openverse credentials",
    )?;
    validate_array_object_fields(
        object.get("builtin_agents"),
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
        object.get("image_gen_profiles"),
        &["id", "name", "provider", "api_key", "model", "base_url"],
        "image profile",
    )?;
    validate_array_object_fields(
        object.get("recent_files"),
        &["path", "modified_at"],
        "recent file",
    )
}

fn validate_optional_object_fields(
    value: Option<&serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    if let Some(value) = value {
        if value.is_null() {
            return Ok(());
        }
        let object = value
            .as_object()
            .ok_or_else(|| format!("invalid {context} schema"))?;
        validate_known_fields(object, allowed, context)?;
    }
    Ok(())
}

fn validate_array_object_fields(
    value: Option<&serde_json::Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let entries = value
        .as_array()
        .ok_or_else(|| format!("invalid {context} list"))?;
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or_else(|| format!("invalid {context} entry"))?;
        validate_known_fields(object, allowed, context)?;
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
        return Err(format!("unknown field in {context}"));
    }
    Ok(())
}

fn validate_general_semantics(payload: &SettingsPayload) -> Result<(), String> {
    if payload
        .theme
        .as_deref()
        .is_some_and(|theme| !matches!(theme, "dark" | "light"))
    {
        return Err("unknown browser theme".into());
    }
    if payload.locale.as_deref().is_some_and(|saved| {
        str_to_locale(saved).is_none_or(|locale| locale_to_str(locale) != saved)
    }) {
        return Err("unknown browser locale".into());
    }
    if payload.mcp_port.is_some_and(|port| port < 1024) {
        return Err("browser MCP port would be normalized".into());
    }
    if payload
        .mcp_cli_enabled
        .as_ref()
        .is_some_and(|enabled| enabled.len() != McpCli::ALL.len())
    {
        return Err("browser MCP CLI flags would be normalized".into());
    }
    if payload
        .recent_files
        .as_ref()
        .is_some_and(|recent| recent.len() > RECENT_FILE_CAP)
    {
        return Err("browser recent-file list would be truncated".into());
    }
    Ok(())
}

fn validate_credential_semantics(
    builtin_agents: &[BuiltinAgentPayload],
    image_profiles: &[ImageGenProfilePayload],
    active_image_profile: Option<&str>,
    openverse: Option<&OpenverseOAuthPayload>,
) -> Result<(), String> {
    let mut parsed_agents = Vec::with_capacity(builtin_agents.len());
    for payload in builtin_agents {
        if !matches!(payload.kind.as_str(), "anthropic" | "openai-compat") {
            return Err("unknown built-in agent kind".into());
        }
        let parsed = builtin_agent_from_payload(payload.clone())
            .ok_or_else(|| "unknown built-in agent kind".to_string())?;
        let saved = payload
            .preset
            .as_deref()
            .and_then(BuiltinAgentPresetKey::from_str)
            .ok_or_else(|| "missing or unknown built-in agent preset".to_string())?;
        if parsed.preset != saved {
            return Err("built-in agent preset would be normalized".into());
        }
        parsed_agents.push(parsed);
    }
    if dedupe_builtin_agents(parsed_agents.clone()).len() != parsed_agents.len() {
        return Err("duplicate built-in agents would be removed".into());
    }

    for profile in image_profiles {
        if !matches!(
            profile.provider.as_str(),
            "openai" | "gemini" | "replicate" | "custom"
        ) {
            return Err("unknown image generation provider".into());
        }
        image_gen_profile_from_payload(profile.clone())
            .ok_or_else(|| "unknown image generation provider".to_string())?;
    }
    if active_image_profile
        .is_some_and(|active| !image_profiles.iter().any(|profile| profile.id == active))
    {
        return Err("active image profile would be replaced".into());
    }
    if !image_profiles.is_empty() && active_image_profile.is_none() {
        return Err("active image profile would be selected implicitly".into());
    }
    if let Some(openverse) = openverse {
        if openverse.client_id.trim() != openverse.client_id
            || openverse.client_secret.trim() != openverse.client_secret
            || (openverse.client_id.is_empty() && openverse.client_secret.is_empty())
        {
            return Err("Openverse credentials would be normalized".into());
        }
    }
    Ok(())
}
