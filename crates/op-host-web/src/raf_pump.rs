// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
//! A `requestAnimationFrame` loop that ticks while work is in flight.
//!
//! Used to drain streamed AI deltas into the transcript ~once per frame (the
//! native shell wakes its winit loop ~30 fps for the same purpose). The loop is
//! self-terminating: when `tick` returns `false` the closure drops itself,
//! releasing its wasm-bindgen closure slot back to the linear-memory pool —
//! unlike a `forget()`-leaked interval, an idle pump leaves nothing alive.
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Shared slot owning the self-rescheduling rAF closure. `Option` so the closure
/// can drop itself out of the slot when the pump stops; `Rc<RefCell<…>>` so the
/// closure body can hold a clone of the slot that owns it.
type FrameSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Schedule `tick` on each animation frame until it returns `false`.
///
/// Standard self-referential rAF idiom: the scheduled closure holds an `Rc` to
/// the same slot that owns it, so it can re-schedule itself for the next frame;
/// when `tick()` returns `false` it takes (and drops) itself out of the slot,
/// freeing the closure. `tick` is `Rc<dyn Fn() -> bool>` so the caller can share
/// the same drain function the rest of the shell holds.
pub fn start(tick: Rc<dyn Fn() -> bool>) {
    let holder: FrameSlot = Rc::new(RefCell::new(None));
    let holder2 = holder.clone();

    *holder.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if tick() {
            // Re-arm for the next frame. The borrow is released before
            // `request_frame` returns, so re-entrancy on a synchronous rAF
            // callback (browsers don't do this, but be safe) can't deadlock.
            if let Some(c) = holder2.borrow().as_ref() {
                request_frame(c);
            }
        } else {
            // Drop the closure to free its slot. Deferred `take` (we're inside
            // the closure being dropped) is fine because the rAF callback has
            // already been handed back to the JS side for this firing.
            let _ = holder2.borrow_mut().take();
        }
    }) as Box<dyn FnMut()>));

    // Kick off the first frame. Bind the `Ref` to a named local inside this
    // block so its temporary is dropped at the block's `}` — before `holder` /
    // `holder2` go out of scope. A trailing `if let ... { }` expression would
    // instead extend the `Ref` temporary to the end of `start()`, after the Rcs
    // are dropped (E0597).
    {
        let slot = holder.borrow();
        if let Some(c) = slot.as_ref() {
            request_frame(c);
        }
    }
}

/// Schedule `c` to run on the next animation frame. Best-effort: if `window` is
/// unavailable (e.g. a WebWorker) the pump simply doesn't run.
fn request_frame(c: &Closure<dyn FnMut()>) {
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(c.as_ref().unchecked_ref());
    }
}
