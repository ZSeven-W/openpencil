//! End-to-end coverage for a saved local ACP agent.
//!
//! A harness-free example binary acts as the fake agent and is launched
//! through the same `tokio::process::Command` stdio path as a user-configured
//! local agent. That keeps the fixture cross-platform while proving the
//! process boundary instead of stopping at an in-memory duplex stream.

use super::*;
use op_editor_core::{AcpAgentConnectPhase, ChatRole};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const FIXTURE_ENV: &str = "OPENPENCIL_LOCAL_ACP_E2E_FIXTURE";
const FIXTURE_AGENT_NAME: &str = "OpenPencil Local ACP Fixture";
const FIXTURE_AGENT_VERSION: &str = "1.0";
const FIXTURE_PROMPT: &str = "LOCAL_ACP_E2E_7C1: reply with the fixture greeting.";
const FIXTURE_REPLY: &str = "Hello from the real local ACP subprocess.";
const FIXTURE_MCP_PORT: u16 = 4_123;

#[test]
fn local_acp_save_connect_initialize_picker_prompt_disconnect_e2e() {
    let executable = fixture_executable();
    let mut app = DesktopApp::new(None);

    // DesktopApp restores machine settings even in tests. Reset only the
    // in-memory agent/chat catalog so this process E2E is deterministic and
    // never reads or writes a user's configured agents.
    {
        let state = app.host.editor_state_mut();
        state.editor_ui.agent_settings = Default::default();
        state.chat.discovered_models.clear();
        state.chat.available_models.clear();
        state.chat.messages.clear();
        state.chat.pending_send = None;
    }

    // Save through the same draft seam used by Settings → Add ACP Agent.
    let agent_id = {
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.begin_acp_agent_draft();
        let draft = settings
            .acp_agent_draft
            .as_mut()
            .expect("draft should be created");
        draft.display_name = FIXTURE_AGENT_NAME.into();
        draft.command = executable.to_string_lossy().into_owned();
        draft.args.clear();
        draft.env.insert(FIXTURE_ENV.into(), "enabled".into());
        settings
            .save_acp_agent_draft()
            .expect("ready local ACP draft should save")
    };
    let agent_index = app
        .host
        .editor_state()
        .editor_ui
        .agent_settings
        .acp_agents
        .iter()
        .position(|agent| agent.id == agent_id)
        .expect("saved ACP agent");

    app.host.editor_state_mut().rebuild_chat_models();
    assert!(
        app.host
            .editor_state()
            .chat
            .available_models
            .iter()
            .all(|model| model.value != format!("acp:{agent_id}")),
        "saving a configuration must not make it selectable before a real probe"
    );

    // Explicit Connect must spawn the command and complete ACP initialize
    // before the current configuration is marked verified.
    assert_eq!(
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_acp_agent_connect(agent_index)
            .as_deref(),
        Some(agent_id.as_str())
    );
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        app.drain_acp_agent_connect();
        if app
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .acp_agent_verified_connected(&agent_id)
        {
            break;
        }
        assert!(
            Instant::now() < connect_deadline,
            "local ACP connect timed out: {:?}",
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .acp_agent_connection_for(&agent_id)
        );
        thread::sleep(Duration::from_millis(10));
    }

    let connection = app
        .host
        .editor_state()
        .editor_ui
        .agent_settings
        .acp_agent_connection_for(&agent_id);
    assert_eq!(connection.phase, AcpAgentConnectPhase::Connected);
    assert_eq!(
        connection.info.as_deref(),
        Some(format!("{FIXTURE_AGENT_NAME} {FIXTURE_AGENT_VERSION}").as_str())
    );

    // Rebuilding after the verified initialize adds the ACP entry; selecting
    // it through the open picker exercises the actual picker/model seam.
    let model_index = app
        .host
        .editor_state()
        .chat
        .available_models
        .iter()
        .position(|model| model.value == format!("acp:{agent_id}"))
        .expect("verified ACP agent should be present in the picker");
    {
        let state = app.host.editor_state_mut();
        assert!(state.editor_ui.toggle_chat_model_picker());
        assert!(state.editor_ui.chat_model_picker.open);
        state.select_chat_model(model_index);
        assert!(!state.editor_ui.chat_model_picker.open);
        assert_eq!(
            state
                .chat
                .selected_model_entry()
                .map(|model| model.value.as_str()),
            Some(format!("acp:{agent_id}").as_str())
        );
        state.editor_ui.agent_settings.mcp_server.running = true;
        state.editor_ui.agent_settings.mcp_server.port = FIXTURE_MCP_PORT;
        state.chat.set_input_text(FIXTURE_PROMPT);
        assert!(state.chat.begin_send());
    }

    // ACP chat reconnects to the saved command, then drives session/new and
    // session/prompt. The child refuses malformed MCP/session/prompt payloads,
    // so receiving this reply proves the full wire sequence.
    assert!(crate::chat_session::launch_if_pending(
        &mut app.host,
        &mut app.current_chat,
        &mut app.current_design,
    ));
    assert!(
        app.current_chat.is_some(),
        "ACP selection should launch a real chat session"
    );
    let prompt_deadline = Instant::now() + Duration::from_secs(15);
    while app.current_chat.is_some() {
        crate::chat_session::pump(
            &mut app.host,
            &mut app.current_chat,
            None,
            None,
            (1_200.0, 800.0),
        );
        assert!(
            Instant::now() < prompt_deadline,
            "local ACP prompt timed out; transcript: {:?}",
            app.host.editor_state().chat.messages
        );
        if app.current_chat.is_some() {
            thread::sleep(Duration::from_millis(10));
        }
    }
    let assistant = app
        .host
        .editor_state()
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
        .expect("assistant response");
    assert_eq!(assistant.content, FIXTURE_REPLY);
    assert!(!assistant.streaming);

    // Explicit disconnect invalidates the verified runtime marker and removes
    // the agent from the picker on the same rebuild path used by the UI.
    {
        let state = app.host.editor_state_mut();
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .disconnect_acp_agent(agent_index)
                .as_deref(),
            Some(agent_id.as_str())
        );
        state.rebuild_chat_models();
    }
    let state = app.host.editor_state();
    let settings = &state.editor_ui.agent_settings;
    assert!(!settings.acp_agent_verified_connected(&agent_id));
    assert_eq!(
        settings.acp_agent_connection_for(&agent_id),
        Default::default()
    );
    assert!(state
        .chat
        .available_models
        .iter()
        .all(|model| model.value != format!("acp:{agent_id}")));
}

/// Cargo compiles examples during an ordinary `cargo test` without replacing
/// their `main` functions with libtest. Resolve that clean executable next to
/// the current profile's `deps` directory. `cargo build --example` creates an
/// un-hashed executable; `cargo test` creates a fingerprinted one, so accept
/// both layouts (including Windows' executable suffix) and prefer the newest.
fn fixture_executable() -> PathBuf {
    let current = std::env::current_exe().expect("resolve current test executable");
    let deps_dir = current
        .parent()
        .expect("test executable should live in a deps directory");
    assert_eq!(
        deps_dir.file_name().and_then(|name| name.to_str()),
        Some("deps"),
        "unexpected Cargo test executable layout: {}",
        current.display()
    );
    let examples_dir = deps_dir
        .parent()
        .expect("deps directory should have a profile parent")
        .join("examples");
    let executable_suffix = std::env::consts::EXE_SUFFIX;
    std::fs::read_dir(&examples_dir)
        .unwrap_or_else(|error| {
            panic!(
                "read ACP fixture example directory {}: {error}",
                examples_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            let Some(stem) = name.strip_suffix(executable_suffix) else {
                return false;
            };
            stem == "op-acp-test-agent"
                || stem
                    .strip_prefix("op_acp_test_agent-")
                    .is_some_and(|hash| hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        })
        .max_by_key(|path| {
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
        .unwrap_or_else(|| {
            panic!(
                "ACP fixture example was not built under {}; run the test without a Cargo target-selection flag",
                examples_dir.display()
            )
        })
}
