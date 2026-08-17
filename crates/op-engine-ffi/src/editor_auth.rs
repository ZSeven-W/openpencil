//! Embedded mobile authentication bridge.
//!
//! The Rust host owns the device-code flow and polling. Platform shells
//! present the verification URL in their native login UI (email/password
//! against the regional SSO JSON API plus a system-browser hand-off for
//! third-party providers), cancel the flow when the user closes that UI, and
//! dismiss it when the engine emits the close action. The account-center
//! snapshot and sign-out calls back the shells' native account screens.

use crate::error::{read_utf8, FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;
use std::path::PathBuf;

/// No shell-owned work is pending.
pub const SHELL_ACTION_NONE: i32 = 0;
/// Present the platform document picker.
pub const SHELL_ACTION_OPEN_DOCUMENT: i32 = 1;
/// Present the pending device-login flow in the shell's native login UI
/// (historically an embedded WebView — the constant name is part of the
/// frozen shell contract).
pub const SHELL_ACTION_OPEN_LOGIN_WEBVIEW: i32 = 2;
/// Dismiss the shell login UI without canceling the completed flow.
pub const SHELL_ACTION_CLOSE_LOGIN_WEBVIEW: i32 = 3;
/// Present the shell's native account-center screen.
pub const SHELL_ACTION_OPEN_ACCOUNT_CENTER: i32 = 5;
/// Start the sign-in flow: the shell configures the auth runtime for its
/// resolved region if needed, then calls [`op_editor_begin_login`].
pub const SHELL_ACTION_REQUEST_LOGIN: i32 = 6;
/// Present the shell's native language picker
/// ([`crate::op_editor_set_locale`] applies the choice).
pub const SHELL_ACTION_OPEN_LANGUAGE_PICKER: i32 = 7;

const AUTH_POLL_INTERVAL_MS: u64 = 250;
const AUTH_STORAGE_PATH_CAP: usize = 16 * 1024;
const AUTH_METADATA_CAP: usize = 1024;
const LOGIN_URL_CAP: usize = 16 * 1024;

#[derive(Default)]
pub(crate) struct EditorAuthShellState {
    pub(crate) configured: bool,
    /// Kept until a shell copies the complete URL successfully. A size-only
    /// query and an undersized destination are non-consuming peeks.
    pub(crate) login_url: Option<String>,
    pub(crate) open_pending: bool,
    pub(crate) webview_open: bool,
    pub(crate) close_pending: bool,
}

/// SSO region codes for [`op_editor_configure_auth`]. Both map to pinned
/// first-party origins inside `op-auth-bridge` — the shell only ever picks a
/// region, never supplies a URL.
pub const AUTH_REGION_CHINA: i32 = 0;
pub const AUTH_REGION_GLOBAL: i32 = 1;

/// Configure the real mobile auth backend with a shell-owned private storage
/// directory and the regional SSO deployment the shell resolved (persisted
/// preference, falling back to an IP-informed default). Both regional
/// origins stay pinned by `op-auth-bridge`.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. Each non-empty UTF-8
/// range must cover readable memory for its declared length.
#[no_mangle]
pub unsafe extern "C" fn op_editor_configure_auth(
    engine: *mut crate::OpEngine,
    storage_dir_ptr: *const u8,
    storage_dir_len: usize,
    device_name_ptr: *const u8,
    device_name_len: usize,
    app_version_ptr: *const u8,
    app_version_len: usize,
    region: i32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let region = match region {
                AUTH_REGION_CHINA => op_host_native::MobileSsoRegion::China,
                AUTH_REGION_GLOBAL => op_host_native::MobileSsoRegion::Global,
                _ => return Err(FfiError::invalid("unknown auth region code")),
            };
            let storage_dir = read_nonempty_utf8(
                storage_dir_ptr,
                storage_dir_len,
                AUTH_STORAGE_PATH_CAP,
                "auth storage directory",
            )?;
            let storage_dir = PathBuf::from(storage_dir);
            if !storage_dir.is_absolute() {
                return Err(FfiError::invalid(
                    "auth storage directory must be an absolute path",
                ));
            }
            let device_name = read_nonempty_utf8(
                device_name_ptr,
                device_name_len,
                AUTH_METADATA_CAP,
                "auth device name",
            )?;
            let app_version = read_nonempty_utf8(
                app_version_ptr,
                app_version_len,
                AUTH_METADATA_CAP,
                "auth app version",
            )?;
            let host = session.editor_mut()?;
            if !host.configure_mobile_auth(storage_dir, device_name, app_version, region) {
                return Err(FfiError::new(
                    OpStatus::NotReady,
                    "mobile authentication backend is unavailable",
                ));
            }
            session.auth_shell.configured = true;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Peek or copy the pending login URL.
///
/// A null buffer with zero capacity reports the required byte length without
/// consuming the URL. A full-size copy consumes it; a short buffer fails and
/// leaves it available for retry. The bytes are UTF-8 and are not NUL-ended.
///
/// # Safety
///
/// `required` must be writable. A non-null `buffer` must cover `capacity`
/// writable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_copy_login_url(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            copy_login_url(&mut session.auth_shell, buffer, capacity, required)
        })
    }
}

/// Cancel an in-flight login because the user dismissed the WebView.
///
/// # Safety
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_cancel_login(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            // A host-driven terminal close can race a queued native dismissal.
            // Once this engine no longer owns an open WebView, that stale
            // callback must not clear the terminal error modal or cancel a
            // subsequent flow that has not presented its URL yet.
            if !session.auth_shell.webview_open {
                return Ok(());
            }
            let host = session.editor_mut()?;
            host.cancel_auth_login();
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.login_modal_open = false;
            ui.login_modal_hover = None;
            session.auth_shell.login_url = None;
            session.auth_shell.open_pending = false;
            session.auth_shell.webview_open = false;
            session.auth_shell.close_pending = false;
            session.request_redraw();
            Ok(())
        })
    }
}

/// Copy a JSON snapshot of the signed-in account for the shell's native
/// account-center screen: `{"signed_in":bool, "display_name":…,
/// "username":…, "primary_email":…, "avatar_url":…, "device_id":…}`.
/// Optional fields are `null` when the runtime did not report them; every
/// field except `signed_in` is absent-as-null when signed out.
///
/// A null buffer with zero capacity reports the required byte length; the
/// snapshot is re-read on every call and never consumed.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. `required` must be
/// writable; a non-null `buffer` must cover `capacity` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_account_snapshot(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if required.is_null() {
                return Err(FfiError::invalid(
                    "account snapshot required-length pointer is null",
                ));
            }
            let json = session
                .editor()
                .map(|host| host.mobile_account_snapshot_json())
                .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?;
            let bytes = json.as_bytes();
            required.write(bytes.len());
            if buffer.is_null() {
                if capacity == 0 {
                    return Ok(());
                }
                return Err(FfiError::invalid(
                    "account snapshot buffer is null with nonzero capacity",
                ));
            }
            if capacity < bytes.len() {
                return Err(FfiError::invalid(format!(
                    "account snapshot buffer covers {capacity} bytes but {} are required",
                    bytes.len()
                )));
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
            Ok(())
        })
    }
}

/// Start the engine's device-login flow after the shell configured the auth
/// runtime (`SHELL_ACTION_REQUEST_LOGIN` follow-up). `NotReady` means the
/// backend is a stub or refused a flow handle — the shell shows a native
/// "sign-in unavailable" notice instead of an engine modal.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_begin_login(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let host = session.editor_mut()?;
            if !host.begin_mobile_login() {
                return Err(FfiError::new(
                    OpStatus::NotReady,
                    "mobile authentication backend is unavailable",
                ));
            }
            session.request_redraw();
            Ok(())
        })
    }
}

/// Sign the account out: revoke the device session in the auth runtime and
/// clear the engine's account mirror. Safe to call when already signed out.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_auth_sign_out(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let host = session.editor_mut()?;
            host.sign_out_account();
            session.request_redraw();
            Ok(())
        })
    }
}

pub(crate) fn take_shell_action(session: &mut Session) -> FfiResult<i32> {
    if let Some(action) = take_auth_shell_action(&mut session.auth_shell) {
        return Ok(action);
    }

    {
        let host = session.editor_mut()?;
        if host.editor_state().editor_ui.pending_account_center {
            host.editor_state_mut().editor_ui.pending_account_center = false;
            host.mark_editor_state_dirty();
            return Ok(SHELL_ACTION_OPEN_ACCOUNT_CENTER);
        }
        if host.editor_state().editor_ui.pending_mobile_login {
            host.editor_state_mut().editor_ui.pending_mobile_login = false;
            host.mark_editor_state_dirty();
            return Ok(SHELL_ACTION_REQUEST_LOGIN);
        }
        if host.editor_state().editor_ui.pending_language_picker {
            host.editor_state_mut().editor_ui.pending_language_picker = false;
            host.mark_editor_state_dirty();
            return Ok(SHELL_ACTION_OPEN_LANGUAGE_PICKER);
        }
    }

    let pending = session
        .editor_mut()?
        .editor_state()
        .editor_ui
        .pending_file_action;
    match pending {
        Some(op_editor_core::FileAction::New) => {
            install_new_document(session)?;
            Ok(SHELL_ACTION_NONE)
        }
        Some(op_editor_core::FileAction::Open) => {
            let host = session.editor_mut()?;
            host.editor_state_mut().editor_ui.pending_file_action = None;
            host.mark_editor_state_dirty();
            Ok(SHELL_ACTION_OPEN_DOCUMENT)
        }
        #[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
        Some(op_editor_core::FileAction::ExportImageConfirm)
        | Some(op_editor_core::FileAction::ExportDeckPdfSelection) => {
            crate::editor_export::stage_export(session, pending)
        }
        _ => Ok(SHELL_ACTION_NONE),
    }
}

fn install_new_document(session: &mut Session) -> FfiResult<()> {
    let starter_document = op_editor_core::EditorState::starter().doc;
    {
        let host = session.editor_mut()?;
        // Consume the one-shot request even when collaboration starts between
        // the press and this drain. A rejected replacement must not retry on
        // every later frame.
        host.editor_state_mut().editor_ui.pending_file_action = None;
        host.install_open_document(starter_document, None, None)
            .map_err(|_| {
                FfiError::new(
                    OpStatus::Busy,
                    "new document is blocked by the collaboration session",
                )
            })?;
    }

    session.selected = None;
    session.gesture.reset();
    session.user_interacted = false;
    session.fit_content_to_viewports();
    // Fitting mutates the host-owned viewport. Clone only afterwards so the
    // lightweight state used by page APIs remains identical to the live host.
    session.state = session
        .editor()
        .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?
        .editor_state()
        .clone();
    session.scene = op_pen_loader::editor_state_to_active_page_layout_scene(&session.state);
    session.request_redraw();
    Ok(())
}

fn take_auth_shell_action(state: &mut EditorAuthShellState) -> Option<i32> {
    if state.close_pending {
        state.close_pending = false;
        return Some(SHELL_ACTION_CLOSE_LOGIN_WEBVIEW);
    }
    if state.open_pending {
        state.open_pending = false;
        return Some(SHELL_ACTION_OPEN_LOGIN_WEBVIEW);
    }
    None
}

/// Poll auth before painting so UI state and shell actions are synchronized to
/// the same frame. Returns a timed wake while either a login or restored-session
/// refresh remains active.
pub(crate) fn pump(session: &mut Session) -> Option<u64> {
    if !session.auth_shell.configured {
        return None;
    }
    let (flow_active, login_active, pending_url) = {
        let host = session.editor.as_mut()?;
        let _ = host.poll_auth();
        (
            host.auth_flow_active(),
            host.auth_login_active(),
            host.take_pending_browser_url(),
        )
    };

    if let Some(url) = pending_url.filter(|url| !url.is_empty()) {
        if url.len() <= LOGIN_URL_CAP {
            session.auth_shell.login_url = Some(url);
            session.auth_shell.open_pending = true;
            session.auth_shell.webview_open = true;
        } else {
            if let Some(host) = session.editor.as_mut() {
                host.cancel_auth_login();
            }
            session.emit_runtime_error(
                2,
                "device-login verification URL exceeds the mobile ABI limit",
                "op-engine-ffi",
            );
        }
    }
    if session.auth_shell.webview_open && !login_active {
        session.auth_shell.login_url = None;
        session.auth_shell.open_pending = false;
        session.auth_shell.webview_open = false;
        session.auth_shell.close_pending = true;
    }

    flow_active.then_some(session.now_ms.saturating_add(AUTH_POLL_INTERVAL_MS))
}

/// Stop background auth work before the platform tears down the engine. This
/// deliberately emits no shell action or callback: native UI is already being
/// dismantled and must not be resurrected during destruction.
pub(crate) fn shutdown(session: &mut Session) {
    if let Some(host) = session.editor.as_mut() {
        host.shutdown_auth();
    }
    session.auth_shell = EditorAuthShellState::default();
}

unsafe fn copy_login_url(
    state: &mut EditorAuthShellState,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> FfiResult<()> {
    if required.is_null() {
        return Err(FfiError::invalid(
            "login URL required-length pointer is null",
        ));
    }
    let Some(url) = state.login_url.as_ref() else {
        unsafe { required.write(0) };
        return Err(FfiError::new(OpStatus::NotReady, "no login URL is pending"));
    };
    let bytes = url.as_bytes();
    unsafe { required.write(bytes.len()) };
    if bytes.len() > LOGIN_URL_CAP {
        return Err(FfiError::invalid(format!(
            "login URL length exceeds {LOGIN_URL_CAP} bytes"
        )));
    }
    if buffer.is_null() {
        if capacity == 0 {
            return Ok(());
        }
        return Err(FfiError::invalid(
            "login URL buffer is null with nonzero capacity",
        ));
    }
    if capacity < bytes.len() {
        return Err(FfiError::invalid(format!(
            "login URL buffer covers {capacity} bytes but {} are required",
            bytes.len()
        )));
    }
    if capacity > isize::MAX as usize {
        return Err(FfiError::invalid("login URL buffer capacity overflows"));
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    state.login_url = None;
    Ok(())
}

unsafe fn read_nonempty_utf8(
    pointer: *const u8,
    length: usize,
    cap: usize,
    label: &str,
) -> FfiResult<String> {
    let value = unsafe { read_utf8(pointer, length, cap, label)? };
    if value.trim().is_empty() {
        return Err(FfiError::invalid(format!("{label} is empty")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::lifecycle::{OpEngine, Session};
    use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
    use op_editor_core::{LoginFlowError, LoginFlowStatus, ThemeMode};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn editor_engine() -> OpEngine {
        OpEngine::new(
            Session::new(CreateOptions {
                document: SAMPLE_DOC.to_owned(),
                width: 390.0,
                height: 844.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                editor_mode: true,
            })
            .expect("editor session"),
        )
    }

    #[test]
    fn empty_editor_session_starts_with_the_canonical_blank_document() {
        let session = Session::new(CreateOptions {
            document: String::new(),
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("blank editor session");
        let starter = op_editor_core::EditorState::starter();
        let host = session.editor().expect("editor host");

        assert_eq!(session.state.doc, starter.doc);
        assert_eq!(host.editor_state().doc, session.state.doc);
        assert_eq!(session.state.page_count(), 1);
        assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
            host.editor_state()
        ));
        assert!(host.editor_state().selection.is_empty());
        assert!(!host.editor_state().is_dirty());
        assert_eq!(host.editor_state().editor_ui.file_name_display, None);
        assert_eq!(session.scene.active_page_index, 0);
    }

    #[test]
    fn new_action_installs_a_clean_untitled_starter_and_synchronizes_session() {
        let mut engine = editor_engine();
        let session = engine.session_mut_for_test();
        let initial_epoch = {
            let host = session.editor_mut().expect("editor host");
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.touch = true;
            ui.size_class = EditorSizeClass::Medium;
            ui.theme_mode = ThemeMode::Light;
            ui.file_name_display = Some("old-document.op".into());
            ui.mobile_sheet = Some(MobileSheetKind::More);
            ui.pending_file_action = Some(op_editor_core::FileAction::New);
            host.editor_state_mut().mark_document_changed();
            host.mark_editor_state_dirty();
            host.document_epoch()
        };
        session.selected = Some("old-selection".into());
        session.user_interacted = true;

        assert_eq!(
            take_shell_action(session).expect("new action"),
            SHELL_ACTION_NONE
        );

        assert_eq!(session.selected, None);
        assert!(!session.user_interacted);
        let host = session.editor().expect("editor host");
        let live = host.editor_state();
        assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
            live
        ));
        assert_eq!(live.page_count(), 1);
        assert!(live.selection.is_empty());
        assert!(!live.is_dirty());
        assert_eq!(live.editor_ui.file_name_display, None);
        assert_eq!(live.editor_ui.mobile_sheet, None);
        assert_eq!(live.editor_ui.pending_file_action, None);
        assert!(live.editor_ui.touch);
        assert_eq!(live.editor_ui.size_class, EditorSizeClass::Medium);
        assert_eq!(live.editor_ui.theme_mode, ThemeMode::Light);
        assert_eq!(host.document_epoch(), initial_epoch.wrapping_add(1));

        assert_eq!(session.state.doc, live.doc);
        assert_eq!(session.state.viewport, live.viewport);
        assert_eq!(
            session.state.ui.active_page_index,
            live.ui.active_page_index
        );
        assert_eq!(session.state.editor_ui.touch, live.editor_ui.touch);
        assert_eq!(
            session.state.editor_ui.size_class,
            live.editor_ui.size_class
        );
        assert_eq!(
            session.state.editor_ui.theme_mode,
            live.editor_ui.theme_mode
        );
        assert_eq!(session.state.editor_ui.file_name_display, None);
        assert!(!session.state.is_dirty());
        assert_eq!(session.scene.active_page_index, live.ui.active_page_index);
    }

    #[test]
    fn login_url_peek_and_short_copy_do_not_consume() {
        let mut state = EditorAuthShellState {
            login_url: Some("https://sso.example/device".to_string()),
            ..Default::default()
        };
        let mut required = 0;
        assert!(
            unsafe { copy_login_url(&mut state, std::ptr::null_mut(), 0, &mut required) }.is_ok()
        );
        assert_eq!(required, "https://sso.example/device".len());
        assert!(state.login_url.is_some());

        let mut short = [0_u8; 4];
        let error =
            unsafe { copy_login_url(&mut state, short.as_mut_ptr(), short.len(), &mut required) }
                .expect_err("short buffer");
        assert_eq!(error.status, OpStatus::InvalidArg);
        assert!(state.login_url.is_some());
    }

    #[test]
    fn complete_login_url_copy_consumes_exactly_once() {
        let expected = "https://sso.example/device";
        let mut state = EditorAuthShellState {
            login_url: Some(expected.to_string()),
            ..Default::default()
        };
        let mut required = 0;
        let mut output = vec![0_u8; expected.len()];
        assert!(unsafe {
            copy_login_url(&mut state, output.as_mut_ptr(), output.len(), &mut required)
        }
        .is_ok());
        assert_eq!(output, expected.as_bytes());
        assert!(state.login_url.is_none());

        let error = unsafe { copy_login_url(&mut state, std::ptr::null_mut(), 0, &mut required) }
            .expect_err("consumed URL");
        assert_eq!(error.status, OpStatus::NotReady);
        assert_eq!(required, 0);
    }

    #[test]
    fn webview_shell_actions_are_each_emitted_once() {
        let mut state = EditorAuthShellState {
            open_pending: true,
            ..Default::default()
        };
        assert_eq!(
            take_auth_shell_action(&mut state),
            Some(SHELL_ACTION_OPEN_LOGIN_WEBVIEW)
        );
        assert_eq!(take_auth_shell_action(&mut state), None);

        state.close_pending = true;
        assert_eq!(
            take_auth_shell_action(&mut state),
            Some(SHELL_ACTION_CLOSE_LOGIN_WEBVIEW)
        );
        assert_eq!(take_auth_shell_action(&mut state), None);
    }

    #[test]
    fn consecutive_flow_close_then_open_preserves_both_actions() {
        let mut state = EditorAuthShellState {
            login_url: Some("https://sso.example/new-device".into()),
            open_pending: true,
            webview_open: true,
            close_pending: true,
            ..Default::default()
        };

        assert_eq!(
            take_auth_shell_action(&mut state),
            Some(SHELL_ACTION_CLOSE_LOGIN_WEBVIEW)
        );
        assert!(state.open_pending);
        assert!(state.login_url.is_some());
        assert_eq!(
            take_auth_shell_action(&mut state),
            Some(SHELL_ACTION_OPEN_LOGIN_WEBVIEW)
        );
        assert_eq!(take_auth_shell_action(&mut state), None);
    }

    #[test]
    fn oversized_login_url_is_never_copied_or_consumed() {
        let mut state = EditorAuthShellState {
            login_url: Some("x".repeat(LOGIN_URL_CAP + 1)),
            ..Default::default()
        };
        let mut required = 0;
        let error = unsafe { copy_login_url(&mut state, std::ptr::null_mut(), 0, &mut required) }
            .expect_err("oversized URL");
        assert_eq!(error.status, OpStatus::InvalidArg);
        assert_eq!(required, LOGIN_URL_CAP + 1);
        assert!(state.login_url.is_some());
    }

    #[test]
    fn user_cancel_closes_the_modal_and_clears_webview_ownership() {
        let mut engine = editor_engine();
        let pointer = &mut engine as *mut OpEngine;
        {
            let session = engine.session_mut_for_test();
            session.auth_shell.webview_open = true;
            session.auth_shell.login_url = Some("https://sso.example/device".into());
            let ui = &mut session.editor_mut().unwrap().editor_state_mut().editor_ui;
            ui.login_modal_open = true;
            ui.login_modal_hover = Some(op_editor_core::account_state::LoginModalButton::Close);
        }

        assert_eq!(unsafe { op_editor_cancel_login(pointer) }, OpStatus::Ok);
        let session = engine.session_mut_for_test();
        assert!(!session.auth_shell.webview_open);
        assert!(session.auth_shell.login_url.is_none());
        let ui = &session.editor().unwrap().editor_state().editor_ui;
        assert!(!ui.login_modal_open);
        assert_eq!(ui.login_modal_hover, None);
    }

    #[test]
    fn stale_native_cancel_does_not_erase_a_terminal_error() {
        let mut engine = editor_engine();
        let pointer = &mut engine as *mut OpEngine;
        {
            let session = engine.session_mut_for_test();
            session.auth_shell.webview_open = false;
            session.auth_shell.close_pending = true;
            let ui = &mut session.editor_mut().unwrap().editor_state_mut().editor_ui;
            ui.login_modal_open = true;
            ui.login_modal_status = Some(LoginFlowStatus::Failed(LoginFlowError::Expired));
        }

        assert_eq!(unsafe { op_editor_cancel_login(pointer) }, OpStatus::Ok);
        let session = engine.session_mut_for_test();
        assert!(session.auth_shell.close_pending);
        let ui = &session.editor().unwrap().editor_state().editor_ui;
        assert!(ui.login_modal_open);
        assert_eq!(
            ui.login_modal_status,
            Some(LoginFlowStatus::Failed(LoginFlowError::Expired))
        );
    }

    #[test]
    fn auth_configuration_rejects_relative_storage_before_backend_init() {
        let mut engine = editor_engine();
        let pointer = &mut engine as *mut OpEngine;
        let storage = "relative/auth";
        let device = "Test Phone";
        let version = "1.0.0";

        assert_eq!(
            unsafe {
                op_editor_configure_auth(
                    pointer,
                    storage.as_ptr(),
                    storage.len(),
                    device.as_ptr(),
                    device.len(),
                    version.as_ptr(),
                    version.len(),
                    AUTH_REGION_CHINA,
                )
            },
            OpStatus::InvalidArg
        );
        assert!(!engine.session_mut_for_test().auth_shell.configured);
    }

    #[test]
    fn shutdown_silently_clears_shell_state() {
        let mut engine = editor_engine();
        let session = engine.session_mut_for_test();
        session.auth_shell = EditorAuthShellState {
            configured: true,
            login_url: Some("https://sso.example/device".into()),
            open_pending: true,
            webview_open: true,
            close_pending: true,
        };

        shutdown(session);

        assert!(!session.auth_shell.configured);
        assert!(session.auth_shell.login_url.is_none());
        assert!(!session.auth_shell.open_pending);
        assert!(!session.auth_shell.webview_open);
        assert!(!session.auth_shell.close_pending);
    }
}
