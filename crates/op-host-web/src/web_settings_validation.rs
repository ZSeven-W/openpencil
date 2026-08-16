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
    /// A built-in agent id contains surrounding whitespace or is empty.
    AgentIdNormalized,
    /// A built-in agent's saved model list would be trimmed, truncated, or
    /// otherwise rewritten by the canonical multi-model representation.
    AgentModelsNormalized,
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
            SettingsValidationError::AgentIdNormalized => {
                write!(f, "built-in agent id would be normalized")
            }
            SettingsValidationError::AgentModelsNormalized => {
                write!(f, "built-in agent models would be normalized")
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
    settings_payload_with_legacy_model_card_migration(value, false)
}

/// Validate a v1 browser-settings snapshot while accepting the one legacy
/// representation that this version can upgrade without losing information:
/// one canonical single-model card per model for the same provider backend.
///
/// This stays separate from [`settings_payload`] so ordinary strict validation
/// does not silently start accepting duplicate provider records. Raw field and
/// schema checks still run before the migration, which keeps unknown or mixed
/// representations read-only.
pub(super) fn legacy_settings_payload(
    value: &serde_json::Value,
) -> Result<SettingsPayload, SettingsValidationError> {
    match settings_payload(value) {
        Ok(payload) => return Ok(payload),
        Err(SettingsValidationError::DuplicateAgents) => {}
        Err(error) => return Err(error),
    }
    settings_payload_with_legacy_model_card_migration(value, true)
}

fn settings_payload_with_legacy_model_card_migration(
    value: &serde_json::Value,
    migrate_legacy_model_cards: bool,
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
    let mut payload: SettingsPayload = serde_json::from_value(value.clone())
        .map_err(|_| SettingsValidationError::SettingsSchema)?;
    if payload.version != SETTINGS_VERSION {
        return Err(SettingsValidationError::SettingsVersion);
    }
    validate_general_semantics(&payload)?;
    if migrate_legacy_model_cards {
        if let Some(agents) = payload.builtin_agents.as_mut() {
            migrate_legacy_single_model_provider_cards(agents);
        }
    }
    validate_credential_semantics(
        payload.builtin_agents.as_deref().unwrap_or_default(),
        payload.image_gen_profiles.as_deref().unwrap_or_default(),
        payload.active_image_gen_profile_id.as_deref(),
        payload.openverse_oauth.as_ref(),
    )?;
    Ok(payload)
}

pub(super) struct ValidatedCredentialPayload {
    pub(super) payload: CredentialPayload,
    /// Older v2 writers represented one provider with multiple cards, one
    /// `model` per card. The current schema represents that same provider as
    /// one card with `models`, so a successful migration must be written back
    /// immediately instead of waiting for an unrelated settings edit.
    pub(super) migrated_legacy_single_model_cards: bool,
}

pub(super) fn credential_payload(
    value: &serde_json::Value,
) -> Result<ValidatedCredentialPayload, SettingsValidationError> {
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
    let mut payload: CredentialPayload = serde_json::from_value(value.clone())
        .map_err(|_| SettingsValidationError::CredentialSchema)?;
    if payload.version != CREDENTIAL_PAYLOAD_VERSION {
        return Err(SettingsValidationError::CredentialVersion);
    }
    let migrated_legacy_single_model_cards =
        migrate_legacy_single_model_provider_cards(&mut payload.builtin_agents);
    validate_credential_semantics(
        &payload.builtin_agents,
        &payload.image_gen_profiles,
        payload.active_image_gen_profile_id.as_deref(),
        payload.openverse_oauth.as_ref(),
    )?;
    Ok(ValidatedCredentialPayload {
        payload,
        migrated_legacy_single_model_cards,
    })
}

/// Upgrade representations written before built-in providers owned a model
/// list. A migration is deliberately narrow: every colliding card must carry
/// the same raw preset, be a canonical legacy single-model card, and name a
/// distinct model. Same-model duplicates, different presets, and mixed
/// legacy/current representations are left untouched so the ordinary duplicate
/// check keeps the raw snapshot read-only.
fn migrate_legacy_single_model_provider_cards(agents: &mut Vec<BuiltinAgentPayload>) -> bool {
    let mut consumed = vec![false; agents.len()];
    let mut migrated = false;
    let mut output = Vec::with_capacity(agents.len());

    for index in 0..agents.len() {
        if consumed[index] {
            continue;
        }
        let group = (index..agents.len())
            .filter(|candidate| {
                !consumed[*candidate]
                    && same_builtin_provider_backend(&agents[index], &agents[*candidate])
            })
            .collect::<Vec<_>>();
        for candidate in &group {
            consumed[*candidate] = true;
        }

        if group.len() <= 1 {
            output.push(agents[index].clone());
            continue;
        }

        let mut seen_ids = std::collections::BTreeSet::new();
        let ids_are_canonical_and_distinct = group.iter().all(|candidate| {
            let id = agents[*candidate].id.as_str();
            !id.is_empty() && id == id.trim() && seen_ids.insert(id)
        });
        let mut seen_models = std::collections::BTreeSet::new();
        let legacy_models = group
            .iter()
            .map(|candidate| legacy_single_model(&agents[*candidate]))
            .collect::<Option<Vec<_>>>();
        let can_migrate = ids_are_canonical_and_distinct
            && group
                .iter()
                .all(|candidate| agents[*candidate].enabled == agents[index].enabled)
            && legacy_models
                .as_ref()
                .is_some_and(|models| models.iter().all(|model| seen_models.insert(*model)));

        if can_migrate {
            let models = legacy_models
                .expect("migration eligibility established above")
                .into_iter()
                .map(str::to_string)
                .collect();
            let mut merged = agents[index].clone();
            merged.models = Some(models);
            output.push(merged);
            migrated = true;
        } else {
            output.extend(group.into_iter().map(|candidate| agents[candidate].clone()));
        }
    }

    if migrated {
        *agents = output;
    }
    migrated
}

fn legacy_single_model(payload: &BuiltinAgentPayload) -> Option<&str> {
    (payload.models.is_none()
        && !payload.model.is_empty()
        && op_editor_host_core::settings_payload::builtin_agent_payload_models_are_canonical(
            payload,
        ))
    .then_some(payload.model.as_str())
}

fn same_builtin_provider_backend(a: &BuiltinAgentPayload, b: &BuiltinAgentPayload) -> bool {
    a.preset == b.preset
        && a.kind == b.kind
        && a.api_key.trim() == b.api_key.trim()
        && a.base_url.trim().trim_end_matches('/') == b.base_url.trim().trim_end_matches('/')
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
            "models",
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
        .is_some_and(|enabled| !matches!(enabled.len(), 6..=8 | 11..=13))
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
    let mut agent_ids = std::collections::HashSet::with_capacity(builtin_agents.len());
    for payload in builtin_agents {
        if payload.id.is_empty() || payload.id != payload.id.trim() {
            return Err(SettingsValidationError::AgentIdNormalized);
        }
        if !agent_ids.insert(payload.id.as_str()) {
            return Err(SettingsValidationError::DuplicateAgents);
        }
        if !op_editor_host_core::settings_payload::builtin_agent_payload_models_are_canonical(
            payload,
        ) {
            return Err(SettingsValidationError::AgentModelsNormalized);
        }
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
    for index in 0..builtin_agents.len() {
        if builtin_agents[index + 1..]
            .iter()
            .any(|candidate| same_builtin_provider_backend(&builtin_agents[index], candidate))
        {
            return Err(SettingsValidationError::DuplicateAgents);
        }
    }
    if dedupe_builtin_agents(parsed_agents.clone()).len() != parsed_agents.len() {
        return Err(SettingsValidationError::DuplicateAgents);
    }

    for profile in image_profiles {
        if !matches!(
            profile.provider.as_str(),
            "openai" | "gemini" | "replicate" | "custom"
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
