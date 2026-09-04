//! Versioned descriptor structs + parsing for the OpenPencil C ABI.
//!
//! Every descriptor is tail-growable: the caller sets `size` to the
//! number of bytes it wrote; missing trailing fields read as zero/null
//! and a size beyond the current struct is rejected. This keeps the ABI
//! stable while shells and engine evolve independently.

use crate::error::{read_utf8, FfiError, FfiResult, DOCUMENT_CAP, STRING_CAP};
use crate::OpStatus;
use std::ffi::c_void;
use std::mem::{offset_of, size_of};
use std::ptr;

/// May run on a collaboration worker so a paused mobile frame pump can be
/// restarted. The shell must marshal UI work and keep `user_data` alive until
/// `op_destroy` returns; the callback must not re-enter the engine.
pub type OpNeedsRedraw =
    Option<unsafe extern "C" fn(user_data: *mut c_void, has_next_wake: bool, next_wake_ms: u64)>;

pub type OpRuntimeErrorCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, error: *const OpRuntimeError)>;

/// Inline text-edit focus transition: the shell shows/hides the system
/// keyboard and configures its input traits. `input_kind` is `0` (text)
/// for the player's editable nodes; `return_key_hint` is `0` (default).
pub type OpInputFocusChanged = Option<
    unsafe extern "C" fn(
        user_data: *mut c_void,
        focused: bool,
        input_kind: i32,
        return_key_hint: i32,
    ),
>;

/// A paint pass recorded a remote (`http(s)`) image miss. The shell
/// fetches the URL and pushes the bytes back via `op_remote_image_result`
/// (empty bytes = fetch failed). The URL is borrowed for the call.
pub type OpRemoteImageRequest = Option<
    unsafe extern "C" fn(
        user_data: *mut c_void,
        request_id: u64,
        url_ptr: *const u8,
        url_len: usize,
    ),
>;

/// Load the platform-protected collaboration credential into `out`.
/// This callback may also run on a collaboration worker.
/// Returns 0 when found, 1 when absent, and a negative value on failure.
pub type OpCredentialLoad = Option<
    unsafe extern "C" fn(
        user_data: *mut c_void,
        out: *mut u8,
        capacity: usize,
        out_len: *mut usize,
    ) -> i32,
>;

/// Atomically store a collaboration credential unless one already exists.
/// Returns 0 for success/already-present and a negative value on failure.
pub type OpCredentialStoreIfAbsent =
    Option<unsafe extern "C" fn(user_data: *mut c_void, value: *const u8, value_len: usize) -> i32>;

/// Runtime diagnostic payload copied synchronously into the callback;
/// the shell owns the copy and may react asynchronously.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpRuntimeError {
    pub kind: i32,
    pub message_ptr: *const u8,
    pub message_len: usize,
    pub source_ptr: *const u8,
    pub source_len: usize,
}

/// Callback table. Future callbacks grow only at the tail.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpCallbacks {
    pub size: usize,
    pub user_data: *mut c_void,
    pub needs_redraw: OpNeedsRedraw,
    pub runtime_error: OpRuntimeErrorCallback,
    pub input_focus_changed: OpInputFocusChanged,
    pub remote_image_request: OpRemoteImageRequest,
    pub credential_load: OpCredentialLoad,
    pub credential_store_if_absent: OpCredentialStoreIfAbsent,
}

/// Engine construction descriptor. `doc_ptr`/`doc_len` normally hold a
/// non-empty canonical `.op` document; a full-editor shell (`mode == 1`) may
/// pass null/zero to open [`op_editor_core::EditorState::starter`]. Viewer mode
/// always requires document bytes. `asset_base` is the filesystem root for
/// doc-referenced media. Mobile editor shells must also provide `storage_root`,
/// their private sandbox root, before any runtime/config service is
/// constructed. `documents_root` is the optional user-visible directory saved
/// documents land in (iOS `NSDocumentDirectory`, which the Files app exposes);
/// omitting it keeps saves under `<storage_root>/documents`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpCreateDesc {
    pub size: usize,
    pub doc_ptr: *const u8,
    pub doc_len: usize,
    pub width: f32,
    pub height: f32,
    pub dpr: f32,
    pub callbacks: *const OpCallbacks,
    pub asset_base_ptr: *const u8,
    pub asset_base_len: usize,
    /// 0 = viewer (default), 1 = full editor (widget-host chrome).
    pub mode: i32,
    /// Private application-sandbox root for process-level config.
    pub storage_root_ptr: *const u8,
    pub storage_root_len: usize,
    /// Optional user-visible directory for saved `.op` documents. Absolute
    /// path; null/zero falls back to `<storage_root>/documents`.
    pub documents_root_ptr: *const u8,
    pub documents_root_len: usize,
}

/// Platform surface descriptor. On iOS `handle` is a borrowed
/// `CAMetalLayer *`; on Android it is a borrowed `ANativeWindow *`
/// (acquired by the shell and released only after suspend/destroy
/// returns).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpSurfaceDesc {
    pub size: usize,
    pub handle: *mut c_void,
}

/// Surrounding-text snapshot for the shell's input connection. The text
/// pointer is BORROWED from the engine and valid only until the next
/// engine call; offsets are UTF-16 code units.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpTextState {
    pub text_ptr: *const u8,
    pub text_len: usize,
    pub selection_start: u32,
    pub selection_end: u32,
    pub has_composing: bool,
    pub composing_start: u32,
    pub composing_end: u32,
}

/// Pointer phases shared with the shells.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpPointerPhase {
    Down = 0,
    Move = 1,
    Up = 2,
    Cancel = 3,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Callbacks {
    pub user_data: *mut c_void,
    pub needs_redraw: OpNeedsRedraw,
    pub runtime_error: OpRuntimeErrorCallback,
    pub input_focus_changed: OpInputFocusChanged,
    pub remote_image_request: OpRemoteImageRequest,
    #[cfg(feature = "editor")]
    pub credential_load: OpCredentialLoad,
    #[cfg(feature = "editor")]
    pub credential_store_if_absent: OpCredentialStoreIfAbsent,
}

pub(crate) struct CreateOptions {
    pub document: String,
    pub width: f32,
    pub height: f32,
    pub dpr: f32,
    pub callbacks: Callbacks,
    pub asset_base: Option<String>,
    #[cfg(feature = "editor")]
    pub editor_mode: bool,
    /// User-visible directory saved documents land in, when the shell has
    /// one. `None` keeps the private `<storage_root>/documents` fallback.
    #[cfg(feature = "editor")]
    pub documents_root: Option<String>,
}

unsafe fn read_covered<T: Copy>(base: *const u8, size: usize, offset: usize) -> Option<T> {
    let end = offset.checked_add(size_of::<T>())?;
    if end > size {
        return None;
    }
    Some(unsafe { ptr::read_unaligned(base.add(offset).cast::<T>()) })
}

unsafe fn parse_callbacks(pointer: *const OpCallbacks) -> FfiResult<Callbacks> {
    if pointer.is_null() {
        return Ok(Callbacks::default());
    }
    let base = pointer.cast::<u8>();
    let size = unsafe { ptr::read_unaligned(pointer.cast::<usize>()) };
    if size < size_of::<usize>() {
        return Err(FfiError::invalid("callbacks size is below the minimum"));
    }
    if size > size_of::<OpCallbacks>() {
        return Err(FfiError::invalid(
            "callbacks size exceeds the known version",
        ));
    }
    Ok(Callbacks {
        user_data: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, user_data)).unwrap_or(ptr::null_mut())
        },
        needs_redraw: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, needs_redraw)).unwrap_or(None)
        },
        runtime_error: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, runtime_error)).unwrap_or(None)
        },
        input_focus_changed: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, input_focus_changed)).unwrap_or(None)
        },
        remote_image_request: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, remote_image_request)).unwrap_or(None)
        },
        #[cfg(feature = "editor")]
        credential_load: unsafe {
            read_covered(base, size, offset_of!(OpCallbacks, credential_load)).unwrap_or(None)
        },
        #[cfg(feature = "editor")]
        credential_store_if_absent: unsafe {
            read_covered(
                base,
                size,
                offset_of!(OpCallbacks, credential_store_if_absent),
            )
            .unwrap_or(None)
        },
    })
}

/// Read the v4 `documents_root` tail: the user-visible directory saved
/// documents land in. Absent (a pre-v4 shell, or one with no such
/// directory) leaves the private `<storage_root>/documents` fallback in
/// place, so Android / HarmonyOS keep today's behaviour untouched.
///
/// Unlike `storage_root` this configures no global — the path travels on
/// [`CreateOptions`] into the session, because `storage_root` legitimately
/// stays the private config root that settings persistence keys on.
///
/// # Safety
///
/// `base` must be readable for `size` bytes.
unsafe fn parse_documents_root(base: *const u8, size: usize) -> FfiResult<Option<String>> {
    let root_ptr = unsafe {
        read_covered::<*const u8>(base, size, offset_of!(OpCreateDesc, documents_root_ptr))
    };
    let root_len =
        unsafe { read_covered::<usize>(base, size, offset_of!(OpCreateDesc, documents_root_len)) }
            .unwrap_or(0);
    if root_len == 0 {
        return Ok(None);
    }
    let root = unsafe {
        read_utf8(
            root_ptr.unwrap_or(ptr::null()),
            root_len,
            STRING_CAP,
            "documents root",
        )?
    };
    if !std::path::Path::new(&root).is_absolute() {
        return Err(FfiError::invalid("documents root must be an absolute path"));
    }
    Ok(Some(root))
}

pub(crate) unsafe fn parse_create(pointer: *const OpCreateDesc) -> FfiResult<CreateOptions> {
    if pointer.is_null() {
        return Err(FfiError::invalid("create descriptor is null"));
    }
    let base = pointer.cast::<u8>();
    let size = unsafe { ptr::read_unaligned(pointer.cast::<usize>()) };
    if size < size_of::<usize>() {
        return Err(FfiError::invalid(
            "create descriptor size is below the minimum",
        ));
    }
    if size > size_of::<OpCreateDesc>() {
        return Err(FfiError::invalid(
            "create descriptor size exceeds the known version",
        ));
    }
    let doc_ptr =
        unsafe { read_covered::<*const u8>(base, size, offset_of!(OpCreateDesc, doc_ptr)) };
    let doc_len = unsafe { read_covered::<usize>(base, size, offset_of!(OpCreateDesc, doc_len)) };
    let width = unsafe { read_covered::<f32>(base, size, offset_of!(OpCreateDesc, width)) };
    let height = unsafe { read_covered::<f32>(base, size, offset_of!(OpCreateDesc, height)) };
    let dpr = unsafe { read_covered::<f32>(base, size, offset_of!(OpCreateDesc, dpr)) };
    let callbacks = unsafe {
        read_covered::<*const OpCallbacks>(base, size, offset_of!(OpCreateDesc, callbacks))
            .unwrap_or(ptr::null())
    };
    let asset_base_ptr =
        unsafe { read_covered::<*const u8>(base, size, offset_of!(OpCreateDesc, asset_base_ptr)) };
    let asset_base_len =
        unsafe { read_covered::<usize>(base, size, offset_of!(OpCreateDesc, asset_base_len)) };
    let mode = unsafe { read_covered::<i32>(base, size, offset_of!(OpCreateDesc, mode)) };
    #[cfg(feature = "editor")]
    let editor_mode = match mode {
        None => false, // pre-mode shells stay in viewer mode
        Some(0) => false,
        Some(1) => true,
        Some(other) => {
            return Err(FfiError::invalid(format!("unknown mode {other}")));
        }
    };
    // Without the editor feature the mode is validated and ignored.
    #[cfg(not(feature = "editor"))]
    if let Some(other) = mode {
        if other != 0 && other != 1 {
            return Err(FfiError::invalid(format!("unknown mode {other}")));
        }
    }
    if doc_len.unwrap_or(0) > DOCUMENT_CAP {
        return Err(FfiError::invalid(format!(
            "document length exceeds {DOCUMENT_CAP} bytes"
        )));
    }
    let empty_editor_document =
        cfg!(feature = "editor") && mode == Some(1) && doc_len.unwrap_or(0) == 0;
    if doc_len.unwrap_or(0) == 0 && !empty_editor_document {
        return Err(FfiError::invalid("document is empty"));
    }
    let document = if empty_editor_document {
        String::new()
    } else {
        unsafe {
            read_utf8(
                doc_ptr.unwrap_or(ptr::null()),
                doc_len.unwrap_or(0),
                DOCUMENT_CAP,
                "document",
            )?
        }
    };
    let width = width.unwrap_or(0.0);
    let height = height.unwrap_or(0.0);
    let dpr = dpr.unwrap_or(1.0);
    if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
        return Err(FfiError::invalid(
            "viewport size must be finite and positive",
        ));
    }
    if !(dpr.is_finite() && dpr > 0.0) {
        return Err(FfiError::invalid("dpr must be finite and positive"));
    }
    let asset_base = if asset_base_len.unwrap_or(0) == 0 {
        None
    } else {
        Some(unsafe {
            read_utf8(
                asset_base_ptr.unwrap_or(ptr::null()),
                asset_base_len.unwrap_or(0),
                STRING_CAP,
                "asset base",
            )?
        })
    };
    let storage_root_ptr = unsafe {
        read_covered::<*const u8>(base, size, offset_of!(OpCreateDesc, storage_root_ptr))
    };
    let storage_root_len =
        unsafe { read_covered::<usize>(base, size, offset_of!(OpCreateDesc, storage_root_len)) }
            .unwrap_or(0);
    let storage_root = if storage_root_len == 0 {
        None
    } else {
        Some(unsafe {
            read_utf8(
                storage_root_ptr.unwrap_or(ptr::null()),
                storage_root_len,
                STRING_CAP,
                "storage root",
            )?
        })
    };
    #[cfg(all(
        feature = "editor",
        any(target_os = "ios", target_os = "android", target_env = "ohos")
    ))]
    if editor_mode && storage_root.is_none() {
        return Err(FfiError::invalid(
            "mobile editor mode requires a private storage root",
        ));
    }
    if let Some(storage_root) = storage_root {
        let path = std::path::PathBuf::from(storage_root);
        if !path.is_absolute() {
            return Err(FfiError::invalid("storage root must be an absolute path"));
        }
        op_config_store::configure_user_root(path).map_err(|error| {
            FfiError::new(
                OpStatus::NotReady,
                format!("could not configure the private storage root: {error}"),
            )
        })?;
    }
    let documents_root = unsafe { parse_documents_root(base, size)? };
    // Viewer-only builds have no document-save surface; the value is still
    // parsed (and validated) so the ABI contract is identical everywhere.
    #[cfg(not(feature = "editor"))]
    let _ = documents_root;
    Ok(CreateOptions {
        document,
        width,
        height,
        dpr,
        callbacks: unsafe { parse_callbacks(callbacks)? },
        asset_base,
        #[cfg(feature = "editor")]
        editor_mode,
        #[cfg(feature = "editor")]
        documents_root,
    })
}

/// Read the borrowed surface handle from a surface descriptor.
///
/// # Safety
///
/// `desc` must be readable per its declared `size`.
pub(crate) unsafe fn surface_handle(desc: *const OpSurfaceDesc) -> FfiResult<*mut c_void> {
    if desc.is_null() {
        return Err(FfiError::invalid("surface descriptor is null"));
    }
    let base = desc.cast::<u8>();
    let size = unsafe { ptr::read_unaligned(desc.cast::<usize>()) };
    if size < size_of::<usize>() {
        return Err(FfiError::invalid(
            "surface descriptor size is below the minimum",
        ));
    }
    if size > size_of::<OpSurfaceDesc>() {
        return Err(FfiError::invalid(
            "surface descriptor size exceeds the known version",
        ));
    }
    let handle =
        unsafe { read_covered::<*mut c_void>(base, size, offset_of!(OpSurfaceDesc, handle)) }
            .unwrap_or(ptr::null_mut());
    if handle.is_null() {
        return Err(FfiError::invalid("surface handle is null"));
    }
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_create_desc(mode: i32) -> OpCreateDesc {
        OpCreateDesc {
            size: size_of::<OpCreateDesc>(),
            doc_ptr: ptr::null(),
            doc_len: 0,
            width: 390.0,
            height: 844.0,
            dpr: 1.0,
            callbacks: ptr::null(),
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode,
            storage_root_ptr: ptr::null(),
            storage_root_len: 0,
            documents_root_ptr: ptr::null(),
            documents_root_len: 0,
        }
    }

    #[test]
    fn viewer_mode_rejects_an_empty_document() {
        let desc = empty_create_desc(0);
        let error = unsafe { parse_create(&desc) }
            .err()
            .expect("viewer must reject empty document bytes");
        assert_eq!(error.status, OpStatus::InvalidArg);
        assert_eq!(error.message, "document is empty");
    }

    #[cfg(feature = "editor")]
    #[test]
    fn editor_mode_accepts_an_empty_document_as_the_starter_sentinel() {
        let desc = empty_create_desc(1);
        let options = match unsafe { parse_create(&desc) } {
            Ok(options) => options,
            Err(error) => panic!("editor blank sentinel rejected: {}", error.message),
        };
        assert!(options.editor_mode);
        assert!(options.document.is_empty());
    }

    /// Tail growth: a shell built against v3 stops its `size` short of the
    /// documents-root pair and must still create an engine, keeping the
    /// private `<storage_root>/documents` fallback.
    #[cfg(feature = "editor")]
    #[test]
    fn a_pre_v4_descriptor_parses_without_a_documents_root() {
        let mut desc = empty_create_desc(1);
        desc.size = offset_of!(OpCreateDesc, documents_root_ptr);
        // Garbage in the tail the caller never wrote must be ignored.
        desc.documents_root_ptr = b"/not/read".as_ptr();
        desc.documents_root_len = 9;
        let options = unsafe { parse_create(&desc) }.expect("pre-v4 descriptor");
        assert_eq!(options.documents_root, None);
    }

    #[cfg(feature = "editor")]
    #[test]
    fn a_documents_root_must_be_absolute() {
        let mut desc = empty_create_desc(1);
        let relative = b"documents";
        desc.documents_root_ptr = relative.as_ptr();
        desc.documents_root_len = relative.len();
        let error = unsafe { parse_create(&desc) }
            .err()
            .expect("relative documents root");
        assert_eq!(error.status, OpStatus::InvalidArg);
        assert_eq!(error.message, "documents root must be an absolute path");

        let absolute = b"/var/mobile/Documents";
        desc.documents_root_ptr = absolute.as_ptr();
        desc.documents_root_len = absolute.len();
        let options = unsafe { parse_create(&desc) }.expect("absolute documents root");
        assert_eq!(
            options.documents_root.as_deref(),
            Some("/var/mobile/Documents")
        );
    }
}
