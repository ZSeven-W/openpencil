//! Storage orchestration for browser settings and credential snapshots.

use super::*;
use crate::web_storage::{
    clear_storage_failure, report_storage_failure, report_unsupported_credential_version,
    storage_get, storage_set_checked,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CredentialLoad {
    pub(crate) loaded: bool,
    pub(crate) write_pending: bool,
    pub(super) unsupported_version: bool,
    /// Theme carried by the account blob that was just read.
    ///
    /// Only the device-theme migration consumes it — the first run after the
    /// split adopts it so a user's existing choice is not reset. Once the
    /// device key exists this is ignored on every later load.
    payload_theme: Option<ThemeMode>,
    settings_write_disabled: bool,
    credential_write_disabled: bool,
    pending_credential_json: Option<String>,
    pending_settings_json: Option<String>,
}

impl CredentialLoad {
    /// See the field docs: the migration source, and nothing else.
    pub(crate) fn payload_theme(&self) -> Option<ThemeMode> {
        self.payload_theme
    }

    pub(crate) fn initial_settings_fingerprint(&self, state: &EditorState) -> Option<Fingerprint> {
        (!self.settings_write_disabled).then(|| fingerprint(state))
    }

    pub(crate) fn initial_fingerprint(&self, state: &EditorState) -> CredentialFingerprint {
        let mut fingerprint = credential_fingerprint(state);
        if self.write_pending {
            fingerprint.write_pending = true;
            fingerprint.pending_credential_json = self.pending_credential_json.clone();
            fingerprint.pending_settings_json = self.pending_settings_json.clone();
        }
        fingerprint.write_disabled = self.credential_write_disabled;
        fingerprint
    }
}

pub(crate) fn load_into(state: &mut EditorState) -> CredentialLoad {
    let settings_raw = storage_get(&super::settings_storage_key());
    let credential_raw = storage_get(&super::credential_storage_key());
    let load = load_into_with(
        state,
        settings_raw.as_deref(),
        credential_raw.as_deref(),
        storage_set_checked,
    );
    if load.unsupported_version {
        report_unsupported_credential_version();
    }
    if load.write_pending {
        report_storage_failure();
    }
    load
}

pub(super) fn load_into_with<F>(
    state: &mut EditorState,
    settings_raw: Option<&str>,
    credential_raw: Option<&str>,
    mut persist: F,
) -> CredentialLoad
where
    F: FnMut(&str, &str) -> bool,
{
    let payload_theme = super::payload_theme_of(settings_raw);
    let stored = apply_stored_snapshots(state, settings_raw, credential_raw);
    let unsupported_settings_version = stored.unsupported_settings_version;
    match stored.source {
        StoredCredentialSource::Separate => {
            let sanitized = stored.sanitized_settings_json;
            let settings_saved = sanitized
                .as_deref()
                .is_none_or(|json| persist(&super::settings_storage_key(), json));
            CredentialLoad {
                payload_theme,
                loaded: true,
                write_pending: !settings_saved,
                unsupported_version: unsupported_settings_version,
                settings_write_disabled: unsupported_settings_version,
                credential_write_disabled: false,
                pending_credential_json: None,
                pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
            }
        }
        StoredCredentialSource::SanitizedSeparate => {
            let sanitized = stored.sanitized_settings_json;
            let canonical_credential = credentials_json(state);
            let credential = stored.sanitized_credential_json.or(canonical_credential);
            let credential_saved = credential
                .as_deref()
                .is_some_and(|json| persist(&super::credential_storage_key(), json));
            let settings_saved = credential_saved
                && sanitized
                    .as_deref()
                    .is_none_or(|json| persist(&super::settings_storage_key(), json));
            let write_pending = !credential_saved || !settings_saved;
            CredentialLoad {
                payload_theme,
                loaded: true,
                write_pending,
                unsupported_version: unsupported_settings_version,
                settings_write_disabled: unsupported_settings_version,
                credential_write_disabled: false,
                pending_credential_json: (!credential_saved).then_some(credential).flatten(),
                pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
            }
        }
        StoredCredentialSource::Legacy | StoredCredentialSource::InvalidSeparate => {
            if unsupported_settings_version {
                let sanitized = stored.sanitized_settings_json;
                let settings_saved = sanitized
                    .as_deref()
                    .is_none_or(|json| persist(&super::settings_storage_key(), json));
                return CredentialLoad {
                    payload_theme,
                    loaded: false,
                    write_pending: !settings_saved,
                    unsupported_version: true,
                    settings_write_disabled: true,
                    credential_write_disabled: true,
                    pending_credential_json: None,
                    pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
                };
            }
            let sanitized = stored.sanitized_settings_json;
            let credential = credentials_json(state);
            let credential_saved = credential
                .as_deref()
                .is_some_and(|json| persist(&super::credential_storage_key(), json));
            let settings_saved = credential_saved
                && sanitized
                    .as_deref()
                    .is_none_or(|json| persist(&super::settings_storage_key(), json));
            let write_pending = !credential_saved || !settings_saved;
            CredentialLoad {
                payload_theme,
                // Both legacy credentials and a healed invalid separate
                // snapshot are authoritative mount-time snapshots. Queue the
                // latter's empty replacement too, so an opt-in daemon drops
                // any stale browser-owned server copy immediately.
                loaded: true,
                write_pending,
                unsupported_version: unsupported_settings_version,
                settings_write_disabled: unsupported_settings_version,
                credential_write_disabled: false,
                pending_credential_json: (!credential_saved).then_some(credential).flatten(),
                pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
            }
        }
        StoredCredentialSource::UnsupportedSeparate
        | StoredCredentialSource::UnsupportedSanitizedSeparate => {
            let credential = stored.sanitized_credential_json;
            let credential_saved = credential
                .as_deref()
                .is_none_or(|json| persist(&super::credential_storage_key(), json));
            let sanitized = stored.sanitized_settings_json;
            let settings_saved = credential_saved
                && sanitized
                    .as_deref()
                    .is_none_or(|json| persist(&super::settings_storage_key(), json));
            CredentialLoad {
                payload_theme,
                loaded: false,
                write_pending: !credential_saved || !settings_saved,
                unsupported_version: true,
                settings_write_disabled: unsupported_settings_version,
                credential_write_disabled: true,
                pending_credential_json: (!credential_saved).then_some(credential).flatten(),
                pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
            }
        }
        StoredCredentialSource::None => {
            // No stored credentials for THIS partition. Clearing is the point:
            // after an account switch the in-memory state still holds the
            // previous account's API keys, and an empty partition must mean
            // "no keys", not "keep whatever was there". At mount the state is
            // fresh, so this is a no-op for the local daemon.
            clear_local_credentials(state);
            let sanitized = stored.sanitized_settings_json;
            let settings_saved = sanitized
                .as_deref()
                .is_none_or(|json| persist(&super::settings_storage_key(), json));
            CredentialLoad {
                payload_theme,
                loaded: false,
                write_pending: !settings_saved,
                unsupported_version: unsupported_settings_version,
                settings_write_disabled: unsupported_settings_version,
                credential_write_disabled: unsupported_settings_version,
                pending_credential_json: None,
                pending_settings_json: (!settings_saved).then_some(sanitized).flatten(),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoredCredentialSource {
    None,
    Separate,
    SanitizedSeparate,
    Legacy,
    InvalidSeparate,
    UnsupportedSeparate,
    UnsupportedSanitizedSeparate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoredLoad {
    pub(super) source: StoredCredentialSource,
    pub(super) sanitize_legacy_settings: bool,
    pub(super) unsupported_settings_version: bool,
    sanitized_settings_json: Option<String>,
    sanitized_credential_json: Option<String>,
}

pub(super) fn apply_stored_snapshots(
    state: &mut EditorState,
    settings_raw: Option<&str>,
    credential_raw: Option<&str>,
) -> StoredLoad {
    let mut sanitize_legacy_settings = false;
    let mut migrate_legacy_credentials = false;
    let mut sanitized_settings_json = None;
    let mut unsupported_settings_version = false;
    if let Some(raw) = settings_raw {
        let prepared = legacy::prepare(raw);
        unsupported_settings_version = prepared.unsupported_version;
        sanitize_legacy_settings = prepared.sanitized_raw.is_some();
        migrate_legacy_credentials = prepared.had_legacy_credentials && prepared.payload.is_some();
        sanitized_settings_json = prepared.sanitized_raw;
        if let Some(payload) = prepared.payload {
            if payload.version == SETTINGS_VERSION {
                apply_payload(state, payload);
            }
        }
    }
    if let Some(raw) = credential_raw {
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(raw) else {
            clear_local_credentials(state);
            return StoredLoad {
                source: StoredCredentialSource::InvalidSeparate,
                sanitize_legacy_settings,
                unsupported_settings_version,
                sanitized_settings_json,
                sanitized_credential_json: None,
            };
        };
        let version = value
            .as_object()
            .and_then(|object| object.get("version"))
            .and_then(serde_json::Value::as_u64);
        if version != Some(u64::from(CREDENTIAL_PAYLOAD_VERSION)) {
            clear_local_credentials(state);
            return StoredLoad {
                source: StoredCredentialSource::UnsupportedSeparate,
                sanitize_legacy_settings,
                unsupported_settings_version,
                sanitized_settings_json,
                sanitized_credential_json: None,
            };
        }
        let stripped_credential_json = strip_legacy_acp_credentials(&mut value);
        let sanitized_credential_json;
        let source = match validation::credential_payload(&value) {
            Ok(validated) => {
                let migrated = validated.migrated_legacy_single_model_cards;
                apply_credential_payload(state, validated.payload);
                // A known v2 one-card-per-model snapshot is safe to migrate,
                // but it must be made durable immediately. Prefer the fully
                // canonical writer over the raw ACP-only scrub when both
                // migrations apply to the same snapshot.
                let canonical_credential_json = migrated
                    .then(|| credentials_json(state))
                    .flatten()
                    .or(stripped_credential_json);
                sanitized_credential_json = canonical_credential_json;
                if sanitized_credential_json.is_some() {
                    StoredCredentialSource::SanitizedSeparate
                } else {
                    StoredCredentialSource::Separate
                }
            }
            Err(_) => {
                clear_local_credentials(state);
                sanitized_credential_json = stripped_credential_json;
                if sanitized_credential_json.is_some() {
                    StoredCredentialSource::UnsupportedSanitizedSeparate
                } else {
                    StoredCredentialSource::UnsupportedSeparate
                }
            }
        };
        return StoredLoad {
            source,
            sanitize_legacy_settings,
            unsupported_settings_version,
            sanitized_settings_json,
            sanitized_credential_json,
        };
    }
    StoredLoad {
        source: if migrate_legacy_credentials {
            StoredCredentialSource::Legacy
        } else {
            StoredCredentialSource::None
        },
        sanitize_legacy_settings,
        unsupported_settings_version,
        sanitized_settings_json,
        sanitized_credential_json: None,
    }
}

fn strip_legacy_acp_credentials(value: &mut serde_json::Value) -> Option<String> {
    let object = value.as_object_mut()?;
    object.remove("acp_agents")?;
    serde_json::to_string(value).ok()
}

fn clear_local_credentials(state: &mut EditorState) {
    apply_credential_payload(
        state,
        CredentialPayload {
            version: CREDENTIAL_PAYLOAD_VERSION,
            builtin_agents: Vec::new(),
            image_gen_profiles: Vec::new(),
            active_image_gen_profile_id: None,
            openverse_oauth: None,
        },
    );
}

pub(crate) fn save_if_changed(state: &EditorState, before: &mut Fingerprint) -> bool {
    let saved = save_if_changed_with(state, before, |json| {
        storage_set_checked(&super::settings_storage_key(), json)
    });
    if saved {
        clear_storage_failure();
    } else if fingerprint(state) != *before {
        report_storage_failure();
    }
    saved
}

pub(super) fn save_if_changed_with<F>(
    state: &EditorState,
    before: &mut Fingerprint,
    persist: F,
) -> bool
where
    F: FnOnce(&str) -> bool,
{
    let next = fingerprint(state);
    if next == *before {
        return false;
    }
    let Ok(json) = serde_json::to_string(&to_payload(state)) else {
        return false;
    };
    if !persist(&json) {
        return false;
    }
    *before = next;
    true
}

pub(crate) fn save_credentials_if_changed(
    state: &EditorState,
    before: &mut CredentialFingerprint,
) -> Option<String> {
    let had_pending_write = before.write_pending;
    let saved = save_credentials_if_changed_with(state, before, storage_set_checked);
    if saved.is_some() || (had_pending_write && !before.write_pending) {
        clear_storage_failure();
    } else if before.write_pending
        || (!before.write_disabled && credential_fingerprint(state) != *before)
    {
        report_storage_failure();
    }
    saved
}

pub(crate) fn credential_migration_pending(before: &CredentialFingerprint) -> bool {
    before.write_pending
}

pub(super) fn save_credentials_if_changed_with<F>(
    state: &EditorState,
    before: &mut CredentialFingerprint,
    mut persist: F,
) -> Option<String>
where
    F: FnMut(&str, &str) -> bool,
{
    if before.write_pending && !retry_pending_writes(before, &mut persist) {
        return None;
    }
    if before.write_disabled {
        return None;
    }
    let next = credential_fingerprint(state);
    if next == *before {
        return None;
    }
    let json = credentials_json(state)?;
    if !persist(&super::credential_storage_key(), &json) {
        return None;
    }
    *before = next;
    Some(json)
}

fn retry_pending_writes<F>(before: &mut CredentialFingerprint, persist: &mut F) -> bool
where
    F: FnMut(&str, &str) -> bool,
{
    if let Some(json) = before.pending_credential_json.clone() {
        if !persist(&super::credential_storage_key(), &json) {
            return false;
        }
        before.pending_credential_json = None;
    }
    if let Some(json) = before.pending_settings_json.clone() {
        if !persist(&super::settings_storage_key(), &json) {
            return false;
        }
        before.pending_settings_json = None;
    }
    before.write_pending =
        before.pending_credential_json.is_some() || before.pending_settings_json.is_some();
    true
}
