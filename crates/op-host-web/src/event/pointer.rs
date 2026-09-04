//! Pure W3C `WheelEvent` → `jian_core::gesture::WheelEvent` mapping.
//!
//! Sign convention (spec §2.4 + §3.5): Jian's internal axis is
//! winit-positive-up (positive deltaY = scroll content up). Browser
//! W3C `WheelEvent.deltaY` is positive-down. The mapper flips the
//! sign on `delta_y` so widget code reads the Jian-internal
//! convention regardless of host.
//!
//! Pure: no host-clock reads here (would panic on
//! wasm32-unknown-unknown). The C2 listener reads the host clock
//! (`web_time::Instant` or any polyfill) and passes the millisecond
//! value in as `t_ms` — matching `jian_core::gesture::WheelEvent`'s own
//! `t_ms: u64` convention (shared with `PointerEvent`). This keeps the
//! mapper unit-testable on native and IT does NOT crash at runtime on
//! wasm32-unknown-unknown — the panic locus moves up to the listener
//! glue, where the polyfill lives.
//!
//! `PointerEvent` mapping (mouse position / button / kind / phase)
//! lives in C2 alongside the listener registration — Phase C1 covers
//! only the wheel mapper because that's the cross-axis-translation
//! piece that benefits from being a pure helper.

use op_editor_ui::{Modifiers, ScrollMode, WheelEvent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelIntent {
    Zoom {
        /// Normalized pixel delta used by panels under the cursor.
        scroll_delta_y: f32,
        /// Device-normalized and sensitivity-adjusted canvas zoom delta.
        canvas_delta_y: f32,
    },
    Pan {
        dx: f32,
        dy: f32,
    },
}

const WHEEL_LINE_PX: f32 = 40.0;
const REGULAR_ZOOM_GAIN: f32 = 1.35;
const MODIFIED_ZOOM_GAIN: f32 = 4.0;
/// Caps one browser event to roughly a 30% viewport scale change.
const MAX_CANVAS_ZOOM_DELTA: f32 = 175.0;

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
    delta_mode: u32,
    page_width: f32,
    page_height: f32,
    modifiers: Modifiers,
) -> WheelIntent {
    let modified_zoom = modifiers.intersects(Modifiers::CTRL | Modifiers::CMD | Modifiers::ALT);
    let horizontal_delta_from_y = normalize_wheel_delta(delta_y, delta_mode, page_width);
    let delta_x = normalize_wheel_delta(delta_x, delta_mode, page_width);
    let delta_y = normalize_wheel_delta(delta_y, delta_mode, page_height);
    if !modified_zoom && (delta_x != 0.0 || modifiers.contains(Modifiers::SHIFT)) {
        if delta_x != 0.0 {
            WheelIntent::Pan {
                dx: -delta_x,
                dy: -delta_y,
            }
        } else {
            WheelIntent::Pan {
                dx: -horizontal_delta_from_y,
                dy: 0.0,
            }
        }
    } else {
        let scroll_delta_y = -delta_y;
        let gain = if modified_zoom {
            MODIFIED_ZOOM_GAIN
        } else {
            REGULAR_ZOOM_GAIN
        };
        WheelIntent::Zoom {
            scroll_delta_y,
            canvas_delta_y: (scroll_delta_y * gain)
                .clamp(-MAX_CANVAS_ZOOM_DELTA, MAX_CANVAS_ZOOM_DELTA),
        }
    }
}

/// Convert W3C wheel units to CSS-like pixels before routing the gesture.
///
/// Browsers report pixels, lines, or pages depending on the input device.
/// Treating all three as pixels made line-mode wheels and trackpad pinch feel
/// dramatically slower than pixel-mode mouse wheels.
pub fn normalize_wheel_delta(delta: f32, delta_mode: u32, page_extent: f32) -> f32 {
    if !delta.is_finite() {
        return 0.0;
    }
    let multiplier = match delta_mode {
        1 => WHEEL_LINE_PX,
        2 if page_extent.is_finite() && page_extent > 0.0 => page_extent,
        _ => 1.0,
    };
    let normalized = delta * multiplier;
    if normalized.is_finite() {
        normalized
    } else {
        0.0
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
/// 1=Line, 2=Page). `t_ms` is whatever the C2 listener captured from
/// its host clock polyfill, in milliseconds.
pub fn map_wheel(
    position: jian_core::geometry::Point,
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
    delta_mode: u32,
    modifiers: Modifiers,
    t_ms: u64,
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
        t_ms,
    }
}
