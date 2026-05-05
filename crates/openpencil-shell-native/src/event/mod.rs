//! Desktop event mapping (spec v19 §5.1.1).
//!
//! `winit::event::WindowEvent` →
//!   ([`PointerTranslator`] from `jian_host_desktop`) →
//!   `jian_core::gesture::PointerEvent` →
//!   ([`JianPointerMapper`]) →
//!   `openpencil_shell_core::ShellEvent`.
//!
//! Window / Key / MouseWheel events do **not** go through Jian — they
//! map straight from winit. Pointer / Touch primitives go through Jian
//! so OP reuses Jian's button-set + cursor-cache state machine
//! (`jian-host-desktop/src/pointer.rs`).
//!
//! This module is **desktop-only** (target-gated in `lib.rs`). Mobile
//! pointer/touch mapping lands in Step 1f.

use std::collections::HashMap;

use jian_core::gesture::{
    Modifiers as JianModifiers, MouseButtons as JianMouseButtons, PointerEvent as JianPointerEvent,
    PointerId as JianPointerId, PointerKind as JianPointerKind, PointerPhase as JianPointerPhase,
};
use openpencil_shell_core::event::{
    ElementState, Modifiers, MouseButton, PointerId, ShellEvent, TouchForce, TouchId, TouchPhase,
};
use openpencil_shell_core::render_backend::Point2D;

/// Stateful mapper from Jian `PointerEvent` to OP `ShellEvent` (spec
/// §5.1.1). Owns a per-`PointerId` snapshot of the last-seen
/// `MouseButtons` bitset so it can diff button transitions across
/// `Down / Up / Move` phases — Jian carries the **current** bitset, not
/// added/removed deltas (see `jian-core/src/gesture/pointer.rs:48-59`
/// + `jian-host-desktop/src/pointer.rs:104-134` for why all three
///   phases must diff).
#[derive(Debug, Clone, Default)]
pub struct JianPointerMapper {
    previous_buttons: HashMap<JianPointerId, JianMouseButtons>,
}

impl JianPointerMapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Translate one Jian `PointerEvent` into zero, one, or many
    /// `ShellEvent`s. Empty `Vec` = degraded input the caller should
    /// ignore (no `ShellEvent::Other` variant exists; see spec §5.1).
    ///
    /// Multi-button transitions during `Move` produce a `PointerButton`
    /// per changed bit followed by a trailing `PointerMove` (spec
    /// §5.1.1 round 3 CONCERN-R3-1 fix).
    pub fn from_jian_pointer(&mut self, p: &JianPointerEvent) -> Vec<ShellEvent> {
        let mut out = Vec::new();
        match p.kind {
            JianPointerKind::Touch => {
                let phase = match p.phase {
                    JianPointerPhase::Down => TouchPhase::Started,
                    JianPointerPhase::Move => TouchPhase::Moved,
                    JianPointerPhase::Up => TouchPhase::Ended,
                    JianPointerPhase::Cancel => TouchPhase::Cancelled,
                    // Touches never `Hover`: return empty so callers
                    // ignore the event rather than fabricate a phase.
                    JianPointerPhase::Hover => return out,
                };
                out.push(ShellEvent::Touch {
                    id: TouchId(u64::from(p.id.0)),
                    phase,
                    pos: jian_point_to_point2d(p.position),
                    force: Some(TouchForce::Normalized(f64::from(p.pressure))),
                });
            }
            JianPointerKind::Mouse
            | JianPointerKind::Pen
            | JianPointerKind::Stylus
            | JianPointerKind::Trackpad => {
                let prev = self
                    .previous_buttons
                    .get(&p.id)
                    .copied()
                    .unwrap_or_default();
                let added = p.buttons.difference(prev);
                let removed = prev.difference(p.buttons);
                self.previous_buttons.insert(p.id, p.buttons);

                let pos = jian_point_to_point2d(p.position);
                let modifiers = from_jian_modifiers(p.modifiers);
                let id = PointerId(u64::from(p.id.0));

                // 1. Emit one `PointerButton{Pressed}` per added bit.
                for bit in added.iter() {
                    if let Some(button) = mouse_buttons_bit_to_button(bit) {
                        out.push(ShellEvent::PointerButton {
                            id,
                            button,
                            state: ElementState::Pressed,
                            pos,
                            modifiers,
                        });
                    }
                }
                // 2. Emit one `PointerButton{Released}` per removed bit.
                for bit in removed.iter() {
                    if let Some(button) = mouse_buttons_bit_to_button(bit) {
                        out.push(ShellEvent::PointerButton {
                            id,
                            button,
                            state: ElementState::Released,
                            pos,
                            modifiers,
                        });
                    }
                }
                // 3. Hover / Move add a trailing `PointerMove` so motion
                //    is preserved alongside the button transitions.
                //    Down / Up don't emit Move (the PointerButton event
                //    carries the current pos). Cancel doesn't appear
                //    here — Jian only raises `Cancel` on Touch, which
                //    was branched out above; an unexpected
                //    `Mouse|Pen|Stylus|Trackpad + Cancel` is treated as
                //    degraded and ignored.
                match p.phase {
                    JianPointerPhase::Hover | JianPointerPhase::Move => {
                        out.push(ShellEvent::PointerMove { id, pos, modifiers });
                    }
                    JianPointerPhase::Down | JianPointerPhase::Up | JianPointerPhase::Cancel => {}
                }
            }
        }
        out
    }
}

/// Convert one `JianMouseButtons` single-bit flag to OP `MouseButton`.
/// Returns `None` if the input has zero or more than one bit set —
/// callers iterate the bitset one bit at a time so `None` should never
/// happen in practice; treating it as a no-op keeps the mapper
/// degradation contract intact (empty `Vec` rather than panic).
fn mouse_buttons_bit_to_button(bit: JianMouseButtons) -> Option<MouseButton> {
    match bit {
        JianMouseButtons::LEFT => Some(MouseButton::Left),
        JianMouseButtons::RIGHT => Some(MouseButton::Right),
        JianMouseButtons::MIDDLE => Some(MouseButton::Middle),
        JianMouseButtons::BACK => Some(MouseButton::Back),
        JianMouseButtons::FORWARD => Some(MouseButton::Forward),
        _ => None,
    }
}

/// Convert Jian `Modifiers` bitflags to OP `Modifiers` struct.
/// Jian's `CMD` bit maps to OP's `meta` (Cmd on macOS, Super/Win
/// elsewhere — same convention as `winit::keyboard::ModifiersState::super_key`).
fn from_jian_modifiers(m: JianModifiers) -> Modifiers {
    Modifiers {
        shift: m.contains(JianModifiers::SHIFT),
        ctrl: m.contains(JianModifiers::CTRL),
        alt: m.contains(JianModifiers::ALT),
        meta: m.contains(JianModifiers::CMD),
    }
}

fn jian_point_to_point2d(p: jian_core::geometry::Point) -> Point2D {
    Point2D::new(p.x, p.y)
}
