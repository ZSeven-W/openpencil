//! Shared settings-persistence payload shapes + conversions.
//!
//! Single source of truth for the serde payload structs and the
//! `*_to_payload` / `*_from_payload` conversions that both settings
//! stores use: the desktop/daemon `settings.json`
//! (`op-host-services::settings_io`) and the browser localStorage
//! snapshots (`op-host-web::web_settings`). Field names and formats are
//! wire format — settings files on disk / in localStorage must
//! round-trip unchanged, so any new field lands here once and both
//! hosts pick it up together instead of silently dropping it on one
//! side.
//!
//! This module is transport-free and wasm32-clean (op-editor-core +
//! serde only).

use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, ImageGenProfile, ImageGenProvider,
    ThemeMode,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecentFilePayload {
    pub path: String,
    pub modified_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuiltinAgentPayload {
    pub id: String,
    #[serde(default)]
    pub preset: Option<String>,
    pub display_name: String,
    pub kind: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenProfilePayload {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenverseOAuthPayload {
    pub client_id: String,
    pub client_secret: String,
}

pub fn builtin_agent_to_payload(agent: &BuiltinAgentConfig) -> BuiltinAgentPayload {
    BuiltinAgentPayload {
        id: agent.id.clone(),
        preset: Some(agent.preset.as_str().into()),
        display_name: agent.display_name.clone(),
        kind: match agent.kind {
            BuiltinAgentKind::Anthropic => "anthropic",
            BuiltinAgentKind::OpenAiCompat => "openai-compat",
        }
        .into(),
        api_key: agent.api_key.clone(),
        model: agent.model.clone(),
        base_url: agent.base_url.clone(),
        enabled: agent.enabled,
    }
}

pub fn builtin_agent_from_payload(payload: BuiltinAgentPayload) -> Option<BuiltinAgentConfig> {
    let kind = match payload.kind.as_str() {
        "anthropic" => BuiltinAgentKind::Anthropic,
        "openai" | "openai-compat" | "openai_compat" => BuiltinAgentKind::OpenAiCompat,
        _ => return None,
    };
    Some(BuiltinAgentConfig {
        id: payload.id,
        preset: payload
            .preset
            .as_deref()
            .and_then(BuiltinAgentPresetKey::from_str)
            .map(|saved| {
                op_editor_core::normalize_builtin_agent_preset(
                    saved,
                    kind,
                    &payload.base_url,
                    &payload.model,
                )
            })
            .unwrap_or_else(|| {
                op_editor_core::infer_builtin_agent_preset(kind, &payload.base_url, &payload.model)
            }),
        display_name: payload.display_name,
        kind,
        api_key: payload.api_key,
        model: payload.model,
        base_url: payload.base_url,
        enabled: payload.enabled,
    })
}

pub fn image_gen_profile_to_payload(profile: &ImageGenProfile) -> ImageGenProfilePayload {
    ImageGenProfilePayload {
        id: profile.id.clone(),
        name: profile.name.clone(),
        provider: match profile.provider {
            ImageGenProvider::OpenAi => "openai",
            ImageGenProvider::Gemini => "gemini",
            ImageGenProvider::Replicate => "replicate",
            ImageGenProvider::Atlas => "atlas",
            ImageGenProvider::Custom => "custom",
        }
        .into(),
        api_key: profile.api_key.clone(),
        model: profile.model.clone(),
        base_url: profile.base_url.clone(),
    }
}

pub fn image_gen_profile_from_payload(payload: ImageGenProfilePayload) -> Option<ImageGenProfile> {
    let provider = match payload.provider.as_str() {
        "openai" => ImageGenProvider::OpenAi,
        "gemini" => ImageGenProvider::Gemini,
        "replicate" => ImageGenProvider::Replicate,
        "atlas" => ImageGenProvider::Atlas,
        "custom" => ImageGenProvider::Custom,
        _ => return None,
    };
    Some(ImageGenProfile {
        id: payload.id,
        name: payload.name,
        provider,
        api_key: payload.api_key,
        model: payload.model,
        base_url: payload.base_url,
        test_status: op_editor_core::agent_settings::ImageTestStatus::Idle,
    })
}

pub fn openverse_oauth_to_payload(
    settings: &op_editor_core::agent_settings::AgentSettings,
) -> Option<OpenverseOAuthPayload> {
    let client_id = settings.openverse_client_id.trim();
    let client_secret = settings.openverse_client_secret.trim();
    if client_id.is_empty() && client_secret.is_empty() {
        None
    } else {
        Some(OpenverseOAuthPayload {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        })
    }
}

pub fn dedupe_builtin_agents(agents: Vec<BuiltinAgentConfig>) -> Vec<BuiltinAgentConfig> {
    let mut deduped: Vec<BuiltinAgentConfig> = Vec::new();
    for agent in agents {
        let is_duplicate = deduped.iter().any(|existing| {
            existing.matches_add_candidate(
                &agent.display_name,
                &agent.api_key,
                &agent.model,
                agent.kind,
                &agent.base_url,
            )
        });
        if !is_duplicate {
            deduped.push(agent);
        }
    }
    deduped
}

pub fn next_builtin_agent_id(agents: &[BuiltinAgentConfig]) -> u64 {
    agents
        .iter()
        .filter_map(|agent| agent.id.strip_prefix("builtin-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

pub fn next_image_gen_profile_id(profiles: &[ImageGenProfile]) -> u64 {
    profiles
        .iter()
        .filter_map(|profile| profile.id.strip_prefix("igp-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

/// Map a positional v1 `mcp_cli_enabled` array onto the current layout.
/// The current layout has twelve slots: the seven-slot layout plus Gemini
/// CLI / Qwen Code / Cursor / Kimi / ZCode appended. Older files may carry
/// six or eight slots, both of which still held a since-retired Gemini CLI
/// slot at index 2 — those get it dropped so every other CLI keeps its
/// toggle. Every other historical length is a prefix of the current layout.
pub fn migrate_mcp_cli_flags(flags: Vec<bool>) -> [bool; 12] {
    let mut migrated = [false; 12];
    match flags.len() {
        // Current layout (or a longer one written by a newer build).
        12.. => migrated.copy_from_slice(&flags[..12]),
        // Prefixes of the current layout: the CLIs added after them stay off.
        11 => migrated[..11].copy_from_slice(&flags),
        7 => migrated[..7].copy_from_slice(&flags),
        // Legacy layouts that carried the retired Gemini CLI at index 2.
        8..=10 => {
            migrated[0] = flags[0];
            migrated[1] = flags[1];
            migrated[2..7].copy_from_slice(&flags[3..8]);
        }
        3..=6 => {
            migrated[0] = flags[0];
            migrated[1] = flags[1];
            migrated[2..flags.len() - 1].copy_from_slice(&flags[3..]);
        }
        _ => {
            migrated[..flags.len()].copy_from_slice(&flags);
        }
    }
    migrated
}

pub fn theme_to_str(t: ThemeMode) -> &'static str {
    match t {
        ThemeMode::Dark => "dark",
        ThemeMode::Light => "light",
    }
}

pub fn str_to_theme(s: &str) -> ThemeMode {
    match s {
        "light" => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}
