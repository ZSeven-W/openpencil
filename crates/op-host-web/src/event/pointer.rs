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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelIntent {
    Zoom { delta_y: f32 },
    Pan { dx: f32, dy: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MousePressAction {
    PrimaryPress,
    MiddlePan,
    ContextPress,
    Ignore,
}

pub fn classify_mouse_press_button(button: i16) -> MousePressAction {
    match button {
        0 => MousePressAction::PrimaryPress,
        1 => MousePressAction::MiddlePan,
        2 => MousePressAction::ContextPress,
        _ => MousePressAction::Ignore,
    }
}

/// Convert a browser event's canvas-local offset into the logical
/// coordinate space the Rust widget host uses.
///
/// `MouseEvent.offset{X,Y}` is reported in the element's displayed CSS
/// pixels, while the editor host lays out chrome against CanvasKit's
/// logical viewport. Those values are usually equal, but they can drift
/// when the canvas backing store is DPR-capped or the browser has a
/// stale/layout-rounded CSS size. Scaling here keeps button hit-testing
/// aligned with what is painted.
pub fn map_offset_to_logical(
    offset_x: f32,
    offset_y: f32,
    css_w: f32,
    css_h: f32,
    logical_w: f32,
    logical_h: f32,
) -> (f32, f32) {
    let scale_x = if css_w.is_finite() && css_w > 0.0 && logical_w.is_finite() {
        logical_w / css_w
    } else {
        1.0
    };
    let scale_y = if css_h.is_finite() && css_h > 0.0 && logical_h.is_finite() {
        logical_h / css_h
    } else {
        1.0
    };
    (offset_x * scale_x, offset_y * scale_y)
}

/// Classify a browser wheel into the editor action the web shell should run.
///
/// Vertical mouse-wheel scroll keeps the existing zoom behavior. Horizontal
/// trackpad deltas and Shift+wheel are pan gestures, matching desktop's
/// `apply_pan_gesture` path. Cmd/Ctrl/Alt modified wheels are reserved for
/// zoom so browser pinch/Cmd-wheel never turns into a pan.
pub fn classify_wheel_intent(
    delta_x: f32,
    delta_y: f32,
    shift: bool,
    ctrl: bool,
    meta: bool,
    alt: bool,
) -> WheelIntent {
    let modified_zoom = ctrl || meta || alt;
    if !modified_zoom && (delta_x != 0.0 || shift) {
        if delta_x != 0.0 {
            WheelIntent::Pan {
                dx: -delta_x,
                dy: -delta_y,
            }
        } else {
            WheelIntent::Pan {
                dx: -delta_y,
                dy: 0.0,
            }
        }
    } else {
        WheelIntent::Zoom { delta_y: -delta_y }
    }
}

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
