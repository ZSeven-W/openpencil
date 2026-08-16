//! Drive the daemon's device-login proxy from the web shell.
//!
//! The wasm bundle ships no auth code. The SignIn press opens a
//! same-origin loading popup and fires `POST /api/auth/login/begin`
//! immediately (both inside the click's user-activation window); the
//! daemon holds that request until the pairing's verification URI is
//! known, and the response callback navigates the popup to it — no poll
//! cycle sits between the click and the sso page. The interval tick then
//! polls `GET /api/auth/login/status` for approval progress and folds it
//! into the same `login_modal_status` / `account` fields the desktop
//! host uses, so the login modal renders identically on both hosts.
//!
//! On startup one `GET /api/auth/status` seeds `account_ui_available`
//! and (when the daemon restored a shared session) the signed-in state;
//! it re-runs every ~30 s as a session health check.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use op_editor_core::{auth_routes, AccountState, LoginFlowError, LoginFlowStatus};

use crate::live_sync;
use crate::repaint_ctx::RepaintContext;
use crate::widget_host::PendingAuthAction;

/// Poll cadence while idle; each tick is a queue drain (usually empty)
/// plus, only during a login flow, one status GET.
const AUTH_POLL_INTERVAL_MS: i32 = 600;
/// Steady-state session health check — every N ticks (~30 s) re-fetch
/// `/api/auth/status` so a session revoked or expired daemon-side (or a
/// sign-in/out from the desktop GUI or another tab) reaches this shell
/// without a reload. Network failures change nothing (the callback
/// simply never fires), so an offline blip can't sign the user out.
const STATUS_REFRESH_TICKS: u32 = 50;
/// Consecutive `idle` login-status answers tolerated before the flow is
/// declared dead. The begin request may still be in flight daemon-side,
/// so a single `idle` MUST be treated as transient — resetting on it
/// immediately was the original "first click opens only a blank window"
/// bug.
const MAX_IDLE_STREAK: u32 = 5;

thread_local! {
    /// Popup opened synchronously inside the SignIn click (the only
    /// moment `window.open` is reliably allowed). It shows the
    /// same-origin `/auth/loading` interstitial until navigated.
    static PENDING_POPUP: RefCell<Option<web_sys::Window>> = const { RefCell::new(None) };
    /// The popup has been pointed at a verification page for the
    /// current flow — both the begin response and the status poll can
    /// learn the URI, whichever lands first navigates, the other skips.
    static POPUP_NAVIGATED: Cell<bool> = const { Cell::new(false) };
    /// The begin POST is in flight — status polls are suppressed so they
    /// can't observe the daemon's pre-begin `idle` state.
    static BEGIN_INFLIGHT: Cell<bool> = const { Cell::new(false) };
    /// Opaque avatar revision requested by the latest authenticated status.
    static ACCOUNT_AVATAR_DESIRED: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Revision currently installed in the bounded profile-avatar cache.
    static ACCOUNT_AVATAR_INSTALLED: RefCell<Option<String>> = const { RefCell::new(None) };
    /// At most one request per revision; a changed revision may supersede an
    /// older in-flight request, whose callback is discarded.
    static ACCOUNT_AVATAR_IN_FLIGHT: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Orders `/api/auth/status` requests. Only the newest request may project
    /// its response into editor state, so a slow pre-token request cannot hide
    /// the account UI after a newer authenticated request has succeeded.
    static STATUS_REQUEST_GATE: StatusRequestGate = const { StatusRequestGate::new() };
}

/// Latest-request-wins gate for the account status projection.
///
/// XHR callbacks can complete out of order. The sequence is assigned before a
/// request starts, and every callback must pass both this ordering check and
/// the HTTP success check before it may touch identity or UI state.
struct StatusRequestGate {
    latest_started: Cell<u64>,
}

impl StatusRequestGate {
    const fn new() -> Self {
        Self {
            latest_started: Cell::new(0),
        }
    }

    fn begin(&self) -> u64 {
        let request_id = self
            .latest_started
            .get()
            .checked_add(1)
            .expect("auth status request sequence exhausted");
        self.latest_started.set(request_id);
        request_id
    }

    fn should_apply(&self, request_id: u64, http_status: u16) -> bool {
        http_status == 200 && self.latest_started.get() == request_id
    }
}

/// Shared latches for the interval tick.
#[derive(Clone)]
struct FlowCells {
    /// One status request in flight at a time.
    busy: Rc<Cell<bool>>,
    /// Consecutive transient `idle` answers observed.
    idle_streak: Rc<Cell<u32>>,
}

/// Called synchronously from the SignIn press: open the loading popup
/// and fire the begin request whose response carries the verification
/// URI to navigate it to.
pub(crate) fn begin_login_now() {
    let base = crate::daemon_base::daemon_base();
    let opened = web_sys::window()
        .and_then(|window| {
            window
                .open_with_url_and_target(&format!("{base}{}", auth_routes::LOADING_PAGE), "_blank")
                .ok()
        })
        .flatten();
    PENDING_POPUP.with(|slot| *slot.borrow_mut() = opened);
    POPUP_NAVIGATED.set(false);
    BEGIN_INFLIGHT.set(true);
    let ok = live_sync::post_json(
        &format!("{base}{}", auth_routes::LOGIN_BEGIN),
        "{}",
        Some(Rc::new(move |body: String| {
            BEGIN_INFLIGHT.set(false);
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                close_login_popup_placeholder();
                return;
            };
            if !parsed["ok"].as_bool().unwrap_or(false) {
                // Refused (stub daemon build / non-loopback bind) — the
                // status poll shows the failure note; drop the popup.
                close_login_popup_placeholder();
                return;
            }
            if let Some(url) = parsed["verification_uri"].as_str() {
                if !url.is_empty() && !POPUP_NAVIGATED.get() {
                    POPUP_NAVIGATED.set(true);
                    navigate_login_popup(url);
                }
            }
        })),
    );
    if !ok {
        BEGIN_INFLIGHT.set(false);
        close_login_popup_placeholder();
    }
}

/// Close a still-pending loading popup (flow canceled or failed before
/// the verification page was known).
pub(crate) fn close_login_popup_placeholder() {
    PENDING_POPUP.with(|slot| {
        if let Some(popup) = slot.borrow_mut().take() {
            let _ = popup.close();
        }
    });
}

/// Point the loading popup at the verification page; when it is missing
/// (blocked, or already consumed) fall back to a direct open — that may
/// be blocked outside a gesture, but the approval also works from any
/// logged-in browser tab, so the flow still completes.
fn navigate_login_popup(url: &str) {
    let pending = PENDING_POPUP.with(|slot| slot.borrow_mut().take());
    match pending {
        Some(popup) => {
            let _ = popup.location().set_href(url);
        }
        None => {
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url_and_target(url, "_blank");
            }
        }
    }
}

/// Wire the auth relay onto the mounted shell. Called once from mount;
/// the interval runs for the page lifetime.
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    refresh_status(inner);

    let cells = FlowCells {
        busy: Rc::new(Cell::new(false)),
        idle_streak: Rc::new(Cell::new(0)),
    };
    let ticks = Rc::new(Cell::new(0u32));
    let inner = inner.clone();
    let tick: Rc<dyn Fn()> = Rc::new(move || {
        drain_actions(&inner, &base);
        maybe_poll_login(&inner, &base, &cells);
        let count = ticks.get() + 1;
        if count >= STATUS_REFRESH_TICKS {
            ticks.set(0);
            fetch_status(&inner, &base);
        } else {
            ticks.set(count);
        }
    });
    let _ = live_sync::start_interval(AUTH_POLL_INTERVAL_MS, tick);
}

/// Refresh the account capability/session projection immediately.
///
/// Managed embeds can receive their bridge token after the first bootstrap
/// request has already left without authentication. Waiting for the regular
/// ~30 s health check in that case leaves the account button missing even
/// though the daemon has a working auth backend. The bridge calls this once
/// when it installs a new token; repeated host init messages with the same
/// token remain no-ops at that layer.
pub(crate) fn refresh_status<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    fetch_status(inner, &base);
}

fn fetch_status<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    let request_id = STATUS_REQUEST_GATE.with(StatusRequestGate::begin);
    let inner = inner.clone();
    let _ = live_sync::get_with_status(
        &format!("{base}{}", auth_routes::STATUS),
        Rc::new(move |http_status, body: String| {
            let should_apply =
                STATUS_REQUEST_GATE.with(|gate| gate.should_apply(request_id, http_status));
            if !should_apply {
                return;
            }
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            // Identity first: everything below paints for an account, so the
            // account has to be settled before any of it runs.
            let observation = crate::identity_epoch::observe_subject(
                crate::identity_epoch::subject_from_status(&body).as_deref(),
            );
            if observation.requires_reset() {
                crate::live_sync_glue::reset_for_new_identity(&inner);
            }
            if observation.requires_storage_reload() {
                // The shell loaded settings and credentials under `anon` at
                // mount; the account's own partition is a different key, so
                // what is in memory belongs to the wrong one until re-read.
                crate::web_settings::reload_for_active_partition(&inner);
            }
            sync_account_avatar(&inner, parsed["avatar_revision"].as_str());
            let mut b = inner.borrow_mut();
            let ui = &mut b.host_mut().editor_state_mut().editor_ui;
            let available = parsed["available"].as_bool().unwrap_or(false);
            let account = if parsed["signed_in"].as_bool().unwrap_or(false) {
                signed_in_account(&parsed)
            } else {
                AccountState::Anonymous
            };
            // Don't clobber mid-flow UI: while a login flow runs the
            // login-status poll owns the account fields.
            let flow_active = ui.login_modal_open && ui.login_modal_status.is_some();
            let changed =
                ui.account_ui_available != available || (!flow_active && ui.account != account);
            if changed {
                ui.account_ui_available = available;
                if !flow_active {
                    ui.account = account;
                }
                b.host_mut().mark_editor_state_dirty();
                let _ = b.repaint();
            }
        }),
    );
}

fn signed_in_account(payload: &serde_json::Value) -> AccountState {
    AccountState::signed_in_profile(
        payload["display_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        payload["username"].as_str().map(str::to_string),
    )
}

fn drain_actions<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, base: &str) {
    let actions = inner.borrow_mut().host_mut().take_pending_auth_actions();
    for action in actions {
        let path = match action {
            PendingAuthAction::CancelLogin => {
                close_login_popup_placeholder();
                auth_routes::LOGIN_CANCEL
            }
            PendingAuthAction::SignOut => {
                clear_account_avatar_sync_state();
                auth_routes::LOGOUT
            }
        };
        let _ = live_sync::post_json(&format!("{base}{path}"), "{}", None);
    }
}

fn maybe_poll_login<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    base: &str,
    cells: &FlowCells,
) {
    {
        let b = inner.borrow();
        let ui = &b.host().editor_state().editor_ui;
        let flow_active = ui.login_modal_open && ui.login_modal_status.is_some();
        // While the begin POST is in flight the daemon may not have the
        // flow yet — a poll now would observe a misleading `idle`.
        if !flow_active || cells.busy.get() || BEGIN_INFLIGHT.get() {
            return;
        }
    }
    cells.busy.set(true);
    let inner = inner.clone();
    let cells_cb = cells.clone();
    let ok = live_sync::get(
        &format!("{base}{}", auth_routes::LOGIN_STATUS),
        Rc::new(move |body: String| {
            cells_cb.busy.set(false);
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            apply_login_status(&inner, &parsed, &cells_cb);
        }),
    );
    if !ok {
        cells.busy.set(false);
    }
}

fn apply_login_status<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    parsed: &serde_json::Value,
    cells: &FlowCells,
) {
    let mut b = inner.borrow_mut();
    let ui = &mut b.host_mut().editor_state_mut().editor_ui;
    if !ui.login_modal_open {
        return; // dismissed while the request was in flight
    }
    let previous = ui.login_modal_status;
    if parsed["state"].as_str() != Some("idle") {
        cells.idle_streak.set(0);
    }
    match parsed["state"].as_str().unwrap_or_default() {
        "starting" => ui.login_modal_status = Some(LoginFlowStatus::WaitingBrowser),
        "waiting_approval" => {
            ui.login_modal_status = Some(LoginFlowStatus::WaitingApproval);
            // Fallback navigation when the begin response missed the URI
            // (daemon-side wait timed out under a slow sso round-trip).
            if !POPUP_NAVIGATED.get() {
                if let Some(url) = parsed["verification_uri"].as_str() {
                    if !url.is_empty() {
                        POPUP_NAVIGATED.set(true);
                        navigate_login_popup(url);
                    }
                }
            }
        }
        "exchanging" => ui.login_modal_status = Some(LoginFlowStatus::Exchanging),
        "signed_in" => {
            sync_account_avatar(inner, parsed["avatar_revision"].as_str());
            ui.account = signed_in_account(parsed);
            ui.login_modal_status = None;
            ui.login_modal_open = false;
            ui.login_modal_hover = None;
        }
        "error" => {
            close_login_popup_placeholder();
            ui.login_modal_status = Some(LoginFlowStatus::Failed(
                match parsed["code"].as_str().unwrap_or_default() {
                    "denied" => LoginFlowError::Denied,
                    "expired" => LoginFlowError::Expired,
                    _ => LoginFlowError::Unavailable,
                },
            ));
        }
        "idle" => {
            // Transient while the just-begun flow races our poll; only a
            // sustained streak means the flow is really gone (daemon
            // restarted, or another tab finished it).
            let streak = cells.idle_streak.get() + 1;
            cells.idle_streak.set(streak);
            if streak >= MAX_IDLE_STREAK {
                close_login_popup_placeholder();
                ui.login_modal_status = None;
            }
        }
        // "canceled": another tab or the daemon finished the flow out
        // from under us — reset the note, keep the modal.
        _ => {
            close_login_popup_placeholder();
            ui.login_modal_status = None;
        }
    }
    if ui.login_modal_status != previous || !ui.login_modal_open {
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
    }
}

fn sync_account_avatar<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    revision: Option<&str>,
) {
    let revision = revision
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .map(str::to_owned);
    ACCOUNT_AVATAR_DESIRED.with(|desired| *desired.borrow_mut() = revision.clone());
    let Some(revision) = revision else {
        clear_account_avatar_sync_state();
        return;
    };
    if account_avatar_revision_active(&revision) {
        return;
    }

    ACCOUNT_AVATAR_INSTALLED.with(|installed| installed.borrow_mut().take());
    let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(None);
    ACCOUNT_AVATAR_IN_FLIGHT.with(|in_flight| {
        *in_flight.borrow_mut() = Some(revision.clone());
    });
    let expected = revision.clone();
    let inner = inner.clone();
    let url = format!(
        "{}{}",
        crate::daemon_base::daemon_base(),
        auth_routes::AVATAR
    );
    let started = live_sync::post_json_with_status(
        &url,
        "{}",
        Rc::new(move |status, body| {
            ACCOUNT_AVATAR_IN_FLIGHT.with(|in_flight| {
                if in_flight.borrow().as_deref() == Some(expected.as_str()) {
                    in_flight.borrow_mut().take();
                }
            });
            let still_desired = ACCOUNT_AVATAR_DESIRED
                .with(|desired| desired.borrow().as_deref() == Some(expected.as_str()));
            if status != 200 || !still_desired {
                return;
            }
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) else {
                return;
            };
            if parsed["revision"].as_str() != Some(expected.as_str()) {
                return;
            }
            use base64::Engine as _;
            let Some(encoded) = parsed["encoded"].as_str().and_then(|value| {
                base64::engine::general_purpose::STANDARD
                    .decode(value.as_bytes())
                    .ok()
            }) else {
                return;
            };
            if !op_editor_ui::collab_avatar_runtime::install_account_avatar_bytes(
                &expected, encoded,
            ) {
                return;
            }
            ACCOUNT_AVATAR_INSTALLED.with(|installed| {
                *installed.borrow_mut() = Some(expected.clone());
            });
            let mut context = inner.borrow_mut();
            context.host_mut().mark_editor_state_dirty();
            let _ = context.repaint();
        }),
    );
    if !started {
        ACCOUNT_AVATAR_IN_FLIGHT.with(|in_flight| in_flight.borrow_mut().take());
    }
}

fn clear_account_avatar_sync_state() {
    clear_account_avatar_revision_latches();
    let _ = op_editor_ui::collab_avatar_runtime::register_account_avatar_url(None);
}

fn clear_account_avatar_revision_latches() {
    ACCOUNT_AVATAR_DESIRED.with(|desired| desired.borrow_mut().take());
    ACCOUNT_AVATAR_INSTALLED.with(|installed| installed.borrow_mut().take());
    ACCOUNT_AVATAR_IN_FLIGHT.with(|in_flight| in_flight.borrow_mut().take());
}

fn account_avatar_revision_active(revision: &str) -> bool {
    ACCOUNT_AVATAR_INSTALLED.with(|installed| installed.borrow().as_deref() == Some(revision))
        || ACCOUNT_AVATAR_IN_FLIGHT
            .with(|in_flight| in_flight.borrow().as_deref() == Some(revision))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_status_gate_rejects_stale_success_after_a_newer_request_starts() {
        let gate = StatusRequestGate::new();
        let pre_token_request = gate.begin();
        let authenticated_request = gate.begin();

        assert!(authenticated_request > pre_token_request);
        assert!(!gate.should_apply(pre_token_request, 200));
        assert!(gate.should_apply(authenticated_request, 200));
    }

    #[test]
    fn auth_status_gate_rejects_non_success_even_for_latest_request() {
        let gate = StatusRequestGate::new();
        let request = gate.begin();

        assert!(!gate.should_apply(request, 0));
        assert!(!gate.should_apply(request, 401));
        assert!(!gate.should_apply(request, 500));
        assert!(gate.should_apply(request, 200));
    }

    #[test]
    fn account_payload_prefers_username_and_never_uses_email_as_a_handle() {
        let with_username = serde_json::json!({
            "display_name": "Kay Shen",
            "username": "kayshen_7",
            "primary_email": "wrong-handle@example.com",
        });
        assert_eq!(
            signed_in_account(&with_username),
            AccountState::SignedIn {
                display_name: "Kay Shen".to_string(),
                username: "kayshen_7".to_string(),
            }
        );

        for payload in [
            serde_json::json!({
                "display_name": "Kay Shen",
                "primary_email": "wrong-handle@example.com",
            }),
            serde_json::json!({
                "display_name": "Kay Shen",
                "username": "",
                "primary_email": "wrong-handle@example.com",
            }),
        ] {
            assert_eq!(
                signed_in_account(&payload),
                AccountState::SignedIn {
                    display_name: "Kay Shen".to_string(),
                    username: "Kay Shen".to_string(),
                }
            );
        }
    }

    #[test]
    fn sign_out_latches_allow_same_revision_to_be_fetched_after_relogin() {
        const REVISION: &str = "same-account-revision";
        ACCOUNT_AVATAR_DESIRED.with(|desired| {
            *desired.borrow_mut() = Some(REVISION.to_string());
        });
        ACCOUNT_AVATAR_INSTALLED.with(|installed| {
            *installed.borrow_mut() = Some(REVISION.to_string());
        });
        ACCOUNT_AVATAR_IN_FLIGHT.with(|in_flight| {
            *in_flight.borrow_mut() = Some(REVISION.to_string());
        });
        assert!(account_avatar_revision_active(REVISION));

        clear_account_avatar_revision_latches();

        assert!(!account_avatar_revision_active(REVISION));
        ACCOUNT_AVATAR_DESIRED.with(|desired| assert!(desired.borrow().is_none()));
    }
}
