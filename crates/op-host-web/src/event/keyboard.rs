//! Pure W3C `KeyboardEvent` → `jian_core::gesture::KeyEvent` mapping.
//!
//! Phase C1 builds a KeyEvent from the W3C primitives:
//!   - `KeyboardEvent.key`         → `KeyValue::{Char|Named|Unidentified}`
//!   - `KeyboardEvent.code`        → `KeyCode::{KeyA..KeyZ|Digit0..|Named|Unknown}`
//!   - `KeyboardEvent.location`    → `KeyLocation::{Standard|Left|Right|Numpad}`
//!   - `KeyboardEvent.repeat`      → `KeyEvent.repeat`
//!   - `KeyboardEvent.isComposing` → `KeyEvent.is_composing`
//!   - up/down                     → `KeyState::{Pressed|Released}`
//!
//! Pure: no host-clock or DOM access. The C2 listener computes the
//! `Modifiers` bitset from `KeyboardEvent.{shiftKey, ctrlKey, …}` and
//! the `is_composing` bool from `KeyboardEvent.isComposing`, then
//! calls into here.

use op_editor_ui::{KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers, NamedKey};

/// Build a `KeyEvent` from the raw W3C primitives the C2 listener
/// extracts off `KeyboardEvent`. `pressed=true` for `keydown`,
/// `pressed=false` for `keyup`.
pub fn map_keyboard_parts(
    key: &str,
    code: &str,
    location: u32,
    repeat: bool,
    pressed: bool,
    modifiers: Modifiers,
    is_composing: bool,
) -> KeyEvent {
    KeyEvent {
        key: map_key_value(key),
        code: map_key_code(code),
        location: map_location(location),
        modifiers,
        state: if pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        },
        repeat,
        is_composing,
    }
}

fn map_key_value(key: &str) -> KeyValue {
    match key {
        "Enter" => KeyValue::Named(NamedKey::Enter),
        "Escape" => KeyValue::Named(NamedKey::Escape),
        "Tab" => KeyValue::Named(NamedKey::Tab),
        " " | "Spacebar" => KeyValue::Named(NamedKey::Space),
        "Backspace" => KeyValue::Named(NamedKey::Backspace),
        "Delete" => KeyValue::Named(NamedKey::Delete),
        "ArrowUp" => KeyValue::Named(NamedKey::ArrowUp),
        "ArrowDown" => KeyValue::Named(NamedKey::ArrowDown),
        "ArrowLeft" => KeyValue::Named(NamedKey::ArrowLeft),
        "ArrowRight" => KeyValue::Named(NamedKey::ArrowRight),
        // Navigation extensions (codex Phase C gate CONCERN):
        "Home" => KeyValue::Named(NamedKey::Home),
        "End" => KeyValue::Named(NamedKey::End),
        "PageUp" => KeyValue::Named(NamedKey::PageUp),
        "PageDown" => KeyValue::Named(NamedKey::PageDown),
        // Function keys F1-F12. jian's NamedKey covers them but
        // KeyCode does not — `map_key_code` falls through to
        // Unknown(code) for these on purpose.
        "F1" => KeyValue::Named(NamedKey::F1),
        "F2" => KeyValue::Named(NamedKey::F2),
        "F3" => KeyValue::Named(NamedKey::F3),
        "F4" => KeyValue::Named(NamedKey::F4),
        "F5" => KeyValue::Named(NamedKey::F5),
        "F6" => KeyValue::Named(NamedKey::F6),
        "F7" => KeyValue::Named(NamedKey::F7),
        "F8" => KeyValue::Named(NamedKey::F8),
        "F9" => KeyValue::Named(NamedKey::F9),
        "F10" => KeyValue::Named(NamedKey::F10),
        "F11" => KeyValue::Named(NamedKey::F11),
        "F12" => KeyValue::Named(NamedKey::F12),
        // Modifier keys themselves (W3C `key` is just "Shift" etc;
        // `code` is "ShiftLeft" / "ShiftRight" — handled in
        // `map_key_code`).
        "Shift" => KeyValue::Named(NamedKey::Shift),
        "Control" => KeyValue::Named(NamedKey::Control),
        "Alt" => KeyValue::Named(NamedKey::Alt),
        "Meta" => KeyValue::Named(NamedKey::Meta),
        "CapsLock" => KeyValue::Named(NamedKey::CapsLock),
        _ => {
            // `KeyValue::Char(char)` covers single-codepoint keys
            // (e.g. "a", "1", "你"). Anything else (multi-codepoint
            // dead-key sequences, app-defined chord names, etc.)
            // falls into `Unidentified` for the host to triage.
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyValue::Char(c),
                _ => KeyValue::Unidentified(key.to_string()),
            }
        }
    }
}

fn map_key_code(code: &str) -> KeyCode {
    match code {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Space,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        // Navigation extensions (codex Phase C gate CONCERN):
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        // Modifier physical codes — distinguish left vs right per
        // jian KeyCode. Note: "MetaLeft" / "MetaRight" cover the
        // Cmd key on macOS, the Win key on Windows, and the Super
        // key on Linux (per Jian Modifiers::CMD).
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "MetaLeft" => KeyCode::MetaLeft,
        "MetaRight" => KeyCode::MetaRight,
        // F1-F12 KeyCode variants don't exist in jian (NamedKey
        // covers them); fall to Unknown(code) for now. Phase D+
        // may upstream F-key codes if a widget needs them.
        _ => KeyCode::Unknown(code.to_string()),
    }
}

fn map_location(location: u32) -> KeyLocation {
    match location {
        1 => KeyLocation::Left,
        2 => KeyLocation::Right,
        3 => KeyLocation::Numpad,
        _ => KeyLocation::Standard,
    }
}
