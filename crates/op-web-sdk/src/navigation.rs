//! Pan / zoom navigation for the read-only Viewer.
//!
//! Pure viewport state management (no browser dependencies) so the math is
//! unit-testable without the `canvaskit` feature. The wheel handler that
//! integrates with the render state lives in the `canvaskit` feature block at
//! the bottom of this file.

use op_editor_core::Viewport as DocViewport;
use op_editor_ui::{Point2D, Rect};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::Viewer;

// ---------------------------------------------------------------------------
// Internal viewport accessor (tuple return — not wasm-exportable; JS reads the
// viewport via the exported `viewport_json()`).
// ---------------------------------------------------------------------------

impl Viewer {
    /// Return the current viewport as `(pan_x, pan_y, zoom)`.
    /// Rust/test-facing; JS callers use `viewport_json()`.
    pub fn viewport(&self) -> (f32, f32, f32) {
        (self.viewport.pan_x, self.viewport.pan_y, self.viewport.zoom)
    }
}

// ---------------------------------------------------------------------------
// JS-facing navigation (feature-independent: set_viewport / zoom_to_fit work
// without the render path, falling back to the no-op push_viewport stub).
// ---------------------------------------------------------------------------

#[wasm_bindgen]
impl Viewer {
    /// Set the pan/zoom state and push the update into the live render state.
    pub fn set_viewport(&mut self, pan_x: f32, pan_y: f32, zoom: f32) {
        self.viewport = DocViewport { pan_x, pan_y, zoom };
        // Push to the RAF pump so it repaints with the new viewport.
        // push_viewport already marks + arms exactly once; a second
        // mark_dirty here would advance the content generation again and
        // reset the failure breaker, granting a duplicate retry budget
        // (and a second console.error) for the same update.
        self.push_viewport();
    }

    /// Fit the active page's content into a `w × h` canvas (in CSS px).
    ///
    /// Computes the union AABB of all top-level nodes on the active page
    /// (via `LayoutScene::content_bounds`), then calls `Viewport::fit_to`
    /// to centre and scale the content within the canvas with a 40 px
    /// padding margin. Updates `self.viewport` and pushes to the live render
    /// state so the RAF pump picks up the new pan/zoom on the next frame.
    /// A no-op when no scene is loaded or the page has no nodes with
    /// positive size.
    pub fn zoom_to_fit(&mut self, w: f32, h: f32) {
        let Some(scene) = self.scene.as_ref() else {
            return;
        };
        let Some(bounds) = scene.content_bounds() else {
            return;
        };
        // Guard: content must have positive area.
        if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
            return;
        }
        self.viewport.fit_to(
            Rect {
                origin: Point2D::new(bounds.origin.x, bounds.origin.y),
                size: Point2D::new(bounds.size.x, bounds.size.y),
            },
            w,
            h,
            40.0,
        );
        // Push to the RAF pump so it repaints with the fitted viewport.
        // (One mark only — see set_viewport for why a second mark_dirty
        // would double the breaker budget.)
        self.push_viewport();
    }
}

// ---------------------------------------------------------------------------
// No-op push_viewport stub for builds without the canvaskit render path.
// Mirrors the mark_dirty no-op pattern in lib.rs.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "canvaskit"))]
impl Viewer {
    pub(crate) fn push_viewport(&self) {}
}

// ---------------------------------------------------------------------------
// Wheel + push_viewport — browser-only (canvaskit feature).
// ---------------------------------------------------------------------------

#[cfg(feature = "canvaskit")]
#[wasm_bindgen]
impl Viewer {
    /// Process a DOM `wheel` event from JS.
    ///
    /// When `ctrl_or_meta` is true (pinch-to-zoom or Ctrl+wheel) the event
    /// zooms about `(cursor_x, cursor_y)` in canvas-local px. Otherwise it
    /// pans by `(dx, dy)` CSS px (typical scroll or two-finger pan).
    ///
    /// After updating the viewport, the new state is pushed into the live
    /// render state so the RAF pump picks it up on the next animation frame.
    pub fn forward_wheel(
        &mut self,
        dx: f32,
        dy: f32,
        ctrl_or_meta: bool,
        cursor_x: f32,
        cursor_y: f32,
    ) {
        if ctrl_or_meta {
            // Zoom about cursor. Negate dy so scroll-up zooms in.
            self.viewport.zoom_at(Point2D::new(cursor_x, cursor_y), -dy);
        } else {
            // Pan in the scroll direction.
            self.viewport.pan(-dx, -dy);
        }
        self.push_viewport();
    }
}

// push_viewport stays out of the `#[wasm_bindgen]` impl (it is `pub(crate)`
// internal plumbing, not part of the JS surface).
#[cfg(feature = "canvaskit")]
impl Viewer {
    /// Push the current `self.viewport` into the live `RenderInner` so the
    /// RAF pump reads the updated value on the next frame.
    pub(crate) fn push_viewport(&self) {
        use crate::render::push_viewport_to_render;
        push_viewport_to_render(self.viewport);
    }
}

#[cfg(test)]
mod single_mark_tests {
    /// Regression guard for the breaker-budget contract: `set_viewport` /
    /// `zoom_to_fit` must mark the dirty state exactly once, via
    /// `push_viewport`. A second `mark_dirty()` call in this file would
    /// advance the content generation again and reset the failure breaker,
    /// granting a duplicate retry budget (and a second `console.error`)
    /// for the same update. The render pump cannot be attached in native
    /// tests, so this guards the source structurally, mirroring the
    /// state-machine tests in `dirty_state_tests.rs` that pin the
    /// double-mark = double-budget semantics.
    #[test]
    fn navigation_never_calls_mark_dirty_directly() {
        let src = include_str!("navigation.rs");
        // Assemble the needle at runtime so this test's own source (embedded
        // by include_str!) cannot match it.
        let needle = format!("self.{}()", "mark_dirty");
        assert!(
            !src.contains(&needle),
            "navigation.rs must mark via push_viewport() only; a direct \
             mark-dirty call doubles the failure-breaker budget"
        );
    }
}
