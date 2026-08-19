//! Platform-free constants and mappings shared by every OHOS binding.
//!
//! Everything here is pure `std` so it compiles and is unit-tested on the
//! host triple — the OHOS bindings are compiled out on macOS/Linux, so this
//! module is where the behaviour that CAN be proven without a device lives:
//! shell-action codes, editor key codes, ArkUI → engine event translation,
//! and wheel intent classification.
//!
//! Every constant below is pinned against `op-engine-ffi/include/op_engine.h`
//! (engine side) or the OpenHarmony NDK headers named in each section (shell
//! side) by the tests at the bottom of this file plus
//! `crate::contract_tests`.

/// Shell-side dispatch-rejected status (never produced by the C ABI).
/// Mirrors `op_engine_jni::STATUS_CLOSING`.
pub const STATUS_CLOSING: i32 = -1;

// ---- OpShellAction (op_engine.h) ----------------------------------------

pub const SHELL_ACTION_NONE: i32 = 0;
pub const SHELL_ACTION_OPEN_DOCUMENT: i32 = 1;
pub const SHELL_ACTION_OPEN_LOGIN_WEBVIEW: i32 = 2;
pub const SHELL_ACTION_CLOSE_LOGIN_WEBVIEW: i32 = 3;
pub const SHELL_ACTION_EXPORT_DOCUMENT: i32 = 4;
pub const SHELL_ACTION_OPEN_ACCOUNT_CENTER: i32 = 5;
pub const SHELL_ACTION_REQUEST_LOGIN: i32 = 6;
pub const SHELL_ACTION_OPEN_LANGUAGE_PICKER: i32 = 7;
/// TopBar red dot — close the shell's window.
pub const SHELL_ACTION_WINDOW_CLOSE: i32 = 8;
/// TopBar yellow dot — minimise the shell's window.
pub const SHELL_ACTION_WINDOW_MINIMIZE: i32 = 9;
/// TopBar green dot — toggle maximised / restored.
pub const SHELL_ACTION_WINDOW_ZOOM: i32 = 10;

/// Every shell action the engine can hand the OHOS shell, in `OpShellAction`
/// order. `SHELL_ACTION_NONE` is deliberately absent — it means "no work".
pub const SHELL_ACTIONS: [i32; 10] = [
    SHELL_ACTION_OPEN_DOCUMENT,
    SHELL_ACTION_OPEN_LOGIN_WEBVIEW,
    SHELL_ACTION_CLOSE_LOGIN_WEBVIEW,
    SHELL_ACTION_EXPORT_DOCUMENT,
    SHELL_ACTION_OPEN_ACCOUNT_CENTER,
    SHELL_ACTION_REQUEST_LOGIN,
    SHELL_ACTION_OPEN_LANGUAGE_PICKER,
    SHELL_ACTION_WINDOW_CLOSE,
    SHELL_ACTION_WINDOW_MINIMIZE,
    SHELL_ACTION_WINDOW_ZOOM,
];

/// Whether `code` is an action the shell must handle. Negative values are
/// engine failures (`-(OpStatus)`) and [`STATUS_CLOSING`] is a dead handle;
/// both are rejected here so a caller cannot mistake them for work.
pub fn is_shell_action(code: i32) -> bool {
    SHELL_ACTIONS.contains(&code)
}

// ---- OpAuthRegion (op_engine.h) -----------------------------------------

pub const AUTH_REGION_CHINA: i32 = 0;
pub const AUTH_REGION_GLOBAL: i32 = 1;

/// Clamps a shell-supplied region code onto a real `OpAuthRegion`. An unknown
/// value falls back to China (the engine's own default deployment) rather
/// than being forwarded, so a typo cannot select an undefined SSO origin.
pub fn auth_region_or_default(region: i32) -> i32 {
    if region == AUTH_REGION_GLOBAL {
        AUTH_REGION_GLOBAL
    } else {
        AUTH_REGION_CHINA
    }
}

// ---- OpPointerPhase (op_engine.h) ---------------------------------------

pub const POINTER_PHASE_DOWN: i32 = 0;
pub const POINTER_PHASE_MOVE: i32 = 1;
pub const POINTER_PHASE_UP: i32 = 2;
pub const POINTER_PHASE_CANCEL: i32 = 3;

// ---- ArkUI TouchType / OH_NativeXComponent_TouchEventType ---------------
//
// Both the ArkTS `TouchType` enum and the NDK
// `OH_NativeXComponent_TouchEventType` (native_interface_xcomponent.h) use
// the SAME ordering, which is NOT the engine's `OpPointerPhase` ordering —
// Up and Move are swapped. Translating here (instead of in ArkTS) is why
// `touchEvent` exists next to the raw `pointer`.

pub const OH_TOUCH_DOWN: i32 = 0;
pub const OH_TOUCH_UP: i32 = 1;
pub const OH_TOUCH_MOVE: i32 = 2;
pub const OH_TOUCH_CANCEL: i32 = 3;
pub const OH_TOUCH_UNKNOWN: i32 = 4;

/// Maps an ArkUI/NDK touch type onto an `OpPointerPhase`. `None` for the
/// `UNKNOWN` type and any out-of-range value — the caller drops the event
/// instead of guessing a phase.
pub fn pointer_phase_from_touch_type(touch_type: i32) -> Option<i32> {
    match touch_type {
        OH_TOUCH_DOWN => Some(POINTER_PHASE_DOWN),
        OH_TOUCH_UP => Some(POINTER_PHASE_UP),
        OH_TOUCH_MOVE => Some(POINTER_PHASE_MOVE),
        OH_TOUCH_CANCEL => Some(POINTER_PHASE_CANCEL),
        _ => None,
    }
}

// ---- Mouse buttons ------------------------------------------------------
//
// `OH_NativeXComponent_MouseEventButton` (native_interface_xcomponent.h) is a
// BITMASK, while the ArkTS `MouseButton` enum is an ordinal. The shell passes
// the ArkTS ordinal; only Left / Right / Middle reach the editor.

pub const MOUSE_BUTTON_LEFT: i32 = 0;
pub const MOUSE_BUTTON_RIGHT: i32 = 1;
pub const MOUSE_BUTTON_MIDDLE: i32 = 2;

/// What a mouse button press means to the editor host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseIntent {
    /// Left button — the ordinary press/release gesture.
    Primary,
    /// Right button — the context-menu press. The editor has no right-release
    /// entry point, so the release half is dropped.
    Context,
    /// Middle / extra buttons — no editor binding; the event is swallowed.
    Ignored,
}

/// Classifies an ArkTS `MouseButton` ordinal.
pub fn mouse_intent(button: i32) -> MouseIntent {
    match button {
        MOUSE_BUTTON_LEFT => MouseIntent::Primary,
        MOUSE_BUTTON_RIGHT => MouseIntent::Context,
        _ => MouseIntent::Ignored,
    }
}

// ---- Wheel --------------------------------------------------------------

/// What a wheel / trackpad scroll does to the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelIntent {
    /// Two-axis canvas pan by these logical-pixel deltas.
    Pan { dx: f32, dy: f32 },
    /// Zoom about the cursor; positive zooms in (matches `op_editor_pinch`).
    Zoom { delta: f32 },
    /// Nothing to dispatch (a zero scroll).
    None,
}

/// Classifies one wheel event. `zoom_modifier` is the Ctrl/Cmd state — the
/// desktop host's rule, mirrored here so the OHOS 2-in-1 lane behaves like
/// the desktop editor. A zero delta never reaches the engine.
pub fn wheel_intent(dx: f32, dy: f32, zoom_modifier: bool) -> WheelIntent {
    if zoom_modifier {
        if dy == 0.0 {
            return WheelIntent::None;
        }
        return WheelIntent::Zoom { delta: dy };
    }
    if dx == 0.0 && dy == 0.0 {
        return WheelIntent::None;
    }
    WheelIntent::Pan { dx, dy }
}

// ---- Editor key codes (op_engine.h `OpKey_*`) ---------------------------

pub const OP_KEY_BACKSPACE: i32 = 1;
pub const OP_KEY_DELETE: i32 = 2;
pub const OP_KEY_ENTER: i32 = 3;
pub const OP_KEY_ESCAPE: i32 = 4;
pub const OP_KEY_DUPLICATE: i32 = 5;
pub const OP_KEY_UNDO: i32 = 6;
pub const OP_KEY_REDO: i32 = 7;
pub const OP_KEY_ARROW_UP: i32 = 9;
pub const OP_KEY_ARROW_DOWN: i32 = 10;
pub const OP_KEY_ARROW_LEFT: i32 = 11;
pub const OP_KEY_ARROW_RIGHT: i32 = 12;
pub const OP_KEY_SELECT_ALL: i32 = 13;
pub const OP_KEY_COPY: i32 = 14;
pub const OP_KEY_CUT: i32 = 15;
pub const OP_KEY_PASTE: i32 = 16;
pub const OP_KEY_GROUP: i32 = 17;
pub const OP_KEY_UNGROUP: i32 = 18;
pub const OP_KEY_REORDER_BACK: i32 = 19;
pub const OP_KEY_REORDER_FORWARD: i32 = 20;
pub const OP_KEY_ARROW_UP_BIG: i32 = 21;
pub const OP_KEY_ARROW_DOWN_BIG: i32 = 22;
pub const OP_KEY_ARROW_LEFT_BIG: i32 = 23;
pub const OP_KEY_ARROW_RIGHT_BIG: i32 = 24;

// ---- Modifier bitmask (this crate's contract with ArkTS) ----------------

pub const MOD_SHIFT: i32 = 1 << 0;
pub const MOD_CTRL: i32 = 1 << 1;
pub const MOD_ALT: i32 = 1 << 2;
pub const MOD_META: i32 = 1 << 3;

/// The "command" modifier: HarmonyOS phones/tablets send Ctrl from an
/// attached keyboard while 2-in-1 desktops may send Meta, so either counts.
fn command(modifiers: i32) -> bool {
    modifiers & (MOD_CTRL | MOD_META) != 0
}

// ---- OHOS key codes (`oh_key_code.h` / `@ohos.multimodalInput.keyCode`) --

pub const KEYCODE_0: i32 = 2000;
pub const KEYCODE_9: i32 = 2009;
pub const KEYCODE_DPAD_UP: i32 = 2012;
pub const KEYCODE_DPAD_DOWN: i32 = 2013;
pub const KEYCODE_DPAD_LEFT: i32 = 2014;
pub const KEYCODE_DPAD_RIGHT: i32 = 2015;
pub const KEYCODE_A: i32 = 2017;
pub const KEYCODE_C: i32 = 2019;
pub const KEYCODE_D: i32 = 2020;
pub const KEYCODE_G: i32 = 2023;
pub const KEYCODE_V: i32 = 2038;
pub const KEYCODE_X: i32 = 2040;
pub const KEYCODE_LEFT_BRACKET: i32 = 2059;
pub const KEYCODE_RIGHT_BRACKET: i32 = 2060;
pub const KEYCODE_Y: i32 = 2041;
pub const KEYCODE_Z: i32 = 2042;
pub const KEYCODE_COMMA: i32 = 2043;
pub const KEYCODE_PERIOD: i32 = 2044;
pub const KEYCODE_SPACE: i32 = 2050;
pub const KEYCODE_ENTER: i32 = 2054;
pub const KEYCODE_DEL: i32 = 2055;
pub const KEYCODE_GRAVE: i32 = 2056;
pub const KEYCODE_MINUS: i32 = 2057;
pub const KEYCODE_EQUALS: i32 = 2058;
pub const KEYCODE_BACKSLASH: i32 = 2061;
pub const KEYCODE_SEMICOLON: i32 = 2062;
pub const KEYCODE_APOSTROPHE: i32 = 2063;
pub const KEYCODE_SLASH: i32 = 2064;
pub const KEYCODE_ESCAPE: i32 = 2070;
pub const KEYCODE_FORWARD_DEL: i32 = 2071;
pub const KEYCODE_NUMPAD_0: i32 = 2100;
pub const KEYCODE_NUMPAD_9: i32 = 2109;
pub const KEYCODE_NUMPAD_DIVIDE: i32 = 2110;
pub const KEYCODE_NUMPAD_MULTIPLY: i32 = 2111;
pub const KEYCODE_NUMPAD_SUBTRACT: i32 = 2112;
pub const KEYCODE_NUMPAD_ADD: i32 = 2113;
pub const KEYCODE_NUMPAD_DOT: i32 = 2114;
pub const KEYCODE_NUMPAD_ENTER: i32 = 2119;

/// Maps an OHOS key code + modifier bitmask onto an `OpKey_*` value, or
/// `None` when the key carries no editor meaning (printable characters go
/// through `editorText` instead, exactly like the Android player).
///
/// The shortcut set matches the desktop host: Cmd/Ctrl+D duplicates,
/// Cmd/Ctrl+Z undoes, Cmd/Ctrl+Shift+Z and Cmd/Ctrl+Y redo.
/// Shift promotes an arrow nudge to the 10-px step (desktop parity).
fn shifted(modifiers: i32, plain: i32, big: i32) -> i32 {
    if modifiers & MOD_SHIFT != 0 {
        big
    } else {
        plain
    }
}

pub fn map_key(key_code: i32, modifiers: i32) -> Option<i32> {
    match key_code {
        KEYCODE_DEL => Some(OP_KEY_BACKSPACE),
        KEYCODE_FORWARD_DEL => Some(OP_KEY_DELETE),
        KEYCODE_ENTER | KEYCODE_NUMPAD_ENTER => Some(OP_KEY_ENTER),
        KEYCODE_ESCAPE => Some(OP_KEY_ESCAPE),
        KEYCODE_DPAD_UP => Some(shifted(modifiers, OP_KEY_ARROW_UP, OP_KEY_ARROW_UP_BIG)),
        KEYCODE_DPAD_DOWN => Some(shifted(modifiers, OP_KEY_ARROW_DOWN, OP_KEY_ARROW_DOWN_BIG)),
        KEYCODE_DPAD_LEFT => Some(shifted(modifiers, OP_KEY_ARROW_LEFT, OP_KEY_ARROW_LEFT_BIG)),
        KEYCODE_DPAD_RIGHT => Some(shifted(
            modifiers,
            OP_KEY_ARROW_RIGHT,
            OP_KEY_ARROW_RIGHT_BIG,
        )),
        KEYCODE_A if command(modifiers) => Some(OP_KEY_SELECT_ALL),
        KEYCODE_C if command(modifiers) => Some(OP_KEY_COPY),
        KEYCODE_X if command(modifiers) => Some(OP_KEY_CUT),
        KEYCODE_V if command(modifiers) => Some(OP_KEY_PASTE),
        KEYCODE_G if command(modifiers) => Some(if modifiers & MOD_SHIFT != 0 {
            OP_KEY_UNGROUP
        } else {
            OP_KEY_GROUP
        }),
        KEYCODE_LEFT_BRACKET => Some(OP_KEY_REORDER_BACK),
        KEYCODE_RIGHT_BRACKET => Some(OP_KEY_REORDER_FORWARD),
        KEYCODE_D if command(modifiers) => Some(OP_KEY_DUPLICATE),
        KEYCODE_Y if command(modifiers) => Some(OP_KEY_REDO),
        KEYCODE_Z if command(modifiers) => Some(if modifiers & MOD_SHIFT != 0 {
            OP_KEY_REDO
        } else {
            OP_KEY_UNDO
        }),
        _ => None,
    }
}

/// Maps an OHOS key code + modifier bitmask onto the character a US-QWERTY
/// keyboard would produce, or `None` when the key has no printable form.
///
/// WHY THIS EXISTS: a SURFACE-type XComponent consumes hardware keys on the
/// NATIVE channel, so on desktop-class (2in1) HarmonyOS the system IME never
/// sees a physical keystroke and therefore never emits `insertText`. Without
/// this table an engine-drawn text input (chat box, property field, canvas
/// text) can be focused but never typed into. Phones and tablets keep the
/// IME conduit — see [`crate::xcomponent`] for the routing rule.
///
/// LIMITATION: this is a US-QWERTY table with no dead keys and no composition,
/// so a CJK or other composing IME still has to go through the conduit. The
/// engine's own preedit ABI (`op_editor_ime_preedit`) is unaffected.
///
/// A Ctrl/Meta chord is never text — those belong to [`map_key`] or the
/// system, so this returns `None` for them.
pub fn printable_char(key_code: i32, modifiers: i32) -> Option<char> {
    if modifiers & (MOD_CTRL | MOD_META) != 0 {
        return None;
    }
    let shift = modifiers & MOD_SHIFT != 0;
    if (KEYCODE_A..=KEYCODE_Z).contains(&key_code) {
        let offset = (key_code - KEYCODE_A) as u8;
        let base = if shift { b'A' } else { b'a' };
        return Some((base + offset) as char);
    }
    if (KEYCODE_0..=KEYCODE_9).contains(&key_code) {
        let digit = (key_code - KEYCODE_0) as usize;
        // US-QWERTY shifted number row, indexed by the digit itself.
        const SHIFTED: [char; 10] = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];
        return Some(if shift {
            SHIFTED[digit]
        } else {
            (b'0' + digit as u8) as char
        });
    }
    // The numeric keypad is layout-independent and ignores Shift.
    if (KEYCODE_NUMPAD_0..=KEYCODE_NUMPAD_9).contains(&key_code) {
        return Some((b'0' + (key_code - KEYCODE_NUMPAD_0) as u8) as char);
    }
    let (plain, shifted) = match key_code {
        KEYCODE_SPACE => (' ', ' '),
        KEYCODE_COMMA => (',', '<'),
        KEYCODE_PERIOD => ('.', '>'),
        KEYCODE_GRAVE => ('`', '~'),
        KEYCODE_MINUS => ('-', '_'),
        KEYCODE_EQUALS => ('=', '+'),
        KEYCODE_LEFT_BRACKET => ('[', '{'),
        KEYCODE_RIGHT_BRACKET => (']', '}'),
        KEYCODE_BACKSLASH => ('\\', '|'),
        KEYCODE_SEMICOLON => (';', ':'),
        KEYCODE_APOSTROPHE => ('\'', '"'),
        KEYCODE_SLASH => ('/', '?'),
        KEYCODE_NUMPAD_DIVIDE => ('/', '/'),
        KEYCODE_NUMPAD_MULTIPLY => ('*', '*'),
        KEYCODE_NUMPAD_SUBTRACT => ('-', '-'),
        KEYCODE_NUMPAD_ADD => ('+', '+'),
        KEYCODE_NUMPAD_DOT => ('.', '.'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_action_codes_reject_failures_and_none() {
        for action in SHELL_ACTIONS {
            assert!(is_shell_action(action), "action {action} must be handled");
        }
        assert!(!is_shell_action(SHELL_ACTION_NONE));
        assert!(!is_shell_action(STATUS_CLOSING));
        assert!(!is_shell_action(-6)); // -(OpStatus::WrongThread)
        assert!(!is_shell_action(11)); // past the last defined action
    }

    #[test]
    fn unknown_auth_regions_fall_back_to_china() {
        assert_eq!(auth_region_or_default(AUTH_REGION_CHINA), AUTH_REGION_CHINA);
        assert_eq!(
            auth_region_or_default(AUTH_REGION_GLOBAL),
            AUTH_REGION_GLOBAL
        );
        assert_eq!(auth_region_or_default(7), AUTH_REGION_CHINA);
        assert_eq!(auth_region_or_default(-1), AUTH_REGION_CHINA);
    }

    #[test]
    fn touch_types_translate_with_up_and_move_swapped() {
        assert_eq!(
            pointer_phase_from_touch_type(OH_TOUCH_DOWN),
            Some(POINTER_PHASE_DOWN)
        );
        // The whole reason this mapping exists: ArkUI Up is 1, engine Up is 2.
        assert_eq!(
            pointer_phase_from_touch_type(OH_TOUCH_UP),
            Some(POINTER_PHASE_UP)
        );
        assert_eq!(
            pointer_phase_from_touch_type(OH_TOUCH_MOVE),
            Some(POINTER_PHASE_MOVE)
        );
        assert_eq!(
            pointer_phase_from_touch_type(OH_TOUCH_CANCEL),
            Some(POINTER_PHASE_CANCEL)
        );
        assert_eq!(pointer_phase_from_touch_type(OH_TOUCH_UNKNOWN), None);
        assert_eq!(pointer_phase_from_touch_type(-1), None);
    }

    #[test]
    fn mouse_buttons_map_to_press_context_and_ignored() {
        assert_eq!(mouse_intent(MOUSE_BUTTON_LEFT), MouseIntent::Primary);
        assert_eq!(mouse_intent(MOUSE_BUTTON_RIGHT), MouseIntent::Context);
        assert_eq!(mouse_intent(MOUSE_BUTTON_MIDDLE), MouseIntent::Ignored);
        assert_eq!(mouse_intent(9), MouseIntent::Ignored);
    }

    #[test]
    fn wheel_zoom_needs_the_modifier_and_a_vertical_delta() {
        assert_eq!(
            wheel_intent(0.0, -3.0, true),
            WheelIntent::Zoom { delta: -3.0 }
        );
        assert_eq!(wheel_intent(4.0, 0.0, true), WheelIntent::None);
        assert_eq!(
            wheel_intent(4.0, -3.0, false),
            WheelIntent::Pan { dx: 4.0, dy: -3.0 }
        );
        assert_eq!(wheel_intent(0.0, 0.0, false), WheelIntent::None);
    }

    #[test]
    fn plain_keys_need_no_modifier_and_shortcuts_do() {
        assert_eq!(map_key(KEYCODE_DEL, 0), Some(OP_KEY_BACKSPACE));
        assert_eq!(map_key(KEYCODE_FORWARD_DEL, 0), Some(OP_KEY_DELETE));
        assert_eq!(map_key(KEYCODE_ENTER, 0), Some(OP_KEY_ENTER));
        assert_eq!(map_key(KEYCODE_NUMPAD_ENTER, 0), Some(OP_KEY_ENTER));
        assert_eq!(map_key(KEYCODE_ESCAPE, 0), Some(OP_KEY_ESCAPE));
        assert_eq!(map_key(KEYCODE_DPAD_UP, 0), Some(OP_KEY_ARROW_UP));
        assert_eq!(map_key(KEYCODE_DPAD_DOWN, 0), Some(OP_KEY_ARROW_DOWN));
        assert_eq!(map_key(KEYCODE_DPAD_LEFT, 0), Some(OP_KEY_ARROW_LEFT));
        assert_eq!(map_key(KEYCODE_DPAD_RIGHT, 0), Some(OP_KEY_ARROW_RIGHT));

        // Bare letters are printable text, not editor keys.
        assert_eq!(map_key(KEYCODE_D, 0), None);
        assert_eq!(map_key(KEYCODE_Z, MOD_SHIFT), None);
    }

    #[test]
    fn undo_redo_and_duplicate_accept_ctrl_or_meta() {
        for command in [MOD_CTRL, MOD_META] {
            assert_eq!(map_key(KEYCODE_D, command), Some(OP_KEY_DUPLICATE));
            assert_eq!(map_key(KEYCODE_Z, command), Some(OP_KEY_UNDO));
            assert_eq!(
                map_key(KEYCODE_Z, command | MOD_SHIFT),
                Some(OP_KEY_REDO),
                "shift+cmd+z redoes"
            );
            assert_eq!(map_key(KEYCODE_Y, command), Some(OP_KEY_REDO));
        }
        // Alt alone is not the command modifier.
        assert_eq!(map_key(KEYCODE_Z, MOD_ALT), None);
    }

    #[test]
    fn printable_letters_follow_shift() {
        assert_eq!(printable_char(KEYCODE_A, 0), Some('a'));
        assert_eq!(printable_char(KEYCODE_A, MOD_SHIFT), Some('A'));
        assert_eq!(printable_char(KEYCODE_Z, 0), Some('z'));
        assert_eq!(printable_char(KEYCODE_Z, MOD_SHIFT), Some('Z'));
    }

    #[test]
    fn printable_digits_and_punctuation_use_the_us_layout() {
        assert_eq!(printable_char(KEYCODE_0, 0), Some('0'));
        assert_eq!(printable_char(KEYCODE_0, MOD_SHIFT), Some(')'));
        assert_eq!(printable_char(KEYCODE_9, 0), Some('9'));
        assert_eq!(printable_char(KEYCODE_9, MOD_SHIFT), Some('('));
        assert_eq!(printable_char(KEYCODE_SPACE, 0), Some(' '));
        assert_eq!(printable_char(KEYCODE_SPACE, MOD_SHIFT), Some(' '));
        assert_eq!(printable_char(KEYCODE_SLASH, MOD_SHIFT), Some('?'));
        assert_eq!(printable_char(KEYCODE_LEFT_BRACKET, 0), Some('['));
        assert_eq!(printable_char(KEYCODE_LEFT_BRACKET, MOD_SHIFT), Some('{'));
        // The keypad is layout-independent: Shift never changes it.
        assert_eq!(printable_char(KEYCODE_NUMPAD_0, MOD_SHIFT), Some('0'));
        assert_eq!(printable_char(KEYCODE_NUMPAD_DOT, MOD_SHIFT), Some('.'));
    }

    #[test]
    fn command_chords_and_named_keys_are_never_text() {
        // The whole point of the ctrl/meta guard: Cmd+A must stay select-all.
        assert_eq!(printable_char(KEYCODE_A, MOD_META), None);
        assert_eq!(printable_char(KEYCODE_A, MOD_CTRL), None);
        assert_eq!(printable_char(KEYCODE_A, MOD_CTRL | MOD_SHIFT), None);
        for code in [
            KEYCODE_ENTER,
            KEYCODE_NUMPAD_ENTER,
            KEYCODE_DEL,
            KEYCODE_FORWARD_DEL,
            KEYCODE_ESCAPE,
            KEYCODE_DPAD_UP,
            KEYCODE_DPAD_DOWN,
            KEYCODE_DPAD_LEFT,
            KEYCODE_DPAD_RIGHT,
        ] {
            assert_eq!(printable_char(code, 0), None, "key {code} is not text");
            assert!(map_key(code, 0).is_some(), "key {code} needs an editor arm");
        }
    }
}
