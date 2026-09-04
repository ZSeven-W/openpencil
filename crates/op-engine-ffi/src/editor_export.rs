//! Frozen editor exports handed from Rust chrome to native save UIs.
//!
//! Rendering happens while the engine is still on its owner thread. The shell
//! later supplies one app-private staging path; Rust creates that file exactly
//! once, so large PNG/PDF payloads never cross the C ABI as a second copy.

use crate::error::{read_utf8, FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;
#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
use op_editor_core::FileAction;
use op_editor_host_core::codegen_export::CodegenArtifact;
use op_render_export::ExportArtifact;
#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
use op_render_export::{EditorExportScope, ExportError};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Save a frozen editor export through the platform document UI.
pub const SHELL_ACTION_EXPORT_DOCUMENT: i32 = 4;

const EXPORT_FILENAME_CAP: usize = 4 * 1024;
const EXPORT_PATH_CAP: usize = 16 * 1024;

#[derive(Default)]
pub(crate) struct EditorExportShellState {
    artifact: Option<ExportArtifact>,
}

/// Peek or copy the pending export's UTF-8 file name.
///
/// A null buffer with zero capacity reports the required byte length. Unlike
/// the login URL bridge, copying the name does not consume the artifact; only
/// a successful file write or explicit cancellation does.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. `required` must be
/// writable; a non-null `buffer` must cover `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_copy_export_file_name(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            copy_export_file_name(&session.export_shell, buffer, capacity, required)
        })
    }
}

/// Write the frozen export to a new shell-owned absolute staging path.
///
/// The target must not exist. A complete write consumes the artifact; any
/// validation, create, or write failure leaves it available for retry.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. A non-empty path byte
/// range must cover readable UTF-8 bytes for its declared length.
#[no_mangle]
pub unsafe extern "C" fn op_editor_export_to_path(
    engine: *mut crate::OpEngine,
    path_ptr: *const u8,
    path_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let path = read_export_path(path_ptr, path_len)?;
            write_pending_export(&mut session.export_shell, &path)
        })
    }
}

/// Discard the frozen export when the platform cannot present its save UI.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_cancel_export(engine: *mut crate::OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.export_shell.artifact.take().is_none() {
                return Err(FfiError::new(OpStatus::NotReady, "no export is pending"));
            }
            Ok(())
        })
    }
}

#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
pub(crate) fn stage_export(session: &mut Session, action: Option<FileAction>) -> FfiResult<i32> {
    let state = {
        let host = session.editor_mut()?;
        // Consume the UI request before rendering. A failed encoder must
        // surface once, not retry on every frame and allocate repeatedly.
        host.editor_state_mut().editor_ui.pending_file_action = None;
        host.editor_state().clone()
    };
    let scope = match action {
        Some(FileAction::ExportImageConfirm) => EditorExportScope::Configured,
        Some(FileAction::ExportDeckPdfSelection) => EditorExportScope::DeckBoards(
            op_editor_core::preview_slideshow::selected_page_boards(&state),
        ),
        _ => return Err(FfiError::invalid("file action is not an export request")),
    };
    if session.export_shell.artifact.is_some() {
        return Err(FfiError::new(
            OpStatus::Busy,
            "the previous export is still waiting for the platform save UI",
        ));
    }
    let artifact =
        op_render_export::export_editor_state(&state, &scope).map_err(export_error_to_ffi)?;
    session.export_shell.artifact = Some(artifact);
    Ok(SHELL_ACTION_EXPORT_DOCUMENT)
}

/// Move one Code-panel artifact into the same frozen export slot used by
/// rendered files. The existing action 4 ABI then drives every native save UI.
pub(crate) fn drain_codegen_export(session: &mut Session) -> FfiResult<Option<i32>> {
    // Do not pop the codegen queue while another file is still owned by a
    // platform picker; the next shell-action poll can drain it after consume.
    if session.export_shell.artifact.is_some() {
        return Ok(None);
    }
    let artifact = session
        .editor
        .as_mut()
        .and_then(|host| session.codegen.drain_artifact(host));
    let Some(artifact) = artifact else {
        return Ok(None);
    };
    stage_codegen_artifact(&mut session.export_shell, artifact)?;
    Ok(Some(SHELL_ACTION_EXPORT_DOCUMENT))
}

fn stage_codegen_artifact(
    state: &mut EditorExportShellState,
    artifact: CodegenArtifact,
) -> FfiResult<()> {
    if state.artifact.is_some() {
        return Err(FfiError::new(
            OpStatus::Busy,
            "the previous export is still waiting for the platform save UI",
        ));
    }
    let CodegenArtifact {
        file_name,
        mime_type,
        bytes,
    } = artifact;
    if file_name.is_empty()
        || file_name.len() > EXPORT_FILENAME_CAP
        || file_name == "."
        || file_name == ".."
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.chars().any(char::is_control)
        || Path::new(&file_name).extension().is_none()
    {
        return Err(FfiError::new(
            OpStatus::LayoutError,
            "code generation produced an invalid export file name",
        ));
    }
    state.artifact = Some(ExportArtifact {
        mime_type,
        file_name,
        bytes,
    });
    Ok(())
}

unsafe fn copy_export_file_name(
    state: &EditorExportShellState,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> FfiResult<()> {
    if required.is_null() {
        return Err(FfiError::invalid(
            "export file-name required-length pointer is null",
        ));
    }
    let Some(artifact) = state.artifact.as_ref() else {
        unsafe { required.write(0) };
        return Err(FfiError::new(OpStatus::NotReady, "no export is pending"));
    };
    let bytes = artifact.file_name.as_bytes();
    unsafe { required.write(bytes.len()) };
    if bytes.is_empty() || bytes.len() > EXPORT_FILENAME_CAP {
        return Err(FfiError::invalid(format!(
            "export file name must contain 1..={EXPORT_FILENAME_CAP} bytes"
        )));
    }
    if buffer.is_null() {
        if capacity == 0 {
            return Ok(());
        }
        return Err(FfiError::invalid(
            "export file-name buffer is null with nonzero capacity",
        ));
    }
    if capacity < bytes.len() {
        return Err(FfiError::invalid(format!(
            "export file-name buffer covers {capacity} bytes but {} are required",
            bytes.len()
        )));
    }
    if capacity > isize::MAX as usize {
        return Err(FfiError::invalid(
            "export file-name buffer capacity overflows",
        ));
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    Ok(())
}

unsafe fn read_export_path(path_ptr: *const u8, path_len: usize) -> FfiResult<PathBuf> {
    let value = unsafe { read_utf8(path_ptr, path_len, EXPORT_PATH_CAP, "export path")? };
    if value.is_empty() {
        return Err(FfiError::invalid("export path is empty"));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(FfiError::invalid("export path must be absolute"));
    }
    Ok(path)
}

fn write_pending_export(state: &mut EditorExportShellState, path: &Path) -> FfiResult<()> {
    let Some(artifact) = state.artifact.as_ref() else {
        return Err(FfiError::new(OpStatus::NotReady, "no export is pending"));
    };
    let target_name = path.file_name().and_then(|name| name.to_str());
    if target_name != Some(artifact.file_name.as_str()) {
        return Err(FfiError::invalid(
            "export target file name does not match the frozen artifact",
        ));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            FfiError::new(
                OpStatus::InvalidArg,
                format!("could not create export staging file: {error}"),
            )
        })?;
    let write_result = file.write_all(&artifact.bytes).and_then(|()| file.flush());
    drop(file);
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(path);
        return Err(FfiError::new(
            OpStatus::OutOfMemory,
            format!("could not write export staging file: {error}"),
        ));
    }
    state.artifact = None;
    Ok(())
}

#[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
fn export_error_to_ffi(error: ExportError) -> FfiError {
    let status = match &error {
        ExportError::OutputTooLarge { .. } | ExportError::SurfaceAlloc => OpStatus::OutOfMemory,
        ExportError::UnsupportedFormat { .. } => OpStatus::NotReady,
        _ => OpStatus::LayoutError,
    };
    FfiError::new(status, format!("could not export the document: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    use crate::lifecycle::Session;
    use op_editor_core::ExportFormat;
    use std::sync::atomic::{AtomicU64, Ordering};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");
    static TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn session() -> Session {
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 800.0,
            height: 600.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session")
    }

    fn stage_png(session: &mut Session) {
        let host = session.editor_mut().expect("editor host");
        host.editor_state_mut().editor_ui.export_format = ExportFormat::Png;
        host.editor_state_mut().editor_ui.export_scale = 1.0;
        host.editor_state_mut().editor_ui.pending_file_action =
            Some(FileAction::ExportImageConfirm);
        assert_eq!(
            stage_export(session, Some(FileAction::ExportImageConfirm)).expect("stage export"),
            SHELL_ACTION_EXPORT_DOCUMENT
        );
    }

    fn code_artifact(file_name: &str, mime_type: &'static str, bytes: &[u8]) -> CodegenArtifact {
        CodegenArtifact {
            file_name: file_name.into(),
            mime_type,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn shell_action_is_emitted_once_while_the_artifact_remains_frozen() {
        let mut session = session();
        let host = session.editor_mut().expect("editor host");
        host.editor_state_mut().editor_ui.export_format = ExportFormat::Png;
        host.editor_state_mut().editor_ui.export_scale = 1.0;
        host.editor_state_mut().editor_ui.pending_file_action =
            Some(FileAction::ExportImageConfirm);

        assert_eq!(
            crate::editor_auth::take_shell_action(&mut session).expect("export action"),
            SHELL_ACTION_EXPORT_DOCUMENT
        );
        assert_eq!(
            crate::editor_auth::take_shell_action(&mut session).expect("second drain"),
            crate::editor_auth::SHELL_ACTION_NONE
        );
        assert!(session.export_shell.artifact.is_some());
    }

    #[test]
    fn file_name_queries_do_not_consume_and_short_buffers_fail() {
        let mut session = session();
        stage_png(&mut session);
        let mut required = 0;
        unsafe {
            copy_export_file_name(
                &session.export_shell,
                std::ptr::null_mut(),
                0,
                &mut required,
            )
        }
        .expect("size query");
        assert!(required > 4);

        let mut short = vec![0_u8; required - 1];
        assert!(unsafe {
            copy_export_file_name(
                &session.export_shell,
                short.as_mut_ptr(),
                short.len(),
                &mut required,
            )
        }
        .is_err());
        assert!(session.export_shell.artifact.is_some());
    }

    #[test]
    fn successful_create_new_write_consumes_the_png() {
        let mut session = session();
        stage_png(&mut session);
        let file_name = session
            .export_shell
            .artifact
            .as_ref()
            .expect("artifact")
            .file_name
            .clone();
        let root = std::env::temp_dir().join(format!(
            "openpencil-ffi-export-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("temp root");
        let path = root.join(file_name);
        write_pending_export(&mut session.export_shell, &path).expect("write export");
        let bytes = std::fs::read(&path).expect("read export");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(session.export_shell.artifact.is_none());
        assert!(write_pending_export(&mut session.export_shell, &path).is_err());
        std::fs::remove_dir_all(root).expect("clean temp root");
    }

    #[test]
    fn codegen_source_uses_the_same_retry_safe_frozen_slot() {
        let mut session = session();
        let source = b"export default function App() {}";
        stage_codegen_artifact(
            &mut session.export_shell,
            code_artifact("component.tsx", "text/plain; charset=utf-8", source),
        )
        .expect("stage code source");
        let artifact = session.export_shell.artifact.as_ref().expect("artifact");
        assert_eq!(artifact.file_name, "component.tsx");
        assert_eq!(artifact.mime_type, "text/plain; charset=utf-8");

        let root = std::env::temp_dir().join(format!(
            "openpencil-ffi-codegen-export-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("temp root");
        assert!(write_pending_export(&mut session.export_shell, &root.join("wrong.tsx")).is_err());
        assert!(session.export_shell.artifact.is_some());
        let path = root.join("component.tsx");
        write_pending_export(&mut session.export_shell, &path).expect("write source");
        assert_eq!(std::fs::read(&path).expect("read source"), source);
        assert!(session.export_shell.artifact.is_none());
        std::fs::remove_dir_all(root).expect("clean temp root");
    }

    #[test]
    fn invalid_codegen_artifact_name_never_reaches_the_platform() {
        let mut session = session();
        let error = stage_codegen_artifact(
            &mut session.export_shell,
            code_artifact("../component.tsx", "text/plain", b"source"),
        )
        .expect_err("path traversal must be rejected");
        assert_eq!(error.status, OpStatus::LayoutError);
        assert!(session.export_shell.artifact.is_none());
    }

    #[test]
    fn codegen_staging_never_overwrites_a_frozen_export() {
        let mut session = session();
        stage_codegen_artifact(
            &mut session.export_shell,
            code_artifact("component.tsx", "text/plain", b"first"),
        )
        .expect("first artifact");
        let error = stage_codegen_artifact(
            &mut session.export_shell,
            code_artifact("bundle.zip", "application/zip", b"second"),
        )
        .expect_err("frozen slot must stay exclusive");
        assert_eq!(error.status, OpStatus::Busy);
        let frozen = session
            .export_shell
            .artifact
            .as_ref()
            .expect("first remains");
        assert_eq!(frozen.file_name, "component.tsx");
        assert_eq!(frozen.bytes, b"first");
    }

    #[test]
    fn queued_codegen_export_waits_for_the_frozen_slot_and_retries() {
        let mut session = session();
        stage_codegen_artifact(
            &mut session.export_shell,
            code_artifact("component.tsx", "text/plain", b"first"),
        )
        .expect("first artifact");
        session
            .editor_mut()
            .expect("editor host")
            .editor_state_mut()
            .codegen
            .pending_export_bundle = true;
        session.pump_editor_codegen(1);

        assert_eq!(
            drain_codegen_export(&mut session).expect("busy drain"),
            None
        );
        let frozen = session
            .export_shell
            .artifact
            .as_ref()
            .expect("first remains");
        assert_eq!(frozen.file_name, "component.tsx");
        assert_eq!(frozen.bytes, b"first");

        session.export_shell.artifact = None;
        assert_eq!(
            drain_codegen_export(&mut session).expect("retry drain"),
            Some(SHELL_ACTION_EXPORT_DOCUMENT)
        );
        assert_eq!(
            session
                .export_shell
                .artifact
                .as_ref()
                .expect("bundle")
                .file_name,
            "bundle.zip"
        );
        assert_eq!(
            drain_codegen_export(&mut session).expect("terminal drain"),
            None
        );
    }

    #[test]
    fn mismatched_or_existing_targets_fail_without_consuming() {
        let mut session = session();
        stage_png(&mut session);
        let artifact_name = session
            .export_shell
            .artifact
            .as_ref()
            .expect("artifact")
            .file_name
            .clone();
        let root = std::env::temp_dir().join(format!(
            "openpencil-ffi-export-retry-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("temp root");

        assert!(write_pending_export(&mut session.export_shell, &root.join("other.png")).is_err());
        assert!(session.export_shell.artifact.is_some());
        let expected = root.join(artifact_name);
        std::fs::write(&expected, b"existing").expect("existing target");
        assert!(write_pending_export(&mut session.export_shell, &expected).is_err());
        assert!(session.export_shell.artifact.is_some());
        assert_eq!(std::fs::read(&expected).expect("read target"), b"existing");

        std::fs::remove_dir_all(root).expect("clean temp root");
    }

    #[cfg(any(target_os = "ios", target_os = "android"))]
    #[test]
    fn unsupported_webp_is_consumed_once_without_staging_bytes() {
        let mut session = session();
        let host = session.editor_mut().expect("editor host");
        host.editor_state_mut().editor_ui.export_format = ExportFormat::Webp;
        host.editor_state_mut().editor_ui.pending_file_action =
            Some(FileAction::ExportImageConfirm);

        let error = stage_export(&mut session, Some(FileAction::ExportImageConfirm))
            .expect_err("mobile WebP is unavailable");
        assert_eq!(error.status, OpStatus::NotReady);
        assert!(session.export_shell.artifact.is_none());
        assert_eq!(
            session
                .editor()
                .expect("editor host")
                .editor_state()
                .editor_ui
                .pending_file_action,
            None
        );
    }
}
