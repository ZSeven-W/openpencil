// SVG (and future format) export for the read-only Viewer.
//
// Only SVG is available in standalone v1. PNG / PDF require a rendering
// daemon; calling `export("png")` returns an explicit error message so
// callers get a clear signal rather than a silent panic or opaque failure.

use crate::Viewer;
use wasm_bindgen::prelude::*;

impl Viewer {
    /// Serialize the active page to an SVG string.
    ///
    /// Returns `Err` when no document has been loaded, `rebuild_scene` has
    /// not been called yet, or the scene has no renderable content.
    pub fn export_svg(&self) -> Result<String, String> {
        let scene = self
            .scene
            .as_ref()
            .ok_or_else(|| "no scene — call load() then rebuild_scene() first".to_string())?;
        op_editor_ui::svg_export::serialize_active_page_svg(scene)
    }
}

/// Export the active page in the requested format.
///
/// `"svg"` — returns UTF-8 SVG bytes.
/// `"png"` / `"pdf"` — returns an explicit error; use a daemon for raster/PDF.
#[wasm_bindgen]
pub fn export(viewer: &Viewer, format: String) -> Result<Vec<u8>, JsValue> {
    match format.to_lowercase().as_str() {
        "svg" => viewer
            .export_svg()
            .map(|s| s.into_bytes())
            .map_err(|e| JsValue::from_str(&e)),
        _ => Err(JsValue::from_str(
            "format not available in standalone v1; use SVG or a daemon",
        )),
    }
}
