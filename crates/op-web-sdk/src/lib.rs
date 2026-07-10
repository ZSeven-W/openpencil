//! OpenPencil read-only web embedding SDK. Parses a `.op` document and
//! renders it to a `<canvas>` via CanvasKit, with pan/zoom navigation,
//! read-only snapshots, and SVG export. No editing.
// `dirty_state` is compiled unconditionally (not gated behind the
// `canvaskit` feature, unlike `render.rs` which owns the wasm rAF plumbing
// around it) so its transition-logic unit tests run in the crate's default
// native test build.
mod dirty_state;
mod document;
mod export;
mod navigation;
#[cfg(feature = "canvaskit")]
mod render;
mod scene;
mod snapshot;
mod viewer_host;

use op_editor_core::Viewport as DocViewport;
use wasm_bindgen::prelude::*;

/// Read-only viewer handle. Owns the parsed document + paint scene.
#[wasm_bindgen]
pub struct Viewer {
    /// Parsed canonical `.op` document; `None` until `load` is called.
    doc: Option<jian_ops_schema::PenDocument>,
    /// Index of the currently visible page (0-based).
    active_page: usize,
    /// Paint-only layout scene built from the loaded document.
    /// `None` until a document is loaded and `rebuild_scene` runs.
    scene: Option<op_editor_ui::layout_scene::LayoutScene>,
    /// Current pan/zoom state. Defaults to identity (origin pan, 100% zoom).
    /// Mutated by `set_viewport`, `zoom_to_fit`, and `forward_wheel`.
    viewport: DocViewport,
}

#[wasm_bindgen]
impl Viewer {
    /// Construct an empty viewer (no document loaded yet).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Viewer {
            doc: None,
            active_page: 0,
            scene: None,
            viewport: DocViewport::IDENTITY,
        }
    }

    /// Parse a canonical `.op` JSON string and render. Wraps `load` for JS.
    pub fn load_str(&mut self, src: &str) -> Result<(), JsValue> {
        self.load(src).map_err(|e| JsValue::from_str(&e))
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Test-only helpers — not part of the wasm public surface.
#[cfg(test)]
impl Viewer {
    /// Construct an empty viewer for in-crate tests. Identical to `new()`;
    /// kept as a named alias so test call sites don't need `Default::default()`.
    pub(crate) fn placeholder() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod scaffold_tests {
    #[test]
    fn viewer_placeholder_constructs() {
        let _v = super::Viewer::placeholder();
    }
}

#[cfg(test)]
mod dirty_state_tests;

#[cfg(test)]
mod document_tests;

#[cfg(test)]
mod scene_tests;

#[cfg(test)]
mod navigation_tests;

#[cfg(test)]
mod export_tests;

#[cfg(test)]
mod snapshot_tests;
