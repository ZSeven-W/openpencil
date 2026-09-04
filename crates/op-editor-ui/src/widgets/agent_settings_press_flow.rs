//! Agent-settings modal press dispatch shared by the native and web
//! widget hosts.
//!
//! Their twins (`widget_host/press_helpers.rs::
//! dispatch_agent_settings_press` on native, `widget_host/
//! agent_settings_press.rs` on the web) carried this ~50-arm
//! `AgentSettingsHit` match twice. Everything here is `EditorState`
//! mutation plus widget-layer hit-tests / lookups, so it stays
//! wasm32-clean; hosts hit-test the panel, call
//! [`apply_agent_settings_hit`], then run only the platform arm the
//! returned [`SettingsPressOutcome`] asks for.
//!
//! The two genuine host differences are parameters, not forks:
//! credential ownership (`SettingsCommitScope`, see
//! `op_editor_core::host_settings_commit`) and the platform effects the
//! outcome names (clipboard, URL launcher, session revoke, font
//! enumeration).

use op_editor_core::agent_settings::{
    ImageSearchField, ImageTestStatus, SettingsFocus, OPENVERSE_AUTH_DOCS_URL,
};
use op_editor_core::host_settings_commit::{commit_settings_focus, SettingsCommitScope};
use op_editor_core::{AccountState, AgentSettingsTab, ButtonPressTarget, EditorState};

use crate::widgets::agent_settings_fonts::FontsHit;
use crate::widgets::agent_settings_panel::AgentSettingsHit;
use crate::widgets::agent_settings_press_focus::focus_with_text;

pub use crate::widgets::agent_settings_press_focus::focus_image_gen_profile;

/// The platform arm a settings press still owes after the shared state
/// walk has run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsPress {
    /// Handled entirely in editor state.
    Handled,
    /// Modal chrome that hit no control — a blank press. The host
    /// commits + blurs every chrome text input under the modal.
    BlankPress,
    /// Put `0` on the platform clipboard (the MCP client-config JSON).
    CopyText(String),
    /// Open this help link in the user's browser.
    OpenUrl(&'static str),
    /// Sign-out row — display state is already `Anonymous`; the host
    /// revokes the device session through its platform transport.
    SignOut,
}

/// [`SettingsPress`] plus the one cross-cutting follow-up that isn't a
/// single arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPressOutcome {
    /// The platform arm to run.
    pub effect: SettingsPress,
    /// The host must re-derive the Settings Fonts-tab data. Native
    /// enumerates system fonts synchronously; the web host schedules
    /// CanvasKit's asynchronous `queryLocalFonts` — hence a flag rather
    /// than shared code.
    pub refresh_fonts: bool,
}

impl SettingsPressOutcome {
    pub(crate) fn handled() -> Self {
        Self {
            effect: SettingsPress::Handled,
            refresh_fonts: false,
        }
    }

    pub(crate) fn effect(effect: SettingsPress) -> Self {
        Self {
            effect,
            refresh_fonts: false,
        }
    }

    pub(crate) fn refresh_fonts(mut self) -> Self {
        self.refresh_fonts = true;
        self
    }
}

/// True when `hit` moves keyboard focus into a settings text field —
/// the presses whose caret should land at the tapped glyph. Both hosts
/// use this to route the tap through the panel's byte-offset mapping
/// after the shared focus transition reseeds the draft (which parks the
/// caret at the end).
pub fn is_settings_focus_press(hit: AgentSettingsHit) -> bool {
    matches!(
        hit,
        AgentSettingsHit::FocusBuiltinAgent { .. }
            | AgentSettingsHit::FocusBuiltinAgentDraft(_)
            | AgentSettingsHit::FocusAcpAgent { .. }
            | AgentSettingsHit::FocusAcpAgentDraft(_)
            | AgentSettingsHit::FocusSearchField(_)
            | AgentSettingsHit::FocusGenConfig { .. }
            | AgentSettingsHit::FocusMcpPort
    )
}

/// Record the press-feedback wash target for `hit`. Split out because
/// both hosts do it before the match, from the same widget mapping.
pub fn record_pressed_button(state: &mut EditorState, hit: AgentSettingsHit) {
    state.editor_ui.pressed_button = crate::widgets::editor_state_ext::agent_settings_button(hit)
        .map(ButtonPressTarget::AgentSettings);
}

/// Apply an agent-settings modal hit. Always consumes the press.
pub fn apply_agent_settings_hit(
    state: &mut EditorState,
    hit: AgentSettingsHit,
    scope: SettingsCommitScope,
    now_ms: u64,
) -> SettingsPressOutcome {
    record_pressed_button(state, hit);
    let commit = |state: &mut EditorState| {
        commit_settings_focus(state, scope, now_ms);
    };
    match hit {
        AgentSettingsHit::Close | AgentSettingsHit::Outside => {
            commit(state);
            state.editor_ui.close_font_picker();
            state.editor_ui.agent_settings_open = false;
            // A modal dismissed mid-drag must not resume dragging when
            // it reopens (only the native twin cleared this).
            state.editor_ui.agent_settings_drag = None;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SelectTab(tab) => {
            commit(state);
            state.editor_ui.close_font_picker();
            state.editor_ui.agent_settings.tab = tab;
            state.editor_ui.agent_settings.scroll_y.offset = 0.0;
            let out = SettingsPressOutcome::handled();
            if matches!(tab, AgentSettingsTab::Fonts) {
                out.refresh_fonts()
            } else {
                out
            }
        }
        AgentSettingsHit::Connect(provider) => {
            let settings = &mut state.editor_ui.agent_settings;
            if settings.provider_verified_connected(provider) {
                // Disconnect mirrors the TS `disconnectProvider` store
                // action — reset the card, no probe.
                settings.disconnect_provider(provider);
            } else if !settings.provider_probe_in_flight(provider) {
                // Connect runs a REAL probe (installed? auth? model
                // list?) — raise the request seam; the host pump drains
                // it and lands the outcome + models on a later frame.
                // Re-presses while Probing are ignored (TS disables the
                // button via isConnecting).
                settings.begin_provider_connect(provider);
            }
            // Either lifecycle edge changes which models the chat picker
            // may list — re-derive from the discovered catalog.
            state.rebuild_chat_models();
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleMcpServer => {
            commit(state);
            state.editor_ui.agent_settings.mcp_server.running ^= true;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleMcpCli(cli) => {
            // `mcp_cli_enabled` is indexed by `McpCli::ALL` order; the row
            // this hit came from is `McpCli::DISPLAY` order, so resolve the
            // storage slot through `McpCli::index`.
            let idx = cli.index();
            let settings = &mut state.editor_ui.agent_settings;
            settings.mcp_cli_enabled[idx] ^= true;
            if settings.mcp_cli_enabled[idx] {
                settings.mcp_server.running = true;
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::CopyMcpClientConfig => {
            commit(state);
            state
                .editor_ui
                .agent_settings
                .mcp_client_config_copied_at_ms = Some(now_ms);
            let config = state
                .editor_ui
                .agent_settings
                .mcp_client_config_clipboard_text();
            SettingsPressOutcome::effect(SettingsPress::CopyText(config))
        }
        AgentSettingsHit::Fonts(hit) => apply_fonts_hit(state, hit),
        AgentSettingsHit::ToggleImagesAdvanced => {
            state.editor_ui.agent_settings.images_advanced_open ^= true;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::FocusSearchField(field) => {
            commit(state);
            let text = match field {
                ImageSearchField::ClientId => {
                    state.editor_ui.agent_settings.openverse_client_id.clone()
                }
                ImageSearchField::ClientSecret => state
                    .editor_ui
                    .agent_settings
                    .openverse_client_secret
                    .clone(),
            };
            focus_with_text(state, SettingsFocus::ImageSearch(field), text, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::OpenImageRegisterLink => {
            commit(state);
            SettingsPressOutcome::effect(SettingsPress::OpenUrl(OPENVERSE_AUTH_DOCS_URL))
        }
        AgentSettingsHit::TestImageSearch => {
            commit(state);
            let settings = &mut state.editor_ui.agent_settings;
            let has_client_id = !settings.openverse_client_id.trim().is_empty();
            let has_client_secret = !settings.openverse_client_secret.trim().is_empty();
            settings.images_search_ready = true;
            settings.images_search_test_status = if has_client_id && has_client_secret {
                ImageTestStatus::Testing
            } else {
                ImageTestStatus::Invalid
            };
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleAutoUpdate => {
            state.editor_ui.agent_settings.auto_update_enabled ^= true;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SelectThemeMode(mode) => {
            state.editor_ui.theme_mode = mode;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SelectPencilCursor(style) => {
            state.editor_ui.pencil_cursor_style = style;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::ToggleExperimental => {
            let settings = &mut state.editor_ui.agent_settings;
            settings.experimental_features_enabled ^= true;
            let enabled = settings.experimental_features_enabled;
            // Preview graduated out of this gate (2026-07) — it no
            // longer force-exits when the gate turns off. Widget-config
            // (the property panel's Widget section) is still gated: drop
            // any stale Widget property focus so hiding the section is a
            // clean cut — a lingering `PropertyFocus::Widget*` could
            // otherwise still commit through dispatch.
            if !enabled {
                state.ui.property_focus = None;
            }
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::OpenLoginModal => {
            // Switching modals is also a Settings dismissal: land the active
            // draft, release keyboard ownership, and close a missing-font
            // picker belonging to the Settings Fonts tab before login appears.
            commit(state);
            state.editor_ui.close_font_picker();
            state.editor_ui.escape_agent_settings_modal();
            state.editor_ui.login_modal_open = true;
            state.editor_ui.login_modal_hover = None;
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::SignOutAccount => {
            state.editor_ui.account = AccountState::Anonymous;
            let _ = crate::collab_avatar_runtime::register_account_avatar_url(None);
            SettingsPressOutcome::effect(SettingsPress::SignOut)
        }
        AgentSettingsHit::FocusMcpPort => {
            commit(state);
            let text = state.editor_ui.agent_settings.mcp_server.port.to_string();
            focus_with_text(state, SettingsFocus::McpPort, text, now_ms);
            SettingsPressOutcome::handled()
        }
        AgentSettingsHit::Inside => {
            // Modal chrome that hit no control — blank press; commits the
            // focused settings input (and blurs the rest of the chrome
            // inputs under the modal).
            SettingsPressOutcome::effect(SettingsPress::BlankPress)
        }
        _ => {
            crate::widgets::agent_settings_press_entries::apply_entry_hit(state, hit, scope, now_ms)
        }
    }
}

/// Settings Fonts-tab press dispatch. `SelectFont` resolves the picked
/// row against the SAME entries list the picker painted, then rewrites
/// every node that used the missing family.
fn apply_fonts_hit(state: &mut EditorState, hit: FontsHit) -> SettingsPressOutcome {
    match hit {
        FontsHit::ChooseFont(row) => {
            state
                .editor_ui
                .open_missing_font_picker(row, op_editor_core::MissingFontSurface::Settings);
            SettingsPressOutcome::handled()
        }
        FontsHit::SelectFont(index) => {
            let replacement =
                crate::widgets::property_panel_dispatch_support::font_picker_family_at(
                    state, index,
                );
            let expected = match state.editor_ui.font_picker_purpose {
                Some(op_editor_core::FontPickerPurpose::MissingFont { row, .. }) => state
                    .editor_ui
                    .missing_fonts_prompt
                    .as_ref()
                    .and_then(|prompt| prompt.entries.get(row))
                    .map(|entry| entry.family.clone()),
                _ => None,
            };
            if let (Some(from), Some(to)) = (expected, replacement) {
                let _ = state.apply(op_editor_core::EditorCommand::ReplaceFontFamily { from, to });
            }
            state.editor_ui.close_font_picker();
            SettingsPressOutcome::handled().refresh_fonts()
        }
        FontsHit::ImportFont(row) => {
            state.editor_ui.missing_fonts_import_row = Some(row);
            SettingsPressOutcome::handled()
        }
        FontsHit::ClosePicker => {
            state.editor_ui.close_font_picker();
            SettingsPressOutcome::handled()
        }
        FontsHit::RemoveImportedFont(index) => {
            if let Some(family) = state.editor_ui.imported_font_families.get(index).cloned() {
                state.editor_ui.pending_font_remove = Some(family);
            }
            SettingsPressOutcome::handled()
        }
        FontsHit::PickerInside | FontsHit::None => SettingsPressOutcome::handled(),
    }
}
