//! `ShellEvent` — OP widget-facing primitive event enum (spec v19 §5.1).
//!
//! Per spec §1.2 (FROZEN 2026-05-04) shell-core must compile on
//! `wasm32-unknown-unknown` and remain platform-neutral on iOS / Android.
//! This module therefore declares **only OP types** — no winit, no Jian,
//! no GL — so the enum is visible everywhere widgets compile (mobile +
//! WASM included). The desktop mapper that lifts Jian `PointerEvent` into
//! `ShellEvent` lives in `openpencil-shell-native::event` (target-gated to
//! macOS / Linux / Windows; Step 1f extends to mobile).
//!
//! ## Spec invariants (§11 mobile-readiness)
//! - 6 variants: `PointerMove / PointerButton / MouseWheel / Touch / Window / Key`.
//! - `Touch` carries `TouchForce` (`Calibrated` mirrors winit::Force 1:1
//!   to avoid Step 1f mobile API break, plus `Normalized` for Android).
//! - Newtype id fields are `pub` (spec round 3 BLOCK-R3-4 fix) so callers
//!   in shell-native can construct them across crate boundaries.

use crate::render_backend::Point2D;

/// Stable identity for a single pointer (mouse/pen/stylus/trackpad cursor).
///
/// Widened to `u64` here so OP can ingest mappers from platforms (iOS, Web)
/// whose finger ids exceed Jian's `u32` `PointerId`. Desktop mapper widens
/// `jian_core::gesture::PointerId(u32)` → `PointerId(u64)` losslessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerId(pub u64);

/// Stable identity for a single touch/finger contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TouchId(pub u64);

/// Lifecycle of a touch contact (spec §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// Finger landed.
    Started,
    /// Finger moved while held.
    Moved,
    /// Finger lifted normally.
    Ended,
    /// System cancelled tracking (focus loss / iOS face-proximity /
    /// Android system-gesture intercept).
    Cancelled,
}

/// Pressure / force for a touch contact (mirrors `winit::event::Force`
/// 1:1 to avoid Step 1f mobile API break, per spec §11.3 invariant).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchForce {
    /// iOS 3D Touch / Apple Pencil. `force` is the raw force value;
    /// `max_possible_force` is the touch sensor's max; `altitude_angle`
    /// is the Pencil tilt angle in radians (π/2 = perpendicular).
    Calibrated {
        force: f64,
        max_possible_force: f64,
        altitude_angle: Option<f64>,
    },
    /// Android pressure (already normalized to [0.0, 1.0]).
    Normalized(f64),
}

/// Mouse buttons (spec §5.1; mirrors winit::event::MouseButton).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// Pressed/released state for buttons + keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}

/// Mouse wheel / two-finger trackpad scroll delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    /// Discrete scroll, in lines (mouse wheel notch).
    LineDelta { x: f32, y: f32 },
    /// Continuous scroll, in logical pixels (trackpad).
    PixelDelta(Point2D),
}

/// Modifier-key state at the moment an event was raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Cmd on macOS, Super/Win on Linux/Windows (mirrors Jian `Modifiers::CMD`).
    pub meta: bool,
}

/// Subset of keys OP currently surfaces (spec §5.1; expanded as widgets
/// need them in Step 1c+). Variant names follow winit::keyboard::KeyCode
/// for easy mapping. `Other(u32)` carries the raw scancode so
/// shell-native can pass through unmapped keys without losing them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // Digits
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    // Whitespace / control
    Space,
    Enter,
    Tab,
    Backspace,
    Escape,
    Delete,
    // Arrows
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    // Modifiers (released as standalone keys)
    Shift,
    Control,
    Alt,
    Meta,
    /// Anything else — raw scancode. Step 1c+ widgets that need a
    /// specific key add a named variant.
    Other(u32),
}

/// Window-level event kinds (spec §5.1; the desktop mapper synthesizes
/// these directly from winit `WindowEvent`, never via Jian).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowEventKind {
    /// Window content area resized (logical pixels via `inner_size`).
    Resized { width: u32, height: u32 },
    /// HiDPI scale factor changed (Retina toggle, monitor switch).
    ScaleFactorChanged(f64),
    /// User clicked the close button / hit Cmd-W / etc.
    CloseRequested,
    /// Focus gained (`true`) or lost (`false`).
    Focused(bool),
}

/// Widget-facing primitive event (spec v19 §5.1).
///
/// Layered on top of Jian `PointerEvent` (which only carries
/// pointer/touch primitives — pos/buttons-bitset/phase). Window / Key /
/// MouseWheel events do **not** route through Jian; they come straight
/// from winit on desktop. See [`crate::event`] module docs and spec
/// §5.1.1 for the full mapping contract.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellEvent {
    /// Pointer moved (cursor or pen) — no button state change.
    PointerMove {
        id: PointerId,
        pos: Point2D,
        modifiers: Modifiers,
    },
    /// One pointer button transitioned Pressed/Released.
    /// Multi-button transitions (e.g. mid-gesture press of an additional
    /// button) are emitted as one `PointerButton` per changed bit, plus
    /// a trailing `PointerMove` if Jian phase was `Move` — see spec
    /// §5.1.1 for the full diff contract.
    PointerButton {
        id: PointerId,
        button: MouseButton,
        state: ElementState,
        pos: Point2D,
        modifiers: Modifiers,
    },
    /// Mouse wheel / trackpad scroll.
    MouseWheel {
        delta: ScrollDelta,
        modifiers: Modifiers,
    },
    /// Touch contact lifecycle event (mobile-ready; Step 1f).
    Touch {
        id: TouchId,
        phase: TouchPhase,
        pos: Point2D,
        force: Option<TouchForce>,
    },
    /// Window-level event (resize, scale change, close, focus).
    Window { kind: WindowEventKind },
    /// Keyboard event.
    Key {
        key: KeyCode,
        state: ElementState,
        modifiers: Modifiers,
    },
}
