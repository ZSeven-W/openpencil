//! Backend-agnostic seam between the web integration modules (chat streaming,
//! live-sync, codegen, file IO, icon search, fonts) and whichever rendering
//! host owns the shell.
//!
//! The production CanvasKit mount wraps its state in `Rc<RefCell<_>>` and
//! exposes the two things the integration modules need: the shared `WidgetHost`
//! (where all `EditorState` lives) and a `repaint()` that re-paints through the
//! CanvasKit backend. Keeping the trait lets those modules stay independent of
//! the concrete `CkInner` type.

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
    /// Register a user-imported font face so canvas text can shape against it
    /// by family name. Returns `true` when the face was registered; backends
    /// that cannot accept runtime font bytes may return `false`.
    fn register_imported_font(&mut self, family: &str, bytes: &[u8]) -> bool;
    /// Register a user-imported font whose family is unknown (a fresh file
    /// import). The backend parses the bytes, extracts the family display name,
    /// and returns it (`None` on parse failure / no family name). Used only by
    /// the import file-picker path; `register_imported_font` re-registers a
    /// known family from IndexedDB.
    fn register_imported_font_from_bytes(&mut self, bytes: &[u8]) -> Option<String>;
    /// Display names of every registered imported family — the web snapshot
    /// source (Rust doesn't own the registry on web).
    fn imported_family_list(&self) -> Vec<String>;
    /// Drop a previously imported font face by family name (no-op if absent).
    fn remove_imported_font(&mut self, family: &str);
    /// Re-paint through the owning backend. Returns the present error if the
    /// backend's present step failed (the CanvasKit path is infallible and
    /// always returns `Ok`).
    fn repaint(&mut self) -> Result<(), JsValue>;
}
