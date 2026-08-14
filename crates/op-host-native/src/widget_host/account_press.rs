//! Sign-in modal + signed-in account-dropdown press dispatchers.
//!
//! The hit-test + state walk lives in the shared
//! `op_editor_ui::widgets::account_press_flow`; this file keeps only the
//! native platform arms — the dev fake-login fast path, the real
//! `op-auth-bridge` browser pairing, and the session revoke.

use super::WidgetHostNative;
use op_editor_ui::widgets::account_press_flow::{
    self as account_flow, AccountMenuPress, LoginModalPress,
};

impl WidgetHostNative {
    /// Sign-in-modal press dispatcher.
    pub(in crate::widget_host) fn dispatch_login_modal_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        match account_flow::press_login_modal(&mut self.editor_state, x, y, viewport_w, viewport_h)
        {
            LoginModalPress::Closed => self.cancel_auth_login(),
            LoginModalPress::Dismissed => {
                self.blur_text_inputs_on_blank_press();
                self.cancel_auth_login();
            }
            LoginModalPress::SignIn => self.begin_login_from_modal(),
            LoginModalPress::Inside => {
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.mark_dirty();
    }

    /// Native sign-in arm: dev fake login → real browser pairing →
    /// honest stub, in that order.
    fn begin_login_from_modal(&mut self) {
        if dev_fake_login_enabled() {
            // Dev/demo fast path — exercises the signed-in UI without a
            // backend.
            self.editor_state.editor_ui.account =
                op_editor_core::AccountState::dev_fake_signed_in();
            account_flow::close_login_modal(&mut self.editor_state);
        } else if op_auth_bridge::available() {
            // Real flow: browser pairing against zseven-sso via the
            // proprietary client library; progress lands in
            // `login_modal_status` through `poll_auth`.
            self.begin_browser_login();
        } else {
            // Honest stub (no auth library linked): no session is
            // created — just reveal the "coming soon" note.
            self.editor_state.editor_ui.login_modal_stub_hint_shown = true;
        }
    }

    /// Signed-in account-dropdown press dispatcher.
    pub(in crate::widget_host) fn dispatch_account_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        _viewport_h: f32,
    ) {
        match account_flow::press_account_menu(
            &mut self.editor_state,
            x,
            y,
            viewport_w,
            self.now_ms,
        ) {
            AccountMenuPress::Vanished => return,
            AccountMenuPress::OpenMcpTokens => {
                // Desktop never emits this: the "MCP Tokens" row is gated
                // behind `account_mcp_tokens_entry`, which only the online
                // web host sets. The local MCP is tokenless and there is no
                // hub portal to open, so there is nothing to do here.
            }
            AccountMenuPress::SignOut => {
                // Revoke the device session (background thread inside
                // the library; an inert no-op in stub builds).
                op_auth_bridge::sign_out();
                self.forget_auth_session_profile();
            }
            AccountMenuPress::Dismissed => {
                // Outside click — blank press: dismiss + blur inputs.
                self.blur_text_inputs_on_blank_press();
            }
            AccountMenuPress::Handled => {
                // The only handled account-menu row opens Settings. The shared
                // flow has already made that modal visible, so release every
                // Property-owned input now before the next keyboard event can
                // reach a field hidden underneath it.
                self.release_property_keyboard_owner();
            }
            AccountMenuPress::Ignored => {}
        }
        self.mark_dirty();
    }
}

/// Dev/demo fast path: `OPENPENCIL_DEV_FAKE_LOGIN=1` makes the sign-in
/// modal's primary button set `AccountState::SignedIn` immediately
/// instead of showing the honest "coming soon" stub. Lets the topbar
/// avatar, its dropdown, and the settings Account tab be exercised
/// end-to-end before the real OIDC backend lands. Never read outside
/// this native host, so it can't leak into the wasm web build.
fn dev_fake_login_enabled() -> bool {
    std::env::var(op_auth_bridge::ENV_DEV_FAKE_LOGIN).as_deref() == Ok("1")
}
