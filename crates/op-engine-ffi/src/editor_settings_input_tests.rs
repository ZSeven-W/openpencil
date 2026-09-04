//! Mobile-shell reproduction of the "API key cannot be deleted" report,
//! driven through the real FFI surface the iOS/Android shells call:
//! `op_editor_press` / `op_editor_release` for the tap on the API-key
//! field, `op_editor_text` for typed characters, and
//! `op_editor_key(KEY_BACKSPACE)` for the system keyboard's delete key.

use crate::desc::{Callbacks, CreateOptions};
use crate::editor::{op_editor_key, op_editor_text, KEY_BACKSPACE, KEY_DELETE};
use crate::editor_ime::{
    op_editor_ime_commit, op_editor_ime_preedit, op_editor_paste_text, op_editor_take_copy_text,
};
use crate::lifecycle::{OpEngine, Session};
use crate::OpStatus;
use crate::{op_editor_press, op_editor_release};
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

const PHONE_W: f32 = 390.0;
const PHONE_H: f32 = 844.0;

fn phone_engine() -> OpEngine {
    OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: PHONE_W,
            height: PHONE_H,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    )
}

fn settings_input_text(engine: &mut OpEngine) -> String {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .settings_input
        .text()
        .to_owned()
}

fn settings_focus(engine: &mut OpEngine) -> Option<SettingsFocus> {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .agent_settings
        .focus
}

/// Reproduce the report end to end: open the settings sheet, expand the
/// provider card, tap the API-key input, type, then delete every
/// character through the shell's backspace key path.
#[test]
fn mobile_api_key_field_typing_and_backspace_round_trip() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    // Open the settings modal with one saved provider and enter its edit
    // form (the card's edit press focuses DisplayName and expands it).
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let ui = host.editor_state_mut();
        ui.editor_ui.agent_settings_open = true;
        ui.editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("Provider", "sk-old", "model-0");
        ui.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::DisplayName,
        });
        op_editor_core::host_ui_transitions::set_settings_input_text(
            &mut ui.editor_ui,
            "Provider".into(),
            0,
        );
    }

    // Locate the API-key input in the expanded card exactly where paint
    // lays it out, then tap it through the real press/release FFI pair.
    let (px, py) = {
        let session = engine.session_mut_for_test();
        let (vw, vh) = session.editor_viewport();
        let host = session.editor_mut().unwrap();
        let restore = host.editor_state().editor_ui.agent_settings.focus;
        host.editor_state_mut().editor_ui.agent_settings.focus =
            Some(SettingsFocus::BuiltinAgent {
                index: 0,
                field: BuiltinAgentField::ApiKey,
            });
        // No keyboard occlusion in this fixture, so the panel rect matches
        // the host's keyboard-aware geometry exactly.
        let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_editor_at(
            host.editor_state(),
            0,
        );
        let panel_rect = panel.rect(vw, vh);
        let mut input = panel
            .focused_input_rect(panel_rect)
            .expect("expanded card exposes the api-key input");
        input.origin.y -= panel.effective_scroll(panel_rect);
        // Tap near the right edge: the tap-to-caret mapping then parks the
        // caret at the end, so the typed characters below append.
        let point = (
            input.origin.x + input.size.x - 10.0,
            input.origin.y + input.size.y / 2.0,
        );
        drop(panel);
        host.editor_state_mut().editor_ui.agent_settings.focus = restore;
        point
    };
    assert_eq!(unsafe { op_editor_press(pointer, px, py) }, OpStatus::Ok);
    assert_eq!(unsafe { op_editor_release(pointer, px, py) }, OpStatus::Ok);
    assert_eq!(
        settings_focus(&mut engine),
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }),
        "tap must focus the api-key input"
    );
    assert_eq!(settings_input_text(&mut engine), "sk-old");

    // Type through the shell text path.
    let typed = "xy";
    assert_eq!(
        unsafe { op_editor_text(pointer, typed.as_ptr(), typed.len()) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "sk-oldxy");

    // Delete through the shell key path — one char per backspace.
    for expected in ["sk-oldx", "sk-old", "sk-ol", "sk-o", "sk-", "sk", "s", ""] {
        assert_eq!(
            unsafe { op_editor_key(pointer, KEY_BACKSPACE) },
            OpStatus::Ok
        );
        assert_eq!(settings_input_text(&mut engine), expected);
    }

    // Close the settings modal; the emptied draft must commit as empty.
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        assert!(host.apply_toggle_agent_settings());
        assert_eq!(
            host.editor_state().editor_ui.agent_settings.builtin_agents[0].api_key,
            "",
            "an emptied api-key draft must commit as empty (the key is deletable)"
        );
    }
}

/// The iOS shell forwards a backspace that lands mid-text in its IME
/// conduit as `KEY_DELETE`. While a settings field is focused that key
/// must forward-delete in the field — it used to fall through the host
/// ladder and silently delete the selected canvas node behind the modal.
#[test]
fn mobile_key_delete_edits_settings_field_and_never_deletes_nodes() {
    use op_editor_core::PenNodeExt;

    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    let (node_count, first) = {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let children = host.editor_state().active_children();
        (children.len(), children[0].base().id.clone())
    };
    {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        let state = host.editor_state_mut();
        state.set_single_selection(op_editor_core::NodeId::new(first));
        state.editor_ui.agent_settings_open = true;
        state
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("Provider", "sk-old", "model-0");
        state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        });
        op_editor_core::host_ui_transitions::set_settings_input_text(
            &mut state.editor_ui,
            "sk-old".into(),
            0,
        );
        // Caret to the start so forward deletion has a visible effect.
        for _ in 0.."sk-old".len() {
            assert!(host.apply_settings_caret(false));
        }
    }

    assert_eq!(unsafe { op_editor_key(pointer, KEY_DELETE) }, OpStatus::Ok);
    assert_eq!(settings_input_text(&mut engine), "k-old");
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert_eq!(
        host.editor_state().active_children().len(),
        node_count,
        "KEY_DELETE in a settings field must never remove canvas nodes"
    );
}

/// Focus a settings field with `text` seeded into the shared draft, the
/// way the modal's edit press does it on device.
fn focus_settings_field(engine: &mut OpEngine, field: BuiltinAgentField, text: &str) {
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let state = host.editor_state_mut();
    state.editor_ui.agent_settings_open = true;
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Provider", "sk-old", "model-0");
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent { index: 0, field });
    op_editor_core::host_ui_transitions::set_settings_input_text(
        &mut state.editor_ui,
        text.into(),
        0,
    );
}

fn ime_preedit(pointer: *mut OpEngine, text: &str) {
    let sel = text.len();
    assert_eq!(
        unsafe { op_editor_ime_preedit(pointer, text.as_ptr(), text.len(), sel, sel) },
        OpStatus::Ok
    );
}

fn ime_commit(pointer: *mut OpEngine, text: &str) {
    assert_eq!(
        unsafe { op_editor_ime_commit(pointer, text.as_ptr(), text.len()) },
        OpStatus::Ok
    );
}

/// The exact iOS Chinese-IME sequence: preedit updates stream while the
/// user types pinyin, the chosen candidate lands as an IME commit, and a
/// following backspace key must delete exactly the committed character.
/// Green here means the engine's IME path into settings inputs is healthy
/// and the on-device "cannot delete" defect lives in the platform bridge.
#[test]
fn mobile_ime_preedit_commit_then_backspace_deletes_in_settings_field() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    focus_settings_field(&mut engine, BuiltinAgentField::DisplayName, "Provider");

    ime_preedit(pointer, "s");
    ime_preedit(pointer, "si");
    assert_eq!(
        settings_input_text(&mut engine),
        "Provider",
        "preedit must not mutate the settings draft"
    );

    ime_commit(pointer, "四");
    assert_eq!(settings_input_text(&mut engine), "Provider四");

    assert_eq!(
        unsafe { op_editor_key(pointer, KEY_BACKSPACE) },
        OpStatus::Ok
    );
    assert_eq!(
        settings_input_text(&mut engine),
        "Provider",
        "one backspace after an IME commit must delete exactly one char"
    );
}

/// Interleaved shell-text and IME-commit input into the API-key field —
/// the way a Chinese keyboard mixes its ASCII passthrough with candidate
/// commits — followed by a full backspace teardown.
#[test]
fn mobile_interleaved_text_and_ime_commit_backspace_teardown() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    focus_settings_field(&mut engine, BuiltinAgentField::ApiKey, "");

    let typed = "sk";
    assert_eq!(
        unsafe { op_editor_text(pointer, typed.as_ptr(), typed.len()) },
        OpStatus::Ok
    );
    ime_preedit(pointer, "si");
    ime_commit(pointer, "四");
    let typed = "x";
    assert_eq!(
        unsafe { op_editor_text(pointer, typed.as_ptr(), typed.len()) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "sk四x");

    for expected in ["sk四", "sk", "s", "", ""] {
        assert_eq!(
            unsafe { op_editor_key(pointer, KEY_BACKSPACE) },
            OpStatus::Ok
        );
        assert_eq!(settings_input_text(&mut engine), expected);
    }
}

/// Screen-space rect of the currently focused settings input, resolved
/// through the same keyboard-aware panel geometry the press ladder uses.
fn focused_input_rect(engine: &mut OpEngine) -> op_editor_ui::Rect {
    let session = engine.session_mut_for_test();
    let (vw, vh) = session.editor_viewport();
    let host = session.editor_mut().unwrap();
    let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_editor_at(
        host.editor_state(),
        0,
    );
    let panel_rect = panel.rect(vw, vh);
    let mut input = panel
        .focused_input_rect(panel_rect)
        .expect("focused settings input has a rect");
    input.origin.y -= panel.effective_scroll(panel_rect);
    input
}

fn settings_caret(engine: &mut OpEngine) -> usize {
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui
        .settings_input
        .caret()
}

/// The mobile defect: tapping inside a focused single-line settings field
/// always left the caret at the end. A tap near the field's left edge must
/// move the caret to the first glyph boundary, and a typed char must land
/// there — through the exact press/release FFI pair the shells call.
#[test]
fn mobile_tap_moves_caret_inside_focused_api_key_field() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    focus_settings_field(&mut engine, BuiltinAgentField::ApiKey, "sk-old");
    assert_eq!(settings_caret(&mut engine), "sk-old".len());

    let input = focused_input_rect(&mut engine);
    let (px, py) = (input.origin.x + 2.0, input.origin.y + input.size.y / 2.0);
    assert_eq!(unsafe { op_editor_press(pointer, px, py) }, OpStatus::Ok);
    assert_eq!(unsafe { op_editor_release(pointer, px, py) }, OpStatus::Ok);

    assert_eq!(
        settings_focus(&mut engine),
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }),
        "the tap must keep the api-key input focused"
    );
    assert_eq!(settings_input_text(&mut engine), "sk-old");
    assert_eq!(
        settings_caret(&mut engine),
        0,
        "a tap at the left edge must move the caret to the start"
    );

    // A typed char lands at the tapped caret, not at the end.
    let typed = "X";
    assert_eq!(
        unsafe { op_editor_text(pointer, typed.as_ptr(), typed.len()) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "Xsk-old");

    // A tap past the text puts the caret back at the end.
    let (px, py) = (
        input.origin.x + input.size.x - 2.0,
        input.origin.y + input.size.y / 2.0,
    );
    assert_eq!(unsafe { op_editor_press(pointer, px, py) }, OpStatus::Ok);
    assert_eq!(unsafe { op_editor_release(pointer, px, py) }, OpStatus::Ok);
    assert_eq!(settings_caret(&mut engine), "Xsk-old".len());
}

/// Long-press paste conduit: `op_editor_paste_text` inserts the platform
/// clipboard payload into the focused settings field as a unit, honoring
/// the single-line fields' control-character filtering.
#[test]
fn mobile_paste_text_lands_in_focused_settings_field() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    focus_settings_field(&mut engine, BuiltinAgentField::ApiKey, "sk");

    let pasted = "-abc";
    assert_eq!(
        unsafe { op_editor_paste_text(pointer, pasted.as_ptr(), pasted.len()) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "sk-abc");

    // Single-line fields strip newlines/control chars from a pasted blob.
    let pasted = "\nx\ty\n";
    assert_eq!(
        unsafe { op_editor_paste_text(pointer, pasted.as_ptr(), pasted.len()) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "sk-abcxy");
}

/// Paste with no focused input must be a no-op: node paste stays on the
/// `KEY_PASTE` path, so clipboard text can never mutate the canvas.
#[test]
fn mobile_paste_text_without_focus_is_a_noop() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    let node_count = {
        let host = engine.session_mut_for_test().editor_mut().unwrap();
        host.editor_state().active_children().len()
    };

    let pasted = "clipboard text";
    assert_eq!(
        unsafe { op_editor_paste_text(pointer, pasted.as_ptr(), pasted.len()) },
        OpStatus::Ok
    );

    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert_eq!(host.editor_state().active_children().len(), node_count);
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "");
    assert_eq!(host.editor_state().chat.input.text(), "");
}

/// Paste routes to the chat input when it owns the keyboard.
#[test]
fn mobile_paste_text_lands_in_focused_chat_input() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .chat
        .focused = true;

    let pasted = "hello from the clipboard";
    assert_eq!(
        unsafe { op_editor_paste_text(pointer, pasted.as_ptr(), pasted.len()) },
        OpStatus::Ok
    );
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert_eq!(host.editor_state().chat.input.text(), pasted);
}

/// Outbound clipboard bridge: an engine copy action (collab invite /
/// share address, MCP config, chat copy — all funnel through
/// `chat.queue_copy_text`) must surface through `op_editor_take_copy_text`
/// with the documented probe/copy/consume contract.
#[test]
fn mobile_copy_action_drains_through_take_copy_text() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;

    // Nothing pending: the per-frame probe reports NotReady + 0 length.
    let mut required = usize::MAX;
    assert_eq!(
        unsafe { op_editor_take_copy_text(pointer, std::ptr::null_mut(), 0, &mut required) },
        OpStatus::NotReady
    );
    assert_eq!(required, 0);

    // The collab panel's copy button (and every other engine copy action)
    // lands its payload through this exact queue call.
    let invite = "OP-1234-ABCD-INVITE";
    engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state_mut()
        .chat
        .queue_copy_text(invite);

    // Probe does not consume.
    let mut required = 0;
    assert_eq!(
        unsafe { op_editor_take_copy_text(pointer, std::ptr::null_mut(), 0, &mut required) },
        OpStatus::Ok
    );
    assert_eq!(required, invite.len());

    // A short buffer fails and must NOT consume the payload.
    let mut short = [0_u8; 4];
    assert_eq!(
        unsafe {
            op_editor_take_copy_text(pointer, short.as_mut_ptr(), short.len(), &mut required)
        },
        OpStatus::InvalidArg
    );

    // A complete copy returns the text and consumes it.
    let mut buffer = vec![0_u8; required];
    assert_eq!(
        unsafe {
            op_editor_take_copy_text(pointer, buffer.as_mut_ptr(), buffer.len(), &mut required)
        },
        OpStatus::Ok
    );
    assert_eq!(std::str::from_utf8(&buffer).unwrap(), invite);

    // Consumed: the next per-frame probe is NotReady again.
    assert_eq!(
        unsafe { op_editor_take_copy_text(pointer, std::ptr::null_mut(), 0, &mut required) },
        OpStatus::NotReady
    );
    assert_eq!(required, 0);
}

/// A cancelled composition arrives as an empty IME commit (iOS) or an
/// empty preedit (Android). Neither may disturb the draft, and backspace
/// must keep working afterwards.
#[test]
fn mobile_cancelled_composition_leaves_settings_backspace_healthy() {
    let mut engine = phone_engine();
    let pointer = &mut engine as *mut OpEngine;
    focus_settings_field(&mut engine, BuiltinAgentField::DisplayName, "Provider");

    ime_preedit(pointer, "si");
    ime_commit(pointer, "");
    assert_eq!(settings_input_text(&mut engine), "Provider");

    ime_preedit(pointer, "si");
    ime_preedit(pointer, "");
    assert_eq!(settings_input_text(&mut engine), "Provider");

    assert_eq!(
        unsafe { op_editor_key(pointer, KEY_BACKSPACE) },
        OpStatus::Ok
    );
    assert_eq!(settings_input_text(&mut engine), "Provide");
}
