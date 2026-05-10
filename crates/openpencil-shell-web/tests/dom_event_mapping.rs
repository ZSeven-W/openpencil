//! Phase C1 DOM event mapper tests.
//!
//! Pure-function coverage — runs natively (no wasm-bindgen-test); the
//! C2 listener integration test runs in browsers as part of Phase E
//! manual smoke. The Instant-using tests rely on `Instant::now()`
//! being available natively; on wasm32-unknown-unknown that call
//! panics, which is exactly why the mapper itself takes `timestamp`
//! as a parameter (the C2 listener supplies a polyfill).

use openpencil_shell_core::{
    ImeKind, KeyCode, KeyLocation, KeyState, KeyValue, Modifiers, NamedKey, ScrollMode,
};
use openpencil_shell_web::event::{
    focus::map_focus,
    ime::{composition_end, composition_start, composition_update, utf16_selection_to_utf8},
    keyboard::map_keyboard_parts,
    pointer::map_wheel,
};
use std::time::Instant;

// ---------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------

#[test]
fn keyboard_mapping_preserves_key_code_location_and_repeat() {
    let event = map_keyboard_parts("a", "KeyA", 0, true, true, Modifiers::SHIFT, false);
    assert_eq!(event.key, KeyValue::Char('a'));
    assert_eq!(event.code, KeyCode::KeyA);
    assert_eq!(event.location, KeyLocation::Standard);
    assert_eq!(event.state, KeyState::Pressed);
    assert!(event.repeat);
    assert!(!event.is_composing);
    assert!(event.modifiers.contains(Modifiers::SHIFT));
}

#[test]
fn named_key_mapping_handles_escape() {
    let event = map_keyboard_parts(
        "Escape",
        "Escape",
        0,
        false,
        false,
        Modifiers::empty(),
        false,
    );
    assert_eq!(event.key, KeyValue::Named(NamedKey::Escape));
    assert_eq!(event.code, KeyCode::Escape);
    assert_eq!(event.state, KeyState::Released);
    assert!(!event.repeat);
}

#[test]
fn keyboard_location_maps_left_right_numpad() {
    let l = map_keyboard_parts(
        "Shift",
        "ShiftLeft",
        1,
        false,
        true,
        Modifiers::empty(),
        false,
    );
    assert_eq!(l.location, KeyLocation::Left);
    let r = map_keyboard_parts(
        "Shift",
        "ShiftRight",
        2,
        false,
        true,
        Modifiers::empty(),
        false,
    );
    assert_eq!(r.location, KeyLocation::Right);
    let n = map_keyboard_parts("1", "Numpad1", 3, false, true, Modifiers::empty(), false);
    assert_eq!(n.location, KeyLocation::Numpad);
}

#[test]
fn keyboard_is_composing_propagates() {
    let composing = map_keyboard_parts(
        "a",
        "KeyA",
        0,
        false,
        true,
        Modifiers::empty(),
        true, // is_composing
    );
    assert!(composing.is_composing);
}

#[test]
fn keyboard_unidentified_for_multi_char_keys() {
    // Multi-codepoint key value (e.g. some IME pre-edit handle) falls
    // through to Unidentified; widget code can decide what to do.
    let event = map_keyboard_parts(
        "Compose",
        "Pause",
        0,
        false,
        true,
        Modifiers::empty(),
        false,
    );
    assert_eq!(event.key, KeyValue::Unidentified("Compose".to_string()));
    // `Pause` isn't in the table, so the mapper falls back to Unknown.
    assert_eq!(event.code, KeyCode::Unknown("Pause".to_string()));
}

#[test]
fn keyboard_navigation_keys_mapped() {
    // Codex Phase C gate Round 3 BLOCK: every NamedKey/KeyCode variant
    // jian exposes must round-trip through the mapper rather than
    // falling to Unidentified / Unknown(...).
    for (k, expected) in [
        ("Home", NamedKey::Home),
        ("End", NamedKey::End),
        ("PageUp", NamedKey::PageUp),
        ("PageDown", NamedKey::PageDown),
        ("CapsLock", NamedKey::CapsLock),
    ] {
        let event = map_keyboard_parts(k, k, 0, false, true, Modifiers::empty(), false);
        assert_eq!(event.key, KeyValue::Named(expected), "key {k}");
    }
}

#[test]
fn keyboard_function_keys_mapped() {
    use openpencil_shell_core::NamedKey::*;
    for (k, expected) in [
        ("F1", F1),
        ("F2", F2),
        ("F3", F3),
        ("F4", F4),
        ("F5", F5),
        ("F6", F6),
        ("F7", F7),
        ("F8", F8),
        ("F9", F9),
        ("F10", F10),
        ("F11", F11),
        ("F12", F12),
    ] {
        let event = map_keyboard_parts(k, k, 0, false, true, Modifiers::empty(), false);
        assert_eq!(event.key, KeyValue::Named(expected), "fn-key {k}");
    }
}

#[test]
fn keyboard_modifier_keys_mapped_to_named_values() {
    // W3C `key` for modifier keys themselves is just the bare name;
    // `code` carries left/right ("ShiftLeft" etc) which map_key_code
    // covers. Verify both halves round-trip.
    for (k, c, expected_key, expected_code) in [
        ("Shift", "ShiftLeft", NamedKey::Shift, KeyCode::ShiftLeft),
        ("Shift", "ShiftRight", NamedKey::Shift, KeyCode::ShiftRight),
        (
            "Control",
            "ControlLeft",
            NamedKey::Control,
            KeyCode::ControlLeft,
        ),
        (
            "Control",
            "ControlRight",
            NamedKey::Control,
            KeyCode::ControlRight,
        ),
        ("Alt", "AltLeft", NamedKey::Alt, KeyCode::AltLeft),
        ("Alt", "AltRight", NamedKey::Alt, KeyCode::AltRight),
        ("Meta", "MetaLeft", NamedKey::Meta, KeyCode::MetaLeft),
        ("Meta", "MetaRight", NamedKey::Meta, KeyCode::MetaRight),
    ] {
        let event = map_keyboard_parts(k, c, 0, false, true, Modifiers::empty(), false);
        assert_eq!(event.key, KeyValue::Named(expected_key), "mod key {k}");
        assert_eq!(event.code, expected_code, "mod code {c}");
    }
}

// ---------------------------------------------------------------------
// IME
// ---------------------------------------------------------------------

#[test]
fn composition_start_carries_empty_text() {
    let start = composition_start();
    assert_eq!(start.kind, ImeKind::CompositionStart);
    assert_eq!(start.text, "");
}

#[test]
fn composition_update_remaps_utf16_selection_to_utf8_for_cjk() {
    // `你好` = 2 CJK chars; each is 1 UTF-16 code unit + 3 UTF-8 bytes.
    // Browser-supplied selection 0..2 (UTF-16 code units) must remap
    // to 0..6 (UTF-8 bytes).
    let update = composition_update("你好".to_string(), Some(0..2));
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(0..6)),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
}

#[test]
fn composition_update_remaps_surrogate_pair() {
    // 🙂 = U+1F642 = 2 UTF-16 code units (surrogate pair) + 4 UTF-8 bytes.
    // Selection 0..2 (UTF-16) must remap to 0..4 (UTF-8).
    let update = composition_update("🙂".to_string(), Some(0..2));
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(0..4)),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
}

#[test]
fn composition_update_handles_partial_selection() {
    // `aé` = 1 ASCII char (1 UTF-16 / 1 UTF-8) + 1 Latin-1 char (1 UTF-16 / 2 UTF-8).
    // Selection 1..2 (UTF-16) covers just the `é` — must remap to 1..3 (UTF-8).
    let update = composition_update("aé".to_string(), Some(1..2));
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(1..3)),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
}

#[test]
fn composition_update_no_selection_passthrough() {
    let update = composition_update("hello".to_string(), None);
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, None),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
    assert_eq!(update.text, "hello");
}

#[test]
fn composition_update_misordered_selection_returns_none() {
    let update = composition_update("hello".to_string(), Some(3..1));
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, None),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
}

#[test]
fn composition_end_carries_commit_text() {
    let end = composition_end("你好".to_string());
    assert_eq!(end.kind, ImeKind::CompositionEnd);
    assert_eq!(end.text, "你好");
}

#[test]
fn utf16_helper_clamps_out_of_range_to_str_len() {
    // Selection past the end of the string falls back to text.len().
    let s = utf16_selection_to_utf8("abc", Some(0..100));
    assert_eq!(s, Some(0..3));
}

#[test]
fn utf16_helper_handles_empty_text_and_zero_length_selection() {
    // Empty text + 0..0 selection — both bounds clamp to 0 (= text.len()).
    let s = utf16_selection_to_utf8("", Some(0..0));
    assert_eq!(s, Some(0..0));

    // Zero-length selection inside non-empty text at offset 1.
    let mid = utf16_selection_to_utf8("abc", Some(1..1));
    assert_eq!(mid, Some(1..1));
}

#[test]
fn composition_update_zero_length_selection_in_cjk() {
    // `你好` — caret at UTF-16 offset 1 (between 你 and 好) maps to
    // UTF-8 byte 3.
    let update = composition_update("你好".to_string(), Some(1..1));
    match update.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(3..3)),
        other => panic!("expected CompositionUpdate, got {other:?}"),
    }
}

#[test]
fn keyboard_empty_string_key_falls_to_unidentified() {
    // The empty string can show up on certain dead-key sequences in
    // some browsers; Phase C1 routes it to `Unidentified("")` rather
    // than panicking.
    let event = map_keyboard_parts("", "", 0, false, true, Modifiers::empty(), false);
    assert_eq!(event.key, KeyValue::Unidentified(String::new()));
    assert_eq!(event.code, KeyCode::Unknown(String::new()));
}

// ---------------------------------------------------------------------
// Wheel (pointer.rs)
// ---------------------------------------------------------------------

#[test]
fn wheel_flips_w3c_deltay_sign() {
    // W3C scroll-down (+120) → Jian winit-positive-up (-120).
    let pos = jian_core::geometry::Point::new(0.0, 0.0);
    let event = map_wheel(pos, 0.0, 120.0, 0.0, 0, Modifiers::empty(), Instant::now());
    assert_eq!(event.delta.y, -120.0);
    assert_eq!(event.delta.x, 0.0);
    assert_eq!(event.mode, ScrollMode::Pixel);
}

#[test]
fn wheel_does_not_flip_x_sign() {
    // delta_x is positive-right on both W3C and winit — no flip.
    let pos = jian_core::geometry::Point::new(10.0, 20.0);
    let event = map_wheel(pos, 5.0, 0.0, 0.0, 0, Modifiers::empty(), Instant::now());
    assert_eq!(event.delta.x, 5.0);
}

#[test]
fn wheel_mode_decoding() {
    let pos = jian_core::geometry::Point::new(0.0, 0.0);
    let pixel = map_wheel(pos, 0.0, 1.0, 0.0, 0, Modifiers::empty(), Instant::now());
    assert_eq!(pixel.mode, ScrollMode::Pixel);
    let line = map_wheel(pos, 0.0, 1.0, 0.0, 1, Modifiers::empty(), Instant::now());
    assert_eq!(line.mode, ScrollMode::Line);
    let page = map_wheel(pos, 0.0, 1.0, 0.0, 2, Modifiers::empty(), Instant::now());
    assert_eq!(page.mode, ScrollMode::Page);
}

#[test]
fn wheel_delta_z_passthrough() {
    let pos = jian_core::geometry::Point::new(0.0, 0.0);
    let event = map_wheel(pos, 0.0, 0.0, 1.5, 0, Modifiers::empty(), Instant::now());
    assert_eq!(event.delta_z, 1.5);
}

// ---------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------

#[test]
fn focus_carries_node_hints() {
    let event = map_focus(true, Some(42), Some(43));
    assert!(event.gained);
    assert_eq!(event.node_id_hint, Some(42));
    assert_eq!(event.related_node_id_hint, Some(43));
}

#[test]
fn focus_loss_with_no_related_target() {
    // Window-level blur — no relatedTarget, no node hint.
    let event = map_focus(false, None, None);
    assert!(!event.gained);
    assert_eq!(event.node_id_hint, None);
    assert_eq!(event.related_node_id_hint, None);
}
