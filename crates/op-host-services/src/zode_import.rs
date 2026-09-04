//! Auto-import Zode (`~/.zode/config.json`) provider configs as
//! OpenPencil built-in ("custom model") agents.
//!
//! The Zode CLI stores its LLM providers in `~/.zode/config.json`, each
//! carrying a protocol `type`, an API key, a base URL, and a set of
//! models. On startup we merge every provider and its saved model list into
//! `agent_settings.builtin_agents` so a user who already configured
//! providers in Zode gets the same custom models in OpenPencil without
//! re-entering keys.
//!
//! Best-effort and idempotent: a missing / malformed file is a silent
//! no-op, and re-running each launch dedupes against existing agents by
//! backend (`preset` + `kind` + `api_key` + `base_url`) so no duplicates
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
    let before_agents = settings.builtin_agents.clone();
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
        let models = op_editor_core::normalize_builtin_models(provider.models.keys().cloned());
        if models.is_empty() {
            continue;
        }
        // One backend is one card. A unique existing operator-owned transport
        // supplies its preset because presets can share a URL while using
        // different model-discovery endpoints. Browser-owned cards are a
        // separate persistence domain; multiple operator candidates are
        // ambiguous even when their preset happens to match.
        let existing_candidates = settings
            .builtin_agents
            .iter()
            .filter(|agent| {
                !agent.id.starts_with("web-credential:builtin:")
                    && agent.kind == kind
                    && agent.api_key.trim() == api_key
                    && agent.base_url.trim().trim_end_matches('/')
                        == base_url.trim().trim_end_matches('/')
            })
            .map(|agent| (agent.preset, agent.enabled))
            .collect::<Vec<_>>();
        if existing_candidates.len() > 1 {
            continue;
        }
        // A disabled operator card is an explicit user choice. Importing
        // must neither re-enable it nor create a same-backend duplicate that
        // strict persistence would reject.
        if existing_candidates
            .first()
            .is_some_and(|(_, enabled)| !enabled)
        {
            continue;
        }
        let before_len = settings.builtin_agents.len();
        let id = if let Some((preset, _)) = existing_candidates.first().copied() {
            settings.add_builtin_agent_configs_with_preset(
                provider_name,
                api_key,
                models,
                kind,
                &base_url,
                Some(preset),
            )
        } else {
            settings.add_builtin_agent_configs(provider_name, api_key, models, kind, &base_url)
        };
        // Only tag an id as imported when a NEW agent was actually created —
        // a dedupe hit against a user-entered provider must stay persisted.
        if settings.builtin_agents.len() > before_len {
            settings.imported_agent_ids.insert(id);
        }
    }
    let added = settings.builtin_agents.len() - before;
    let agents_changed = settings.builtin_agents != before_agents;
    if agents_changed {
        // New rows and models merged into existing rows both change which
        // models the chat picker lists.
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

    /// The real deepseek + LongCat shape → one agent per provider, preserving
    /// each provider's complete saved model list.
    #[test]
    fn imports_one_agent_per_provider_with_all_models() {
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
        assert_eq!(added, 2);

        let agents = &state.editor_ui.agent_settings.builtin_agents;
        assert_eq!(agents.len(), 2);

        let deepseek = agents
            .iter()
            .find(|a| a.display_name == "deepseek")
            .expect("deepseek imported");
        assert_eq!(
            deepseek.models,
            ["deepseek-chat", "deepseek-v4-pro"],
            "BTreeMap input keeps deterministic model order"
        );
        assert_eq!(deepseek.kind, BuiltinAgentKind::Anthropic);
        assert_eq!(deepseek.api_key, "sk-deepseek");
        assert_eq!(deepseek.base_url, "https://api.deepseek.com/anthropic");
        assert!(deepseek.enabled);

        let lc = agents
            .iter()
            .find(|a| a.has_model("LongCat-2.0"))
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
        assert_eq!(first, 1);
        assert_eq!(second, 0);
        assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 1);
        assert_eq!(state.editor_ui.agent_settings.imported_agent_ids.len(), 1);
    }

    #[test]
    fn models_merged_into_existing_agent_rebuild_the_chat_catalog() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Existing",
            "sk-existing",
            "model-a",
            BuiltinAgentKind::OpenAiCompat,
            "https://example.com/v1",
        );
        state.rebuild_chat_models();
        assert_eq!(
            state
                .chat
                .available_models
                .iter()
                .filter(|entry| entry.builtin_provider_id.as_deref() == Some(id.as_str()))
                .count(),
            1
        );

        let config = parse(
            r#"{
              "providers": {
                "Zode label": {
                  "type": "openai",
                  "apiKey": "sk-existing",
                  "baseUrl": "https://example.com/v1",
                  "models": {"model-a": {}, "model-b": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 0);
        assert_eq!(
            state.editor_ui.agent_settings.builtin_agents[0].models,
            ["model-a", "model-b"]
        );
        assert!(state.chat.available_models.iter().any(|entry| {
            entry.builtin_provider_id.as_deref() == Some(id.as_str())
                && entry.builtin_model_id() == Some("model-b")
        }));
        assert!(!state
            .editor_ui
            .agent_settings
            .imported_agent_ids
            .contains(&id));
    }

    #[test]
    fn import_reuses_a_unique_custom_anthropic_preset_for_the_same_transport() {
        let mut state = EditorState::new();
        let settings = &mut state.editor_ui.agent_settings;
        settings.begin_builtin_agent_draft();
        settings.set_builtin_agent_draft_preset(op_editor_core::BuiltinAgentPresetKey::Custom);
        let draft = settings.builtin_agent_draft.as_mut().expect("draft exists");
        draft.toggle_kind_for_preset();
        draft.api_key = "sk-existing".into();
        draft.base_url = "https://custom.example/anthropic".into();
        draft.set_models(["model-a"]);
        let existing_id = settings.save_builtin_agent_draft().expect("draft saves");

        let config = parse(
            r#"{
              "providers": {
                "Zode label": {
                  "type": "anthropic",
                  "apiKey": "sk-existing",
                  "baseUrl": "https://custom.example/anthropic",
                  "models": {"model-b": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 0);
        let agents = &state.editor_ui.agent_settings.builtin_agents;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, existing_id);
        assert_eq!(
            agents[0].preset,
            op_editor_core::BuiltinAgentPresetKey::Custom
        );
        assert_eq!(agents[0].models, ["model-a", "model-b"]);
    }

    #[test]
    fn import_does_not_guess_between_presets_sharing_one_transport() {
        let mut state = EditorState::new();
        let settings = &mut state.editor_ui.agent_settings;
        let base_url = "https://ark.cn-beijing.volces.com/api/coding";
        for (preset, model) in [
            (
                op_editor_core::BuiltinAgentPresetKey::Doubao,
                "doubao-seed-2-0-pro-260215",
            ),
            (
                op_editor_core::BuiltinAgentPresetKey::ArkCoding,
                "ark-code-latest",
            ),
        ] {
            settings.add_builtin_agent_configs_with_preset(
                preset.as_str(),
                "shared-key",
                [model],
                BuiltinAgentKind::Anthropic,
                base_url,
                Some(preset),
            );
        }

        let config = parse(
            r#"{
              "providers": {
                "Ambiguous": {
                  "type": "anthropic",
                  "apiKey": "shared-key",
                  "baseUrl": "https://ark.cn-beijing.volces.com/api/coding",
                  "models": {"zode-only-model": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 0);
        assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 2);
        assert!(state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .all(|agent| !agent.has_model("zode-only-model")));
    }

    #[test]
    fn import_never_merges_zode_models_into_a_browser_owned_card() {
        let mut state = EditorState::new();
        let settings = &mut state.editor_ui.agent_settings;
        settings.add_builtin_agent_config(
            "Browser",
            "shared-key",
            "browser-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://custom.example/v1",
        );
        settings.builtin_agents[0].id = "web-credential:builtin:browser-1".into();

        let config = parse(
            r#"{
              "providers": {
                "Zode": {
                  "type": "openai",
                  "apiKey": "shared-key",
                  "baseUrl": "https://custom.example/v1",
                  "models": {"zode-model": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 1);
        let agents = &state.editor_ui.agent_settings.builtin_agents;
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].models, ["browser-model"]);
        let imported = agents
            .iter()
            .find(|agent| agent.has_model("zode-model"))
            .expect("Zode model gets an operator-owned card");
        assert!(!imported.id.starts_with("web-credential:builtin:"));
        assert!(state
            .editor_ui
            .agent_settings
            .imported_agent_ids
            .contains(&imported.id));
    }

    #[test]
    fn import_does_not_choose_between_mixed_enabled_operator_cards() {
        let mut state = EditorState::new();
        let settings = &mut state.editor_ui.agent_settings;
        let first_id = settings.add_builtin_agent_config(
            "Enabled",
            "shared-key",
            "enabled-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://custom.example/v1",
        );
        let mut disabled = settings
            .builtin_agents
            .iter()
            .find(|agent| agent.id == first_id)
            .expect("enabled card")
            .clone();
        disabled.id = "builtin-2".into();
        disabled.enabled = false;
        disabled.set_models(["disabled-model"]);
        settings.builtin_agents.push(disabled);

        let config = parse(
            r#"{
              "providers": {
                "Zode": {
                  "type": "openai",
                  "apiKey": "shared-key",
                  "baseUrl": "https://custom.example/v1",
                  "models": {"zode-model": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 0);
        assert!(state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .all(|agent| !agent.has_model("zode-model")));
    }

    #[test]
    fn import_respects_a_single_disabled_operator_card() {
        let mut state = EditorState::new();
        let settings = &mut state.editor_ui.agent_settings;
        settings.add_builtin_agent_config(
            "Disabled",
            "shared-key",
            "disabled-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://custom.example/v1",
        );
        settings.builtin_agents[0].enabled = false;

        let config = parse(
            r#"{
              "providers": {
                "Zode": {
                  "type": "openai",
                  "apiKey": "shared-key",
                  "baseUrl": "https://custom.example/v1",
                  "models": {"zode-model": {}}
                }
              }
            }"#,
        );

        assert_eq!(import_from_config(&mut state, &config), 0);
        let agents = &state.editor_ui.agent_settings.builtin_agents;
        assert_eq!(agents.len(), 1);
        assert!(!agents[0].enabled);
        assert!(!agents[0].has_model("zode-model"));
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
