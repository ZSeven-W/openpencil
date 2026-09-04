//! Full-editor mode: drives op-host-native's `WidgetHostNative` — the
//! COMPLETE desktop chrome (top bar, layer panel, toolbar, property panel,
//! status bar, chat, settings, context menus, canvas editing, 15-locale
//! i18n) — through the same `NativeFrameBackend` the desktop uses.
//!
//! The shells stay thin: single-finger presses become host press/move/
//! release (the host owns selection, marquee, node drags, panel presses,
//! and text-edit entry), a long-press is a right-click, two-finger drags
//! pan (`apply_pan_gesture`), pinches zoom (`apply_pinch_gesture`), and
//! system-keyboard events flow through [`op_editor_key`] /
//! [`op_editor_text`] / [`op_editor_ime_preedit`] /
//! [`op_editor_ime_commit`].

use crate::editor_auth;
use crate::error::{FfiError, DOCUMENT_CAP, STRING_CAP};
use crate::lifecycle::call_session;
use crate::OpStatus;

/// Key codes the shells map their platform keys to.
pub const KEY_BACKSPACE: i32 = 1;
pub const KEY_DELETE: i32 = 2;
pub const KEY_ENTER: i32 = 3;
pub const KEY_ESCAPE: i32 = 4;
pub const KEY_DUPLICATE: i32 = 5;
pub const KEY_UNDO: i32 = 6;
pub const KEY_REDO: i32 = 7;
pub const KEY_ARROW_UP: i32 = 9;
pub const KEY_ARROW_DOWN: i32 = 10;
pub const KEY_ARROW_LEFT: i32 = 11;
pub const KEY_ARROW_RIGHT: i32 = 12;
pub const KEY_SELECT_ALL: i32 = 13;
pub const KEY_COPY: i32 = 14;
pub const KEY_CUT: i32 = 15;
pub const KEY_PASTE: i32 = 16;
pub const KEY_GROUP: i32 = 17;
pub const KEY_UNGROUP: i32 = 18;
pub const KEY_REORDER_BACK: i32 = 19;
pub const KEY_REORDER_FORWARD: i32 = 20;
pub const KEY_ARROW_UP_BIG: i32 = 21;
pub const KEY_ARROW_DOWN_BIG: i32 = 22;
pub const KEY_ARROW_LEFT_BIG: i32 = 23;
pub const KEY_ARROW_RIGHT_BIG: i32 = 24;

/// Drain the next shell-owned editor action.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_editor_take_shell_action(
    engine: *mut crate::OpEngine,
    out: *mut i32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid("shell-action output pointer is null"));
            }
            out.write(editor_auth::take_shell_action(session)?);
            Ok(())
        })
    }
}

/// Apply a UI locale by BCP-47 tag ("zh-CN", "en-US", …). Unsupported tags
/// are rejected so the shell's picker cannot silently fall back.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; a non-empty byte
/// range must cover readable UTF-8 for its declared length.
#[no_mangle]
pub unsafe extern "C" fn op_editor_set_locale(
    engine: *mut crate::OpEngine,
    tag_ptr: *const u8,
    tag_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let tag = crate::error::read_utf8(tag_ptr, tag_len, STRING_CAP, "locale tag")?;
            let Some(locale) = op_editor_core::editor_ui_state::Locale::from_tag(&tag) else {
                return Err(FfiError::invalid("unsupported locale tag"));
            };
            let host = session.editor_mut()?;
            host.editor_state_mut().editor_ui.locale = locale;
            host.mark_editor_state_dirty();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Copy the current UI locale's BCP-47 tag (never consumed).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; `required` must be
/// writable and a non-null `buffer` must cover `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_locale_code(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let code = session
                .editor()
                .map(|host| host.editor_state().editor_ui.locale.code())
                .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?;
            crate::error::write_bytes(code.as_bytes(), buffer, capacity, required)
        })
    }
}

/// Parse and atomically install a document selected by the platform shell.
/// Parsing, compatibility migration, editor metadata extraction, and scene
/// validation all finish before the live host is touched, so a rejected file
/// leaves the current document intact.
///
/// `file_name` is optional UTF-8 display text (normally the picker URL's last
/// path component); pass a null pointer with length zero when unavailable.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. Non-empty byte ranges
/// must cover readable memory for their declared lengths.
#[no_mangle]
pub unsafe extern "C" fn op_editor_open_document(
    engine: *mut crate::OpEngine,
    doc_ptr: *const u8,
    doc_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let source =
                crate::error::read_utf8(doc_ptr, doc_len, DOCUMENT_CAP, "editor document")?;
            if source.is_empty() {
                return Err(FfiError::invalid("editor document is empty"));
            }
            let file_name = if file_name_len == 0 {
                None
            } else {
                Some(crate::error::read_utf8(
                    file_name_ptr,
                    file_name_len,
                    STRING_CAP,
                    "editor file name",
                )?)
            };

            let editor_meta =
                op_pen_loader::extract_editor_meta_with_report(&source).map(|value| value.meta);
            let loaded = op_pen_loader::payload::load_canonical_with_compatibility(&source)
                .map_err(|error| {
                    FfiError::new(
                        OpStatus::BadDocument,
                        format!("document rejected by the canonical schema: {error}"),
                    )
                })?;
            let document = loaded.loaded.value;
            // A canonical load associates its pending `imageThumbs` table with
            // the parsed document. Validate against a seed-free clone so
            // `EditorState::from_document` cannot publish unaccepted bytes.
            // The original seed remains pending until the live host accepts
            // the replacement and `replace_document` activates it.
            let validation_document = document.clone();
            jian_ops_schema::image_thumbs::discard_for_document(&validation_document);
            let mut next_state = op_editor_core::EditorState::from_document(validation_document);
            op_pen_loader::apply_editor_meta_or_legacy_fallback(
                &mut next_state,
                editor_meta.clone(),
            );
            next_state.editor_ui.file_name_display = file_name.clone();
            next_state.mark_saved_revision();
            let next_scene = op_pen_loader::editor_state_to_active_page_layout_scene(&next_state);
            if next_scene.active_page().is_none() {
                jian_ops_schema::image_thumbs::discard_for_document(&document);
                return Err(FfiError::new(
                    OpStatus::LayoutError,
                    "document has no renderable page",
                ));
            }

            let host = match session.editor_mut() {
                Ok(host) => host,
                Err(error) => {
                    jian_ops_schema::image_thumbs::discard_for_document(&document);
                    return Err(error);
                }
            };
            if let Err(rejected_document) =
                host.install_open_document(document, editor_meta, file_name)
            {
                jian_ops_schema::image_thumbs::discard_for_document(&rejected_document);
                return Err(FfiError::new(
                    OpStatus::Busy,
                    "document replacement is blocked by the collaboration session",
                ));
            }

            // Keep the lightweight player state in exact lockstep with the
            // full widget host: page APIs and the viewer scene still read it.
            session.state = next_state;
            session.scene = next_scene;
            session.selected = None;
            // Picker documents arrive as bytes without a writable path —
            // the next Save prompts for a sandbox file name.
            crate::editor_document::forget_current_document(session);
            // Whole-document replacement: forget in-flight image-search jobs
            // bound to the outgoing document's node ids (id aliasing — see
            // MobileImageSearch::reset).
            session.image_search.reset();
            session.gesture.reset();
            session.user_interacted = false;
            session.fit_content_to_viewports();
            session.request_redraw();
            Ok(())
        })
    }
}

/// Long-press → right-click (context menus, layer rows, canvas).
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_right_press(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let right_press = if session.safe_area_contains_surface_point(x, y) {
                let (w, h) = session.editor_viewport();
                let (x, y) = session.editor_point(x, y);
                session
                    .editor_mut()
                    .map(|host| host.apply_right_press(x, y, w, h))
            } else {
                Ok(false)
            };

            // Mobile shells intentionally suppress the terminal Up after a
            // long press. Close the Down-owned collaboration transaction and
            // pointer gate here, including safe-area misses and host errors,
            // so the runtime cannot remain busy forever.
            let collab_changed = session.finish_collab_pointer_edit();
            let presence_changed = session.clear_editor_presence_pointer();
            session.reset_editor_pointer_capture();
            if right_press? | collab_changed | presence_changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Two-finger pan after `op_editor_begin_transform`; `dx`/`dy` are midpoint deltas.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_pan(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if !session.editor_transform_captured() {
                return Ok(());
            }
            let (w, h) = session.editor_viewport();
            let (x, y) = session.editor_point(x, y);
            let (changed, camera_changed) = {
                let host = session.editor_mut()?;
                let before = host.editor_state().viewport;
                let changed = host.apply_pan_gesture(x, y, dx, dy, w, h);
                (changed, host.editor_state().viewport != before)
            };
            if camera_changed {
                session.user_interacted = true;
            }
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Pinch after `op_editor_begin_transform` (positive `delta_y` = zoom in).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_pinch(
    engine: *mut crate::OpEngine,
    x: f32,
    y: f32,
    delta_y: f32,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if !session.editor_transform_captured() {
                return Ok(());
            }
            let (w, h) = session.editor_viewport();
            let (x, y) = session.editor_point(x, y);
            let (changed, camera_changed) = {
                let host = session.editor_mut()?;
                let before = host.editor_state().viewport;
                let changed = host.apply_pinch_gesture(x, y, delta_y, w, h);
                (changed, host.editor_state().viewport != before)
            };
            if camera_changed {
                session.user_interacted = true;
            }
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Text input from the system keyboard (printable characters; the engine
/// routes each char through the host's focused-input path).
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_text(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let text = crate::error::read_utf8(text_ptr, text_len, STRING_CAP, "editor text")?;
            let changed = session.with_collab_local_edit(|host| {
                let mut changed = false;
                for c in text.chars() {
                    changed |= host.apply_text(c);
                }
                changed
            })?;
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Non-printable key from the system keyboard (see `KEY_*` constants).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_key(engine: *mut crate::OpEngine, key: i32) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let changed = session.apply_collab_key(key)?;
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::editor_auth::{SHELL_ACTION_NONE, SHELL_ACTION_OPEN_DOCUMENT};
    use crate::lifecycle::{OpEngine, Session};
    use op_editor_core::{
        size_class::{EditorSizeClass, MobileSheetKind},
        AuthenticatedCollabSession, CollabConnectionPhase, CollabUiRole,
    };

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");
    const TWO_PAGE_DOC: &str =
        include_str!("../../../packaging/android/app/src/main/assets/two-page.op");

    fn editor_engine(doc: &str) -> OpEngine {
        OpEngine::new(
            Session::new(CreateOptions {
                document: doc.to_owned(),
                width: 800.0,
                height: 600.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                editor_mode: true,
                documents_root: None,
            })
            .expect("editor session"),
        )
    }

    fn with_editor_meta(source: &str) -> String {
        let end = source.rfind('}').expect("top-level object close");
        format!(
            "{},\"editorMeta\":{{\"activePageIndex\":1,\"preserveAuthoredGeometry\":true}}{}",
            &source[..end],
            &source[end..]
        )
    }

    fn with_image_thumb(source: &str, paint_id: u64) -> String {
        let end = source.rfind('}').expect("top-level object close");
        format!(
            "{},\"imageThumbs\":{{\"{paint_id}\":\"/9j/2Q==\"}}{}",
            &source[..end],
            &source[end..]
        )
    }

    #[test]
    fn shell_action_drains_open_and_save_opens_the_name_dialog() {
        let mut engine = editor_engine(SAMPLE_DOC);
        let pointer = &mut engine as *mut OpEngine;
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .pending_file_action = Some(op_editor_core::FileAction::Open);

        assert_eq!(
            unsafe { op_editor_take_shell_action(pointer, std::ptr::null_mut()) },
            OpStatus::InvalidArg
        );
        let mut action = -1;
        assert_eq!(
            unsafe { op_editor_take_shell_action(pointer, &mut action) },
            OpStatus::Ok
        );
        assert_eq!(action, SHELL_ACTION_OPEN_DOCUMENT);
        assert_eq!(
            unsafe { op_editor_take_shell_action(pointer, &mut action) },
            OpStatus::Ok
        );
        assert_eq!(action, SHELL_ACTION_NONE);

        // Save is engine-owned on mobile: with no sandbox binding yet, it
        // consumes the queued action and opens the save-name dialog instead
        // of emitting any shell action.
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        host.editor_state_mut().editor_ui.pending_file_action =
            Some(op_editor_core::FileAction::Save);
        assert_eq!(
            unsafe { op_editor_take_shell_action(pointer, &mut action) },
            OpStatus::Ok
        );
        assert_eq!(action, SHELL_ACTION_NONE);
        let ui = &engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui;
        assert_eq!(ui.pending_file_action, None);
        assert!(ui.save_name_dialog.open);
        assert!(!ui.save_name_dialog.save_as);
    }

    #[test]
    fn open_document_syncs_state_metadata_and_preserves_touch_chrome() {
        let mut engine = editor_engine(SAMPLE_DOC);
        let pointer = &mut engine as *mut OpEngine;
        let initial_epoch = {
            let host = engine.session_mut_for_test().editor_mut().unwrap();
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.touch = true;
            ui.size_class = EditorSizeClass::Medium;
            ui.sidebar_open = false;
            ui.theme_mode = op_editor_core::ThemeMode::Light;
            ui.mobile_sheet = Some(MobileSheetKind::More);
            host.document_epoch()
        };
        let document = with_editor_meta(TWO_PAGE_DOC);
        let file_name = "旅行计划.op";

        assert_eq!(
            unsafe {
                op_editor_open_document(
                    pointer,
                    document.as_ptr(),
                    document.len(),
                    file_name.as_ptr(),
                    file_name.len(),
                )
            },
            OpStatus::Ok
        );

        let session = engine.session_mut_for_test();
        assert_eq!(session.state.page_count(), 2);
        assert_eq!(session.state.ui.active_page_index, 1);
        assert!(session.state.editor_ui.preserve_authored_geometry);
        assert_eq!(
            session.state.editor_ui.file_name_display.as_deref(),
            Some(file_name)
        );
        assert_eq!(session.scene.active_page_index, 1);
        assert!(!session.user_interacted);

        let host = session.editor_mut().unwrap();
        let state = host.editor_state();
        assert_eq!(state.page_count(), 2);
        assert_eq!(state.ui.active_page_index, 1);
        assert!(state.editor_ui.preserve_authored_geometry);
        assert_eq!(
            state.editor_ui.file_name_display.as_deref(),
            Some(file_name)
        );
        assert!(state.editor_ui.touch);
        assert_eq!(state.editor_ui.size_class, EditorSizeClass::Medium);
        assert!(!state.editor_ui.sidebar_open);
        assert_eq!(state.editor_ui.theme_mode, op_editor_core::ThemeMode::Light);
        assert_eq!(state.editor_ui.mobile_sheet, None);
        assert!(!state.is_dirty());
        assert_eq!(host.document_epoch(), initial_epoch.wrapping_add(1));
    }

    #[test]
    fn rejected_document_leaves_live_document_and_chrome_unchanged() {
        let mut engine = editor_engine(TWO_PAGE_DOC);
        let pointer = &mut engine as *mut OpEngine;
        let before = {
            let session = engine.session_mut_for_test();
            let host = session.editor_mut().unwrap();
            host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
            (
                host.editor_state().doc.clone(),
                host.document_epoch(),
                host.editor_state().editor_ui.size_class,
            )
        };
        let bad = br#"{"version":"1.0.0","children":[{"type":"nonsense"}]}"#;

        assert_eq!(
            unsafe {
                op_editor_open_document(pointer, bad.as_ptr(), bad.len(), std::ptr::null(), 0)
            },
            OpStatus::BadDocument
        );

        let session = engine.session_mut_for_test();
        assert_eq!(session.state.page_count(), 2);
        let host = session.editor_mut().unwrap();
        assert_eq!(host.editor_state().doc, before.0);
        assert_eq!(host.document_epoch(), before.1);
        assert_eq!(host.editor_state().editor_ui.size_class, before.2);
        assert_eq!(
            host.editor_state().editor_ui.mobile_sheet,
            Some(MobileSheetKind::More)
        );
    }

    #[test]
    fn open_document_activates_thumbnails_only_after_final_acceptance() {
        const OLD_ID: u64 = 8_100_001;
        const LAYOUT_REJECT_ID: u64 = 8_100_002;
        const COLLAB_REJECT_ID: u64 = 8_100_003;
        const ACCEPTED_ID: u64 = 8_100_004;

        struct ClearThumbnailRegistry;
        impl Drop for ClearThumbnailRegistry {
            fn drop(&mut self) {
                jian_ops_schema::image_thumbs::clear_registry();
            }
        }

        jian_ops_schema::image_thumbs::clear_registry();
        let _clear = ClearThumbnailRegistry;
        jian_ops_schema::image_thumbs::store_thumb(OLD_ID, vec![1, 2, 3]);

        let mut layout_engine = editor_engine(SAMPLE_DOC);
        let layout_pointer = &mut layout_engine as *mut OpEngine;
        let layout_reject = format!(
            r#"{{"version":"1.0.0","children":[],"pages":[],"imageThumbs":{{"{LAYOUT_REJECT_ID}":"/9j/2Q=="}}}}"#
        );
        assert_eq!(
            unsafe {
                op_editor_open_document(
                    layout_pointer,
                    layout_reject.as_ptr(),
                    layout_reject.len(),
                    std::ptr::null(),
                    0,
                )
            },
            OpStatus::LayoutError
        );
        assert_eq!(
            &*jian_ops_schema::image_thumbs::thumb_for(OLD_ID).expect("old thumbnail survives"),
            &[1, 2, 3]
        );
        assert!(jian_ops_schema::image_thumbs::thumb_for(LAYOUT_REJECT_ID).is_none());

        let mut collab_engine = editor_engine(SAMPLE_DOC);
        assert!(collab_engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .collab
            .set_authenticated_session(
                CollabConnectionPhase::Active,
                AuthenticatedCollabSession {
                    session_name: "Test session".to_string(),
                    role: CollabUiRole::Viewer,
                    share_endpoint: None,
                },
                Vec::new(),
            ));
        let collab_pointer = &mut collab_engine as *mut OpEngine;
        let collab_reject = with_image_thumb(TWO_PAGE_DOC, COLLAB_REJECT_ID);
        assert_eq!(
            unsafe {
                op_editor_open_document(
                    collab_pointer,
                    collab_reject.as_ptr(),
                    collab_reject.len(),
                    std::ptr::null(),
                    0,
                )
            },
            OpStatus::Busy
        );
        assert_eq!(
            &*jian_ops_schema::image_thumbs::thumb_for(OLD_ID).expect("old thumbnail survives"),
            &[1, 2, 3]
        );
        assert!(jian_ops_schema::image_thumbs::thumb_for(COLLAB_REJECT_ID).is_none());

        let mut accepted_engine = editor_engine(SAMPLE_DOC);
        let accepted_pointer = &mut accepted_engine as *mut OpEngine;
        let accepted = with_image_thumb(TWO_PAGE_DOC, ACCEPTED_ID);
        assert_eq!(
            unsafe {
                op_editor_open_document(
                    accepted_pointer,
                    accepted.as_ptr(),
                    accepted.len(),
                    std::ptr::null(),
                    0,
                )
            },
            OpStatus::Ok
        );
        assert!(jian_ops_schema::image_thumbs::thumb_for(OLD_ID).is_none());
        assert_eq!(
            &*jian_ops_schema::image_thumbs::thumb_for(ACCEPTED_ID)
                .expect("accepted thumbnail activates"),
            &[0xff, 0xd8, 0xff, 0xd9]
        );
    }
}
