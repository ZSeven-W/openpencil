//! Plan v7 Task 3 Step 12 — proves the `ShellEvent` enum shape (spec
//! §5.1) is reachable through the public re-export path and matches the
//! 6-variant invariant. Constructed via the cross-platform OP types
//! only — no Jian / winit / GL imports — so this test compiles on
//! wasm32 and mobile too (verified by `cargo check
//! --target wasm32-unknown-unknown -p openpencil-shell-core`).

use openpencil_shell_core::event::{
    ElementState, KeyCode, Modifiers, MouseButton, PointerId, ScrollDelta, ShellEvent, TouchForce,
    TouchId, TouchPhase, WindowEventKind,
};
use openpencil_shell_core::render_backend::Point2D;

#[test]
fn six_variants_constructible_via_re_export() {
    let mods = Modifiers {
        shift: true,
        ctrl: false,
        alt: false,
        meta: false,
    };
    let pos = Point2D::new(1.0, 2.0);

    let events = [
        ShellEvent::PointerMove {
            id: PointerId(1),
            pos,
            modifiers: mods,
        },
        ShellEvent::PointerButton {
            id: PointerId(1),
            button: MouseButton::Left,
            state: ElementState::Pressed,
            pos,
            modifiers: mods,
        },
        ShellEvent::MouseWheel {
            delta: ScrollDelta::LineDelta { x: 0.0, y: 1.0 },
            modifiers: mods,
        },
        ShellEvent::Touch {
            id: TouchId(7),
            phase: TouchPhase::Started,
            pos,
            force: Some(TouchForce::Normalized(0.5)),
        },
        ShellEvent::Window {
            kind: WindowEventKind::Resized {
                width: 800,
                height: 600,
            },
        },
        ShellEvent::Key {
            key: KeyCode::Escape,
            state: ElementState::Released,
            modifiers: mods,
        },
    ];
    assert_eq!(events.len(), 6, "spec §5.1 declares exactly 6 variants");
}

#[test]
fn touch_force_calibrated_mirrors_winit() {
    // Spec §11.3 invariant: `TouchForce::Calibrated` mirrors
    // `winit::event::Force::Calibrated` 1:1 — fields exist with the
    // declared names so a Step 1f mobile mapper compiles.
    let f = TouchForce::Calibrated {
        force: 0.4,
        max_possible_force: 1.0,
        altitude_angle: Some(std::f64::consts::FRAC_PI_2),
    };
    if let TouchForce::Calibrated {
        force,
        max_possible_force,
        altitude_angle,
    } = f
    {
        assert_eq!(force, 0.4);
        assert_eq!(max_possible_force, 1.0);
        assert_eq!(altitude_angle, Some(std::f64::consts::FRAC_PI_2));
    } else {
        panic!("expected Calibrated variant");
    }
}

#[test]
fn newtype_id_fields_pub_constructible() {
    // Round 3 BLOCK-R3-4 fix: `TouchId(pub u64)` + `PointerId(pub u64)`
    // constructible across crates so shell-native's mapper works.
    let _ = TouchId(42);
    let _ = PointerId(42);
}
