// Document parsing and page snapshot accessors for Viewer.
use crate::Viewer;
use wasm_bindgen::prelude::wasm_bindgen;

impl Viewer {
    /// Parse a canonical `.op` JSON string (best-effort, legacy-tolerant).
    /// Immediately rebuilds the cached `LayoutScene` from the loaded document.
    /// Internal Rust API; the JS-facing wrapper is `Viewer::load_str` (maps the
    /// error to a `JsValue` so it surfaces as a thrown exception).
    pub fn load(&mut self, src: &str) -> Result<(), String> {
        let loaded = op_pen_loader::load_canonical(src).map_err(|e| format!("{e:?}"))?;
        self.doc = Some(loaded.value);
        self.active_page = 0;
        self.rebuild_scene();
        Ok(())
    }
}

/// Read-only page accessors. Exported to JS so consumers (and the smoke page)
/// can query the loaded document without a full snapshot round-trip.
#[wasm_bindgen]
impl Viewer {
    /// Number of pages in the loaded document.
    /// Returns 0 when no document is loaded; at least 1 otherwise.
    pub fn page_count(&self) -> usize {
        self.doc
            .as_ref()
            .map(|d| d.pages.as_ref().map(|p| p.len()).unwrap_or(1).max(1))
            .unwrap_or(0)
    }

    /// Index of the currently active (visible) page (0-based).
    pub fn active_page_index(&self) -> usize {
        self.active_page
    }
}
