//! Mobile editor collaboration runtime ownership and frame pumping.

use crate::desc::Callbacks;
use crate::error::{FfiError, FfiResult};
use crate::lifecycle::Session;
use crate::OpStatus;
use op_collab_host::{BlockingExecutor, CollabRuntime, LocalEditOutcome};
use op_collab_transport::{
    CredentialStore, CredentialStoreKeyStore, DeviceStaticKey, KeyStoreError, StaticKeyStore,
};
use op_editor_core::CollabTransportCapabilities;
use op_host_native::widget_host::collab_history_hit::CollabHistoryAction;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use zeroize::{Zeroize, Zeroizing};

const COLLAB_POLL_INTERVAL_MS: u64 = 33;
const COLLAB_PRIVATE_KEY_BYTES: usize = 32;

pub(crate) struct EditorCollabState {
    runtime: CollabRuntime,
    wake_pending: Arc<AtomicBool>,
    suppressed_pointer: Option<u32>,
    pointer_edit_open: bool,
}

impl EditorCollabState {
    pub(crate) fn new(callbacks: Callbacks) -> FfiResult<Self> {
        install_executor()?;
        let mut runtime = runtime_for_callbacks(callbacks);
        runtime.set_transport_capabilities(CollabTransportCapabilities::RELAY_AND_MANUAL_JOIN);
        let wake_pending = Arc::new(AtomicBool::new(false));
        let worker_wake = Arc::clone(&wake_pending);
        // The wake notifier touches only this atomic. Credential callbacks
        // use their separate worker-safe JNI path; no ordinary platform UI
        // callback is invoked from a collaboration worker.
        runtime.set_wake_notifier(Arc::new(move || {
            worker_wake.store(true, Ordering::Release);
        }));
        Ok(Self {
            runtime,
            wake_pending,
            suppressed_pointer: None,
            pointer_edit_open: false,
        })
    }
}

struct MobileBlockingExecutor;

impl BlockingExecutor for MobileBlockingExecutor {
    fn block_on_erased(&self, future: Pin<&mut (dyn Future<Output = ()> + '_)>) {
        match Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future));
            }
            Ok(_) => panic!("collaboration cannot block inside a current-thread Tokio runtime"),
            Err(_) => mobile_runtime()
                .expect("mobile collaboration executor was initialized")
                .block_on(future),
        }
    }
}

fn mobile_runtime() -> Option<&'static Runtime> {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    if let Some(runtime) = RUNTIME.get() {
        return Some(runtime);
    }
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .thread_name("op-mobile-collab")
        .build()
        .ok()?;
    let _ = RUNTIME.set(runtime);
    RUNTIME.get()
}

fn install_executor() -> FfiResult<()> {
    if mobile_runtime().is_none() {
        return Err(FfiError::new(
            OpStatus::NotReady,
            "could not initialize the mobile collaboration executor",
        ));
    }
    let _ = op_collab_host::install_blocking_executor(Arc::new(MobileBlockingExecutor));
    Ok(())
}

#[derive(Clone, Copy)]
struct FfiCredentialStore {
    user_data: usize,
    load: unsafe extern "C" fn(*mut std::ffi::c_void, *mut u8, usize, *mut usize) -> i32,
    store_if_absent: unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize) -> i32,
}

impl CredentialStore for FfiCredentialStore {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
        let mut bytes = [0_u8; COLLAB_PRIVATE_KEY_BYTES];
        let mut length = 0_usize;
        // SAFETY: callbacks and user_data are retained by Session until the
        // collaboration runtime has synchronously joined its workers.
        let status = unsafe {
            (self.load)(
                self.user_data as *mut std::ffi::c_void,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut length,
            )
        };
        let result = match status {
            0 if length == COLLAB_PRIVATE_KEY_BYTES => Some(Zeroizing::new(bytes.to_vec())),
            1 => None,
            _ => {
                bytes.zeroize();
                return Err(KeyStoreError::PlatformStoreFailure);
            }
        };
        bytes.zeroize();
        Ok(result)
    }

    fn store_if_absent(&self, value: &[u8]) -> Result<(), KeyStoreError> {
        if value.len() != COLLAB_PRIVATE_KEY_BYTES {
            return Err(KeyStoreError::InvalidLength(value.len()));
        }
        // SAFETY: the callback borrows `value` only for this call.
        let status = unsafe {
            (self.store_if_absent)(
                self.user_data as *mut std::ffi::c_void,
                value.as_ptr(),
                value.len(),
            )
        };
        (status == 0)
            .then_some(())
            .ok_or(KeyStoreError::PlatformStoreFailure)
    }
}

struct UnavailableStaticKeyStore;

impl StaticKeyStore for UnavailableStaticKeyStore {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        Err(KeyStoreError::PlatformStoreUnavailable)
    }
}

fn runtime_for_callbacks(callbacks: Callbacks) -> CollabRuntime {
    match (
        callbacks.credential_load,
        callbacks.credential_store_if_absent,
    ) {
        (Some(load), Some(store_if_absent)) => {
            let store = FfiCredentialStore {
                user_data: callbacks.user_data as usize,
                load,
                store_if_absent,
            };
            let key_store = CredentialStoreKeyStore::new(Arc::new(store));
            CollabRuntime::with_key_store(Arc::new(key_store))
        }
        (None, None) if !cfg!(target_os = "android") => CollabRuntime::new(),
        _ => CollabRuntime::with_key_store(Arc::new(UnavailableStaticKeyStore)),
    }
}

impl Session {
    /// Runtime order mirrors desktop: availability, one queued action when no
    /// local capture is open, inbound events, then lossy presence.
    pub(crate) fn pump_editor_collab(&mut self) -> Option<u64> {
        let now_ms = self.now_ms;
        let (_changed, should_poll, save_as_fork) = {
            let (Some(collab), Some(host)) = (self.editor_collab.as_mut(), self.editor.as_mut())
            else {
                return None;
            };
            let runtime = &mut collab.runtime;
            let mut changed = runtime.refresh_availability(host);
            if !runtime.local_edit_in_flight() {
                changed |= runtime.drain_ui_action(host);
            }
            changed |= runtime.poll(host);
            let save_as_fork = runtime.take_save_as_fork_request();
            for _ in runtime.drain_status_events() {}
            changed |= runtime.publish_local_presence(host, None);
            let worker_woke = collab.wake_pending.swap(false, Ordering::AcqRel);
            (
                changed,
                runtime.needs_poll() || runtime.next_presence_deadline().is_some() || worker_woke,
                save_as_fork,
            )
        };
        if save_as_fork {
            if let Some(host) = self.editor.as_mut() {
                host.editor_state_mut().editor_ui.pending_file_action =
                    Some(op_editor_core::FileAction::SaveAs);
                host.mark_editor_state_dirty();
            }
        }
        should_poll.then_some(now_ms.saturating_add(COLLAB_POLL_INTERVAL_MS))
    }

    pub(crate) fn begin_collab_local_edit(&mut self) -> bool {
        let (Some(collab), Some(host)) = (self.editor_collab.as_mut(), self.editor.as_mut()) else {
            return false;
        };
        collab.runtime.begin_local_edit(host)
    }

    fn finish_owned_collab_local_edit(&mut self, opened: bool) -> bool {
        if !opened {
            return false;
        }
        let (Some(collab), Some(host)) = (self.editor_collab.as_mut(), self.editor.as_mut()) else {
            return false;
        };
        if !collab.runtime.local_edit_in_flight() {
            return false;
        }
        !matches!(
            collab.runtime.finish_local_edit(host),
            LocalEditOutcome::NoChange
        )
    }

    pub(crate) fn begin_collab_pointer_edit(&mut self) {
        if self
            .editor_collab
            .as_ref()
            .is_some_and(|collab| collab.pointer_edit_open)
        {
            return;
        }
        let opened = self.begin_collab_local_edit();
        if let Some(collab) = self.editor_collab.as_mut() {
            collab.pointer_edit_open = opened;
        }
    }

    pub(crate) fn finish_collab_pointer_edit(&mut self) -> bool {
        let opened = self
            .editor_collab
            .as_mut()
            .is_some_and(|collab| std::mem::take(&mut collab.pointer_edit_open));
        self.finish_owned_collab_local_edit(opened)
    }

    pub(crate) fn with_collab_local_edit(
        &mut self,
        edit: impl FnOnce(&mut op_host_native::WidgetHostNative) -> bool,
    ) -> FfiResult<bool> {
        let opened = self.begin_collab_local_edit();
        let changed = edit(self.editor_mut()?);
        Ok(changed | self.finish_owned_collab_local_edit(opened))
    }

    pub(crate) fn request_collab_history(
        &mut self,
        action: CollabHistoryAction,
    ) -> FfiResult<bool> {
        let consumed = {
            let (Some(collab), Some(host)) = (self.editor_collab.as_mut(), self.editor.as_mut())
            else {
                return Ok(false);
            };
            if collab.runtime.local_edit_in_flight() {
                return Ok(true);
            }
            match action {
                CollabHistoryAction::Undo => collab.runtime.request_undo(host),
                CollabHistoryAction::Redo => collab.runtime.reject_redo(host),
            }
        };
        if consumed {
            return Ok(true);
        }
        Ok(match action {
            CollabHistoryAction::Undo => self.editor_mut()?.apply_undo(),
            CollabHistoryAction::Redo => self.editor_mut()?.apply_redo(),
        })
    }

    pub(crate) fn apply_collab_key(&mut self, key: i32) -> FfiResult<bool> {
        use crate::editor::*;
        match key {
            KEY_UNDO => self.request_collab_history(CollabHistoryAction::Undo),
            KEY_REDO => self.request_collab_history(CollabHistoryAction::Redo),
            KEY_BACKSPACE | KEY_DELETE | KEY_ENTER | KEY_ESCAPE | KEY_DUPLICATE | KEY_ARROW_UP
            | KEY_ARROW_DOWN | KEY_ARROW_LEFT | KEY_ARROW_RIGHT => {
                self.with_collab_local_edit(|host| match key {
                    KEY_BACKSPACE => host.apply_backspace(),
                    KEY_DELETE => host.apply_delete(),
                    KEY_ENTER => host.apply_send(),
                    KEY_ESCAPE => host.apply_escape(),
                    KEY_DUPLICATE => host.apply_duplicate(),
                    KEY_ARROW_UP => host.apply_nudge(0.0, -1.0),
                    KEY_ARROW_DOWN => host.apply_nudge(0.0, 1.0),
                    KEY_ARROW_LEFT => host.apply_nudge(-1.0, 0.0),
                    KEY_ARROW_RIGHT => host.apply_nudge(1.0, 0.0),
                    _ => unreachable!("key was classified above"),
                })
            }
            other => Err(FfiError::invalid(format!("unknown editor key {other}"))),
        }
    }

    pub(crate) fn collab_history_at(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Option<CollabHistoryAction> {
        self.editor()
            .and_then(|host| host.collab_history_action_at(x, y, width, height))
    }

    pub(crate) fn suppress_collab_pointer(&mut self, pointer: u32) {
        if let Some(collab) = self.editor_collab.as_mut() {
            collab.suppressed_pointer = Some(pointer);
        }
    }

    pub(crate) fn take_suppressed_collab_pointer(&mut self, pointer: u32, end: bool) -> bool {
        let Some(collab) = self.editor_collab.as_mut() else {
            return false;
        };
        let suppressed = collab.suppressed_pointer == Some(pointer);
        if suppressed && end {
            collab.suppressed_pointer = None;
        }
        suppressed
    }

    pub(crate) fn cancel_editor_collab_gesture(&mut self) -> FfiResult<bool> {
        if let Some(collab) = self.editor_collab.as_mut() {
            collab.suppressed_pointer = None;
        }
        let cancelled = self.editor_mut()?.cancel_native_touch_gestures();
        Ok(cancelled | self.finish_collab_pointer_edit())
    }

    pub(crate) fn shutdown_editor_collab(&mut self) {
        if self.editor_collab.is_none() || self.editor.is_none() {
            return;
        }
        if let Some(host) = self.editor.as_mut() {
            let _ = host.cancel_native_touch_gestures();
        }
        let pointer_opened = self
            .editor_collab
            .as_mut()
            .is_some_and(|collab| std::mem::take(&mut collab.pointer_edit_open));
        let _ = self.finish_owned_collab_local_edit(pointer_opened);
        let any_open = self
            .editor_collab
            .as_ref()
            .is_some_and(|collab| collab.runtime.local_edit_in_flight());
        let _ = self.finish_owned_collab_local_edit(any_open);
        let (Some(collab), Some(host)) = (self.editor_collab.as_mut(), self.editor.as_mut()) else {
            return;
        };
        collab.runtime.shutdown(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::CreateOptions;
    use crate::{OpEngine, OpStatus};
    use op_editor_core::{NodeId, PenNodeExt};

    const SAMPLE_DOCUMENT: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn editor_session(width: f32, height: f32) -> Session {
        Session::new(CreateOptions {
            document: SAMPLE_DOCUMENT.to_owned(),
            width,
            height,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .unwrap()
    }

    unsafe extern "C" fn short_credential_load(
        _user_data: *mut std::ffi::c_void,
        out: *mut u8,
        capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        assert_eq!(capacity, COLLAB_PRIVATE_KEY_BYTES);
        unsafe {
            std::ptr::write_bytes(out, 7, capacity);
            *out_len = COLLAB_PRIVATE_KEY_BYTES - 1;
        }
        0
    }

    unsafe extern "C" fn unused_credential_store(
        _user_data: *mut std::ffi::c_void,
        _value: *const u8,
        _length: usize,
    ) -> i32 {
        0
    }

    static CREDENTIAL_STORE_CALLS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    unsafe extern "C" fn counting_credential_store(
        _user_data: *mut std::ffi::c_void,
        _value: *const u8,
        _length: usize,
    ) -> i32 {
        CREDENTIAL_STORE_CALLS.fetch_add(1, Ordering::Relaxed);
        0
    }

    #[test]
    fn credential_adapter_rejects_non_exact_private_key_lengths() {
        let store = FfiCredentialStore {
            user_data: 0,
            load: short_credential_load,
            store_if_absent: unused_credential_store,
        };
        assert!(matches!(
            store.load(),
            Err(KeyStoreError::PlatformStoreFailure)
        ));

        CREDENTIAL_STORE_CALLS.store(0, Ordering::Relaxed);
        let store = FfiCredentialStore {
            user_data: 0,
            load: short_credential_load,
            store_if_absent: counting_credential_store,
        };
        match store.store_if_absent(&[3; COLLAB_PRIVATE_KEY_BYTES - 1]) {
            Err(KeyStoreError::InvalidLength(length)) => {
                assert_eq!(length, COLLAB_PRIVATE_KEY_BYTES - 1)
            }
            other => panic!("unexpected short credential result: {other:?}"),
        }
        assert_eq!(CREDENTIAL_STORE_CALLS.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn executor_install_is_idempotent() {
        install_executor().unwrap();
        install_executor().unwrap();
    }

    #[test]
    fn nested_discrete_edit_does_not_finish_pointer_owned_transaction() {
        let mut session = editor_session(390.0, 844.0);
        let baseline = session.editor().unwrap().editor_state().doc.clone();
        let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
        session.editor_collab.as_mut().unwrap().runtime = runtime;

        session.begin_collab_pointer_edit();
        assert!(session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        assert!(!session.with_collab_local_edit(|_| false).unwrap());
        assert!(session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());

        assert!(!session.finish_collab_pointer_edit());
        assert!(!session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        session.shutdown_editor_collab();
        drop(lane);
    }

    #[test]
    fn gpu_and_cpu_frames_both_pump_collaboration_before_paint() {
        let root =
            std::env::temp_dir().join(format!("openpencil-ffi-collab-pump-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        op_config_store::redirect_user_root_for_tests(root);

        let mut session = editor_session(80.0, 80.0);
        session
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .collab
            .pending_action = Some(op_editor_core::CollabUiAction::StartLan);
        assert!(session.frame_gpu(1).is_err());
        assert!(session
            .editor()
            .unwrap()
            .editor_state()
            .editor_ui
            .collab
            .pending_action
            .is_none());

        session
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .collab
            .pending_action = Some(op_editor_core::CollabUiAction::BeginDiscovery);
        let mut pixels = vec![0_u8; 80 * 80 * 4];
        session
            .frame_cpu(2, pixels.as_mut_ptr(), pixels.len(), 80 * 4)
            .unwrap();
        assert!(session
            .editor()
            .unwrap()
            .editor_state()
            .editor_ui
            .collab
            .pending_action
            .is_none());
    }

    #[test]
    fn suspend_finishes_pointer_owned_transaction() {
        let mut session = editor_session(390.0, 844.0);
        let baseline = session.editor().unwrap().editor_state().doc.clone();
        let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
        session.editor_collab.as_mut().unwrap().runtime = runtime;
        session.begin_collab_pointer_edit();

        session.suspend();

        assert!(!session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        session.shutdown_editor_collab();
        drop(lane);
    }

    #[test]
    fn mobile_long_press_closes_capture_and_commits_rename_once() {
        let mut session = editor_session(390.0, 844.0);
        let baseline = session.editor().unwrap().editor_state().doc.clone();
        let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
        session.editor_collab.as_mut().unwrap().runtime = runtime;
        let state = session.editor_mut().unwrap().editor_state_mut();
        assert!(state.start_rename_layer(NodeId::new("f18")));
        assert!(state.rename_append("!"));

        let mut engine = OpEngine::new(session);
        let engine_ptr = &mut engine as *mut OpEngine;
        assert_eq!(
            unsafe { crate::op_editor_press(engine_ptr, 200.0, 400.0) },
            OpStatus::Ok
        );
        assert!(engine
            .session_mut_for_test()
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());

        assert_eq!(
            unsafe { crate::op_editor_right_press(engine_ptr, 200.0, 400.0) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert!(!session.editor_pointer_captured());
        assert!(!session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        let renamed = op_editor_core::walkers::find_node(
            session.editor().unwrap().editor_state().active_children(),
            &NodeId::new("f18"),
        )
        .and_then(|node| node.base().name.as_deref());
        assert!(renamed.is_some_and(|name| name.ends_with('!')));
        assert_eq!(lane.drain_command_count(), 1);

        // Both mobile shells suppress this Up. A defensive late Up must also
        // be harmless and cannot close or emit the transaction a second time.
        assert_eq!(
            unsafe { crate::op_editor_release(engine_ptr, 200.0, 400.0) },
            OpStatus::Ok
        );
        assert_eq!(lane.drain_command_count(), 0);
        engine.session_mut_for_test().shutdown_editor_collab();
        drop(lane);
    }

    #[test]
    fn ordinary_drag_remains_open_until_release() {
        let mut session = editor_session(390.0, 844.0);
        let baseline = session.editor().unwrap().editor_state().doc.clone();
        let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
        session.editor_collab.as_mut().unwrap().runtime = runtime;
        let mut engine = OpEngine::new(session);
        let engine_ptr = &mut engine as *mut OpEngine;

        assert_eq!(
            unsafe { crate::op_editor_press(engine_ptr, 200.0, 400.0) },
            OpStatus::Ok
        );
        assert_eq!(
            unsafe { crate::op_editor_move(engine_ptr, 220.0, 420.0) },
            OpStatus::Ok
        );
        assert!(engine
            .session_mut_for_test()
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());

        assert_eq!(
            unsafe { crate::op_editor_release(engine_ptr, 220.0, 420.0) },
            OpStatus::Ok
        );
        assert!(!engine
            .session_mut_for_test()
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        let _ = lane.drain_command_count();
        engine.session_mut_for_test().shutdown_editor_collab();
        drop(lane);
    }

    #[test]
    fn selected_text_ime_preedit_and_commit_each_close_against_the_right_baseline() {
        let mut session = editor_session(390.0, 844.0);
        let state = session.editor_mut().unwrap().editor_state_mut();
        assert!(state.start_text_edit(NodeId::new("t2")));
        assert!(state.text_edit_select_all_now(1));
        let baseline = state.doc.clone();
        let (runtime, lane) = op_collab_host::test_support::owner_session(baseline);
        session.editor_collab.as_mut().unwrap().runtime = runtime;
        let mut engine = OpEngine::new(session);
        let engine_ptr = &mut engine as *mut OpEngine;

        let preedit = "xin";
        assert_eq!(
            unsafe {
                crate::op_editor_ime_preedit(
                    engine_ptr,
                    preedit.as_ptr(),
                    preedit.len(),
                    preedit.len(),
                    preedit.len(),
                )
            },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert_eq!(
            session.editor().unwrap().editor_state().text_edit_content(),
            Some("")
        );
        assert!(!session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        assert_eq!(lane.drain_command_count(), 1);

        let commit = "新";
        assert_eq!(
            unsafe { crate::op_editor_ime_commit(engine_ptr, commit.as_ptr(), commit.len()) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert_eq!(
            session.editor().unwrap().editor_state().text_edit_content(),
            Some(commit)
        );
        assert!(!session
            .editor_collab
            .as_ref()
            .unwrap()
            .runtime
            .local_edit_in_flight());
        assert_eq!(lane.drain_command_count(), 1);
        session.shutdown_editor_collab();
        drop(lane);
    }
}
