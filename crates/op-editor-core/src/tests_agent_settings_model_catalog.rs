use crate::{
    AgentSettings, BuiltinAgentKind, BuiltinAgentPresetKey, BuiltinModelCatalogPhase,
    BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogTarget, BuiltinModelOption,
};

fn ready_agent(settings: &mut AgentSettings, name: &str, model: &str) -> String {
    settings.add_builtin_agent_config(
        name,
        format!("secret-{name}"),
        model,
        BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    )
}

#[test]
fn builtin_model_catalog_defaults_are_runtime_quiescent() {
    let settings = AgentSettings::default();

    assert!(settings.builtin_model_catalogs.is_empty());
    assert!(settings.pending_builtin_model_catalog_refreshes.is_empty());
    assert_eq!(settings.builtin_model_catalog_generation, 0);
}

#[test]
fn ready_builtin_catalog_requests_are_target_only_and_queued_per_agent() {
    let mut settings = AgentSettings::default();
    let first = ready_agent(&mut settings, "One", "one-default");
    let second = ready_agent(&mut settings, "Two", "two-default");
    settings.add_builtin_agent_config(
        "Missing key",
        "",
        "fallback",
        BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );

    assert_eq!(settings.request_ready_builtin_model_catalog_refreshes(1), 2);
    let requests = [
        settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("first request"),
        settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("second request"),
    ];

    assert_eq!(
        requests
            .iter()
            .map(|request| request.target.clone())
            .collect::<Vec<_>>(),
        vec![
            BuiltinModelCatalogTarget::Agent(first),
            BuiltinModelCatalogTarget::Agent(second),
        ]
    );
    let debug = format!("{settings:?} {requests:?}");
    assert!(!debug.contains("secret-One"));
    assert!(!debug.contains("secret-Two"));
}

#[test]
fn picker_open_reuses_catalog_until_ttl_expires() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    let started_at = 100;

    assert_eq!(
        settings.request_ready_builtin_model_catalog_refreshes(started_at),
        1
    );
    let request = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("initial request");
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("provider snapshot");
    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![BuiltinModelOption::new("live", "Live")],
            },
        )
    );

    assert_eq!(
        settings.request_ready_builtin_model_catalog_refreshes(
            started_at + crate::agent_settings_builtin_models::BUILTIN_MODEL_CATALOG_TTL_MS - 1,
        ),
        0
    );
    assert_eq!(settings.builtin_model_catalog_options(&id).len(), 1);
    assert_eq!(
        settings.request_ready_builtin_model_catalog_refreshes(
            started_at + crate::agent_settings_builtin_models::BUILTIN_MODEL_CATALOG_TTL_MS,
        ),
        1
    );
}

#[test]
fn discovery_does_not_require_a_preselected_model() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "");

    assert!(!settings.builtin_agents[0].ready());
    assert!(settings.builtin_agents[0].discovery_ready());
    assert!(settings.has_discovery_ready_builtin_agent());
    assert_eq!(settings.request_ready_builtin_model_catalog_refreshes(1), 1);
    assert_eq!(
        settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("key and endpoint are sufficient for discovery")
            .target,
        BuiltinModelCatalogTarget::Agent(id)
    );
}

#[test]
fn manual_refresh_bypasses_ttl_without_duplicating_loading_requests() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    let target = BuiltinModelCatalogTarget::Agent(id);
    let started_at = 100;

    assert_eq!(
        settings.request_ready_builtin_model_catalog_refreshes(started_at),
        1
    );
    let initial = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("initial request");
    let expected = settings
        .builtin_model_catalog_config_for_request(&initial)
        .expect("provider snapshot");
    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &initial,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![BuiltinModelOption::new("live", "Live")],
            },
        )
    );

    assert_eq!(
        settings.request_ready_builtin_model_catalog_refreshes(started_at + 1),
        0,
        "ordinary picker opens remain TTL-debounced"
    );
    assert_eq!(
        settings.force_ready_builtin_model_catalog_refreshes(started_at + 1),
        1,
        "manual refresh bypasses the TTL"
    );
    let forced = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("forced request");
    assert_eq!(forced.target, target);
    assert!(forced.generation > initial.generation);
    assert_eq!(
        settings.force_ready_builtin_model_catalog_refreshes(started_at + 2),
        0,
        "an in-flight manual refresh is not duplicated"
    );
    assert_eq!(settings.builtin_model_catalog_generation, forced.generation);
}

#[test]
fn drained_loading_request_is_not_queued_again() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    let first = settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id.clone()), 1)
        .expect("first request");
    assert_eq!(
        settings.take_pending_builtin_model_catalog_refresh(),
        Some(first.clone())
    );

    assert!(settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id), 1)
        .is_none());
    assert_eq!(settings.builtin_model_catalog_generation, first.generation);
}

#[test]
fn success_normalizes_options_and_failed_refresh_keeps_them_stale() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    let target = BuiltinModelCatalogTarget::Agent(id.clone());
    let request = settings
        .begin_builtin_model_catalog_refresh(target.clone(), 1)
        .expect("request");
    settings.take_pending_builtin_model_catalog_refresh();
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("snapshot");

    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    BuiltinModelOption::new(" live-a ", " Live A "),
                    BuiltinModelOption::new("live-a", "duplicate"),
                    BuiltinModelOption::new("live-b", ""),
                    BuiltinModelOption::new(" ", "ignored"),
                ],
            },
        )
    );
    assert_eq!(
        settings.builtin_model_catalog_options(&id),
        [
            BuiltinModelOption::new("live-a", "Live A"),
            BuiltinModelOption::new("live-b", "live-b"),
        ]
    );

    let refresh = settings
        .begin_builtin_model_catalog_refresh(target.clone(), 2)
        .expect("refresh");
    settings.take_pending_builtin_model_catalog_refresh();
    let refresh_expected = settings
        .builtin_model_catalog_config_for_request(&refresh)
        .expect("refresh snapshot");
    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &refresh_expected,
            &refresh,
            BuiltinModelCatalogRefreshOutcome::Error {
                error: "temporary".into(),
            },
        )
    );
    let catalog = settings
        .builtin_model_catalogs
        .get(&target)
        .expect("catalog remains");
    assert_eq!(catalog.phase, BuiltinModelCatalogPhase::Error);
    assert!(catalog.stale);
    assert_eq!(catalog.models.len(), 2);
}

#[test]
fn changed_credentials_reject_and_clear_an_in_flight_result() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    let target = BuiltinModelCatalogTarget::Agent(id.clone());
    let request = settings
        .begin_builtin_model_catalog_refresh(target.clone(), 1)
        .expect("request");
    settings.take_pending_builtin_model_catalog_refresh();
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("snapshot");
    settings.builtin_agents[0].api_key = "different-secret".into();

    assert!(
        !settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![BuiltinModelOption::new("late", "Late")],
            },
        )
    );
    assert!(!settings.builtin_model_catalogs.contains_key(&target));
    assert!(settings.builtin_model_catalog_options(&id).is_empty());
}

#[test]
fn model_change_does_not_invalidate_an_in_flight_catalog() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback-a");
    let request = settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id.clone()), 1)
        .expect("request");
    settings.take_pending_builtin_model_catalog_refresh();
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("snapshot");
    settings.builtin_agents[0].set_models(["fallback-b"]);

    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![BuiltinModelOption::new("live", "Live")],
            },
        )
    );
    assert_eq!(settings.builtin_model_catalog_options(&id).len(), 1);
}

#[test]
fn preset_remove_draft_cancel_and_operator_takeover_clear_runtime_catalogs() {
    let mut settings = AgentSettings::default();
    let id = ready_agent(&mut settings, "One", "fallback");
    settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id.clone()), 1)
        .expect("request");
    settings.set_builtin_agent_preset(0, BuiltinAgentPresetKey::OpenAi);
    assert!(settings.builtin_model_catalog_options(&id).is_empty());
    assert!(settings
        .take_pending_builtin_model_catalog_refresh()
        .is_none());

    settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id.clone()), 2)
        .expect("new request");
    let removed = settings.remove_builtin_agent(0).expect("agent removed");
    assert_eq!(removed.id, id);
    assert!(settings.builtin_model_catalogs.is_empty());

    settings.begin_builtin_agent_draft();
    settings.builtin_agent_draft.as_mut().unwrap().api_key = "draft-secret".into();
    settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Draft, 3)
        .expect("draft request");
    settings.cancel_builtin_agent_draft();
    assert!(!settings
        .builtin_model_catalogs
        .contains_key(&BuiltinModelCatalogTarget::Draft));

    let old_id = settings.add_builtin_agent_config(
        "Browser",
        "browser-secret",
        "fallback",
        BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    settings.builtin_agents[0].id = format!("web-credential:builtin:{old_id}");
    let old_id = settings.builtin_agents[0].id.clone();
    settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(old_id.clone()), 4)
        .expect("browser request");
    assert!(settings.take_over_browser_builtin_agent(0));
    assert!(settings.builtin_model_catalog_options(&old_id).is_empty());
    assert!(settings.pending_builtin_model_catalog_refreshes.is_empty());
}
