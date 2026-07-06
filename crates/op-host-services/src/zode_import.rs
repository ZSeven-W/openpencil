//! Auto-import Zode (`~/.zode/config.json`) provider configs as
//! OpenPencil built-in ("custom model") agents.
//!
//! The Zode CLI stores its LLM providers in `~/.zode/config.json`, each
//! carrying a protocol `type`, an API key, a base URL, and a set of
//! models. On startup we merge every `(provider, model)` pair into
//! `agent_settings.builtin_agents` so a user who already configured
//! providers in Zode gets the same custom models in OpenPencil without
//! re-entering keys.
//!
//! Best-effort and idempotent: a missing / malformed file is a silent
//! no-op, and re-running each launch dedupes against existing agents by
//! backend (`kind` + `api_key` + `model` + `base_url`) so no duplicates
//! accumulate. New Zode providers appear automatically; the trade-off is
//! that an imported agent deleted inside OpenPencil reappears next launch
//! because Zode still lists it.
//!
//! Imported agents are recorded in `agent_settings.imported_agent_ids`
//! and are NOT written to OpenPencil's own `settings.json`: Zode's config
//! stays the single source of truth for those keys (they're re-imported
//! every launch), so we never silently duplicate a Zode API key onto a
//! second on-disk location.

use std::collections::BTreeMap;
use std::path::PathBuf;

use op_editor_core::{BuiltinAgentKind, EditorState};
use serde::Deserialize;

/// Only the fields we map; everything else in the file (theme, images,
/// per-model pricing, the active `provider.model`, …) is ignored.
#[derive(Debug, Deserialize)]
struct ZodeConfig {
    #[serde(default)]
    providers: BTreeMap<String, ZodeProvider>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZodeProvider {
    /// Protocol dialect — `"anthropic"` maps to the Anthropic backend,
    /// anything else (openai / openai-compat / unset) to OpenAI-compat.
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    base_url: String,
    /// Keyed by model id; the values (contextWindow / prices) carry
    /// metadata OpenPencil doesn't model, so they're discarded.
    #[serde(default)]
    models: BTreeMap<String, serde::de::IgnoredAny>,
}

/// Resolve `~/.zode/config.json`. `None` when no home dir is known.
fn zode_config_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".zode").join("config.json"))
}

/// Best-effort startup import: read + parse + merge. Silent no-op on a
/// missing home dir, missing file, or malformed JSON.
pub fn import_zode_builtin_agents(state: &mut EditorState) {
    let Some(path) = zode_config_path() else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let Ok(config) = serde_json::from_slice::<ZodeConfig>(&bytes) else {
        return;
    };
    import_from_config(state, &config);
}

/// Merge parsed Zode providers into `builtin_agents`. Pure (no IO) so it
/// is unit-testable. Returns the number of newly-added agents.
fn import_from_config(state: &mut EditorState, config: &ZodeConfig) -> usize {
    let settings = &mut state.editor_ui.agent_settings;
    let before = settings.builtin_agents.len();
    for (provider_name, provider) in &config.providers {
        let api_key = provider.api_key.trim();
        // A provider with no key or no models can't produce a usable
        // custom model — skip it.
        if api_key.is_empty() || provider.models.is_empty() {
            continue;
        }
        let kind = match provider.kind.trim().to_ascii_lowercase().as_str() {
            "anthropic" => BuiltinAgentKind::Anthropic,
            _ => BuiltinAgentKind::OpenAiCompat,
        };
        let base_url = {
            let trimmed = provider.base_url.trim();
            if trimmed.is_empty() {
                kind.default_base_url().to_string()
            } else {
                trimmed.to_string()
            }
        };
        // With multiple models the provider name alone is ambiguous, so
        // qualify the card with the model id.
        let multi = provider.models.len() > 1;
        for model_id in provider.models.keys() {
            let model = model_id.trim();
            if model.is_empty() {
                continue;
            }
            let display_name = if multi {
                format!("{provider_name} · {model}")
            } else {
                provider_name.clone()
            };
            // `add_builtin_agent_config` backend-dedupes + assigns the
            // id + infers the preset, so this stays idempotent across
            // launches. Only tag an id as imported when a NEW agent was
            // actually created — a dedupe hit against a user-entered
            // agent returns that agent's id, which must stay persisted.
            let before_len = settings.builtin_agents.len();
            let id =
                settings.add_builtin_agent_config(display_name, api_key, model, kind, &base_url);
            if settings.builtin_agents.len() > before_len {
                settings.imported_agent_ids.insert(id);
            }
        }
    }
    let added = settings.builtin_agents.len() - before;
    if added > 0 {
        // New rows change which models the chat picker lists.
        state.rebuild_chat_models();
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> ZodeConfig {
        serde_json::from_str(json).expect("fixture parses")
    }

    /// The real deepseek + LongCat shape → one agent per model, with the
    /// provider name qualified by model id when a provider has >1 model.
    #[test]
    fn imports_one_agent_per_model() {
        let config = parse(
            r#"{
              "providers": {
                "deepseek": {
                  "type": "anthropic",
                  "apiKey": "sk-deepseek",
                  "baseUrl": "https://api.deepseek.com/anthropic",
                  "models": {
                    "deepseek-v4-pro": {"contextWindow": 1000000, "inputPrice": 0.435},
                    "deepseek-chat": {"contextWindow": 1000000, "inputPrice": 0.14}
                  }
                },
                "LongCat": {
                  "type": "anthropic",
                  "apiKey": "ak-longcat",
                  "baseUrl": "https://api.longcat.chat/anthropic",
                  "models": {"LongCat-2.0": {"contextWindow": 1000000}}
                }
              }
            }"#,
        );
        let mut state = EditorState::new();
        let added = import_from_config(&mut state, &config);
        assert_eq!(added, 3);

        let agents = &state.editor_ui.agent_settings.builtin_agents;
        assert_eq!(agents.len(), 3);

        // deepseek has two models → qualified display names.
        let ds_pro = agents
            .iter()
            .find(|a| a.model == "deepseek-v4-pro")
            .expect("deepseek-v4-pro imported");
        assert_eq!(ds_pro.display_name, "deepseek · deepseek-v4-pro");
        assert_eq!(ds_pro.kind, BuiltinAgentKind::Anthropic);
        assert_eq!(ds_pro.api_key, "sk-deepseek");
        assert_eq!(ds_pro.base_url, "https://api.deepseek.com/anthropic");
        assert!(ds_pro.enabled);
        assert!(agents
            .iter()
            .any(|a| a.model == "deepseek-chat" && a.display_name == "deepseek · deepseek-chat"));

        // LongCat has one model → bare provider name.
        let lc = agents
            .iter()
            .find(|a| a.model == "LongCat-2.0")
            .expect("LongCat-2.0 imported");
        assert_eq!(lc.display_name, "LongCat");
        assert_eq!(lc.base_url, "https://api.longcat.chat/anthropic");
    }

    /// `type` drives the backend kind; unknown / missing → OpenAI-compat.
    #[test]
    fn kind_maps_from_type_field() {
        let config = parse(
            r#"{
              "providers": {
                "anth":   {"type": "anthropic", "apiKey": "k", "baseUrl": "https://a", "models": {"m": {}}},
                "oai":    {"type": "openai",    "apiKey": "k", "baseUrl": "https://o", "models": {"m": {}}},
                "custom": {"type": "whatever",  "apiKey": "k", "baseUrl": "https://c", "models": {"m": {}}}
              }
            }"#,
        );
        let mut state = EditorState::new();
        import_from_config(&mut state, &config);
        let agents = &state.editor_ui.agent_settings.builtin_agents;
        let kind_of = |name: &str| {
            agents
                .iter()
                .find(|a| a.display_name == name)
                .map(|a| a.kind)
                .unwrap()
        };
        assert_eq!(kind_of("anth"), BuiltinAgentKind::Anthropic);
        assert_eq!(kind_of("oai"), BuiltinAgentKind::OpenAiCompat);
        assert_eq!(kind_of("custom"), BuiltinAgentKind::OpenAiCompat);
    }

    /// Empty base URL falls back to the kind's default endpoint.
    #[test]
    fn empty_base_url_falls_back_to_kind_default() {
        let config = parse(
            r#"{"providers": {"p": {"type": "openai", "apiKey": "k", "models": {"m": {}}}}}"#,
        );
        let mut state = EditorState::new();
        import_from_config(&mut state, &config);
        let agent = &state.editor_ui.agent_settings.builtin_agents[0];
        assert_eq!(
            agent.base_url,
            BuiltinAgentKind::OpenAiCompat.default_base_url()
        );
    }

    /// Providers with no key or no models never produce an agent.
    #[test]
    fn skips_providers_without_key_or_models() {
        let config = parse(
            r#"{
              "providers": {
                "nokey":    {"type": "anthropic", "apiKey": "",  "baseUrl": "https://a", "models": {"m": {}}},
                "nomodels": {"type": "anthropic", "apiKey": "k", "baseUrl": "https://a", "models": {}},
                "good":     {"type": "anthropic", "apiKey": "k", "baseUrl": "https://a", "models": {"m": {}}}
              }
            }"#,
        );
        let mut state = EditorState::new();
        let added = import_from_config(&mut state, &config);
        assert_eq!(added, 1);
        assert_eq!(
            state.editor_ui.agent_settings.builtin_agents[0].display_name,
            "good"
        );
    }

    /// Re-running the import (every launch) adds nothing the second time.
    #[test]
    fn import_is_idempotent() {
        let config = parse(
            r#"{
              "providers": {
                "deepseek": {
                  "type": "anthropic",
                  "apiKey": "sk-deepseek",
                  "baseUrl": "https://api.deepseek.com/anthropic",
                  "models": {"deepseek-v4-pro": {}, "deepseek-chat": {}}
                }
              }
            }"#,
        );
        let mut state = EditorState::new();
        let first = import_from_config(&mut state, &config);
        let second = import_from_config(&mut state, &config);
        assert_eq!(first, 2);
        assert_eq!(second, 0);
        assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 2);
        // Both imported agents are tagged so persistence can skip them.
        assert_eq!(state.editor_ui.agent_settings.imported_agent_ids.len(), 2);
    }

    /// An empty / provider-less config is a clean no-op.
    #[test]
    fn empty_config_adds_nothing() {
        let config = parse(r#"{"providers": {}}"#);
        let mut state = EditorState::new();
        assert_eq!(import_from_config(&mut state, &config), 0);
        assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
    }
}
