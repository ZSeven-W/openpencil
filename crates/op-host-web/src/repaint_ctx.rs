//! Backend-agnostic seam between the web integration modules (chat streaming,
//! live-sync, codegen, file IO, icon search, fonts) and whichever rendering
//! host owns the shell.
//!
//! Both mount paths wrap their state in `Rc<RefCell<_>>` and expose the same
//! two things the integration modules need: the shared `WidgetHost` (where all
//! `EditorState` lives) and a `repaint()` that re-paints through that path's
//! backend. The skia raster mount (`Inner`, `lib.rs`) and the CanvasKit GPU
//! mount (`CkInner`, `canvaskit.rs`) each implement this trait, so the modules
//! can be written once against `Rc<RefCell<impl RepaintContext>>` instead of a
//! concrete `Inner`.

use crate::widget_host::WidgetHost;
use wasm_bindgen::JsValue;

/// The minimal surface the web integration modules drive: read/write the
/// shared widget host, then schedule a repaint.
pub(crate) trait RepaintContext {
    fn host(&self) -> &WidgetHost;
    fn host_mut(&mut self) -> &mut WidgetHost;
    /// Logical viewport size in CSS pixels. File-open/import flows fit the
    /// loaded content to this size after replacing the document.
    fn viewport_size(&self) -> (f32, f32);
    /// Register an OS font face (from Local Font Access bytes) with the
    /// backend so canvas text can shape against it. Returns `true` when the
    /// face was registered. Backends that cannot accept runtime font bytes may
    /// return `false`.
    fn register_system_font(&mut self, family: &str, bytes: &[u8]) -> bool;
    /// Re-paint through the owning backend. Returns the present error if the
    /// backend's present step failed (the CanvasKit path is infallible and
    /// always returns `Ok`).
    fn repaint(&mut self) -> Result<(), JsValue>;
}
