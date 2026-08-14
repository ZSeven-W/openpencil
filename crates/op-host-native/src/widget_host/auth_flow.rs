//! Browser device-login flow driver: bridges `op-auth-bridge` status
//! polls into editor UI state. The desktop event loop calls
//! [`WidgetHostNative::poll_auth`] from its background-drain pass and
//! opens URLs drained via [`WidgetHostNative::take_pending_browser_url`]
//! — this module never spawns processes itself.

use super::WidgetHostNative;
use op_auth_bridge::AuthStatus;
use op_editor_core::{AccountState, LoginFlowError, LoginFlowStatus};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

const RESTORED_SESSION_REFRESH_WINDOW: Duration = Duration::from_secs(30);

impl WidgetHostNative {
    /// Configure the process-global auth runtime for an embedded mobile shell
    /// and mirror any persisted session into this editor host.
    ///
    /// The caller owns platform storage protection and supplies a private app
    /// directory. The SSO origin remains pinned inside `op-auth-bridge`.
    pub fn configure_mobile_auth(
        &mut self,
        storage_dir: std::path::PathBuf,
        device_name: String,
        app_version: String,
    ) -> bool {
        let config = op_auth_bridge::mobile_init_config(storage_dir, device_name, app_version);
        if !op_auth_bridge::available() || !op_auth_bridge::init_mobile(&config) {
            self.editor_state.editor_ui.account_ui_available = false;
            return false;
        }

        self.editor_state.editor_ui.account_ui_available = true;
        let restored = op_auth_bridge::restore();
        if let AuthStatus::SignedIn {
            display_name,
            username,
            avatar_url,
            ..
        } = op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE)
        {
            let _ = self.adopt_auth_session_profile(display_name, username, avatar_url);
        }
        if restored {
            self.arm_auth_session_refresh();
        }
        self.mark_dirty();
        true
    }

    /// Start the browser device-login flow (called from the sign-in
    /// modal's primary button when a real auth backend is linked).
    pub(in crate::widget_host) fn begin_browser_login(&mut self) {
        if self.auth_login_handle.is_some() {
            return; // one flow at a time; the running one owns the modal
        }
        let handle = op_auth_bridge::login_begin();
        if handle == 0 {
            self.editor_state.editor_ui.login_modal_status =
                Some(LoginFlowStatus::Failed(LoginFlowError::Unavailable));
            return;
        }
        self.auth_login_handle = Some(handle);
        self.auth_browser_opened = false;
        self.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser);
        self.editor_state.editor_ui.login_modal_stub_hint_shown = false;
    }

    /// Abort an in-flight login (modal closed / outside click). The
    /// bridge flow settles into Canceled on its own thread; UI state
    /// resets immediately.
    pub fn cancel_auth_login(&mut self) {
        self.cancel_auth_login_with(op_auth_bridge::cancel);
    }

    /// Engine-teardown hook. It is deliberately idempotent so both an
    /// explicit platform cancellation and final engine destruction can run
    /// without sending duplicate cancellation to the backend.
    pub fn shutdown_auth(&mut self) {
        self.shutdown_auth_with(op_auth_bridge::cancel);
    }

    fn shutdown_auth_with(&mut self, cancel: impl FnOnce(u64)) {
        self.cancel_auth_login_with(cancel);
    }

    fn cancel_auth_login_with(&mut self, cancel: impl FnOnce(u64)) {
        if let Some(handle) = self.auth_login_handle.take() {
            cancel(handle);
        }
        self.auth_browser_opened = false;
        self.auth_pending_browser_url = None;
        self.editor_state.editor_ui.login_modal_status = None;
    }

    /// Whether auth status still needs periodic polling — drives the desktop
    /// event loop's wakeups while a login is active or a restored credential
    /// is completing its asynchronous profile revalidation.
    pub fn auth_flow_active(&self) -> bool {
        self.auth_login_handle.is_some()
            || self
                .auth_session_refresh_deadline
                .is_some_and(|deadline| Instant::now() < deadline)
    }

    /// Whether a user-initiated login (as opposed to session refresh) is live.
    pub fn auth_login_active(&self) -> bool {
        self.auth_login_handle.is_some()
    }

    /// Keep the idle desktop awake briefly after optimistic session restore.
    ///
    /// The private runtime adopts the persisted credential synchronously, then
    /// refreshes its profile from `/api/v1/account` on a background thread.
    /// Without this bounded wake window an otherwise idle app can retain the
    /// older display name, username, or avatar until the next user event.
    pub fn arm_auth_session_refresh(&mut self) {
        self.auth_session_refresh_deadline = Some(Instant::now() + RESTORED_SESSION_REFRESH_WINDOW);
    }

    /// Mirror one authenticated profile snapshot into display-only editor
    /// state. Returns whether the account text or avatar source changed.
    pub fn adopt_auth_session_profile(
        &mut self,
        display_name: String,
        username: Option<String>,
        avatar_url: Option<String>,
    ) -> bool {
        let account = AccountState::signed_in_profile(display_name, username);
        let account_changed = self.editor_state.editor_ui.account != account;
        if account_changed {
            self.editor_state.editor_ui.account = account;
        }

        let avatar_revision = avatar_source_revision(avatar_url.as_deref());
        let avatar_changed = self.auth_account_avatar_revision != Some(avatar_revision);
        if avatar_changed {
            let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(
                avatar_url.as_deref(),
            );
            self.auth_account_avatar_revision = Some(avatar_revision);
        }
        account_changed || avatar_changed
    }

    /// The verification URL the host should open in the system browser,
    /// at most once per flow.
    pub fn take_pending_browser_url(&mut self) -> Option<String> {
        self.auth_pending_browser_url.take()
    }

    /// Poll the in-flight login flow (and the restored session) and fold
    /// status changes into editor UI state. Returns whether anything
    /// visible changed.
    pub fn poll_auth(&mut self) -> bool {
        let Some(handle) = self.auth_login_handle else {
            return self.mirror_session_snapshot();
        };
        let previous = self.editor_state.editor_ui.login_modal_status;
        match op_auth_bridge::poll(handle) {
            AuthStatus::Idle | AuthStatus::Starting => {
                self.editor_state.editor_ui.login_modal_status =
                    Some(LoginFlowStatus::WaitingBrowser);
            }
            AuthStatus::WaitingApproval { verification_uri } => {
                self.editor_state.editor_ui.login_modal_status =
                    Some(LoginFlowStatus::WaitingApproval);
                if !self.auth_browser_opened && !verification_uri.is_empty() {
                    self.auth_browser_opened = true;
                    self.auth_pending_browser_url = Some(verification_uri);
                }
            }
            AuthStatus::Exchanging => {
                self.editor_state.editor_ui.login_modal_status = Some(LoginFlowStatus::Exchanging);
            }
            AuthStatus::SignedIn {
                display_name,
                username,
                avatar_url,
                ..
            } => {
                let _ = self.adopt_auth_session_profile(display_name, username, avatar_url);
                let ui = &mut self.editor_state.editor_ui;
                ui.login_modal_status = None;
                ui.login_modal_open = false;
                ui.login_modal_hover = None;
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
                self.auth_session_refresh_deadline = None;
                self.mark_dirty();
                return true;
            }
            AuthStatus::Error { code } => {
                let ui = &mut self.editor_state.editor_ui;
                ui.login_modal_status = Some(LoginFlowStatus::Failed(match code.as_str() {
                    "denied" => LoginFlowError::Denied,
                    "expired" => LoginFlowError::Expired,
                    _ => LoginFlowError::Unavailable,
                }));
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
            }
            AuthStatus::Canceled => {
                self.editor_state.editor_ui.login_modal_status = None;
                self.auth_login_handle = None;
                self.auth_browser_opened = false;
            }
        }
        let changed = self.editor_state.editor_ui.login_modal_status != previous
            || self.auth_pending_browser_url.is_some();
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Mirror the steady session snapshot after optimistic startup restore.
    /// This catches both a rejected credential and fresher profile fields
    /// published by the background revalidation worker.
    fn mirror_session_snapshot(&mut self) -> bool {
        if !op_auth_bridge::available() {
            return false;
        }
        let changed = match op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE) {
            AuthStatus::SignedIn {
                display_name,
                username,
                avatar_url,
                ..
            } => self.adopt_auth_session_profile(display_name, username, avatar_url),
            AuthStatus::Idle => {
                self.auth_session_refresh_deadline = None;
                let account_changed =
                    self.editor_state.editor_ui.account != AccountState::Anonymous;
                let avatar_changed = self.auth_account_avatar_revision.take().is_some();
                if account_changed {
                    self.editor_state.editor_ui.account = AccountState::Anonymous;
                }
                if avatar_changed {
                    let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(None);
                }
                account_changed || avatar_changed
            }
            _ => false,
        };
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Forget host-local comparison state after either native sign-out path.
    pub(in crate::widget_host) fn forget_auth_session_profile(&mut self) {
        self.auth_session_refresh_deadline = None;
        self.auth_account_avatar_revision = None;
        let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(None);
    }
}

fn avatar_source_revision(url: Option<&str>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_session_refresh_window_is_a_bounded_work_signal() {
        let mut host = WidgetHostNative::new();
        assert!(!host.auth_flow_active());

        host.arm_auth_session_refresh();
        assert!(host.auth_flow_active());

        host.auth_session_refresh_deadline = Some(Instant::now() - Duration::from_millis(1));
        assert!(!host.auth_flow_active());
    }

    #[test]
    fn adopting_a_profile_prefers_username_and_is_idempotent() {
        let mut host = WidgetHostNative::new();
        // Avoid touching the process-global avatar registry in this unit test.
        host.auth_account_avatar_revision = Some(avatar_source_revision(None));

        assert!(host.adopt_auth_session_profile(
            "Kay Shen".to_string(),
            Some("kayshen_7".to_string()),
            None,
        ));
        assert_eq!(
            host.editor_state.editor_ui.account,
            AccountState::SignedIn {
                display_name: "Kay Shen".to_string(),
                username: "kayshen_7".to_string(),
            }
        );
        assert!(!host.adopt_auth_session_profile(
            "Kay Shen".to_string(),
            Some("kayshen_7".to_string()),
            None,
        ));
    }

    #[test]
    fn forgetting_a_session_clears_avatar_work_and_comparison_state() {
        let mut host = WidgetHostNative::new();
        assert!(
            op_editor_ui::collab_avatar_runtime::register_account_avatar_url(Some(
                "https://cdn.example/profile.png",
            ))
        );
        assert!(op_editor_ui::collab_avatar_runtime::has_pending_collab_avatar_requests());
        host.auth_account_avatar_revision = Some(avatar_source_revision(Some(
            "https://cdn.example/profile.png",
        )));
        host.arm_auth_session_refresh();

        host.forget_auth_session_profile();

        assert!(host.auth_account_avatar_revision.is_none());
        assert!(!host.auth_flow_active());
        assert!(!op_editor_ui::collab_avatar_runtime::has_pending_collab_avatar_requests());
    }

    #[test]
    fn shutting_down_auth_invokes_backend_cancel_once() {
        let mut host = WidgetHostNative::new();
        host.auth_login_handle = Some(42);
        let mut canceled = Vec::new();

        host.shutdown_auth_with(|handle| canceled.push(handle));
        host.shutdown_auth_with(|handle| canceled.push(handle));

        assert_eq!(canceled, vec![42]);
        assert!(host.auth_login_handle.is_none());
    }
}
