//! The engine session — document state, viewport, gesture tracker, and
//! the GPU/raster surface — plus the ABI call wrapper enforcing the
//! owner-thread + panic-safety contract.

use crate::desc::{Callbacks, CreateOptions};
use crate::error::{FfiError, FfiResult};
use crate::input::GestureTracker;
use crate::render::{fit_viewport, paint_frame};
use crate::viewport::OpInsets;
use crate::OpStatus;
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::Point2D;
use op_host_native::NativeBackend;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// GPU surface slot. The shell's borrowed handle (CAMetalLayer /
/// ANativeWindow) is released only after suspend/destroy returns, so the
/// surface lives exactly as long as the borrow.
pub(crate) enum SurfaceSlot {
    None,
    #[cfg(all(feature = "metal", any(target_os = "macos", target_os = "ios")))]
    Metal(jian_skia::surface::metal::MetalSurface),
    #[cfg(all(feature = "gl", target_os = "android"))]
    Egl(jian_skia::surface::egl_android::EglSurface),
}

pub(crate) struct Session {
    /// Canonical editor state the scene was derived from. Held for the
    /// session's lifetime so future interactive features (page switching,
    /// text editing) rebuild the scene from the live model.
    #[allow(dead_code)]
    pub state: EditorState,
    pub scene: LayoutScene,
    pub backend: NativeBackend,
    pub logical: (f32, f32),
    pub dpr: f32,
    pub insets: OpInsets,
    pub keyboard: f32,
    pub surface: SurfaceSlot,
    /// Doc-space point at the viewport's top-left (logical points).
    pub viewport_origin: Point2D,
    pub zoom: f32,
    pub selected: Option<String>,
    pub gesture: GestureTracker,
    pub callbacks: Callbacks,
    pub suspended: bool,
    /// False until the user pans/zooms — resize then re-fits the page.
    pub user_interacted: bool,
    /// Engine clock (ms, monotonic) — fed by `op_frame` / `op_pointer`;
    /// drives text-edit history coalescing and the caret blink.
    pub now_ms: u64,
    /// Full-editor host (the complete desktop chrome) — `Some` only in
    /// editor mode (`OpCreateDesc.mode == 1`).
    #[cfg(feature = "editor")]
    pub editor: Option<op_host_native::WidgetHostNative>,
    /// Shell-provided media root (APK-extracted assets / bundle
    /// resources). Reserved for future local-media resolution.
    #[allow(dead_code)]
    pub asset_base: Option<String>,
}

impl Session {
    pub(crate) fn new(options: CreateOptions) -> FfiResult<Session> {
        let src = options.document;
        // Canonical load: strict schema with legacy-major compatibility,
        // exactly the path `op-host-services::doc_io` uses for the
        // desktop editor.
        let meta = op_pen_loader::extract_editor_meta_with_report(&src).map(|e| e.meta);
        let loaded =
            op_pen_loader::payload::load_canonical_with_compatibility(&src).map_err(|e| {
                FfiError::new(
                    OpStatus::BadDocument,
                    format!("document rejected by the canonical schema: {e}"),
                )
            })?;
        let mut state = EditorState::from_document(loaded.loaded.value);
        op_pen_loader::apply_editor_meta_or_legacy_fallback(&mut state, meta);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);
        if scene.active_page().is_none() {
            return Err(FfiError::new(
                OpStatus::LayoutError,
                "document has no renderable page",
            ));
        }
        let backend = NativeBackend::with_dpi(options.dpr);
        #[cfg(feature = "editor")]
        let editor = if options.editor_mode {
            // The host seeds an empty starter doc; replace it with the
            // loaded one (resets the document epoch + scene cache).
            let mut host = op_host_native::WidgetHostNative::new();
            if !host.replace_editor_state(state.clone()) {
                // No collab session gates user edits; fall back to the
                // document-level replace just in case.
                host.editor_state_mut().replace_document(state.doc.clone());
            }
            // Responsive layout: the size class is computed from the
            // usable rectangle (insets are zero at create time) and
            // recomputed on every resize and safe-area update. Keyboard
            // occlusion changes visible height without changing class. Overlay classes
            // start with the left rail closed so the canvas spans the
            // full width; rail classes keep it open.
            let ui = &mut host.editor_state_mut().editor_ui;
            ui.touch = true;
            ui.size_class = op_editor_core::size_class::size_class(options.width, options.height);
            ui.sidebar_open = ui.size_class.is_rail_layout();
            Some(host)
        } else {
            None
        };
        let mut session = Session {
            state,
            scene,
            backend,
            logical: (options.width, options.height),
            dpr: options.dpr,
            insets: OpInsets::default(),
            keyboard: 0.0,
            surface: SurfaceSlot::None,
            viewport_origin: Point2D::ZERO,
            zoom: 1.0,
            selected: None,
            gesture: GestureTracker::default(),
            callbacks: options.callbacks,
            suspended: false,
            user_interacted: false,
            now_ms: 0,
            asset_base: options.asset_base,
            #[cfg(feature = "editor")]
            editor,
        };
        session.fit_content_to_viewports();
        Ok(session)
    }

    /// The editor host (editor mode only).
    #[cfg(feature = "editor")]
    pub(crate) fn editor(&self) -> Option<&op_host_native::WidgetHostNative> {
        self.editor.as_ref()
    }

    /// The editor host, or a `WrongThread`-style error when not in editor
    /// mode.
    #[cfg(feature = "editor")]
    pub(crate) fn editor_mut(&mut self) -> FfiResult<&mut op_host_native::WidgetHostNative> {
        self.editor
            .as_mut()
            .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))
    }

    /// Replace the whole document (editor mode: through the host, which
    /// resets its document epoch + scene cache; viewer mode: direct).
    /// Reserved for future file-open flows.
    #[allow(dead_code)]
    pub(crate) fn replace_document(&mut self, doc: jian_ops_schema::PenDocument) {
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            let _ = host.replace_editor_state(op_editor_core::EditorState::from_document(doc));
            return;
        }
        self.state.replace_document(doc);
        self.rebuild_scene();
    }

    /// Fire the needs-redraw upcall (synchronous; shells defer).
    pub(crate) fn request_redraw(&self) {
        self.request_wake(None);
    }

    /// Fire the needs-redraw upcall, optionally scheduling a timed wake
    /// (used by the caret blink: the shell pauses its display pump and
    /// resumes at `next_wake_ms`).
    pub(crate) fn request_wake(&self, next_wake_ms: Option<u64>) {
        if self.suspended {
            return;
        }
        if let Some(callback) = self.callbacks.needs_redraw {
            // SAFETY: the shell registered a live user_data pointer that
            // outlives the engine; the callback must not re-enter this
            // engine (the `in_call` guard reports WrongThread then).
            unsafe {
                callback(
                    self.callbacks.user_data,
                    next_wake_ms.is_some(),
                    next_wake_ms.unwrap_or(0),
                )
            };
        }
    }

    /// Rebuild the layout scene from the live editor state (text edits,
    /// page switches, font registration). The desktop keeps its scene
    /// cache in lockstep with document generations; the player rebuilds
    /// on demand.
    pub(crate) fn rebuild_scene(&mut self) {
        self.scene = op_pen_loader::editor_state_to_active_page_layout_scene(&self.state);
    }

    /// Fire the inline text-edit focus transition upcall.
    pub(crate) fn focus_changed(&self, focused: bool, input_kind: i32, return_key_hint: i32) {
        if let Some(callback) = self.callbacks.input_focus_changed {
            // SAFETY: the shell registered a live user_data pointer that
            // outlives the engine.
            unsafe {
                callback(
                    self.callbacks.user_data,
                    focused,
                    input_kind,
                    return_key_hint,
                )
            };
        }
    }

    /// The active text-edit input state, if any.
    pub(crate) fn editing_input(&self) -> Option<&jian_core::text_input::TextInputState> {
        (self.state.ui.text_editing.is_some()).then_some(&self.state.ui.text_edit_input)
    }

    /// Caret rect in logical viewport points (see `crate::text::caret_rect`).
    pub(crate) fn caret_rect(&mut self) -> Option<(f32, f32, f32, f32)> {
        crate::text::caret_rect(self)
    }

    /// Editor mode: honor the host's next animation deadline (caret blink,
    /// chat streaming, reveal animations).
    #[cfg(feature = "editor")]
    pub(crate) fn schedule_editor_animation(&self) {
        if let Some(host) = self.editor.as_ref() {
            if let Some(deadline) = host.next_animation_deadline_ms() {
                self.request_wake(Some(deadline));
            }
        }
    }

    /// When the caret is blinking, schedule the next flip wake.
    #[cfg(not(feature = "editor"))]
    pub(crate) fn schedule_caret_blink(&self) {
        let Some(input) = self.editing_input() else {
            return;
        };
        if input.selection().is_collapsed() {
            let flip = input.next_blink_flip_ms(self.now_ms);
            if flip > self.now_ms {
                self.request_wake(Some(flip));
            }
        }
    }

    /// Emit a runtime diagnostic through the callback table.
    pub(crate) fn emit_runtime_error(&self, kind: i32, message: &str, source: &str) {
        if let Some(callback) = self.callbacks.runtime_error {
            let payload = crate::desc::OpRuntimeError {
                kind,
                message_ptr: message.as_ptr(),
                message_len: message.len(),
                source_ptr: source.as_ptr(),
                source_len: source.len(),
            };
            // SAFETY: the payload pointers are valid for the duration of
            // the call and the shell copies before returning.
            unsafe { callback(self.callbacks.user_data, &payload) };
        }
    }

    /// Attach a borrowed platform surface; only valid once (suspend
    /// before re-attaching).
    pub(crate) fn attach_surface(&mut self, handle: *mut c_void) -> FfiResult<()> {
        if !matches!(self.surface, SurfaceSlot::None) {
            return Err(FfiError::new(
                OpStatus::Busy,
                "a surface is already attached (suspend first)",
            ));
        }
        let surface = build_surface(handle)?;
        self.surface = surface;
        self.suspended = false;
        Ok(())
    }

    pub(crate) fn suspend(&mut self) {
        self.surface = SurfaceSlot::None;
        self.suspended = true;
        self.gesture.reset();
    }

    /// Resume with an optional fresh surface handle.
    pub(crate) fn resume(&mut self, handle: Option<*mut c_void>) -> FfiResult<()> {
        if let Some(handle) = handle {
            // Replace any stale surface synchronously (the shell has
            // already released its previous borrow).
            self.surface = build_surface(handle)?;
        }
        self.suspended = false;
        Ok(())
    }

    pub(crate) fn resize(&mut self, width: f32, height: f32, dpr: f32) -> FfiResult<()> {
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            return Err(FfiError::invalid(
                "viewport size must be finite and positive",
            ));
        }
        if !(dpr.is_finite() && dpr > 0.0) {
            return Err(FfiError::invalid("dpr must be finite and positive"));
        }
        self.logical = (width, height);
        self.dpr = dpr;
        self.backend.set_dpi(dpr);
        // The host derives its canvas region from the responsive layout,
        // so update the size class before fitting the editor viewport.
        self.recompute_responsive_layout();
        if !self.user_interacted {
            self.fit_content_to_viewports();
        }
        Ok(())
    }

    /// The usable editor viewport (logical points): the logical size
    /// minus safe-area insets and keyboard occlusion. Paint and hit-test
    /// both derive from this so sheets and the dock never sit under the
    /// status bar / IME.
    #[cfg(feature = "editor")]
    pub(crate) fn editor_viewport(&self) -> (f32, f32) {
        let w = (self.logical.0 - self.insets.left - self.insets.right).max(1.0);
        let h = (self.logical.1 - self.insets.top - self.insets.bottom.max(self.keyboard)).max(1.0);
        (w, h)
    }

    /// Map a shell touch point (full-viewport logical, top-left origin)
    /// into the usable editor rect.
    #[cfg(feature = "editor")]
    pub(crate) fn editor_point(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.insets.left, y - self.insets.top)
    }

    /// Fit both rendering modes after document load or an automatic
    /// viewport change. Viewer mode owns the lightweight session camera;
    /// editor mode owns a separate camera inside `WidgetHostNative` and
    /// must be fitted explicitly against the safe-area-adjusted viewport.
    pub(crate) fn fit_content_to_viewports(&mut self) {
        fit_viewport(self);
        #[cfg(feature = "editor")]
        {
            // Compute before borrowing `editor` mutably: `editor_viewport`
            // reads other session fields and the two borrows would overlap.
            let (viewport_w, viewport_h) = self.editor_viewport();
            if let Some(host) = self.editor.as_mut() {
                host.fit_content_to_viewport(viewport_w, viewport_h);
            }
        }
    }

    /// Recompute the editor's responsive size class from the stable window
    /// content rectangle (logical viewport minus safe areas). Keyboard
    /// occlusion changes the visible panel height, never the primary chrome
    /// class; otherwise opening an iPad keyboard would turn a tablet layout
    /// into a phone layout mid-edit.
    pub(crate) fn recompute_responsive_layout(&mut self) {
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            let usable_w = (self.logical.0 - self.insets.left - self.insets.right).max(1.0);
            let usable_h = (self.logical.1 - self.insets.top - self.insets.bottom).max(1.0);
            let ui = &mut host.editor_state_mut().editor_ui;
            let next = op_editor_core::size_class::size_class(usable_w, usable_h);
            let previous = ui.size_class;
            if previous == next {
                return;
            }
            ui.size_class = next;
            ui.mobile_sheet = None;
            if next.is_expanded() {
                ui.sidebar_open = true;
            } else if previous.is_expanded() {
                ui.sidebar_open = false;
            }
        }
    }

    /// Pump and present one GPU frame. `Ok(())` means painted (or the
    /// drawable was unavailable and a redraw is queued).
    pub(crate) fn frame_gpu(&mut self, now_ms: u64) -> FfiResult<()> {
        self.now_ms = now_ms;
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            host.set_now_ms(now_ms);
        }
        #[cfg(feature = "editor")]
        crate::editor_template::drain_pending_scene_template(self)?;
        let slot = std::mem::replace(&mut self.surface, SurfaceSlot::None);
        let (restored, outcome) = match slot {
            SurfaceSlot::None => (
                None,
                Err(FfiError::new(
                    OpStatus::NotReady,
                    "no GPU surface attached (call op_attach_surface)",
                )),
            ),
            #[cfg(all(feature = "metal", any(target_os = "macos", target_os = "ios")))]
            SurfaceSlot::Metal(mut metal) => {
                let result = metal.draw_frame(|skia| paint_frame(self, skia));
                (
                    Some(SurfaceSlot::Metal(metal)),
                    result.map_err(|e| FfiError::new(OpStatus::GpuError, e)),
                )
            }
            #[cfg(all(feature = "gl", target_os = "android"))]
            SurfaceSlot::Egl(mut egl) => {
                let result = egl.draw_frame(|skia| paint_frame(self, skia));
                if let Err(e) = &result {
                    eprintln!("op-engine-ffi: EGL draw_frame error: {e}");
                }
                (
                    Some(SurfaceSlot::Egl(egl)),
                    result.map_err(|e| FfiError::new(OpStatus::GpuError, e.to_string())),
                )
            }
        };
        if let Some(surface) = restored {
            self.surface = surface;
        }
        match outcome {
            Ok(true) => {
                crate::media::drain_remote_image_requests(self);
                #[cfg(feature = "editor")]
                self.schedule_editor_animation();
                #[cfg(not(feature = "editor"))]
                self.schedule_caret_blink();
                Ok(())
            }
            // Drawable unavailable (Metal `nextDrawable` nil / EGL
            // BadAccess): leave the dirty bit set and ask the shell to
            // retry promptly.
            Ok(false) => {
                self.request_redraw();
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Pump and copy one CPU frame into caller-owned RGBA8888 storage.
    ///
    /// # Safety
    ///
    /// `buffer` must cover `buffer_len` writable bytes and
    /// `stride >= physical_width * 4`.
    pub(crate) fn frame_cpu(
        &mut self,
        now_ms: u64,
        buffer: *mut u8,
        buffer_len: usize,
        stride: usize,
    ) -> FfiResult<()> {
        self.now_ms = now_ms;
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            host.set_now_ms(now_ms);
        }
        #[cfg(feature = "editor")]
        crate::editor_template::drain_pending_scene_template(self)?;
        let width = ((self.logical.0 * self.dpr).round() as i32).max(1);
        let height = ((self.logical.1 * self.dpr).round() as i32).max(1);
        let row_bytes = width as usize * 4;
        if stride < row_bytes {
            return Err(FfiError::invalid(
                "frame stride is smaller than the physical row",
            ));
        }
        let required = height as usize * stride;
        if buffer_len < required {
            return Err(FfiError::invalid(format!(
                "frame buffer covers {buffer_len} bytes but {required} are required"
            )));
        }
        let mut surface = jian_skia::SkiaSurface::new_raster(width, height);
        paint_frame(self, &mut surface);
        crate::media::drain_remote_image_requests(self);
        let mut rows = vec![0u8; row_bytes * height as usize];
        if !surface.read_rgba8(&mut rows) {
            return Err(FfiError::new(
                OpStatus::GpuError,
                "CPU frame readback failed",
            ));
        }
        // SAFETY: the caller's buffer covers `height * stride` bytes; each
        // row copy is exactly `row_bytes`.
        unsafe {
            for row in 0..height as usize {
                let src = &rows[row * row_bytes..(row + 1) * row_bytes];
                std::ptr::copy_nonoverlapping(src.as_ptr(), buffer.add(row * stride), row_bytes);
            }
        }
        Ok(())
    }
}

/// Build the platform GPU surface from a borrowed handle.
///
/// # Safety
///
/// `handle` must point to a live platform layer/window the caller keeps
/// alive until suspend/destroy.
fn build_surface(handle: *mut c_void) -> FfiResult<SurfaceSlot> {
    #[cfg(all(feature = "metal", any(target_os = "macos", target_os = "ios")))]
    {
        // SAFETY: the handle contract is the caller's (see above); the
        // surface borrows the layer and never retains it.
        let metal = unsafe { jian_skia::surface::metal::MetalSurface::from_ca_metal_layer(handle) }
            .map_err(|e| FfiError::new(OpStatus::GpuError, e))?;
        return Ok(SurfaceSlot::Metal(metal));
    }
    #[cfg(all(feature = "gl", target_os = "android"))]
    {
        // SAFETY: the handle contract is the caller's (see above); the
        // surface borrows the window and never retains it.
        let egl =
            unsafe { jian_skia::surface::egl_android::EglSurface::from_native_window(handle) }
                .map_err(|e| FfiError::new(OpStatus::GpuError, e))?;
        return Ok(SurfaceSlot::Egl(egl));
    }
    #[cfg(not(any(
        all(feature = "metal", any(target_os = "macos", target_os = "ios")),
        all(feature = "gl", target_os = "android")
    )))]
    {
        let _ = handle;
        Err(FfiError::new(
            OpStatus::NotReady,
            "no GPU surface backend compiled for this target/features",
        ))
    }
}

/// The engine handle handed across the ABI. Owned by exactly one thread:
/// every call checks `owner` and refuses to re-enter while a call is in
/// flight, and any panic crossing the boundary poisons the engine.
pub struct OpEngine {
    owner: std::thread::ThreadId,
    in_call: Cell<bool>,
    poisoned: Cell<bool>,
    last_error: RefCell<String>,
    session: UnsafeCell<Session>,
}

impl OpEngine {
    /// Wrap a session as an ABI handle owned by the calling thread.
    pub(crate) fn new(session: Session) -> OpEngine {
        OpEngine {
            owner: std::thread::current().id(),
            in_call: Cell::new(false),
            poisoned: Cell::new(false),
            last_error: RefCell::new(String::new()),
            session: UnsafeCell::new(session),
        }
    }

    /// Clone of the last call error (for `op_last_error`).
    pub(crate) fn last_error(&self) -> String {
        self.last_error.borrow().clone()
    }

    #[cfg(all(test, feature = "editor"))]
    pub(crate) fn session_mut_for_test(&mut self) -> &mut Session {
        // Unit tests own the engine on this thread and never overlap an ABI
        // call with this inspection seam.
        unsafe { &mut *self.session.get() }
    }
}

/// Dispatch one engine call with the thread + re-entrancy + panic
/// contract. The session's per-call diagnostics are emitted afterwards.
///
/// # Safety
///
/// `pointer` must be a live `OpEngine` from `op_create` on the calling
/// thread.
pub(crate) unsafe fn call_session(
    pointer: *mut OpEngine,
    call: impl FnOnce(&mut Session) -> FfiResult<()>,
) -> OpStatus {
    if pointer.is_null() {
        return OpStatus::InvalidArg;
    }
    let engine = unsafe { &*pointer };
    if engine.owner != std::thread::current().id() || engine.in_call.get() {
        return OpStatus::WrongThread;
    }
    if engine.poisoned.get() {
        return OpStatus::Poisoned;
    }
    engine.in_call.set(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let session = unsafe { &mut *engine.session.get() };
        let result = call(session);
        if let Err(error) = &result {
            session.emit_runtime_error(2, &error.message, "op-engine-ffi");
        }
        result
    }));
    engine.in_call.set(false);
    match outcome {
        Ok(Ok(())) => OpStatus::Ok,
        Ok(Err(error)) => {
            *engine.last_error.borrow_mut() = error.message;
            error.status
        }
        Err(_) => {
            engine.poisoned.set(true);
            *engine.last_error.borrow_mut() =
                "panic crossed the OpenPencil ABI boundary".to_owned();
            OpStatus::Poisoned
        }
    }
}

pub(crate) unsafe fn destroy_engine(pointer: *mut OpEngine) -> OpStatus {
    if pointer.is_null() {
        return OpStatus::InvalidArg;
    }
    let engine = unsafe { &*pointer };
    if engine.owner != std::thread::current().id() || engine.in_call.get() {
        return OpStatus::WrongThread;
    }
    engine.in_call.set(true);
    let outcome = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(pointer));
    }));
    match outcome {
        Ok(()) => OpStatus::Ok,
        Err(_) => OpStatus::Poisoned,
    }
}

#[cfg(all(test, feature = "editor"))]
mod editor_fit_tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn editor_session(width: f32, height: f32) -> Session {
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width,
            height,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("editor session")
    }

    fn assert_host_is_fitted(session: &Session) {
        let (viewport_w, viewport_h) = session.editor_viewport();
        let host = session.editor().expect("editor host");
        let (_, _, canvas_w, canvas_h) = op_editor_ui::widgets::host_canvas_geometry::canvas_region(
            host.editor_state(),
            viewport_w,
            viewport_h,
        );
        let view = host.editor_state().viewport;
        let expected_zoom = ((canvas_w - 128.0).max(1.0) / 1080.0)
            .min((canvas_h - 128.0).max(1.0) / 1440.0)
            .clamp(0.1, 1.0);
        assert!((view.zoom - expected_zoom).abs() < 0.001);
        assert!((view.pan_x + 540.0 * view.zoom - canvas_w / 2.0).abs() < 0.01);
        assert!((view.pan_y + 720.0 * view.zoom - canvas_h / 2.0).abs() < 0.01);
    }

    #[test]
    fn editor_host_fits_on_create_and_uninteracted_resize() {
        let mut session = editor_session(800.0, 600.0);
        assert_host_is_fitted(&session);
        let initial_zoom = session.editor().unwrap().editor_state().viewport.zoom;

        session.resize(1_200.0, 900.0, 1.0).unwrap();
        assert_host_is_fitted(&session);
        assert!(session.editor().unwrap().editor_state().viewport.zoom > initial_zoom);
    }

    #[test]
    fn safe_area_refits_but_user_camera_survives_resize() {
        let mut engine = OpEngine::new(editor_session(800.0, 600.0));
        let engine_ptr = &mut engine as *mut OpEngine;

        assert_eq!(
            unsafe { crate::op_set_safe_area(engine_ptr, 24.0, 10.0, 220.0, 10.0) },
            OpStatus::Ok
        );
        let session = unsafe { &*engine.session.get() };
        assert_host_is_fitted(session);

        assert_eq!(
            unsafe { crate::op_editor_pan(engine_ptr, 400.0, 180.0, 32.0, 12.0) },
            OpStatus::Ok
        );
        let session = unsafe { &*engine.session.get() };
        assert!(session.user_interacted);
        let panned = session.editor().unwrap().editor_state().viewport;

        assert_eq!(
            unsafe { crate::op_resize(engine_ptr, 1_200.0, 900.0, 1.0) },
            OpStatus::Ok
        );
        let session = unsafe { &*engine.session.get() };
        assert_eq!(session.editor().unwrap().editor_state().viewport, panned);
    }
}
