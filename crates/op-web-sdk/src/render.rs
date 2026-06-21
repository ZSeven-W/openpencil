//! CanvasKit render loop for the read-only `Viewer`.
//!
//! `attach_canvas` initialises a `CanvasKitBackend` from `op-host-web` and
//! installs a `requestAnimationFrame` pump that repaints the document scene
//! whenever the `dirty` flag is set. The widget facade is entirely delegated
//! to `viewer_host::paint_scene` — this file owns only the lifecycle glue.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use op_editor_core::Viewport as DocViewport;
use op_editor_ui::theme::Theme;
use op_editor_ui::RenderBackend;
use op_host_web::canvaskit::{init_backend, CanvasKitBackend};

use crate::Viewer;

// ---------------------------------------------------------------------------
// Per-instance render state.
// ---------------------------------------------------------------------------

/// Live render state for one mounted Viewer.
struct RenderInner {
    backend: CanvasKitBackend,
    /// Physical width of the canvas element (CSS pixels).
    logical_w: f32,
    /// Physical height of the canvas element (CSS pixels).
    logical_h: f32,
    /// Set to `true` after any state change that requires a repaint.
    dirty: Cell<bool>,
    /// The most recent layout scene to paint. Updated by `push_scene` so
    /// `load_str()` + `mark_dirty()` shows the newly loaded document rather
    /// than the stale snapshot captured at `attach_canvas` time.
    scene: Option<op_editor_ui::layout_scene::LayoutScene>,
    /// Current pan/zoom viewport. Updated by `push_viewport_to_render` on
    /// every `set_viewport` / `forward_wheel` call so the pump always reads
    /// the live value rather than the snapshot captured at attach time.
    viewport: DocViewport,
}

// ---------------------------------------------------------------------------
// Thread-local render slot — one Viewer can be mounted at a time.
// ---------------------------------------------------------------------------

thread_local! {
    /// The active render state. `None` until `attach_canvas` succeeds.
    static RENDER: RefCell<Option<Rc<RefCell<RenderInner>>>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// wasm-bindgen impl on Viewer
// ---------------------------------------------------------------------------

#[wasm_bindgen]
impl Viewer {
    /// Attach a `<canvas>` element and start the rAF render loop.
    ///
    /// Reads the canvas's current pixel dimensions to set the logical viewport.
    /// Subsequent calls to `mark_dirty()` (or an internal repaint trigger)
    /// will cause a repaint on the next animation frame.
    ///
    /// If a previous rAF pump is running it is stopped before the new one
    /// starts: clearing the `RENDER` thread-local slot causes the old pump to
    /// self-terminate on its next tick, so only one pump is ever live.
    pub async fn attach_canvas(&self, canvas_id: String) -> Result<(), JsValue> {
        // Tear down any existing render state so the previous rAF pump sees an
        // empty slot on its next tick and terminates, preventing two concurrent
        // pumps from racing on begin_frame / end_frame.
        self.detach();

        console_error_panic_hook::set_once();

        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("attach_canvas: no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("attach_canvas: no document"))?;
        let canvas: web_sys::HtmlCanvasElement = document
            .get_element_by_id(&canvas_id)
            .ok_or_else(|| JsValue::from_str("attach_canvas: canvas not found"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("attach_canvas: element is not a <canvas>"))?;

        // Derive logical size from the canvas element's current CSS client area.
        let css_w = canvas.client_width().max(1) as u32;
        let css_h = canvas.client_height().max(1) as u32;
        // Use a device-pixel-ratio of 1.0 when the canvas has no physical size
        // set yet; the caller can resize the element and call again.
        let dev_w = canvas.width().max(1) as f32;
        let css_w_f = css_w as f32;
        let dpr = (dev_w / css_w_f).max(1.0);

        let backend = init_backend(&canvas_id, dpr, css_w, css_h).await?;

        // Snapshot the current scene and viewport into the shared state so
        // the pump can paint immediately. Subsequent `load_str` + `push_scene`
        // and `set_viewport` / `forward_wheel` calls update the fields without
        // re-attaching the canvas.
        let initial_scene = self.scene().cloned();
        let initial_viewport = self.viewport;

        let inner = Rc::new(RefCell::new(RenderInner {
            backend,
            logical_w: css_w as f32,
            logical_h: css_h as f32,
            dirty: Cell::new(true),
            scene: initial_scene,
            viewport: initial_viewport,
        }));

        // Install the render slot so the rAF closure can reach it.
        RENDER.with(|slot| {
            *slot.borrow_mut() = Some(inner.clone());
        });

        let inner_for_raf = inner.clone();
        start_raf_pump(inner_for_raf);
        Ok(())
    }

    /// Detach the CanvasKit backend, stopping the rAF loop.
    ///
    /// Safe to call when not attached — it is a no-op in that case.
    pub fn detach(&self) {
        RENDER.with(|slot| {
            slot.borrow_mut().take();
        });
    }

    /// Mark the canvas dirty so the rAF pump issues a repaint on the next
    /// animation frame.
    ///
    /// After calling `load_str()` call `push_scene()` (which also sets the
    /// dirty flag) to update both the displayed scene and trigger a repaint.
    /// `mark_dirty()` alone repaints the currently stored scene without
    /// replacing it.
    pub fn mark_dirty(&self) {
        RENDER.with(|slot| {
            if let Some(inner_rc) = slot.borrow().as_ref() {
                if let Ok(inner) = inner_rc.try_borrow() {
                    inner.dirty.set(true);
                }
            }
        });
    }

    /// Push the viewer's current layout scene into the live render state and
    /// mark the canvas dirty.
    ///
    /// Call this after `load_str()` to display the newly loaded document
    /// without re-attaching the canvas. If no canvas is attached yet, this
    /// is a no-op; the scene will be picked up automatically by the next
    /// `attach_canvas` call via the `initial_scene` snapshot.
    pub fn push_scene(&self) {
        let scene = self.scene().cloned();
        RENDER.with(|slot| {
            if let Some(inner_rc) = slot.borrow().as_ref() {
                if let Ok(mut inner) = inner_rc.try_borrow_mut() {
                    inner.scene = scene;
                    inner.dirty.set(true);
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// rAF pump helpers.
// ---------------------------------------------------------------------------

/// Push an updated viewport into the live render state so the RAF pump reads
/// it on the next animation frame. Called from `navigation::push_viewport`.
/// No-op if no canvas is attached.
pub(crate) fn push_viewport_to_render(vp: DocViewport) {
    RENDER.with(|slot| {
        if let Some(inner_rc) = slot.borrow().as_ref() {
            if let Ok(mut inner) = inner_rc.try_borrow_mut() {
                inner.viewport = vp;
                inner.dirty.set(true);
            }
        }
    });
}

/// Schedule a self-rescheduling `requestAnimationFrame` pump.
///
/// The pump fires once per animation frame, checks the `dirty` flag, paints
/// if needed, then reschedules as long as the RENDER thread-local is occupied.
/// When `detach` clears the slot the next frame fires, finds the slot empty,
/// and drops the closure.
///
/// The scene is read from `inner.scene` each frame so callers can update it
/// via `push_scene` without restarting the pump.
fn start_raf_pump(inner: Rc<RefCell<RenderInner>>) {
    // Wrap the closure in a shared slot so it can re-schedule itself.
    type FrameSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;
    let holder: FrameSlot = Rc::new(RefCell::new(None));
    let holder2 = holder.clone();

    *holder.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // Check whether the render slot is still live.  If `detach` was
        // called (or a new `attach_canvas` replaced the slot) the Rc inside
        // RENDER is a *different* allocation from the one captured in
        // `inner`, so the pump can no longer reach the active state — stop.
        let still_live = RENDER.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|rc| Rc::ptr_eq(rc, &inner))
                .unwrap_or(false)
        });
        if !still_live {
            // Drop the self-reference so the closure can be freed.
            let _ = holder2.borrow_mut().take();
            return;
        }

        // Repaint if dirty.
        if let Ok(mut b) = inner.try_borrow_mut() {
            if b.dirty.get() {
                // Clone the scene out to avoid holding the borrow across the
                // mutable backend calls (which borrow `b` mutably).
                if let Some(scene_snap) = b.scene.clone() {
                    // Read the live viewport — updated by push_viewport_to_render
                    // on each set_viewport / forward_wheel call.
                    let vp = b.viewport;
                    let w = b.logical_w;
                    let h = b.logical_h;
                    b.backend.begin_frame();
                    crate::viewer_host::paint_scene(
                        &mut b.backend,
                        &scene_snap,
                        vp,
                        Theme::dark(),
                        w,
                        h,
                    );
                    b.backend.end_frame();
                    b.dirty.set(false);
                }
            }
        }

        // Reschedule for the next frame.
        if let Some(c) = holder2.borrow().as_ref() {
            request_frame(c);
        }
    }) as Box<dyn FnMut()>));

    // Kick off the first frame.
    {
        let slot = holder.borrow();
        if let Some(c) = slot.as_ref() {
            request_frame(c);
        }
    }
}

/// Schedule `c` to run on the next animation frame. Best-effort: if `window`
/// is unavailable (e.g. a WebWorker) the pump simply does not run.
fn request_frame(c: &Closure<dyn FnMut()>) {
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(c.as_ref().unchecked_ref());
    }
}
