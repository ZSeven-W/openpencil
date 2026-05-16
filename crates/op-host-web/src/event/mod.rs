//! Step 1b §2.4 + §3.5 DOM event mapping. Pure functions that turn
//! browser-side primitives (`KeyboardEvent.key` / `code`,
//! `CompositionEvent.data`, `WheelEvent.deltaY` / `deltaMode`,
//! `FocusEvent`) into Jian gesture types
//! (`jian_core::gesture::{KeyEvent, ImeEvent, FocusEvent, WheelEvent}`).
//!
//! Pure means: no `Instant::now()` calls (those panic on
//! wasm32-unknown-unknown), no DOM access, no JS interop. The browser
//! listener wiring (Phase C2) reads `event::*::map_*` outputs and
//! enriches them with the runtime timestamp it captures from
//! `web_time::Instant` / `js_sys::Date::now()` / similar host clock.

pub mod focus;
pub mod ime;
pub mod keyboard;
pub mod pointer;
