//! Lossless validation for browser-owned settings snapshots.
//!
//! Serde's default forward-compatible behavior ignores unknown object fields,
//! while the settings applicators also normalize or filter several values.
//! That is convenient for native best-effort loading, but unsafe for a browser
//! store: a later edit would rewrite the snapshot and could silently delete a
//! newer client's data or secret. Validate the raw value before applying it so
//! incompatible same-version snapshots can stay untouched and read-only.

use super::*;

/// A browser settings / credential snapshot that must not be rewritten.
///
/// Every `Display` arm reproduces the ad-hoc `String` message it replaced byte
/// for byte, so the reasons the store logs (and refuses to overwrite on) are
/// unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SettingsValidationError {
    /// The raw snapshot is not valid JSON. Only the test-only `apply_json`
    /// entry points parse raw text; the production path is handed a value.
    #[cfg(test)]
    Json(String),
    /// The settings snapshot is not a JSON object.
    SettingsNotObject,
    /// The settings snapshot does not deserialize into `SettingsPayload`.
    SettingsSchema,
    /// The settings snapshot carries a different `version`.
    SettingsVersion,
    /// The credential snapshot is not a JSON object.
    CredentialsNotObject,
    /// The credential snapshot does not deserialize into `CredentialPayload`.
    CredentialSchema,
    /// The credential snapshot carries a different `version`.
    CredentialVersion,
    /// A nested object field is not an object.
    NestedSchema(&'static str),
    /// A nested array field is not an array.
    NestedList(&'static str),
    /// An entry inside a nested array field is not an object.
    NestedEntry(&'static str),
    /// A field name outside the known set for that context.
    UnknownField(&'static str),
    /// `theme` is neither `dark` nor `light`.
    UnknownTheme,
    /// `locale` does not round-trip through the locale table.
    UnknownLocale,
    /// `mcp_port` is below the clamp floor the applicator enforces.
    McpPortNormalized,
    /// `mcp_cli_enabled` has a length no migration accepts.
    McpCliLayout,
    /// `recent_files` is longer than the cap the applicator enforces.
    RecentFilesTruncated,
    /// A built-in agent declares an unsupported `kind`.
    UnknownAgentKind,
    /// A built-in agent declares no recognizable `preset`.
    UnknownAgentPreset,
    /// A built-in agent's stored preset differs from the reparsed one.
    AgentPresetNormalized,
    /// Two built-in agents would collapse into one.
    DuplicateAgents,
    /// An image profile declares an unsupported `provider`.
    UnknownImageProvider,
    /// `active_image_gen_profile_id` names no stored profile.
    ActiveImageProfileReplaced,
    /// Profiles exist but no active one is named.
    ActiveImageProfileImplicit,
    /// Openverse credentials are padded or wholly empty.
    OpenverseNormalized,
}

impl std::fmt::Display for SettingsValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(test)]
            SettingsValidationError::Json(error) => write!(f, "{error}"),
            SettingsValidationError::SettingsNotObject => {
                write!(f, "browser settings must be an object")
            }
            SettingsValidationError::SettingsSchema => {
                write!(f, "invalid browser settings schema")
            }
            SettingsValidationError::SettingsVersion => {
                write!(f, "unsupported browser settings version")
            }
            SettingsValidationError::CredentialsNotObject => {
                write!(f, "browser credentials must be an object")
            }
            SettingsValidationError::CredentialSchema => {
                write!(f, "invalid browser credential schema")
            }
            SettingsValidationError::CredentialVersion => {
                write!(f, "unsupported browser credential version")
            }
            SettingsValidationError::NestedSchema(context) => {
                write!(f, "invalid {context} schema")
            }
            SettingsValidationError::NestedList(context) => write!(f, "invalid {context} list"),
            SettingsValidationError::NestedEntry(context) => write!(f, "invalid {context} entry"),
            SettingsValidationError::UnknownField(context) => {
                write!(f, "unknown field in {context}")
            }
            SettingsValidationError::UnknownTheme => write!(f, "unknown browser theme"),
            SettingsValidationError::UnknownLocale => write!(f, "unknown browser locale"),
            SettingsValidationError::McpPortNormalized => {
                write!(f, "browser MCP port would be normalized")
            }
            SettingsValidationError::McpCliLayout => {
                write!(f, "unsupported browser MCP CLI flag layout")
            }
            SettingsValidationError::RecentFilesTruncated => {
                write!(f, "browser recent-file list would be truncated")
            }
            SettingsValidationError::UnknownAgentKind => {
                write!(f, "unknown built-in agent kind")
            }
            SettingsValidationError::UnknownAgentPreset => {
                write!(f, "missing or unknown built-in agent preset")
            }
            SettingsValidationError::AgentPresetNormalized => {
                write!(f, "built-in agent preset would be normalized")
            }
            SettingsValidationError::DuplicateAgents => {
                write!(f, "duplicate built-in agents would be removed")
            }
            SettingsValidationError::UnknownImageProvider => {
                write!(f, "unknown image generation provider")
            }
            SettingsValidationError::ActiveImageProfileReplaced => {
                write!(f, "active image profile would be replaced")
            }
            SettingsValidationError::ActiveImageProfileImplicit => {
                write!(f, "active image profile would be selected implicitly")
            }
            SettingsValidationError::OpenverseNormalized => {
                write!(f, "Openverse credentials would be normalized")
            }
        }
    }
}

impl std::error::Error for SettingsValidationError {}

impl From<SettingsValidationError> for String {
    fn from(error: SettingsValidationError) -> String {
        error.to_string()
    }
}

pub(super) fn settings_payload(
    value: &serde_json::Value,
) -> Result<SettingsPayload, SettingsValidationError> {
    let object = value
        .as_object()
        .ok_or(SettingsValidationError::SettingsNotObject)?;
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
        .map_err(|_| SettingsValidationError::SettingsSchema)?;
    if payload.version != SETTINGS_VERSION {
        return Err(SettingsValidationError::SettingsVersion);
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

pub(super) fn credential_payload(
    value: &serde_json::Value,
) -> Result<CredentialPayload, SettingsValidationError> {
    let object = value
        .as_object()
        .ok_or(SettingsValidationError::CredentialsNotObject)?;
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
        .map_err(|_| SettingsValidationError::CredentialSchema)?;
    if payload.version != CREDENTIAL_PAYLOAD_VERSION {
        return Err(SettingsValidationError::CredentialVersion);
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
) -> Result<(), SettingsValidationError> {
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
    context: &'static str,
) -> Result<(), SettingsValidationError> {
    if let Some(value) = value {
        if value.is_null() {
            return Ok(());
        }
        let object = value
            .as_object()
            .ok_or(SettingsValidationError::NestedSchema(context))?;
        validate_known_fields(object, allowed, context)?;
    }
    Ok(())
}

fn validate_array_object_fields(
    value: Option<&serde_json::Value>,
    allowed: &[&str],
    context: &'static str,
) -> Result<(), SettingsValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let entries = value
        .as_array()
        .ok_or(SettingsValidationError::NestedList(context))?;
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or(SettingsValidationError::NestedEntry(context))?;
        validate_known_fields(object, allowed, context)?;
    }
    Ok(())
}

fn validate_known_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    context: &'static str,
) -> Result<(), SettingsValidationError> {
    if object
        .keys()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(SettingsValidationError::UnknownField(context));
    }
    Ok(())
}

fn validate_general_semantics(payload: &SettingsPayload) -> Result<(), SettingsValidationError> {
    if payload
        .theme
        .as_deref()
        .is_some_and(|theme| !matches!(theme, "dark" | "light"))
    {
        return Err(SettingsValidationError::UnknownTheme);
    }
    if payload.locale.as_deref().is_some_and(|saved| {
        str_to_locale(saved).is_none_or(|locale| locale_to_str(locale) != saved)
    }) {
        return Err(SettingsValidationError::UnknownLocale);
    }
    if payload.mcp_port.is_some_and(|port| port < 1024) {
        return Err(SettingsValidationError::McpPortNormalized);
    }
    if payload
        .mcp_cli_enabled
        .as_ref()
        .is_some_and(|enabled| !matches!(enabled.len(), 6..=8 | 11 | 12))
    {
        return Err(SettingsValidationError::McpCliLayout);
    }
    if payload
        .recent_files
        .as_ref()
        .is_some_and(|recent| recent.len() > RECENT_FILE_CAP)
    {
        return Err(SettingsValidationError::RecentFilesTruncated);
    }
    Ok(())
}

fn validate_credential_semantics(
    builtin_agents: &[BuiltinAgentPayload],
    image_profiles: &[ImageGenProfilePayload],
    active_image_profile: Option<&str>,
    openverse: Option<&OpenverseOAuthPayload>,
) -> Result<(), SettingsValidationError> {
    let mut parsed_agents = Vec::with_capacity(builtin_agents.len());
    for payload in builtin_agents {
        if !matches!(payload.kind.as_str(), "anthropic" | "openai-compat") {
            return Err(SettingsValidationError::UnknownAgentKind);
        }
        let parsed = builtin_agent_from_payload(payload.clone())
            .ok_or(SettingsValidationError::UnknownAgentKind)?;
        let saved = payload
            .preset
            .as_deref()
            .and_then(BuiltinAgentPresetKey::from_str)
            .ok_or(SettingsValidationError::UnknownAgentPreset)?;
        if parsed.preset != saved {
            return Err(SettingsValidationError::AgentPresetNormalized);
        }
        parsed_agents.push(parsed);
    }
    if dedupe_builtin_agents(parsed_agents.clone()).len() != parsed_agents.len() {
        return Err(SettingsValidationError::DuplicateAgents);
    }

    for profile in image_profiles {
        if !matches!(
            profile.provider.as_str(),
            "openai" | "gemini" | "replicate" | "atlas" | "custom"
        ) {
            return Err(SettingsValidationError::UnknownImageProvider);
        }
        image_gen_profile_from_payload(profile.clone())
            .ok_or(SettingsValidationError::UnknownImageProvider)?;
    }
    if active_image_profile
        .is_some_and(|active| !image_profiles.iter().any(|profile| profile.id == active))
    {
        return Err(SettingsValidationError::ActiveImageProfileReplaced);
    }
    if !image_profiles.is_empty() && active_image_profile.is_none() {
        return Err(SettingsValidationError::ActiveImageProfileImplicit);
    }
    if let Some(openverse) = openverse {
        if openverse.client_id.trim() != openverse.client_id
            || openverse.client_secret.trim() != openverse.client_secret
            || (openverse.client_id.is_empty() && openverse.client_secret.is_empty())
        {
            return Err(SettingsValidationError::OpenverseNormalized);
        }
    }
    Ok(())
}
