//! Shell-owned image / SVG import for the mobile editor.
//!
//! The engine-painted shape picker only queues a one-shot shell action. The
//! platform owns the document picker and returns bounded bytes through the C
//! ABI. The live collaboration policy is checked again at that return seam so
//! a role or session transition while UIKit is open cannot authorize a stale
//! import.

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;

/// Append-only shell action code. Existing mobile shells keep their meanings.
pub const SHELL_ACTION_IMPORT_IMAGE_OR_SVG: i32 = 12;

/// Matches the iOS bounded picker reader and keeps the transient input plus
/// the embedded data URL within a phone-safe allocation envelope.
const IMAGE_IMPORT_CAP: usize = 32 * 1024 * 1024;
const FILE_NAME_CAP: usize = 4 * 1024;
const LAYER_NAME_CHAR_CAP: usize = 120;

/// Consume the queued toolbar request before asking the platform to present
/// its picker. A cancelled picker therefore remains a silent, exactly-once
/// no-op instead of reopening on every later frame.
pub(crate) fn begin_import(session: &mut Session) -> FfiResult<i32> {
    let host = session.editor_mut()?;
    host.editor_state_mut().editor_ui.pending_file_action = None;
    host.mark_editor_state_dirty();
    Ok(SHELL_ACTION_IMPORT_IMAGE_OR_SVG)
}

/// Insert one image or editable SVG selected by the platform picker.
///
/// Raster content is magic-sniffed and embedded as a portable `data:` URL.
/// An `.svg` file is parsed by the canonical editable SVG importer. The byte
/// and file-name pointers are borrowed only for this call.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. Non-empty byte ranges
/// must cover readable memory for their declared lengths.
#[no_mangle]
pub unsafe extern "C" fn op_editor_import_image_or_svg(
    engine: *mut crate::OpEngine,
    data_ptr: *const u8,
    data_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            validate_import_bytes(data_ptr, data_len)?;
            // SAFETY: validation above rejects null, overflowing, and
            // over-cap ranges; the C contract keeps the borrow live for this
            // call and no reference escapes the closure.
            let bytes = std::slice::from_raw_parts(data_ptr, data_len);
            let file_name = crate::error::read_utf8(
                file_name_ptr,
                file_name_len,
                FILE_NAME_CAP,
                "image import file name",
            )?;
            let name = import_layer_name(&file_name)?;
            let is_svg = file_extension(&file_name).as_deref() == Some("svg");

            if is_svg {
                let svg = std::str::from_utf8(bytes)
                    .map_err(|_| FfiError::invalid("SVG import is not valid UTF-8"))?;
                import_svg(session, svg, &name)
            } else {
                let mime =
                    op_image_enrich::net::providers::sniff_image_mime(bytes).ok_or_else(|| {
                        FfiError::new(
                            OpStatus::BadDocument,
                            "image import is not PNG, JPEG, GIF, WebP, or SVG",
                        )
                    })?;
                let data_url = op_image_enrich::net::fetch::image_bytes_to_data_url(mime, bytes)
                    .ok_or_else(|| {
                        FfiError::new(OpStatus::BadDocument, "image import could not be embedded")
                    })?;
                import_raster(session, &data_url, &name)
            }
        })
    }
}

fn import_svg(session: &mut Session, svg: &str, name: &str) -> FfiResult<()> {
    gate_external_asset_import(session)?;
    let count = {
        let host = session.editor_mut()?;
        let state = host.editor_state_mut();
        let zoom = (state.viewport.zoom as f64).max(0.001);
        let centre_x = -(state.viewport.pan_x as f64) / zoom;
        let centre_y = -(state.viewport.pan_y as f64) / zoom;
        let mut next_id = 0_u64;
        let count = state.import_svg_named(
            &mut next_id,
            svg,
            (centre_x - 200.0, centre_y - 150.0),
            Some(name),
        );
        if count != 0 {
            host.mark_editor_state_dirty();
        }
        count
    };
    if count == 0 {
        return Err(FfiError::new(
            OpStatus::BadDocument,
            "SVG import contains no supported editable nodes",
        ));
    }
    sync_after_import(session)
}

fn import_raster(session: &mut Session, data_url: &str, name: &str) -> FfiResult<()> {
    gate_external_asset_import(session)?;
    let inserted = {
        let host = session.editor_mut()?;
        let inserted = host
            .editor_state_mut()
            .insert_image_node_at_viewport(name, data_url)
            .is_some();
        if inserted {
            host.mark_editor_state_dirty();
        }
        inserted
    };
    if !inserted {
        return Err(FfiError::new(
            OpStatus::OutOfMemory,
            "image import could not allocate a document node id",
        ));
    }
    sync_after_import(session)
}

/// Re-check immediately before mutation. The picker can remain open across a
/// collaboration role or phase change, so the earlier shape-picker gate is
/// not sufficient on its own.
fn gate_external_asset_import(session: &mut Session) -> FfiResult<()> {
    let allowed = session.editor_mut()?.gate_collaboration_action(
        op_editor_core::CollabGateAction::Document(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ),
        op_editor_core::CollabEditSource::Import,
    );
    if allowed {
        return Ok(());
    }
    // The host installed a bounded rejection notice; ensure it is painted
    // even though this ABI call returns a typed failure to the shell.
    session.request_redraw();
    Err(FfiError::new(
        OpStatus::Busy,
        "image import is blocked by the collaboration session",
    ))
}

fn sync_after_import(session: &mut Session) -> FfiResult<()> {
    let next_state = session
        .editor()
        .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?
        .editor_state()
        .clone();
    let next_scene = op_pen_loader::editor_state_to_active_page_layout_scene(&next_state);
    if next_scene.active_page().is_none() {
        return Err(FfiError::new(
            OpStatus::LayoutError,
            "image import left no renderable page",
        ));
    }
    session.selected = next_state
        .selection
        .anchor
        .is_real()
        .then(|| next_state.selection.anchor.as_str().to_string());
    session.state = next_state;
    session.scene = next_scene;
    session.request_redraw();
    Ok(())
}

fn validate_import_bytes(pointer: *const u8, length: usize) -> FfiResult<()> {
    if length == 0 {
        return Err(FfiError::invalid("image import is empty"));
    }
    if length > IMAGE_IMPORT_CAP {
        return Err(FfiError::invalid(format!(
            "image import length exceeds {IMAGE_IMPORT_CAP} bytes"
        )));
    }
    if pointer.is_null() {
        return Err(FfiError::invalid(
            "image import pointer is null with nonzero length",
        ));
    }
    if length > isize::MAX as usize {
        return Err(FfiError::invalid("image import length overflows"));
    }
    Ok(())
}

fn file_extension(file_name: &str) -> Option<String> {
    std::path::Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn import_layer_name(file_name: &str) -> FfiResult<String> {
    if file_name.chars().any(char::is_control)
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(FfiError::invalid("image import file name is invalid"));
    }
    let stem = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .trim();
    let name: String = stem.chars().take(LAYER_NAME_CHAR_CAP).collect();
    Ok(if name.is_empty() {
        "Image".to_string()
    } else {
        name
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::editor::op_editor_take_shell_action;
    use crate::editor_auth::SHELL_ACTION_NONE;
    use crate::lifecycle::OpEngine;
    use op_editor_core::{
        AuthenticatedCollabSession, CollabConnectionPhase, CollabUiRole, FileAction,
    };

    fn engine() -> OpEngine {
        OpEngine::new(
            Session::new(CreateOptions {
                document: String::new(),
                width: 1_024.0,
                height: 768.0,
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
        let mut action = -1;
        assert_eq!(
            unsafe { op_editor_take_shell_action(engine, &mut action) },
            OpStatus::Ok
        );
        action
    }

    fn import(engine: &mut OpEngine, bytes: &[u8], name: &str) -> OpStatus {
        unsafe {
            op_editor_import_image_or_svg(
                engine,
                bytes.as_ptr(),
                bytes.len(),
                name.as_ptr(),
                name.len(),
            )
        }
    }

    fn children(engine: &mut OpEngine) -> usize {
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .active_children()
            .len()
    }

    #[test]
    fn import_shell_action_consumes_the_file_request_exactly_once() {
        let mut engine = engine();
        engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .pending_file_action = Some(FileAction::ImportImageOrSvg);

        assert_eq!(drain(&mut engine), SHELL_ACTION_IMPORT_IMAGE_OR_SVG);
        assert_eq!(drain(&mut engine), SHELL_ACTION_NONE);
        assert_eq!(
            engine
                .session_mut_for_test()
                .editor_mut()
                .unwrap()
                .editor_state()
                .editor_ui
                .pending_file_action,
            None
        );
    }

    #[test]
    fn raster_bytes_become_a_portable_data_url_image_node() {
        let mut engine = engine();
        let before = children(&mut engine);
        let png = b"\x89PNG\r\n\x1a\nmobile-picker-payload";

        assert_eq!(import(&mut engine, png, "照片.PNG"), OpStatus::Ok);
        assert_eq!(children(&mut engine), before + 1);
        let state = engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state();
        let image = state
            .active_children()
            .iter()
            .find_map(|node| match node {
                jian_ops_schema::node::PenNode::Image(image) => Some(image),
                _ => None,
            })
            .expect("imported image");
        assert_eq!(image.base.name.as_deref(), Some("照片"));
        assert!(image.src.as_str().starts_with("data:image/png;base64,"));
    }

    #[test]
    fn jpeg_gif_and_webp_picker_payloads_are_embedded() {
        for (bytes, file_name, prefix) in [
            (
                b"\xff\xd8\xffmobile-jpeg".as_slice(),
                "photo.jpg",
                "data:image/jpeg;base64,",
            ),
            (
                b"GIF89amobile-gif".as_slice(),
                "motion.gif",
                "data:image/gif;base64,",
            ),
            (
                b"RIFF\x10\x00\x00\x00WEBPVP8 mobile-webp".as_slice(),
                "image.webp",
                "data:image/webp;base64,",
            ),
        ] {
            let mut engine = engine();
            assert_eq!(import(&mut engine, bytes, file_name), OpStatus::Ok);
            let state = engine
                .session_mut_for_test()
                .editor_mut()
                .unwrap()
                .editor_state();
            let src = state
                .active_children()
                .iter()
                .find_map(|node| match node {
                    jian_ops_schema::node::PenNode::Image(image) => Some(image.src.as_str()),
                    _ => None,
                })
                .expect("imported image source");
            assert!(src.starts_with(prefix), "{file_name}: {src}");
        }
    }

    #[test]
    fn svg_import_uses_editable_nodes_and_the_file_stem() {
        let mut engine = engine();
        let before = children(&mut engine);
        let svg = br##"<svg width="24" height="24"><rect x="1" y="2" width="20" height="18" fill="#123456"/></svg>"##;

        assert_eq!(import(&mut engine, svg, "mark.svg"), OpStatus::Ok);
        assert_eq!(children(&mut engine), before + 1);
        let state = engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state();
        let group = state
            .active_children()
            .iter()
            .find_map(|node| match node {
                jian_ops_schema::node::PenNode::Group(group) => Some(group),
                _ => None,
            })
            .expect("editable SVG group");
        assert_eq!(group.base.name.as_deref(), Some("mark"));
        assert!(group
            .children
            .as_ref()
            .is_some_and(|nodes| !nodes.is_empty()));
    }

    #[test]
    fn picker_result_rechecks_collaboration_before_mutation() {
        let mut engine = engine();
        let before = children(&mut engine);
        let collab = &mut engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state_mut()
            .editor_ui
            .collab;
        assert!(collab.set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "iPad review".to_string(),
                role: CollabUiRole::Viewer,
                share_endpoint: None,
            },
            Vec::new(),
        ));

        assert_eq!(
            import(&mut engine, b"\x89PNG\r\n\x1a\nblocked", "blocked.png"),
            OpStatus::Busy
        );
        assert_eq!(children(&mut engine), before);
    }

    #[test]
    fn invalid_or_unbounded_payloads_fail_without_mutating() {
        let mut engine = engine();
        let before = children(&mut engine);
        assert_eq!(
            import(&mut engine, b"not-an-image", "bad.png"),
            OpStatus::BadDocument
        );
        assert_eq!(children(&mut engine), before);
        assert_eq!(
            unsafe {
                op_editor_import_image_or_svg(
                    &mut engine,
                    std::ptr::null(),
                    IMAGE_IMPORT_CAP + 1,
                    b"large.png".as_ptr(),
                    b"large.png".len(),
                )
            },
            OpStatus::InvalidArg
        );
        assert_eq!(children(&mut engine), before);
    }
}
