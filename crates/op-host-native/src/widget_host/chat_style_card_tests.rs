//! Host wiring for the pinned-style chip's hover card: a real cursor move
//! onto the chip starts the dwell clock, the dwell's expiry reaches the wake
//! scheduler, and every path that takes the cursor away — leaving the chip,
//! leaving the panel, an overlay claiming the point — stops the clock.
//!
//! The clock is the whole mechanism, so a break in any of those reads as
//! either a card that never appears or one that will not go away.

use super::*;
use op_editor_ui::widgets::ai_chat_style_card::STYLE_CARD_DWELL_MS;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1400.0;
const VIEWPORT_H: f32 = 900.0;

const IMPORTED: &str = "\
---
name: Dimension
---

## Overview

A dark reference system built around one violet accent.

## Tokens — Colors

| Name | Value | Token | Role |
| --- | --- | --- | --- |
| Void Canvas | `#0a0a0a` | `--color-void-canvas` | Primary page background |
| Bone | `#ededed` | `--color-bone` | Primary readable text |
";

/// A host with a live pin, so the chip row paints and can be pointed at.
fn hosted_with_pin() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let imported =
        op_ai_skills::style_guide::import_design_md(IMPORTED, "dimension.md").expect("imports");
    host.editor_state_mut().editor_ui.pinned_style_guide = Some(imported.id.clone());
    host
}

/// Centre of the pinned-style chip, in window coordinates.
fn chip_centre(host: &WidgetHostNative) -> Point2D {
    let rect = host
        .ai_chat_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("the chat panel is on screen");
    let chip = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .style_chip_rect(rect)
        .expect("a live pin paints a chip");
    Point2D::new(
        chip.origin.x + chip.size.x / 2.0,
        chip.origin.y + chip.size.y / 2.0,
    )
}

#[test]
fn pointing_at_the_chip_starts_the_dwell_clock_but_shows_nothing_yet() {
    let mut host = hosted_with_pin();
    host.set_now_ms(1_000);
    let point = chip_centre(&host);
    host.apply_cursor_move(point.x, point.y);

    assert_eq!(
        host.editor_state().editor_ui.chat_style_chip_hover_since_ms,
        Some(1_000)
    );
    assert!(
        !AIChatPlaceholder::from_editor_at(host.editor_state(), 1_000).style_card_showing(),
        "a card the instant the cursor lands would flash on every pass-through"
    );
    assert!(
        AIChatPlaceholder::from_editor_at(host.editor_state(), 1_000 + STYLE_CARD_DWELL_MS)
            .style_card_showing(),
        "and it must appear once the cursor has actually rested"
    );
}

#[test]
fn a_dwelling_card_asks_the_scheduler_to_wake_for_it() {
    let mut host = hosted_with_pin();
    host.set_now_ms(1_000);
    let point = chip_centre(&host);
    host.apply_cursor_move(point.x, point.y);

    assert_eq!(
        host.next_animation_deadline_ms(),
        Some(1_000 + STYLE_CARD_DWELL_MS),
        "the dwell's expiry has to reach the wake scheduler"
    );
}

#[test]
fn moving_off_the_chip_stops_the_clock() {
    let mut host = hosted_with_pin();
    host.set_now_ms(1_000);
    let point = chip_centre(&host);
    host.apply_cursor_move(point.x, point.y);
    assert!(host
        .editor_state()
        .editor_ui
        .chat_style_chip_hover_since_ms
        .is_some());

    // Into the canvas, well clear of the panel.
    host.apply_cursor_move(200.0, 300.0);
    assert_eq!(
        host.editor_state().editor_ui.chat_style_chip_hover_since_ms,
        None
    );
    assert!(
        !AIChatPlaceholder::from_editor_at(host.editor_state(), 1_000 + STYLE_CARD_DWELL_MS)
            .style_card_showing()
    );
}

/// Sliding down into the text area is a move off the chip like any other. It
/// is worth its own case because that is the direction the cursor actually
/// travels — the chip sits directly above where a person is going to type.
#[test]
fn sliding_down_into_the_input_stops_the_clock() {
    let mut host = hosted_with_pin();
    host.set_now_ms(1_000);
    let chip = chip_centre(&host);
    host.apply_cursor_move(chip.x, chip.y);

    host.apply_cursor_move(chip.x, chip.y + 24.0);
    assert_eq!(
        host.editor_state().editor_ui.chat_style_chip_hover_since_ms,
        None
    );
}

/// A surface painted over the chat takes the cursor, and every hover the chat
/// owned goes with it — including this clock, or the card would hang over the
/// modal that displaced it.
#[test]
fn a_clear_path_that_drops_chat_hover_drops_the_clock_too() {
    let mut host = hosted_with_pin();
    host.set_now_ms(1_000);
    let point = chip_centre(&host);
    host.apply_cursor_move(point.x, point.y);
    assert!(host
        .editor_state()
        .editor_ui
        .chat_style_chip_hover_since_ms
        .is_some());

    assert!(
        host.clear_chat_and_lower_hover(),
        "dropping a live clock is a change worth a repaint"
    );
    assert_eq!(
        host.editor_state().editor_ui.chat_style_chip_hover_since_ms,
        None
    );
}
