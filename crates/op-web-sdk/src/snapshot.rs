// Read-only JSON snapshot getters for the embedded Viewer.
//
// These methods serialize the current in-memory state to JSON strings so
// JavaScript callers can read document structure without touching Rust internals.
// All methods are available without the `canvaskit` feature (they use serde_json
// directly, with no browser dependencies).
//
// Error handling: all three getters return `Result<String, JsValue>`. On the
// JS side wasm-bindgen converts `Err(JsValue)` into a thrown exception, so
// serialisation failures surface as real JS errors rather than fake-JSON
// strings that `JSON.parse` would silently swallow.

use crate::Viewer;
use jian_ops_schema::node::PenNode;
use op_editor_core::Viewport as DocViewport;
use serde::Serialize;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Viewport serialization helper — DocViewport does not derive Serialize so we
// project its fields into a small local struct that does.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[allow(dead_code)]
struct ViewportSnapshot {
    pan_x: f32,
    pan_y: f32,
    zoom: f32,
}

impl From<DocViewport> for ViewportSnapshot {
    fn from(v: DocViewport) -> Self {
        ViewportSnapshot {
            pan_x: v.pan_x,
            pan_y: v.pan_y,
            zoom: v.zoom,
        }
    }
}

// ---------------------------------------------------------------------------
// Synthetic single-page wrapper for the single-page-fallback path.
//
// When `PenDocument.pages` is `None` but `children` is non-empty, the nodes
// live in `doc.children` (the legacy/single-page-fallback layout). We wrap
// them in a synthetic page so JS callers always receive the same
// `[{id, name, children}]` shape regardless of which storage format was used.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SyntheticPage<'a> {
    id: &'a str,
    name: &'a str,
    children: &'a [PenNode],
}

// ---------------------------------------------------------------------------
// Wasm-exposed snapshot getters.
// ---------------------------------------------------------------------------

#[wasm_bindgen]
impl Viewer {
    /// Serialize the full loaded `PenDocument` to a JSON string.
    ///
    /// Returns `Ok("{}")` when no document has been loaded yet so callers always
    /// receive valid JSON they can safely parse without null-checking.
    ///
    /// # Errors
    /// Throws a JS exception if `serde_json` fails to serialize the document.
    pub fn document_json(&self) -> Result<String, JsValue> {
        match &self.doc {
            Some(doc) => serde_json::to_string(doc).map_err(|e| JsValue::from_str(&e.to_string())),
            None => Ok("{}".to_string()),
        }
    }

    /// Serialize the pages array of the loaded document to a JSON string.
    ///
    /// The returned JSON always has the shape `[{id, name, children}, ...]`.
    ///
    /// - No document loaded → `Ok("[]")`.
    /// - Document has an explicit `pages` array → serialize that array.
    /// - Document has `pages: None` but non-empty `children` (single-page
    ///   fallback) → return a synthetic single-element array wrapping those
    ///   children under `{id: "default", name: "Page 1", children: [...]}`.
    /// - Document has `pages: None` and empty `children` → `Ok("[]")`.
    ///
    /// # Errors
    /// Throws a JS exception if `serde_json` fails to serialize the data.
    pub fn pages_json(&self) -> Result<String, JsValue> {
        let doc = match &self.doc {
            Some(d) => d,
            None => return Ok("[]".to_string()),
        };

        match &doc.pages {
            Some(pages) => {
                serde_json::to_string(pages).map_err(|e| JsValue::from_str(&e.to_string()))
            }
            None => {
                if doc.children.is_empty() {
                    Ok("[]".to_string())
                } else {
                    // Single-page fallback: wrap doc.children in a synthetic page.
                    let synthetic = [SyntheticPage {
                        id: "default",
                        name: "Page 1",
                        children: &doc.children,
                    }];
                    serde_json::to_string(&synthetic).map_err(|e| JsValue::from_str(&e.to_string()))
                }
            }
        }
    }

    /// Serialize the current viewport (pan + zoom) to a JSON string.
    ///
    /// The returned object always contains `pan_x`, `pan_y`, and `zoom` fields.
    ///
    /// # Errors
    /// Throws a JS exception if `serde_json` fails to serialize the viewport.
    pub fn viewport_json(&self) -> Result<String, JsValue> {
        let snap = ViewportSnapshot::from(self.viewport);
        serde_json::to_string(&snap).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
