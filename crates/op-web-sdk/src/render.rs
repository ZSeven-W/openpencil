//! CanvasKit render loop for the read-only `Viewer`.
//!
//! `attach_canvas` initialises a `CanvasKitBackend` from `op-host-web` and
//! installs a `requestAnimationFrame` pump that repaints the document scene
//! whenever the dirty state says so. The widget facade is entirely delegated
//! to `viewer_host::paint_scene` — this file owns only the lifecycle glue.
//!
//! Invariants this module maintains (the pure transition rules live in
//! `crate::dirty_state`; this file owns the wasm plumbing around them):
//!
//! 1. The scene is never deep-cloned per frame. `RenderInner::scene` is an
//!    `Rc<LayoutScene>`; the pump clones the `Rc` (a refcount bump) instead
//!    of the `LayoutScene` itself.
//! 2. The pump idles when clean instead of rescheduling forever. Every
//!    dirty-setting entry point (`mark_dirty`, `push_scene`,
//!    `push_viewport_to_render`) routes through `mark_and_arm`, the only
//!    place allowed to make the clean-to-dirty transition. `scheduled` is
//!    committed only after `requestAnimationFrame` actually succeeded. A
//!    rejected request is retried synchronously inside `arm_pump` against
//!    the SAME bounded failure breaker a failed paint uses
//!    (`DirtyState::record_failure`, budget `MAX_CONSECUTIVE_FAILURES`) —
//!    so a persistently rejected request cannot freeze the viewer waiting
//!    on some future external mark that may never come: within one
//!    `arm_pump` call the state either ends up armed, or the breaker trips
//!    and logs once, with no external help required.
//! 3. Dirty is consumed only by a paint that ran to completion, and only
//!    for the generation snapshotted before the paint — a mark arriving
//!    while the paint runs survives (the dirty state lives outside the
//!    `RefCell` the pump borrows, so re-entrant marks land) and re-arms.
//!    A failed paint re-marks + re-arms with the same bounded budget as
//!    (2), then stops with one console error until new content arrives.
//!    Generation counters (`content_gen` / `painted_gen`) wrap via
//!    `wrapping_add`; every comparison against them is `!=`, never `>`,
//!    so a wrap can never alias a newer generation as already-painted —
//!    see `crate::dirty_state`'s module doc. The failure breaker is owned
//!    by whichever generation is current when it trips, not by whichever
//!    paint happens to finish afterwards: an older, already in-flight
//!    paint's success (`finish_paint_success`) never resets a newer
//!    generation's tripped breaker and never asks this pump to call
//!    `arm_pump` again while it is tripped — only a fresh `mark_dirty()`
//!    reopens it. See `DirtyState::finish_paint_success`'s doc comment.
//! 4. A `push_scene` / `push_viewport_to_render` call queues its payload
//!    in a dedicated `Cell` (`pending_scene` / `pending_viewport`)
//!    OUTSIDE the `inner` `RefCell` the pump holds during paint, rather
//!    than writing `inner` directly. That keeps the dirty-generation bump
//!    those calls make honest even when they land while a paint is in
//!    flight: the write can never be silently skipped by a failed
//!    `try_borrow_mut`, and `paint_frame` drains the pending slot into
//!    `inner` at the very start of every paint attempt, before reading
//!    scene/viewport state to paint.
//! 5. `detach` (and the `attach_canvas` re-attach path through it) cancels
//!    any armed-but-not-yet-fired animation frame AND explicitly breaks the
//!    closure ownership cycle, so an *idle* pump does not leak the closure,
//!    render state, CanvasKit backend, or scene. Drop chain: `detach` takes
//!    the `PumpHandle` out of `PUMP` and drops the `Closure` stored inside
//!    it; the closure's captured `Rc<RenderShared>` and `Rc<PumpHandle>`
//!    strong refs die with it; `RENDER` was already cleared, so
//!    `RenderShared` (→ `RenderInner` → `CanvasKitBackend` + the scene
//!    `Rc`) deallocates, and the local `PumpHandle` Rc is the last one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use op_editor_core::Viewport as DocViewport;
use op_editor_ui::theme::Theme;
use op_editor_ui::RenderBackend;
use op_host_web::canvaskit::{init_backend, CanvasKitBackend};

use crate::dirty_state::{DirtyState, FailureAction};
use crate::Viewer;

/// Shared alias for the `Rc`-wrapped paint scene, so the pending-payload
/// `Cell` types below don't have to repeat the fully-qualified path.
type SceneRc = Rc<op_editor_ui::layout_scene::LayoutScene>;

#[wasm_bindgen]
extern "C" {
    /// `console.error` binding, declared here (instead of enabling the
    /// web-sys `console` feature) to keep the dependency surface unchanged.
    /// Used exactly once per paint-failure breaker trip.
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn js_console_error(msg: &str);
}

// ---------------------------------------------------------------------------
// Per-instance render state.
// ---------------------------------------------------------------------------

/// Live render state for one mounted Viewer: the dirty/schedule machine
/// (interior-mutable `Cell`s, deliberately OUTSIDE the `RefCell`) plus the
/// mutable paint payload.
///
/// Keeping `state` outside the `RefCell` matters: the pump holds
/// `inner.borrow_mut()` across the actual paint calls, and a JS callback
/// re-entering `mark_dirty` / `push_scene` during that paint must still be
/// able to record its mark — otherwise a mark-during-paint would be
/// silently dropped by a failed `try_borrow`.
struct RenderShared {
    /// Dirty/schedule/failure state machine — see `crate::dirty_state`.
    state: DirtyState,
    /// Pending `push_scene` payload, queued OUTSIDE the paint-held
    /// `inner` `RefCell` (see the F3 fix note on `push_scene`). Nested
    /// `Option`: the outer layer is `None` when there is nothing pending
    /// and `Some(_)` when a push is waiting to be applied; the inner
    /// layer is the actual scene value, which can itself legitimately be
    /// `None` (a push while no document is loaded must still clear
    /// `inner.scene`, not leave a stale one painted).
    pending_scene: Cell<Option<Option<SceneRc>>>,
    /// Pending `push_viewport_to_render` payload, same pending-slot
    /// pattern as `pending_scene`. A single `Option` layer suffices here
    /// because that call always supplies a concrete viewport.
    pending_viewport: Cell<Option<DocViewport>>,
    /// Mutable paint payload; borrowed mutably only around the paint.
    inner: RefCell<RenderInner>,
}

/// The mutable paint payload for one mounted Viewer.
struct RenderInner {
    backend: CanvasKitBackend,
    /// Physical width of the canvas element (CSS pixels).
    logical_w: f32,
    /// Physical height of the canvas element (CSS pixels).
    logical_h: f32,
    /// The most recent layout scene to paint, shared via `Rc` so the pump
    /// can clone the handle (cheap refcount bump) instead of deep-cloning
    /// the whole scene to dodge a `RefCell` borrow conflict. Updated by
    /// `push_scene` so `load_str()` + `mark_dirty()` shows the newly loaded
    /// document rather than the stale snapshot captured at `attach_canvas`
    /// time. The scene is read-only during paint, so sharing it via `Rc`
    /// instead of taking `&mut` access is sound.
    scene: Option<Rc<op_editor_ui::layout_scene::LayoutScene>>,
    /// Current pan/zoom viewport. Updated by `push_viewport_to_render` on
    /// every `set_viewport` / `forward_wheel` call so the pump always reads
    /// the live value rather than the snapshot captured at attach time.
    viewport: DocViewport,
}

/// The pump's rAF closure plus the id of the currently armed (requested but
/// not yet fired) animation frame, so `detach` can cancel it.
struct PumpHandle {
    /// The self-referencing rAF closure. `Option` because the slot starts
    /// empty (before the closure is built) and is emptied again when the
    /// pump self-terminates or `detach` breaks the cycle.
    closure: RefCell<Option<Closure<dyn FnMut()>>>,
    /// Id returned by the last successful `requestAnimationFrame` call;
    /// cleared when the callback fires. `detach` cancels it if still set.
    raf_id: Cell<Option<i32>>,
}

// ---------------------------------------------------------------------------
// Thread-local render slot — one Viewer can be mounted at a time.
// ---------------------------------------------------------------------------

thread_local! {
    /// The active render state. `None` until `attach_canvas` succeeds.
    static RENDER: RefCell<Option<Rc<RenderShared>>> = const { RefCell::new(None) };
    /// The active pump's closure + armed-frame id, kept alive here (not
    /// inside `RenderShared`) so `mark_and_arm` can request a fresh frame
    /// after the pump has gone idle. Keeping it out of `RenderShared`
    /// avoids a reference cycle: the closure itself holds a strong
    /// `Rc<RenderShared>`, so `RenderShared` must never hold a strong
    /// reference back to its own closure. `start_raf_pump` overwrites this
    /// on every `attach_canvas`; `detach` cancels + drops it.
    static PUMP: RefCell<Option<Rc<PumpHandle>>> = const { RefCell::new(None) };
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
    /// If a previous pump exists it is torn down first (`detach`): its armed
    /// frame, if any, is cancelled and its closure dropped, so only one pump
    /// is ever live.
    pub async fn attach_canvas(&self, canvas_id: String) -> Result<(), JsValue> {
        // Tear down any existing render state (cancel the armed frame, break
        // the closure cycle) so two pumps never race on begin/end_frame.
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
        // re-attaching the canvas. This clone happens once here (and once per
        // `push_scene` call) — never per frame; see the pump below.
        let initial_scene = self.scene().cloned().map(Rc::new);
        let initial_viewport = self.viewport;

        let shared = Rc::new(RenderShared {
            // Dirty (the initial content has never been painted) but NOT
            // pre-scheduled: the first frame is armed by `start_raf_pump`
            // through the same request-then-commit path as every later one.
            state: DirtyState::new_dirty(),
            pending_scene: Cell::new(None),
            pending_viewport: Cell::new(None),
            inner: RefCell::new(RenderInner {
                backend,
                logical_w: css_w as f32,
                logical_h: css_h as f32,
                scene: initial_scene,
                viewport: initial_viewport,
            }),
        });

        // Install the render slot so the rAF closure can reach it.
        RENDER.with(|slot| {
            *slot.borrow_mut() = Some(shared.clone());
        });

        start_raf_pump(shared);
        Ok(())
    }

    /// Detach the CanvasKit backend, stopping the rAF loop and releasing
    /// every resource the pump kept alive.
    ///
    /// Safe to call when not attached — it is a no-op in that case.
    ///
    /// This is the idle-safe teardown path: an idle pump has no pending
    /// callback that could run the closure's own self-cleanup, so `detach`
    /// must (a) cancel the armed animation frame if one is pending and
    /// (b) explicitly drop the closure to break the `Closure → RenderShared
    /// / PumpHandle` ownership cycle. See the module doc for the full drop
    /// chain.
    pub fn detach(&self) {
        let shared = RENDER.with(|slot| slot.borrow_mut().take());
        if let Some(shared) = shared {
            // State-machine side of detach: the pending schedule is being
            // cancelled below, so clear the flag too. Keeps the machine
            // consistent for anything still holding a reference to it.
            shared.state.cancel_schedule();
        }
        let handle = PUMP.with(|slot| slot.borrow_mut().take());
        if let Some(handle) = handle {
            // Cancel the armed-but-not-yet-fired frame, if any, so the
            // browser never invokes a closure we are about to drop.
            if let Some(id) = handle.raf_id.take() {
                if let Some(window) = web_sys::window() {
                    let _ = window.cancel_animation_frame(id);
                }
            }
            // Break the ownership cycle: the closure strongly holds both
            // `Rc<RenderShared>` and `Rc<PumpHandle>`; dropping it here
            // releases them even when the pump is idle (the in-closure
            // self-cleanup only runs if a callback actually fires).
            drop(handle.closure.borrow_mut().take());
        }
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
            if let Some(shared) = slot.borrow().as_ref() {
                mark_and_arm(shared);
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
    ///
    /// Writes into `pending_scene`, NOT `inner.scene` directly (F3 fix): a
    /// push landing while the pump holds `inner.borrow_mut()` mid-paint
    /// (a re-entrant call from JS) must still land instead of being
    /// silently dropped by a failed `try_borrow_mut` — the dirty
    /// generation this call bumps (via `mark_and_arm`, below) would
    /// otherwise be consumed by a frame that painted the stale payload.
    /// The pump applies `pending_scene` at the start of `paint_frame`.
    pub fn push_scene(&self) {
        let scene = self.scene().cloned().map(Rc::new);
        RENDER.with(|slot| {
            if let Some(shared) = slot.borrow().as_ref() {
                shared.pending_scene.set(Some(scene));
                mark_and_arm(shared);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// The dirty-write funnel. Every entry point that needs a repaint routes
// through this so a re-arm can never be missed — see `crate::dirty_state`.
// ---------------------------------------------------------------------------

/// Mark `shared` dirty and, if no frame is pending, try to arm one. This is
/// the ONLY function in the crate allowed to make the clean-to-dirty
/// transition; `mark_dirty`, `push_scene`, and `push_viewport_to_render`
/// all route through here. Touches only `Cell`s (never the `RefCell`), so
/// it is safe to call re-entrantly while the pump is mid-paint.
fn mark_and_arm(shared: &RenderShared) {
    if shared.state.mark_dirty() {
        arm_pump(&shared.state);
    }
}

/// Attempt to arm one animation frame, committing `scheduled` only if the
/// `requestAnimationFrame` call succeeded.
///
/// F1 fix: a failed request is no longer a silent no-op that waits on some
/// future external `mark_and_arm` call to retry (which may never come,
/// freezing the viewer with content newer than painted and nothing armed
/// or logged). Instead the failure is recorded through the SAME bounded
/// breaker `record_failure` a failed paint uses, and retried synchronously
/// within this call until either a frame gets armed or the shared budget
/// is exhausted — at which point the pump stops and logs exactly once,
/// same as a persistently failing paint. Because the retry loop lives
/// entirely inside this one call, the invariant ("dirty implies an armed
/// frame OR a logged, deliberate stop") holds with no external help. A
/// later `mark_dirty()` (new content) resets the breaker and the next arm
/// attempt starts its budget over, exactly like a paint failure recovering
/// on new content.
fn arm_pump(state: &DirtyState) {
    loop {
        if request_pump_frame() {
            state.commit_arm();
            return;
        }
        match state.record_failure() {
            FailureAction::Rearm => continue,
            FailureAction::Stop => {
                js_console_error(
                    "op-web-sdk: requestAnimationFrame failed repeatedly; render pump \
                     stopped. The next scene or viewport update will retry.",
                );
                return;
            }
        }
    }
}

/// Request a fresh animation frame for the currently registered pump.
/// Returns `true` only when the browser accepted the request (the armed id
/// is then recorded for `detach` to cancel). Returns `false` when no pump
/// is registered, `window` is unavailable, or the request itself failed.
fn request_pump_frame() -> bool {
    PUMP.with(|slot| {
        let borrow = slot.borrow();
        let Some(handle) = borrow.as_ref() else {
            return false;
        };
        let closure = handle.closure.borrow();
        let Some(c) = closure.as_ref() else {
            return false;
        };
        let Some(window) = web_sys::window() else {
            return false;
        };
        match window.request_animation_frame(c.as_ref().unchecked_ref()) {
            Ok(id) => {
                handle.raf_id.set(Some(id));
                true
            }
            Err(_) => false,
        }
    })
}

// ---------------------------------------------------------------------------
// rAF pump helpers.
// ---------------------------------------------------------------------------

/// Push an updated viewport into the live render state so the RAF pump reads
/// it on the next animation frame. Called from `navigation::push_viewport`.
/// No-op if no canvas is attached.
///
/// Writes into `pending_viewport`, NOT `inner.viewport` directly — same F3
/// fix and rationale as `Viewer::push_scene` above.
pub(crate) fn push_viewport_to_render(vp: DocViewport) {
    RENDER.with(|slot| {
        if let Some(shared) = slot.borrow().as_ref() {
            shared.pending_viewport.set(Some(vp));
            mark_and_arm(shared);
        }
    });
}

/// Build the pump's rAF closure, publish it to `PUMP`, and arm the first
/// frame (through the same request-then-commit path as every later frame).
///
/// The pump fires once per animation frame, snapshots the dirty generation,
/// paints, then goes idle — it does NOT reschedule itself unconditionally.
/// Future dirty-setting calls (`mark_and_arm`) request the next frame; a
/// mark arriving *during* the paint re-arms itself, and its generation is
/// not consumed by this frame's post-paint bookkeeping. A failed paint
/// keeps the generation dirty and retries with a bounded budget.
///
/// The scene is read from `inner.scene` each frame (as a cheap `Rc` clone,
/// never a deep clone) so callers can update it via `push_scene` without
/// restarting the pump.
fn start_raf_pump(shared: Rc<RenderShared>) {
    let handle = Rc::new(PumpHandle {
        closure: RefCell::new(None),
        raf_id: Cell::new(None),
    });
    let handle2 = handle.clone();

    // Publish the handle so `mark_and_arm` (called from JS-facing methods,
    // possibly long after this frame's closure has already fired and gone
    // idle) can request a fresh frame using the same closure.
    PUMP.with(|slot| {
        *slot.borrow_mut() = Some(handle.clone());
    });

    // The closure captures its own strong ref; `shared` stays available for
    // the kickoff arm below.
    let shared_for_closure = shared.clone();
    *handle.closure.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let shared = &shared_for_closure;
        // This callback just consumed the armed frame.
        handle2.raf_id.set(None);

        // Check whether the render slot is still live. If `detach` was
        // called (or a new `attach_canvas` replaced the slot) the Rc inside
        // RENDER is a *different* allocation from the one captured in
        // `shared`, so the pump can no longer reach the active state — stop.
        // (`detach` also cancels the armed frame and drops the closure
        // itself, so this branch mostly covers a frame already in the
        // browser's queue at cancel time.)
        let still_live = RENDER.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|rc| Rc::ptr_eq(rc, shared))
                .unwrap_or(false)
        });
        if !still_live {
            // Drop the self-reference so the closure can be freed.
            let _ = handle2.closure.borrow_mut().take();
            return;
        }

        // Snapshot the generation to paint. `begin_paint` clears only the
        // schedule flag — the dirty condition survives until
        // `finish_paint_success` consumes exactly this generation, so a
        // paint that dies midway cannot lose the frame, and a mark landing
        // during the paint (a newer generation) is never consumed here.
        let Some(gen) = shared.state.begin_paint() else {
            // Clean, or the failure breaker is tripped — idle until the
            // next mark_and_arm.
            return;
        };

        if paint_frame(shared) {
            if shared.state.finish_paint_success(gen) {
                // Newer content arrived during the paint and no frame is
                // pending for it — arm one now. `finish_paint_success`
                // itself suppresses this `true` result while the breaker
                // is tripped (by a generation newer than `gen`, since this
                // paint's own generation could only have tripped it via
                // the failure branch below, not this success branch), so
                // reaching this call site never reopens an already-logged
                // breaker trip with a fresh budget.
                arm_pump(&shared.state);
            }
        } else {
            match shared.state.record_failure() {
                FailureAction::Rearm => arm_pump(&shared.state),
                FailureAction::Stop => {
                    // Bounded stop: don't spin rAF on a persistently failing
                    // paint. Log exactly once per breaker trip; the next
                    // scene/viewport update (mark_dirty) resets the breaker
                    // and retries.
                    js_console_error(
                        "op-web-sdk: repaint failed repeatedly; render pump stopped. \
                         The next scene or viewport update will retry.",
                    );
                }
            }
        }

        // Do NOT unconditionally reschedule: the pump goes idle here and
        // waits for `mark_and_arm` to request the next frame. This is what
        // lets an idle tab actually sleep instead of pumping rAF forever.
    }) as Box<dyn FnMut()>));

    // Kick off the first frame through the shared arm path (commit only on
    // request success), matching `DirtyState::new_dirty()` in
    // `attach_canvas`: if this initial request fails the machine stays
    // dirty-but-unscheduled and the next mark retries.
    arm_pump(&shared.state);
}

/// Paint one frame from the current render state. Returns `true` on
/// success, `false` when the frame could not be painted and must stay
/// dirty (routed into the bounded failure handling by the pump).
///
/// Today every backend call in here is infallible from Rust's point of
/// view — a JS exception crossing a non-`catch` wasm-bindgen import aborts
/// the wasm instance and cannot be observed by this code — so the `false`
/// path is only reachable via the defensive borrow check. It exists so any
/// future fallible backend call has a correct, already-tested place to
/// report into (`DirtyState::record_paint_failure`).
fn paint_frame(shared: &RenderShared) -> bool {
    // The pump is the only mutable borrower and this is single-threaded, so
    // a conflict here is unexpected; treat it as a failed (retryable) paint
    // rather than falsely advancing `painted_gen` past unpainted content.
    let Ok(mut b) = shared.inner.try_borrow_mut() else {
        return false;
    };
    // Apply any pending push_scene / push_viewport_to_render payload
    // BEFORE reading state to paint (F3 fix). A push that landed while a
    // previous frame's paint held this same borrow could only queue into
    // `pending_scene` / `pending_viewport` (see those functions); draining
    // it here — at the very start of every paint attempt, unconditionally
    // — is what makes the dirty generation bumped at push time honest:
    // the generation is never consumed by a frame that painted stale
    // (or missing) payload.
    if let Some(pending_scene) = shared.pending_scene.take() {
        b.scene = pending_scene;
    }
    if let Some(pending_viewport) = shared.pending_viewport.take() {
        b.viewport = pending_viewport;
    }
    // No scene yet (document not loaded) — nothing to paint counts as
    // success: the generation is consumed, and `push_scene` re-arms when
    // content arrives, so the pump never polls for a missing scene.
    let Some(scene_rc) = b.scene.clone() else {
        return true;
    };
    // Rc clone (refcount bump), not a deep LayoutScene clone. Cloning the
    // handle — rather than reading `b.scene` directly — avoids holding an
    // aliasing borrow across the mutable `b.backend` paint calls below.
    let vp = b.viewport;
    let w = b.logical_w;
    let h = b.logical_h;
    b.backend.begin_frame();
    crate::viewer_host::paint_scene(&mut b.backend, &scene_rc, vp, Theme::dark(), w, h);
    b.backend.end_frame();
    true
}
