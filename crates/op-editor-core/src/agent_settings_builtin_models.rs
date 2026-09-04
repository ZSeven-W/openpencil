//! Runtime model catalogs for configured built-in API providers.
//!
//! The configured model list remains the persisted source of truth. Catalogs are
//! process-local discovery results: hosts drain target-only requests, clone the
//! current credential, and land an outcome only while the request still owns
//! the same configuration generation.

use std::collections::{hash_map::DefaultHasher, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::agent_settings::{AgentSettings, BuiltinAgentConfig};

/// Picker opens within this window reuse the last runtime catalog. Credential
/// or endpoint edits invalidate the catalog immediately and bypass the TTL.
pub const BUILTIN_MODEL_CATALOG_TTL_MS: u64 = 5 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuiltinModelCatalogTarget {
    Agent(String),
    Draft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinModelOption {
    pub id: String,
    pub display_name: String,
}

impl BuiltinModelOption {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltinModelCatalogPhase {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuiltinModelCatalog {
    pub phase: BuiltinModelCatalogPhase,
    pub generation: u64,
    pub models: Vec<BuiltinModelOption>,
    /// A failed refresh keeps the last successful catalog usable.
    pub stale: bool,
    pub error: Option<String>,
    last_attempt_ms: Option<u64>,
    credential_fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinModelCatalogRefreshRequest {
    pub target: BuiltinModelCatalogTarget,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinModelCatalogRefreshOutcome {
    Success { models: Vec<BuiltinModelOption> },
    Unsupported { error: Option<String> },
    Error { error: String },
}

impl AgentSettings {
    /// Whether provider settings contain any entry ready for discovery.
    /// Discovery deliberately does not require an already selected model.
    pub fn has_discovery_ready_builtin_agent(&self) -> bool {
        self.builtin_agents
            .iter()
            .any(BuiltinAgentConfig::discovery_ready)
    }

    /// Queue one catalog refresh without copying credentials into the request
    /// seam. The host resolves the target to a cloned config when it drains.
    pub fn begin_builtin_model_catalog_refresh(
        &mut self,
        target: BuiltinModelCatalogTarget,
        now_ms: u64,
    ) -> Option<BuiltinModelCatalogRefreshRequest> {
        let (ready, fingerprint) = self.builtin_model_catalog_config(&target).map(|config| {
            (
                config.discovery_ready(),
                builtin_model_credential_fingerprint(config),
            )
        })?;
        if !ready {
            return None;
        }
        if self
            .builtin_model_catalogs
            .get(&target)
            .is_some_and(|catalog| catalog.credential_fingerprint != fingerprint)
        {
            self.invalidate_builtin_model_catalog(&target);
        }
        if self
            .pending_builtin_model_catalog_refreshes
            .iter()
            .any(|request| request.target == target)
            || self
                .builtin_model_catalogs
                .get(&target)
                .is_some_and(|catalog| catalog.phase == BuiltinModelCatalogPhase::Loading)
        {
            return None;
        }
        let generation = self.builtin_model_catalog_generation.checked_add(1)?;
        self.builtin_model_catalog_generation = generation;
        let previous = self
            .builtin_model_catalogs
            .remove(&target)
            .unwrap_or_default();
        self.builtin_model_catalogs.insert(
            target.clone(),
            BuiltinModelCatalog {
                phase: BuiltinModelCatalogPhase::Loading,
                generation,
                last_attempt_ms: Some(now_ms),
                credential_fingerprint: fingerprint,
                ..previous
            },
        );
        let request = BuiltinModelCatalogRefreshRequest { target, generation };
        self.pending_builtin_model_catalog_refreshes
            .push_back(request.clone());
        Some(request)
    }

    /// Queue all ready saved built-ins for an explicit settings-side refresh.
    /// The chat picker must not call this: chat exposes only saved models.
    pub fn request_ready_builtin_model_catalog_refreshes(&mut self, now_ms: u64) -> usize {
        let targets = self
            .builtin_agents
            .iter()
            .filter(|agent| agent.discovery_ready())
            .map(|agent| BuiltinModelCatalogTarget::Agent(agent.id.clone()))
            .collect::<Vec<_>>();
        targets
            .into_iter()
            .filter(|target| {
                let due = self
                    .builtin_model_catalogs
                    .get(target)
                    .is_none_or(|catalog| {
                        self.builtin_model_catalog_config(target)
                            .is_some_and(|config| {
                                catalog.credential_fingerprint
                                    != builtin_model_credential_fingerprint(config)
                                    || catalog.last_attempt_ms.is_none_or(|last_attempt_ms| {
                                        now_ms.saturating_sub(last_attempt_ms)
                                            >= BUILTIN_MODEL_CATALOG_TTL_MS
                                    })
                            })
                    });
                due && self
                    .begin_builtin_model_catalog_refresh(target.clone(), now_ms)
                    .is_some()
            })
            .count()
    }

    /// Force one ready built-in provider through the normal generation seam,
    /// bypassing only the picker-open TTL. An already queued or in-flight
    /// request still wins so repeated presses never duplicate network work.
    pub fn force_builtin_model_catalog_refresh(
        &mut self,
        target: BuiltinModelCatalogTarget,
        now_ms: u64,
    ) -> Option<BuiltinModelCatalogRefreshRequest> {
        self.begin_builtin_model_catalog_refresh(target, now_ms)
    }

    /// Force-refresh every ready saved built-in provider for an explicit
    /// settings action; draft credentials remain scoped to their own form.
    pub fn force_ready_builtin_model_catalog_refreshes(&mut self, now_ms: u64) -> usize {
        let targets = self
            .builtin_agents
            .iter()
            .filter(|agent| agent.discovery_ready())
            .map(|agent| BuiltinModelCatalogTarget::Agent(agent.id.clone()))
            .collect::<Vec<_>>();
        targets
            .into_iter()
            .filter(|target| {
                self.force_builtin_model_catalog_refresh(target.clone(), now_ms)
                    .is_some()
            })
            .count()
    }

    pub fn take_pending_builtin_model_catalog_refresh(
        &mut self,
    ) -> Option<BuiltinModelCatalogRefreshRequest> {
        self.pending_builtin_model_catalog_refreshes.pop_front()
    }

    pub fn builtin_model_catalog_config_for_request(
        &self,
        request: &BuiltinModelCatalogRefreshRequest,
    ) -> Option<BuiltinAgentConfig> {
        let catalog = self.builtin_model_catalogs.get(&request.target)?;
        let config = self.builtin_model_catalog_config(&request.target)?;
        (config.discovery_ready()
            && catalog.phase == BuiltinModelCatalogPhase::Loading
            && catalog.generation == request.generation
            && catalog.credential_fingerprint == builtin_model_credential_fingerprint(config))
        .then(|| config.clone())
    }

    pub fn apply_builtin_model_catalog_refresh_outcome_if_current(
        &mut self,
        expected: &BuiltinAgentConfig,
        request: &BuiltinModelCatalogRefreshRequest,
        outcome: BuiltinModelCatalogRefreshOutcome,
    ) -> bool {
        let Some(current) = self.builtin_model_catalog_config(&request.target) else {
            return false;
        };
        let current_id = current.id.clone();
        let fingerprint = builtin_model_credential_fingerprint(current);
        let expected_fingerprint = builtin_model_credential_fingerprint(expected);
        let request_state = self.builtin_model_catalogs.get(&request.target);
        let same_generation =
            request_state.is_some_and(|catalog| catalog.generation == request.generation);
        let current_request = request_state.is_some_and(|catalog| {
            catalog.phase == BuiltinModelCatalogPhase::Loading
                && catalog.generation == request.generation
                && catalog.credential_fingerprint == fingerprint
        });
        if current_id != expected.id
            || fingerprint != expected_fingerprint
            || !current.enabled
            || !current_request
        {
            if same_generation && (fingerprint != expected_fingerprint || !current.enabled) {
                self.invalidate_builtin_model_catalog(&request.target);
            }
            return false;
        }
        let catalog = self
            .builtin_model_catalogs
            .get_mut(&request.target)
            .expect("current request has catalog state");
        match outcome {
            BuiltinModelCatalogRefreshOutcome::Success { models } => {
                catalog.models = normalize_model_options(models);
                catalog.phase = BuiltinModelCatalogPhase::Ready;
                catalog.stale = false;
                catalog.error = None;
            }
            BuiltinModelCatalogRefreshOutcome::Unsupported { error } => {
                catalog.phase = BuiltinModelCatalogPhase::Unsupported;
                catalog.stale = !catalog.models.is_empty();
                catalog.error = error;
            }
            BuiltinModelCatalogRefreshOutcome::Error { error } => {
                catalog.phase = BuiltinModelCatalogPhase::Error;
                catalog.stale = !catalog.models.is_empty();
                catalog.error = Some(error);
            }
        }
        self.pending_builtin_model_catalog_refreshes
            .retain(|pending| pending != request);
        true
    }

    pub fn builtin_model_catalog_options(&self, id: &str) -> &[BuiltinModelOption] {
        self.builtin_model_catalog_options_for(&BuiltinModelCatalogTarget::Agent(id.to_string()))
    }

    /// Catalog options keyed by catalog target — agents by id, and the
    /// unsaved add-provider draft by [`BuiltinModelCatalogTarget::Draft`].
    pub fn builtin_model_catalog_options_for(
        &self,
        target: &BuiltinModelCatalogTarget,
    ) -> &[BuiltinModelOption] {
        self.builtin_model_catalogs
            .get(target)
            .map(|catalog| catalog.models.as_slice())
            .unwrap_or(&[])
    }

    /// The full runtime catalog for `target`, if one exists.
    pub fn builtin_model_catalog(
        &self,
        target: &BuiltinModelCatalogTarget,
    ) -> Option<&BuiltinModelCatalog> {
        self.builtin_model_catalogs.get(target)
    }

    /// Queue a discovery refresh for `target` only when the runtime
    /// catalog is due: absent, keyed to a different credential, or older
    /// than the TTL. Error and Unsupported states obey the same cooldown;
    /// an explicit retry uses the force path. The settings-form
    /// model dropdown opens through this — a menu the user just closed
    /// must not re-hit the provider on every reopen inside the TTL.
    pub fn begin_builtin_model_catalog_refresh_if_due(
        &mut self,
        target: BuiltinModelCatalogTarget,
        now_ms: u64,
    ) -> Option<BuiltinModelCatalogRefreshRequest> {
        let due = self
            .builtin_model_catalogs
            .get(&target)
            .is_none_or(|catalog| {
                self.builtin_model_catalog_config(&target)
                    .is_some_and(|config| {
                        catalog.credential_fingerprint
                            != builtin_model_credential_fingerprint(config)
                            || (catalog.phase != BuiltinModelCatalogPhase::Loading
                                && catalog.last_attempt_ms.is_none_or(|last_attempt_ms| {
                                    now_ms.saturating_sub(last_attempt_ms)
                                        >= BUILTIN_MODEL_CATALOG_TTL_MS
                                }))
                    })
            });
        if due {
            self.begin_builtin_model_catalog_refresh(target, now_ms)
        } else {
            None
        }
    }

    pub fn invalidate_builtin_model_catalog(&mut self, target: &BuiltinModelCatalogTarget) -> bool {
        let mut changed = self.builtin_model_catalogs.remove(target).is_some();
        let before = self.pending_builtin_model_catalog_refreshes.len();
        self.pending_builtin_model_catalog_refreshes
            .retain(|request| &request.target != target);
        changed |= before != self.pending_builtin_model_catalog_refreshes.len();
        changed
    }

    pub fn invalidate_builtin_model_catalog_for_agent(&mut self, id: &str) -> bool {
        self.invalidate_builtin_model_catalog(&BuiltinModelCatalogTarget::Agent(id.to_string()))
    }

    pub fn clear_builtin_model_catalogs(&mut self) -> bool {
        let changed = !self.builtin_model_catalogs.is_empty()
            || !self.pending_builtin_model_catalog_refreshes.is_empty();
        self.builtin_model_catalogs.clear();
        self.pending_builtin_model_catalog_refreshes.clear();
        changed
    }

    /// Remove catalogs for deleted agents or changed credentials/endpoints.
    pub fn prune_builtin_model_catalogs(&mut self) -> bool {
        let valid = self
            .builtin_agents
            .iter()
            .map(|agent| {
                (
                    BuiltinModelCatalogTarget::Agent(agent.id.clone()),
                    builtin_model_credential_fingerprint(agent),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let catalogs_before = self.builtin_model_catalogs.len();
        let pending_before = self.pending_builtin_model_catalog_refreshes.len();
        self.builtin_model_catalogs
            .retain(|target, catalog| match target {
                BuiltinModelCatalogTarget::Agent(_) => valid
                    .get(target)
                    .is_some_and(|fingerprint| *fingerprint == catalog.credential_fingerprint),
                BuiltinModelCatalogTarget::Draft => {
                    self.builtin_agent_draft.as_ref().is_some_and(|draft| {
                        builtin_model_credential_fingerprint(draft)
                            == catalog.credential_fingerprint
                    })
                }
            });
        self.pending_builtin_model_catalog_refreshes
            .retain(|request| self.builtin_model_catalogs.contains_key(&request.target));
        catalogs_before != self.builtin_model_catalogs.len()
            || pending_before != self.pending_builtin_model_catalog_refreshes.len()
    }

    fn builtin_model_catalog_config(
        &self,
        target: &BuiltinModelCatalogTarget,
    ) -> Option<&BuiltinAgentConfig> {
        match target {
            BuiltinModelCatalogTarget::Agent(id) => {
                self.builtin_agents.iter().find(|agent| agent.id == *id)
            }
            BuiltinModelCatalogTarget::Draft => self.builtin_agent_draft.as_ref(),
        }
    }
}

fn normalize_model_options(models: Vec<BuiltinModelOption>) -> Vec<BuiltinModelOption> {
    let mut seen = BTreeSet::new();
    models
        .into_iter()
        .filter_map(|model| {
            let id = model.id.trim().to_string();
            if id.is_empty() || !seen.insert(id.clone()) {
                return None;
            }
            let display_name = model.display_name.trim();
            Some(BuiltinModelOption {
                display_name: if display_name.is_empty() {
                    id.clone()
                } else {
                    display_name.to_string()
                },
                id,
            })
        })
        .collect()
}

/// Non-cryptographic process-local comparison value. It never leaves core and
/// must not be logged; its purpose is rejecting results for edited configs.
fn builtin_model_credential_fingerprint(config: &BuiltinAgentConfig) -> u64 {
    let mut hasher = DefaultHasher::new();
    config.kind.hash(&mut hasher);
    config.preset.as_str().hash(&mut hasher);
    config
        .base_url
        .trim()
        .trim_end_matches('/')
        .hash(&mut hasher);
    config.api_key.trim().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_settings::BuiltinAgentKind;
    use crate::agent_settings_builtin_presets::BuiltinAgentPresetKey;

    fn ready_agent(id: &str) -> BuiltinAgentConfig {
        BuiltinAgentConfig {
            id: id.into(),
            preset: BuiltinAgentPresetKey::Anthropic,
            display_name: "Anthropic".into(),
            kind: BuiltinAgentKind::Anthropic,
            api_key: "sk-test".into(),
            models: vec!["claude-sonnet-4-6-20250916".into()],
            base_url: "https://api.anthropic.com".into(),
            enabled: true,
        }
    }

    fn land_success(settings: &mut AgentSettings, target: &BuiltinModelCatalogTarget) {
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("pending request");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable request");
        assert!(
            settings.apply_builtin_model_catalog_refresh_outcome_if_current(
                &expected,
                &request,
                BuiltinModelCatalogRefreshOutcome::Success {
                    models: vec![BuiltinModelOption::new(
                        "claude-sonnet-4-6-20250916",
                        "Claude Sonnet 4.6",
                    )],
                },
            )
        );
        let _ = target;
    }

    #[test]
    fn refresh_if_due_runs_once_inside_the_ttl() {
        let mut settings = AgentSettings::default();
        settings.builtin_agents.push(ready_agent("a1"));
        let target = BuiltinModelCatalogTarget::Agent("a1".into());

        let first = settings.begin_builtin_model_catalog_refresh_if_due(target.clone(), 0);
        assert!(first.is_some(), "a missing catalog is always due");
        assert_eq!(
            settings.begin_builtin_model_catalog_refresh_if_due(target.clone(), 10),
            None,
            "an in-flight request must not be duplicated"
        );

        land_success(&mut settings, &target);
        assert_eq!(
            settings.begin_builtin_model_catalog_refresh_if_due(target.clone(), 1_000),
            None,
            "a fresh Ready catalog stays cached inside the TTL"
        );
        assert!(
            settings
                .begin_builtin_model_catalog_refresh_if_due(
                    target.clone(),
                    1_000 + BUILTIN_MODEL_CATALOG_TTL_MS
                )
                .is_some(),
            "a Ready catalog past the TTL is due again"
        );
    }

    #[test]
    fn refresh_if_due_throttles_failed_catalogs_but_force_can_retry() {
        let mut settings = AgentSettings::default();
        settings.builtin_agents.push(ready_agent("a1"));
        let target = BuiltinModelCatalogTarget::Agent("a1".into());
        settings
            .begin_builtin_model_catalog_refresh_if_due(target.clone(), 0)
            .expect("first request");
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("pending");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("resolvable");
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Error {
                error: "offline".into(),
            },
        );
        assert_eq!(
            settings.begin_builtin_model_catalog_refresh_if_due(target.clone(), 10),
            None,
            "reopening an errored catalog inside the TTL must not hammer the provider"
        );
        assert!(
            settings
                .force_builtin_model_catalog_refresh(target, 10)
                .is_some(),
            "an explicit retry bypasses the cooldown"
        );
    }

    #[test]
    fn refresh_if_due_refuses_unconfigured_targets_and_supports_the_draft() {
        let mut settings = AgentSettings::default();
        settings.builtin_agents.push(ready_agent("a1"));
        settings.builtin_agents[0].api_key.clear();
        assert_eq!(
            settings.begin_builtin_model_catalog_refresh_if_due(
                BuiltinModelCatalogTarget::Agent("a1".into()),
                0,
            ),
            None,
            "discovery needs a credential; the form falls back to typing"
        );

        settings.builtin_agent_draft = Some(ready_agent(""));
        let draft = BuiltinModelCatalogTarget::Draft;
        let request = settings
            .begin_builtin_model_catalog_refresh_if_due(draft.clone(), 0)
            .expect("a credentialed draft is discovery-ready");
        assert_eq!(request.target, draft);
        land_success(&mut settings, &draft);
        assert_eq!(settings.builtin_model_catalog_options_for(&draft).len(), 1);
    }
}
