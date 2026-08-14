//! Sign-in modal + signed-in account-dropdown press flow shared by the
//! native and web widget hosts.
//!
//! Their `widget_host/account_press.rs` twins used to carry this
//! hit-test + state walk twice; the only genuine difference is the
//! platform arm each press ends in (native talks to `op-auth-bridge`
//! directly, the web host queues a `PendingAuthAction` for the daemon
//! poll). Everything here is `EditorState` mutation plus widget-layer
//! hit-tests, so it stays wasm32-clean; hosts run only the arm the
//! returned outcome asks for and then `mark_dirty()`.

use op_editor_core::{AccountMenuRow, AccountState, ButtonPressTarget, EditorState};

use crate::widgets::account_menu::AccountMenu;
use crate::widgets::login_modal::{LoginModal, LoginModalHit};
use crate::Point2D;

/// What the host still owes after a sign-in-modal press. Every variant
/// consumes the press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginModalPress {
    /// Close button — the modal already closed itself; the host cancels
    /// any in-flight login flow.
    Closed,
    /// Outside press — as [`Self::Closed`], plus the host blurs chrome
    /// text inputs (blank-press parity).
    Dismissed,
    /// Primary sign-in button — the host runs its platform login arm.
    SignIn,
    /// Press on modal chrome — the host blurs chrome text inputs.
    Inside,
}

/// Sign-in-modal press: hit-test, record the pressed button for the
/// press-feedback wash, and close the modal on the two dismiss paths.
///
/// The blur / cancel tails are host-side and touch disjoint state
/// (text inputs, `login_modal_status`), so running them after this
/// returns matches the twins' original inline order.
pub fn press_login_modal(
    state: &mut EditorState,
    x: f32,
    y: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> LoginModalPress {
    let modal = LoginModal::for_editor(state);
    let panel_rect = modal.rect(viewport_width, viewport_height);
    let hit = modal.hit_test(panel_rect, Point2D::new(x, y));
    state.editor_ui.pressed_button = crate::widgets::editor_state_ext::login_modal_button(hit)
        .map(ButtonPressTarget::LoginModal);
    match hit {
        LoginModalHit::Close => {
            close_login_modal(state);
            LoginModalPress::Closed
        }
        LoginModalHit::Outside => {
            close_login_modal(state);
            LoginModalPress::Dismissed
        }
        LoginModalHit::SignIn => LoginModalPress::SignIn,
        LoginModalHit::Inside => LoginModalPress::Inside,
    }
}

/// Close the sign-in modal and drop its transient hover + stub-hint
/// state. Shared by the dismiss paths and by the hosts' sign-in arms
/// (the dev fake-login fast path closes the modal on success).
pub fn close_login_modal(state: &mut EditorState) {
    state.editor_ui.login_modal_open = false;
    state.editor_ui.login_modal_hover = None;
    state.editor_ui.login_modal_stub_hint_shown = false;
}

/// What the host still owes after an account-dropdown press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountMenuPress {
    /// Row handled entirely in editor state (Settings).
    Handled,
    /// MCP-tokens row — the menu closed; the host opens the hub portal's
    /// token page (online web only; other hosts never surface this row).
    OpenMcpTokens,
    /// Sign-out row — display state is already `Anonymous`; the host
    /// revokes the device session through its platform transport.
    SignOut,
    /// Outside press — the menu closed; the host blurs chrome text
    /// inputs (blank-press parity).
    Dismissed,
    /// Press inside the menu chrome but not on a row.
    Ignored,
    /// Account state flipped back to `Anonymous` out from under an open
    /// menu (shouldn't normally happen) — the menu closed defensively
    /// and the host returns without a repaint, as both twins did.
    Vanished,
}

/// Signed-in account-dropdown press: anchor the menu under the topbar
/// avatar button, hit-test it, and apply the row's editor-state effect.
pub fn press_account_menu(
    state: &mut EditorState,
    x: f32,
    y: f32,
    viewport_width: f32,
    now_ms: u64,
) -> AccountMenuPress {
    let Some(menu) = AccountMenu::for_editor_ui(&state.editor_ui) else {
        state.editor_ui.account_menu_open = false;
        return AccountMenuPress::Vanished;
    };
    let menu_rect =
        crate::widgets::touch_overlay_geometry::account_menu_rect(state, &menu, viewport_width);
    let point = Point2D::new(x, y);
    match menu.hit_test(menu_rect, point) {
        Some(AccountMenuRow::Settings) => {
            close_account_menu(state);
            state.editor_ui.agent_settings_open = true;
            state.editor_ui.agent_settings.tab =
                op_editor_core::agent_settings::AgentSettingsTab::Account;
            state.chat.blur_input(now_ms);
            AccountMenuPress::Handled
        }
        Some(AccountMenuRow::McpToken) => {
            close_account_menu(state);
            AccountMenuPress::OpenMcpTokens
        }
        Some(AccountMenuRow::SignOut) => {
            close_account_menu(state);
            state.editor_ui.account = AccountState::Anonymous;
            let _ = crate::collab_avatar_runtime::register_account_avatar_url(None);
            AccountMenuPress::SignOut
        }
        None => {
            if menu_rect.contains(point) {
                AccountMenuPress::Ignored
            } else {
                close_account_menu(state);
                AccountMenuPress::Dismissed
            }
        }
    }
}

/// Close the account dropdown and clear its row hover.
pub fn close_account_menu(state: &mut EditorState) {
    state.editor_ui.account_menu_open = false;
    state.editor_ui.account_menu_hover = None;
}
