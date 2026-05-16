//! Pure W3C `WheelEvent` → `jian_core::gesture::WheelEvent` mapping.
//!
//! Sign convention (spec §2.4 + §3.5): Jian's internal axis is
//! winit-positive-up (positive deltaY = scroll content up). Browser
//! W3C `WheelEvent.deltaY` is positive-down. The mapper flips the
//! sign on `delta_y` so widget code reads the Jian-internal
//! convention regardless of host.
//!
//! Pure: no `Instant::now()` (panics on wasm32-unknown-unknown). The
//! C2 listener reads the host clock via `web_time::Instant` (or any
//! polyfill that yields a `std::time::Instant`) and passes it in.
//! This keeps the mapper unit-testable on native and IT does NOT
//! crash at runtime on wasm32-unknown-unknown — the panic locus
//! moves up to the listener glue, where the polyfill lives.
//!
//! `PointerEvent` mapping (mouse position / button / kind / phase)
//! lives in C2 alongside the listener registration — Phase C1 covers
//! only the wheel mapper because that's the cross-axis-translation
//! piece that benefits from being a pure helper.

use op_editor_ui::{Modifiers, ScrollMode, WheelEvent};
use std::time::Instant;

/// Build a Jian `WheelEvent` from the W3C primitives.
///
/// `position` is the cursor's canvas-local position (already
/// transformed from `WheelEvent.client{X,Y}` minus the canvas
/// bounding rect by the caller). `delta_x` / `delta_y` come straight
/// from `WheelEvent.delta{X,Y}` in W3C sign (positive-down for Y);
/// we flip Y here. `delta_z` mirrors `WheelEvent.deltaZ` (almost
/// always 0). `delta_mode` is `WheelEvent.deltaMode` (0=Pixel,
/// 1=Line, 2=Page). `timestamp` is whatever the C2 listener
/// captured from its host clock polyfill.
pub fn map_wheel(
    position: jian_core::geometry::Point,
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
    delta_mode: u32,
    modifiers: Modifiers,
    timestamp: Instant,
) -> WheelEvent {
    WheelEvent {
        position,
        // W3C → Jian sign flip on Y. `delta_x` is positive-right on
        // both sides (W3C and winit) so no flip there; only the Y
        // axis differs (W3C positive-down vs Jian/winit positive-up).
        delta: jian_core::geometry::Point::new(delta_x, -delta_y),
        delta_z,
        mode: match delta_mode {
            1 => ScrollMode::Line,
            2 => ScrollMode::Page,
            _ => ScrollMode::Pixel,
        },
        modifiers,
        timestamp,
    }
}
