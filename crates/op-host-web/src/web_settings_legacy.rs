//! Legacy browser-settings inspection and credential-field sanitization.

use super::{SettingsPayload, SETTINGS_VERSION};

pub(super) struct PreparedSettings {
    pub(super) payload: Option<SettingsPayload>,
    pub(super) sanitized_raw: Option<String>,
    pub(super) unsupported_version: bool,
    pub(super) had_legacy_credentials: bool,
}

pub(super) fn prepare(raw: &str) -> PreparedSettings {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return PreparedSettings {
            payload: None,
            sanitized_raw: None,
            unsupported_version: true,
            had_legacy_credentials: false,
        };
    };
    if value.get("version").and_then(serde_json::Value::as_u64) != Some(u64::from(SETTINGS_VERSION))
    {
        return PreparedSettings {
            payload: None,
            sanitized_raw: None,
            unsupported_version: true,
            had_legacy_credentials: false,
        };
    }
    let Some(object) = value.as_object_mut() else {
        return PreparedSettings {
            payload: None,
            sanitized_raw: None,
            unsupported_version: true,
            had_legacy_credentials: false,
        };
    };

    let had_legacy_credentials = [
        "openverse_oauth",
        "builtin_agents",
        "image_gen_profiles",
        "active_image_gen_profile_id",
    ]
    .iter()
    .any(|key| object.contains_key(*key));

    // ACP and the legacy CLI connection cache are forbidden in the web store.
    // Remove only those fields before validating the rest, so an incompatible
    // same-version snapshot can retain every unrelated future field verbatim.
    let mut removed_forbidden = false;
    for key in ["acp_agents", "connected"] {
        removed_forbidden |= object.remove(key).is_some();
    }

    let payload = match super::validation::settings_payload(&value) {
        Ok(payload) => payload,
        Err(_) => {
            let sanitized_raw = removed_forbidden
                .then(|| serde_json::to_string(&value).ok())
                .flatten();
            return PreparedSettings {
                payload: None,
                sanitized_raw,
                unsupported_version: true,
                had_legacy_credentials,
            };
        }
    };

    // Supported v1 snapshots may still carry the pre-split credential fields.
    // They can now be migrated to the dedicated credential key losslessly.
    let object = value
        .as_object_mut()
        .expect("browser settings object was checked above");
    let mut sanitized = removed_forbidden;
    for key in [
        "openverse_oauth",
        "builtin_agents",
        "image_gen_profiles",
        "active_image_gen_profile_id",
    ] {
        sanitized |= object.remove(key).is_some();
    }

    if !sanitized {
        return PreparedSettings {
            payload: Some(payload),
            sanitized_raw: None,
            unsupported_version: false,
            had_legacy_credentials,
        };
    }

    let sanitized_raw = serde_json::to_string(&value).ok();
    PreparedSettings {
        payload: Some(payload),
        sanitized_raw,
        unsupported_version: false,
        had_legacy_credentials,
    }
}
