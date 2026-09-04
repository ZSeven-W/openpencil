use std::sync::mpsc;

use super::*;
use op_ai::agent_settings_state::AgentProvider as AiProvider;
use op_editor_core::agent_settings::ProviderConnectPhase;

/// Mark a provider verified-connected the way a landed connect probe does,
/// so `verified_connected_mask` (and therefore `rebuild_chat_models`) lists it.
fn connect(state: &mut EditorState, provider: AgentProvider) {
    let index = AgentProvider::ALL
        .iter()
        .position(|candidate| *candidate == provider)
        .expect("known provider");
    let settings = &mut state.editor_ui.agent_settings;
    settings.connected[index] = true;
    settings.provider_connection[index].phase = ProviderConnectPhase::Connected;
}

fn seed(state: &mut EditorState, provider: AgentProvider, value: &str) {
    state
        .chat
        .discovered_models
        .push(op_editor_core::ModelEntry::new(provider, value, value));
}

fn catalog_values(state: &EditorState) -> Vec<&str> {
    state
        .chat
        .discovered_models
        .iter()
        .map(|entry| entry.value.as_str())
        .collect()
}

/// Block until the spawned worker's result has landed. The worker is a
/// detached one-shot; allow enough wall time for process-heavy full-suite
/// contention while keeping a hard deadline for a genuinely stuck worker.
fn drain(refresh: &mut ModelCatalogRefresh, state: &mut EditorState) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if refresh.poll_into(state) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

#[test]
fn a_second_open_inside_the_ttl_does_not_reprobe() {
    let mut refresh = ModelCatalogRefresh::new();
    let now = Instant::now();
    let connected = [false, true, false, false, false, false, false];

    assert!(refresh.request_with(connected, now, |_| Vec::new()));
    let mut state = EditorState::default();
    assert!(drain(&mut refresh, &mut state));

    // Reflexive re-open a few seconds later: the catalog is still fresh.
    assert!(
        !refresh.request_with(connected, now + Duration::from_secs(30), |_| {
            panic!("a refresh inside the TTL must not spawn a probe")
        })
    );
    assert!(!refresh.is_pending());
}

#[test]
fn an_open_past_the_ttl_reprobes() {
    let mut refresh = ModelCatalogRefresh::new();
    let now = Instant::now();
    let connected = [false, true, false, false, false, false, false];

    assert!(refresh.request_with(connected, now, |_| Vec::new()));
    let mut state = EditorState::default();
    assert!(drain(&mut refresh, &mut state));

    assert!(refresh.request_with(connected, now + MODEL_CATALOG_TTL, |_| Vec::new()));
}

#[test]
fn a_refresh_is_dropped_while_one_is_already_in_flight() {
    let mut refresh = ModelCatalogRefresh::new();
    let now = Instant::now();
    let connected = [false, true, false, false, false, false, false];

    assert!(refresh.request_with(connected, now, |_| Vec::new()));
    assert!(refresh.is_pending());
    assert!(
        !refresh.request_with(connected, now + MODEL_CATALOG_TTL * 2, |_| {
            panic!("the single job slot must absorb a concurrent request")
        }),
        "a second request must not spawn a rival worker"
    );
}

#[test]
fn only_verified_connected_providers_are_probed() {
    let mut refresh = ModelCatalogRefresh::new();
    let mut state = EditorState::default();
    connect(&mut state, AgentProvider::CodexCli);
    let (tx, rx) = mpsc::channel();

    assert!(refresh.request_with(
        state.editor_ui.agent_settings.verified_connected_mask(),
        Instant::now(),
        move |mask| {
            tx.send(mask).expect("mask");
            Vec::new()
        },
    ));
    assert!(drain(&mut refresh, &mut state));

    assert_eq!(
        rx.recv().expect("the worker ran"),
        [false, true, false, false, false, false, false],
        "a provider the user never connected must not be probed"
    );
}

#[test]
fn nothing_connected_means_no_worker() {
    let mut refresh = ModelCatalogRefresh::new();
    assert!(!refresh.request_with([false; 7], Instant::now(), |_| {
        panic!("no connected provider means nothing to discover")
    }));
}

#[test]
fn a_failed_refresh_keeps_the_previously_listed_models() {
    let mut refresh = ModelCatalogRefresh::new();
    let mut state = EditorState::default();
    connect(&mut state, AgentProvider::CodexCli);
    seed(&mut state, AgentProvider::CodexCli, "gpt-5.5");
    state.rebuild_chat_models();

    assert!(refresh.request_with(
        state.editor_ui.agent_settings.verified_connected_mask(),
        Instant::now(),
        // Every probe under the provider failed — CLI mid-upgrade, machine
        // offline, whatever. The catalog comes back empty.
        |_| Vec::new(),
    ));
    assert!(drain(&mut refresh, &mut state));

    assert_eq!(catalog_values(&state), ["gpt-5.5"]);
    assert!(
        state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.value == "gpt-5.5"),
        "a silent refresh failure must never empty the picker the user is looking at"
    );
}

#[test]
fn landing_replaces_the_refreshed_slice_and_selection_follows() {
    let mut refresh = ModelCatalogRefresh::new();
    let mut state = EditorState::default();
    connect(&mut state, AgentProvider::ClaudeCode);
    connect(&mut state, AgentProvider::CodexCli);
    seed(&mut state, AgentProvider::ClaudeCode, "claude-sonnet-4-6");
    seed(&mut state, AgentProvider::CodexCli, "gpt-5.5");
    state.rebuild_chat_models();
    // The user is mid-session on the Claude model; the refresh below must
    // not move that selection out from under them.
    let selected = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.value == "claude-sonnet-4-6")
        .expect("seeded model");
    state.chat.selected_model = selected;

    let mut mask = [false; 7];
    mask[1] = true;
    assert!(refresh.request_with(mask, Instant::now(), |_| vec![
        ModelEntry::new(AiProvider::CodexCli, "gpt-5.6-sol", "GPT-5.6 Sol"),
        ModelEntry::new(AiProvider::CodexCli, "gpt-5.6-terra", "GPT-5.6 Terra"),
    ]));
    assert!(drain(&mut refresh, &mut state));

    assert_eq!(
        catalog_values(&state),
        ["claude-sonnet-4-6", "gpt-5.6-sol", "gpt-5.6-terra"],
        "the stale Codex entry is gone, Claude's untouched slice keeps its place"
    );
    assert_eq!(
        state
            .chat
            .selected_model_entry()
            .map(|entry| entry.value.as_str()),
        Some("claude-sonnet-4-6"),
        "the selection tracks the entry, not the index"
    );
}

/// End-to-end through a real external process: the worker execs a fake CLI
/// and the catalog it prints reaches the picker. Everything above injects a
/// pure closure; this one proves the async path survives a subprocess.
#[cfg(unix)]
#[test]
fn a_catalog_printed_by_a_real_cli_lands_in_the_picker() {
    use std::os::unix::fs::PermissionsExt;

    let script = std::env::temp_dir().join(format!(
        "openpencil-model-catalog-refresh-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::write(
        &script,
        "#!/bin/sh\nprintf 'gpt-5.6-sol\\ngpt-5.6-luna\\n'\n",
    )
    .expect("write");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let mut refresh = ModelCatalogRefresh::new();
    let mut state = EditorState::default();
    connect(&mut state, AgentProvider::CodexCli);
    seed(&mut state, AgentProvider::CodexCli, "gpt-5.5");
    state.rebuild_chat_models();

    let exe = script.clone();
    assert!(refresh.request_with(
        state.editor_ui.agent_settings.verified_connected_mask(),
        Instant::now(),
        move |_| {
            let output = std::process::Command::new(&exe).output().expect("fake cli");
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(|id| ModelEntry::new(AiProvider::CodexCli, id, id))
                .collect()
        },
    ));
    assert!(drain(&mut refresh, &mut state));
    let _ = std::fs::remove_file(&script);

    assert_eq!(catalog_values(&state), ["gpt-5.6-sol", "gpt-5.6-luna"]);
    assert!(state
        .chat
        .available_models
        .iter()
        .any(|entry| entry.value == "gpt-5.6-sol"));
}
