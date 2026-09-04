//! FFI-level tests for one-finger transcript scrolling inside the mobile AI
//! chat sheet: a body drag must scroll the conversation (unpinning from the
//! bottom), dragging back to the bottom must re-pin (so streamed content
//! auto-follows again), and a stationary tap must not scroll.

use super::Session;
use crate::desc::{Callbacks, CreateOptions};
use crate::{OpEngine, OpStatus};
use op_editor_core::size_class::MobileSheetKind;
use op_editor_core::ChatMessage;
use op_editor_ui::widgets::{host_canvas_geometry, AIChatPlaceholder};
use op_editor_ui::{Point2D, Rect};

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

const VIEWPORT_W: f32 = 390.0;
const VIEWPORT_H: f32 = 844.0;

/// A phone-sized editor engine showing the AI sheet over a long transcript.
fn phone_engine_with_chat_sheet() -> OpEngine {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: VIEWPORT_W,
            height: VIEWPORT_H,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let host = engine
        .session_mut_for_test()
        .editor_mut()
        .expect("editor host");
    let state = host.editor_state_mut();
    state.editor_ui.touch = true;
    assert!(
        state.editor_ui.size_class.is_compact(),
        "390x844 must resolve the compact size class"
    );
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Ai);
    for i in 0..12 {
        state.chat.messages.push(ChatMessage::user(format!(
            "question {i}: how should the hero section work on mobile?"
        )));
        state.chat.messages.push(ChatMessage::assistant(format!(
            "answer {i}: use a vertical layout with a fill-width hero,\n\
             a supporting subtitle, and a primary call to action.\n\
             Keep paddings at 16 and the type scale consistent."
        )));
    }
    engine
}

/// The compact-phone AI sheet rect — mirrors `ai_chat_rect`'s compact
/// branch (no keyboard, chat not focused).
fn sheet_rect(state: &op_editor_core::EditorState) -> Rect {
    let max_h = (VIEWPORT_H - host_canvas_geometry::touch_app_bar_height(state)).max(0.0);
    let min_h = 280.0_f32.min(max_h);
    let sheet_h = (VIEWPORT_H * 0.58).clamp(min_h, max_h);
    Rect {
        origin: Point2D::new(0.0, VIEWPORT_H - sheet_h),
        size: Point2D::new(VIEWPORT_W, sheet_h),
    }
}

struct TranscriptProbe {
    press: Point2D,
    max: f32,
}

fn transcript_probe(engine: &mut OpEngine) -> TranscriptProbe {
    let host = engine
        .session_mut_for_test()
        .editor_mut()
        .expect("editor host");
    let state = host.editor_state();
    let rect = sheet_rect(state);
    // Any non-UNOWNED owner satisfies the transcript cache's ownership
    // enforcement for this out-of-crate probe.
    let panel = AIChatPlaceholder::from_editor(state).owned_by(0x00C0_FFEE);
    let body = panel.body_rect(rect);
    let max = panel.transcript_scroll_max(rect);
    assert!(
        max > 100.0,
        "the fixture transcript must overflow the sheet body (max = {max})"
    );
    TranscriptProbe {
        press: Point2D::new(
            body.origin.x + body.size.x / 2.0,
            body.origin.y + body.size.y / 2.0,
        ),
        max,
    }
}

fn chat_scroll(engine: &mut OpEngine) -> (f32, bool) {
    let host = engine
        .session_mut_for_test()
        .editor_mut()
        .expect("editor host");
    let chat = &host.editor_state().chat;
    (chat.transcript_scroll.offset, chat.transcript_pinned)
}

#[test]
fn touch_drag_scrolls_transcript_and_repins_at_bottom() {
    let mut engine = phone_engine_with_chat_sheet();
    let engine_ptr = &mut engine as *mut OpEngine;
    let probe = transcript_probe(&mut engine);
    let (_, pinned) = chat_scroll(&mut engine);
    assert!(pinned, "a fresh transcript starts pinned to the bottom");

    // Finger down inside the transcript body, then drag DOWN (toward
    // earlier content). The press must defer — not paint a selection —
    // and crossing slop must promote to scrolling.
    assert_eq!(
        unsafe { crate::op_editor_press(engine_ptr, probe.press.x, probe.press.y) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, probe.press.x, probe.press.y + 80.0) },
        OpStatus::Ok
    );
    let (offset, pinned) = chat_scroll(&mut engine);
    assert!(
        !pinned,
        "dragging away from the bottom must unpin the transcript"
    );
    assert!(
        offset < probe.max - 40.0,
        "the drag must scroll toward earlier content (offset {offset}, max {})",
        probe.max
    );

    // Drag back UP well past the bottom: the offset clamps to max and the
    // transcript re-pins so streamed content auto-follows again.
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, probe.press.x, probe.press.y - 400.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_release(engine_ptr, probe.press.x, probe.press.y - 400.0) },
        OpStatus::Ok
    );
    let (offset, pinned) = chat_scroll(&mut engine);
    assert!(pinned, "reaching the bottom must re-pin the transcript");
    assert!((offset - probe.max).abs() < 0.6);
}

#[test]
fn stationary_tap_does_not_scroll_the_transcript() {
    let mut engine = phone_engine_with_chat_sheet();
    let engine_ptr = &mut engine as *mut OpEngine;
    let probe = transcript_probe(&mut engine);

    assert_eq!(
        unsafe { crate::op_editor_press(engine_ptr, probe.press.x, probe.press.y) },
        OpStatus::Ok
    );
    // Sub-slop jitter stays a tap.
    assert_eq!(
        unsafe { crate::op_editor_move(engine_ptr, probe.press.x + 2.0, probe.press.y + 2.0) },
        OpStatus::Ok
    );
    assert_eq!(
        unsafe { crate::op_editor_release(engine_ptr, probe.press.x, probe.press.y) },
        OpStatus::Ok
    );
    let (offset, pinned) = chat_scroll(&mut engine);
    assert!(pinned, "a tap must not unpin the transcript");
    assert_eq!(offset, 0.0, "a tap must not move the scroll offset");
}
