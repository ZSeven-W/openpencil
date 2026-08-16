use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use op_editor_core::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState,
    ImageGenProfile, ImageGenProvider, ImageTestStatus,
};
use serde::Deserialize;

pub use crate::web_credentials_error::WebCredentialError;

/// Every fallible step of this module fails with [`WebCredentialError`]. The
/// one exception is the `String`-pinned
/// [`validate_web_provider_base_url`] wrapper — see the error module's docs.
type Result<T> = std::result::Result<T, WebCredentialError>;

pub const MAX_CREDENTIAL_BODY_BYTES: usize = 256 * 1024;

const MAX_TEXT_BYTES: usize = 512;
const MAX_URL_BYTES: usize = 4 * 1024;
const MAX_CREDENTIAL_BYTES: usize = 16 * 1024;
const MAX_LOCAL_ID_BYTES: usize = 128;
const MAX_ENTRIES: usize = 64;
const MAX_MODELS_PER_BUILTIN: usize = 64;
const MAX_TOTAL_ENTRIES: usize = 256;
const PAYLOAD_VERSION: u32 = 2;
const WEB_CREDENTIAL_PREFIX: &str = "web-credential:";
const WEB_CREDENTIAL_OWNER: &str = "browser";
pub const WEB_AI_ENDPOINT_ALLOWLIST_ENV: &str = "OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialPayload {
    version: u32,
    #[serde(default)]
    builtin_agents: Vec<BuiltinAgentPayload>,
    #[serde(default)]
    image_gen_profiles: Vec<ImageGenProfilePayload>,
    active_image_gen_profile_id: Option<String>,
    openverse_oauth: Option<OpenverseOAuthPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuiltinAgentPayload {
    id: String,
    #[serde(default)]
    preset: Option<String>,
    display_name: String,
    kind: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    models: Option<Vec<String>>,
    base_url: String,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageGenProfilePayload {
    id: String,
    name: String,
    provider: String,
    #[serde(default)]
    api_key: String,
    model: String,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenverseOAuthPayload {
    client_id: String,
    client_secret: String,
}

/// Validate and atomically merge a browser credential snapshot into daemon
/// settings. Unrelated daemon-managed entries remain intact.
pub fn apply_json(state: &mut EditorState, body: &str) -> Result<()> {
    if body.len() > MAX_CREDENTIAL_BODY_BYTES {
        return Err(WebCredentialError::PayloadTooLarge);
    }
    let payload: CredentialPayload =
        serde_json::from_str(body).map_err(|_| WebCredentialError::InvalidPayload)?;
    if payload.version != PAYLOAD_VERSION {
        return Err(WebCredentialError::UnsupportedPayloadVersion);
    }
    if payload.builtin_agents.len() > MAX_ENTRIES || payload.image_gen_profiles.len() > MAX_ENTRIES
    {
        return Err(WebCredentialError::TooManyEntries);
    }
    let mut next = state.editor_ui.agent_settings.clone();
    validate_and_merge(&mut next, payload)?;
    state.editor_ui.agent_settings = next;
    state.rebuild_chat_models();
    Ok(())
}

pub(crate) fn parse_transient_builtin(value: &serde_json::Value) -> Option<BuiltinAgentConfig> {
    let payload: BuiltinAgentPayload = serde_json::from_value(value.clone()).ok()?;
    validate_required_id(&payload.id, "built-in agent id").ok()?;
    let agent = parse_builtin_agent(payload).ok()?;
    if !agent.enabled
        || agent.api_key.trim().is_empty()
        || agent.first_model().is_none()
        || agent.base_url.trim().is_empty()
    {
        return None;
    }
    Some(agent)
}

/// Discovery variant of [`parse_transient_builtin`]. Listing models is what
/// lets a user make the first model choice, so this path intentionally accepts
/// an empty model while retaining the same identity/key/endpoint checks.
pub(crate) fn parse_transient_builtin_for_discovery(
    value: &serde_json::Value,
) -> Option<BuiltinAgentConfig> {
    let payload: BuiltinAgentPayload = serde_json::from_value(value.clone()).ok()?;
    validate_required_id(&payload.id, "built-in agent id").ok()?;
    let agent = parse_builtin_agent(payload).ok()?;
    agent.discovery_ready().then_some(agent)
}

/// Whether a browser-supplied provider endpoint may be dialed. Any public
/// HTTPS origin passes; private/loopback/metadata targets need an explicit
/// `OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST` entry. Connect-time DNS pinning
/// (`chat_builtin_http`) closes the rebinding gap this hostname check leaves.
pub(crate) fn public_demo_transient_endpoint_allowed(agent: &BuiltinAgentConfig) -> bool {
    validate_web_provider_base_url_with_env_allowlist(agent.base_url.trim()).is_ok()
}

/// Validate a browser-controlled provider endpoint independently of whether
/// browser credentials are persisted. Web AI request paths repeat this check,
/// so enabling server persistence never doubles as an SSRF opt-in.
pub(crate) fn validate_web_provider_base_url(base_url: &str) -> Result<reqwest::Url> {
    let allowlist = std::env::var(WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
    validate_web_provider_base_url_with_allowlist(base_url, allowlist.as_deref())
}

fn validate_web_provider_base_url_with_allowlist(
    base_url: &str,
    allowlist: Option<&str>,
) -> Result<reqwest::Url> {
    let url =
        reqwest::Url::parse(base_url.trim()).map_err(|_| WebCredentialError::EndpointInvalid)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(WebCredentialError::EndpointNotAllowed);
    }
    if endpoint_is_explicitly_allowlisted(&url, allowlist) {
        return Ok(url);
    }
    if url.scheme() != "https" {
        return Err(WebCredentialError::EndpointRequiresHttps);
    }

    let host = url
        .host_str()
        .expect("host presence checked above")
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.parse::<IpAddr>().is_ok_and(is_restricted_ip) || is_restricted_hostname(&host) {
        return Err(WebCredentialError::EndpointNotAllowed);
    }
    Ok(url)
}

/// Typed twin of [`validate_web_provider_base_url`] for this module's own
/// merge path, which already reports [`WebCredentialError`] and must not
/// round-trip through the wrapper's `String`.
fn validate_web_provider_base_url_with_env_allowlist(base_url: &str) -> Result<reqwest::Url> {
    let allowlist = std::env::var(WEB_AI_ENDPOINT_ALLOWLIST_ENV).ok();
    validate_web_provider_base_url_with_allowlist(base_url, allowlist.as_deref())
}

/// Whether a raw base URL is an exact scheme/host/port match for an
/// `OPENPENCIL_WEB_AI_ENDPOINT_ALLOWLIST` entry. Unparseable URLs are not.
pub(crate) fn base_url_is_explicitly_allowlisted(base_url: &str, allowlist: Option<&str>) -> bool {
    reqwest::Url::parse(base_url.trim())
        .is_ok_and(|url| endpoint_is_explicitly_allowlisted(&url, allowlist))
}

fn endpoint_is_explicitly_allowlisted(url: &reqwest::Url, allowlist: Option<&str>) -> bool {
    allowlist
        .into_iter()
        .flat_map(|value| value.split(','))
        .any(|entry| {
            let Ok(allowed) = reqwest::Url::parse(entry.trim()) else {
                return false;
            };
            matches!(allowed.scheme(), "http" | "https")
                && allowed.host_str().is_some()
                && allowed.username().is_empty()
                && allowed.password().is_none()
                && allowed.query().is_none()
                && allowed.fragment().is_none()
                && url.scheme() == allowed.scheme()
                && url
                    .host_str()
                    .zip(allowed.host_str())
                    .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
                && url.port_or_known_default() == allowed.port_or_known_default()
        })
}

fn is_restricted_hostname(host: &str) -> bool {
    host.is_empty()
        || !host.contains('.')
        || [
            "localhost",
            ".localhost",
            ".local",
            ".internal",
            ".home",
            ".lan",
            ".test",
            ".invalid",
        ]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(suffix))
        || matches!(
            host,
            "metadata.google.internal"
                | "metadata.google"
                | "instance-data"
                | "instance-data.ec2.internal"
        )
}

pub(crate) fn is_restricted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_restricted_ipv4(ip),
        IpAddr::V6(ip) => is_restricted_ipv6(ip),
    }
}

fn is_restricted_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, d] = ip.octets();
    a == 0
        || a == 10
        || (a == 100 && (64..=127).contains(&b))
        || a == 127
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224
        || [a, b, c, d] == [168, 63, 129, 16]
}

fn is_restricted_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
        let [a, b] = segments[6].to_be_bytes();
        let [c, d] = segments[7].to_be_bytes();
        return is_restricted_ipv4(Ipv4Addr::new(a, b, c, d));
    }
    (segments[0] & 0xe000) != 0x2000
        || segments[0] == 0x2002
        || segments[0] == 0x3fff
        || (segments[0] == 0x2001
            && (matches!(segments[1], 0 | 2 | 0x0db8)
                || (segments[1] & 0xfff0) == 0x0010
                || (segments[1] & 0xfff0) == 0x0020))
}

pub(crate) fn browser_owns_builtin_agent(agent: &BuiltinAgentConfig) -> bool {
    agent.id.starts_with(&owner_prefix("builtin"))
}

pub(crate) fn remove_browser_owned_credentials(state: &mut EditorState) -> bool {
    let builtin_prefix = owner_prefix("builtin");
    let image_prefix = owner_prefix("image");
    let settings = &mut state.editor_ui.agent_settings;
    let had_browser_credentials = settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id.starts_with(&builtin_prefix))
        || settings
            .image_gen_profiles
            .iter()
            .any(|profile| profile.id.starts_with(&image_prefix))
        || settings.openverse_credential_owner.as_deref() == Some(WEB_CREDENTIAL_OWNER);
    settings
        .builtin_agents
        .retain(|agent| !agent.id.starts_with(&builtin_prefix));
    settings
        .image_gen_profiles
        .retain(|profile| !profile.id.starts_with(&image_prefix));
    if settings
        .active_image_gen_profile_id
        .as_deref()
        .is_some_and(|id| id.starts_with(&image_prefix))
    {
        settings.active_image_gen_profile_id = settings
            .image_gen_profiles
            .first()
            .map(|profile| profile.id.clone());
    }
    if settings.openverse_credential_owner.as_deref() == Some(WEB_CREDENTIAL_OWNER) {
        settings.openverse_client_id.clear();
        settings.openverse_client_secret.clear();
        settings.openverse_credential_owner = None;
    }
    settings.next_builtin_agent_id = next_numeric_id(
        settings
            .builtin_agents
            .iter()
            .map(|agent| agent.id.as_str()),
        "builtin-",
    );
    settings.next_image_gen_profile_id = next_numeric_id(
        settings
            .image_gen_profiles
            .iter()
            .map(|profile| profile.id.as_str()),
        "igp-",
    );
    state.rebuild_chat_models();
    had_browser_credentials
}

fn validate_and_merge(settings: &mut AgentSettings, payload: CredentialPayload) -> Result<()> {
    let builtin_prefix = owner_prefix("builtin");
    let image_prefix = owner_prefix("image");

    let mut builtin_ids = HashSet::with_capacity(payload.builtin_agents.len());
    let mut builtins = Vec::with_capacity(payload.builtin_agents.len());
    for payload in payload.builtin_agents {
        validate_local_id(&payload.id, "built-in agent id")?;
        let local_id = payload.id.clone();
        if !builtin_ids.insert(local_id.clone()) {
            return Err(WebCredentialError::DuplicateBuiltinAgentIds);
        }
        let mut agent = parse_builtin_agent(payload)?;
        validate_web_provider_base_url_with_env_allowlist(&agent.base_url)?;
        if !public_demo_transient_endpoint_allowed(&agent) {
            return Err(WebCredentialError::EndpointNotExplicitlyAllowed);
        }
        agent.id = scoped_id(&builtin_prefix, &local_id);
        builtins.push(agent);
    }

    let mut image_ids = HashSet::with_capacity(payload.image_gen_profiles.len());
    let mut image_profiles = Vec::with_capacity(payload.image_gen_profiles.len());
    for payload in payload.image_gen_profiles {
        validate_local_id(&payload.id, "image profile id")?;
        let local_id = payload.id.clone();
        if !image_ids.insert(local_id.clone()) {
            return Err(WebCredentialError::DuplicateImageProfileIds);
        }
        let mut profile = parse_image_profile(payload)?;
        if let Some(base_url) = profile
            .base_url
            .as_deref()
            .filter(|base_url| !base_url.trim().is_empty())
        {
            validate_web_provider_base_url_with_env_allowlist(base_url)?;
        }
        profile.id = scoped_id(&image_prefix, &local_id);
        image_profiles.push(profile);
    }

    let active_image_gen_profile_id = payload
        .active_image_gen_profile_id
        .as_deref()
        .map(|id| {
            validate_local_id(id, "active image profile id")?;
            if !image_ids.contains(id) {
                return Err(WebCredentialError::ActiveImageProfileNotInSnapshot);
            }
            Ok(scoped_id(&image_prefix, id))
        })
        .transpose()?;
    if let Some(oauth) = payload.openverse_oauth.as_ref() {
        validate_len(
            &oauth.client_id,
            MAX_CREDENTIAL_BYTES,
            "Openverse client id",
        )?;
        validate_len(
            &oauth.client_secret,
            MAX_CREDENTIAL_BYTES,
            "Openverse client secret",
        )?;
        if oauth.client_id.trim().is_empty() && oauth.client_secret.trim().is_empty() {
            return Err(WebCredentialError::OpenverseCredentialsBothEmpty);
        }
    }

    let browser_can_manage_openverse = settings.openverse_credential_owner.as_deref()
        == Some(WEB_CREDENTIAL_OWNER)
        || (settings.openverse_credential_owner.is_none()
            && settings.openverse_client_id.trim().is_empty()
            && settings.openverse_client_secret.trim().is_empty());

    settings
        .builtin_agents
        .retain(|agent| !agent.id.starts_with(&builtin_prefix));
    settings
        .image_gen_profiles
        .retain(|profile| !profile.id.starts_with(&image_prefix));
    settings.builtin_agents.extend(builtins);
    settings.image_gen_profiles.extend(image_profiles);
    if settings.builtin_agents.len() > MAX_TOTAL_ENTRIES
        || settings.image_gen_profiles.len() > MAX_TOTAL_ENTRIES
    {
        return Err(WebCredentialError::StoreTooManyEntries);
    }

    if let Some(id) = active_image_gen_profile_id {
        settings.active_image_gen_profile_id = Some(id);
    } else if settings
        .active_image_gen_profile_id
        .as_deref()
        .is_some_and(|id| id.starts_with(&image_prefix))
    {
        settings.active_image_gen_profile_id = settings
            .image_gen_profiles
            .first()
            .map(|profile| profile.id.clone());
    }
    if browser_can_manage_openverse {
        if let Some(oauth) = payload.openverse_oauth {
            settings.openverse_client_id = oauth.client_id;
            settings.openverse_client_secret = oauth.client_secret;
            settings.openverse_credential_owner = Some(WEB_CREDENTIAL_OWNER.into());
        } else if settings.openverse_credential_owner.as_deref() == Some(WEB_CREDENTIAL_OWNER) {
            settings.openverse_client_id.clear();
            settings.openverse_client_secret.clear();
            settings.openverse_credential_owner = None;
        }
    }

    settings.next_builtin_agent_id = next_numeric_id(
        settings
            .builtin_agents
            .iter()
            .map(|agent| agent.id.as_str()),
        "builtin-",
    );
    settings.next_image_gen_profile_id = next_numeric_id(
        settings
            .image_gen_profiles
            .iter()
            .map(|profile| profile.id.as_str()),
        "igp-",
    );
    Ok(())
}

fn parse_builtin_agent(payload: BuiltinAgentPayload) -> Result<BuiltinAgentConfig> {
    validate_len(
        &payload.display_name,
        MAX_TEXT_BYTES,
        "built-in display name",
    )?;
    validate_len(&payload.model, MAX_TEXT_BYTES, "built-in model")?;
    if payload
        .models
        .as_ref()
        .is_some_and(|models| models.len() > MAX_MODELS_PER_BUILTIN)
    {
        return Err(WebCredentialError::TooManyEntries);
    }
    if let Some(models) = payload.models.as_ref() {
        for model in models {
            validate_len(model, MAX_TEXT_BYTES, "built-in model")?;
        }
    }
    validate_len(&payload.base_url, MAX_URL_BYTES, "built-in base URL")?;
    validate_len(&payload.api_key, MAX_CREDENTIAL_BYTES, "built-in API key")?;
    if let Some(preset) = payload.preset.as_deref() {
        validate_len(preset, MAX_TEXT_BYTES, "built-in preset")?;
    }

    let kind = match payload.kind.as_str() {
        "anthropic" => BuiltinAgentKind::Anthropic,
        "openai" | "openai-compat" | "openai_compat" => BuiltinAgentKind::OpenAiCompat,
        _ => return Err(WebCredentialError::UnsupportedBuiltinAgentKind),
    };
    let models = op_editor_core::normalize_builtin_models(
        payload
            .models
            .clone()
            .unwrap_or_else(|| vec![payload.model.clone()]),
    );
    let preset_model = models.first().map(String::as_str).unwrap_or_default();
    let preset = payload
        .preset
        .as_deref()
        .and_then(BuiltinAgentPresetKey::from_str)
        .unwrap_or_else(|| {
            op_editor_core::infer_builtin_agent_preset(kind, &payload.base_url, preset_model)
        });

    Ok(BuiltinAgentConfig {
        id: payload.id,
        preset,
        display_name: payload.display_name,
        kind,
        api_key: payload.api_key,
        models,
        base_url: payload.base_url,
        enabled: payload.enabled,
    })
}

fn parse_image_profile(payload: ImageGenProfilePayload) -> Result<ImageGenProfile> {
    validate_len(&payload.name, MAX_TEXT_BYTES, "image profile name")?;
    validate_len(&payload.model, MAX_TEXT_BYTES, "image profile model")?;
    validate_len(
        &payload.api_key,
        MAX_CREDENTIAL_BYTES,
        "image profile API key",
    )?;
    if let Some(base_url) = payload.base_url.as_deref() {
        validate_len(base_url, MAX_URL_BYTES, "image profile base URL")?;
    }

    let provider = match payload.provider.as_str() {
        "openai" => ImageGenProvider::OpenAi,
        "gemini" => ImageGenProvider::Gemini,
        "replicate" => ImageGenProvider::Replicate,
        "custom" => ImageGenProvider::Custom,
        _ => return Err(WebCredentialError::UnsupportedImageGenProvider),
    };

    Ok(ImageGenProfile {
        id: payload.id,
        name: payload.name,
        provider,
        api_key: payload.api_key,
        model: payload.model,
        base_url: payload.base_url,
        test_status: ImageTestStatus::Idle,
    })
}

fn validate_required_id(value: &str, field: &str) -> Result<()> {
    validate_len(value, MAX_TEXT_BYTES, field)?;
    if value.trim().is_empty() {
        return Err(WebCredentialError::RequiredIdEmpty {
            field: field.to_string(),
        });
    }
    Ok(())
}

fn validate_local_id(value: &str, field: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_LOCAL_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(WebCredentialError::InvalidLocalId {
            field: field.to_string(),
        });
    }
    Ok(())
}

fn owner_prefix(category: &str) -> String {
    format!("{WEB_CREDENTIAL_PREFIX}{category}:")
}

fn scoped_id(prefix: &str, local_id: &str) -> String {
    format!("{prefix}{local_id}")
}

fn validate_len(value: &str, max: usize, field: &str) -> Result<()> {
    if value.len() > max {
        return Err(WebCredentialError::FieldTooLong {
            field: field.to_string(),
        });
    }
    Ok(())
}

fn next_numeric_id<'a>(ids: impl Iterator<Item = &'a str>, prefix: &str) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

#[cfg(test)]
#[path = "web_credentials_tests.rs"]
mod tests;
