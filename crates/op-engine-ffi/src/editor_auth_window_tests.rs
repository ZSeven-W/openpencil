//! The TopBar window-control shell actions (`OpShellAction_Window*`).
//!
//! Sibling of `editor_auth.rs` (which sits at the repo's per-file line cap).
//! These pin the half of the traffic-light wiring that is platform-free: a
//! one-shot `pending_window_control` becomes exactly one shell action, in the
//! same drain the document / login / language actions already use, and the
//! flag clears so a dot press cannot close the window twice.

#![cfg(test)]

use crate::desc::{Callbacks, CreateOptions};
use crate::editor::op_editor_take_shell_action;
use crate::editor_auth::{
    SHELL_ACTION_NONE, SHELL_ACTION_WINDOW_CLOSE, SHELL_ACTION_WINDOW_MINIMIZE,
    SHELL_ACTION_WINDOW_ZOOM,
};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use op_editor_core::WindowControlRequest;

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

fn editor_engine() -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 1_100.0,
            height: 734.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    )
}

fn drain(engine: &mut OpEngine) -> i32 {
    let pointer = engine as *mut OpEngine;
    let mut action = -1;
    assert_eq!(
        unsafe { op_editor_take_shell_action(pointer, &mut action) },
        OpStatus::Ok
    );
    action
}

#[test]
fn each_window_request_drains_once_as_its_own_action() {
    for (request, expected) in [
        (WindowControlRequest::Close, SHELL_ACTION_WINDOW_CLOSE),
        (WindowControlRequest::Minimize, SHELL_ACTION_WINDOW_MINIMIZE),
        (WindowControlRequest::Zoom, SHELL_ACTION_WINDOW_ZOOM),
    ] {
        let mut engine = editor_engine();
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .pending_window_control = Some(request);

        assert_eq!(drain(&mut engine), expected, "{request:?} drains as itself");
        // One press, one action: a shell that pumps the queue must not see
        // the same close request again on its next tick.
        assert_eq!(drain(&mut engine), SHELL_ACTION_NONE);
        assert_eq!(
            engine
                .session_mut_for_test()
                .editor_mut()
                .unwrap()
                .editor_state()
                .editor_ui
                .pending_window_control,
            None
        );
    }
}

#[test]
fn an_idle_editor_asks_for_no_window_work() {
    let mut engine = editor_engine();
    assert_eq!(drain(&mut engine), SHELL_ACTION_NONE);
}
