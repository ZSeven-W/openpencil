//! The retired minimized AI chat bar. On desktop the conversation has
//! ONE home — the rail's Agent tab — and everywhere else the chat is
//! the composer card docked at the canvas floor, so `chat.minimize()`
//! no longer has a form to take. These tests pin that retirement, and
//! the floor-docked card that took the bar's place.

use super::WidgetHostNative;
use op_editor_core::LeftPanelTab;
use op_editor_ui::widgets::host_canvas_geometry::{
    AICHAT_INSET_BOTTOM, AICHAT_INSET_LEFT, COMPOSER_CARD_W,
};
use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, AI_CHAT_MINIMIZED_HEIGHT};
use op_editor_ui::Point2D;

const VIEWPORT: (f32, f32) = (1200.0, 800.0);

/// The card's width at this viewport, derived the way
/// `pinned_chat` derives it — not a second copy of the constant.
fn composer_card_width(host: &WidgetHostNative) -> f32 {
    let (_cx0, _cy0, cw, _ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);
    COMPOSER_CARD_W.min((cw - AICHAT_INSET_LEFT * 2.0).max(0.0))
}

/// RETIRED BEHAVIOUR (minimizing to a bar): the chat used to collapse
/// into a `AI_CHAT_MINIMIZED_HEIGHT` strip docked at the canvas floor.
/// On the Agent tab `chat.minimize()` is now inert state — the pinned
/// rect neither shrinks to a bar nor moves to the floor. If a floating
/// panel ever comes back and honors this flag again, the rect equality
/// below breaks.
#[test]
fn minimizing_on_the_agent_tab_leaves_the_pinned_rect_untouched() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let before = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("pinned panel placed");

    host.editor_state_mut().chat.minimize();
    assert!(host.editor_state().chat.is_minimized(), "state flag set");

    let after = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("still pinned, minimized is not a form");
    assert_eq!(after, before);
    assert!(
        after.size.y > AI_CHAT_MINIMIZED_HEIGHT,
        "the pinned panel must not shrink to the old bar height"
    );
    let (_cx0, cy0, _cw, ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);
    assert_ne!(
        after.origin.y,
        cy0 + ch - AI_CHAT_MINIMIZED_HEIGHT - AICHAT_INSET_BOTTOM,
        "the minimized bar's old floor dock must not come back"
    );
}

/// The composer card is what appears everywhere the rail is NOT on the
/// Agent tab: a card docked at the canvas floor's bottom-left, at the
/// card width — the spot the old minimized bar used to occupy.
#[test]
fn the_composer_card_docks_at_the_canvas_floor_when_the_rail_is_on_layers() {
    let host = WidgetHostNative::new();
    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
        LeftPanelTab::Layers,
        "fixture starts on the default Layers tab"
    );
    assert!(host.editor_state().editor_ui.chat_composer_only());

    let (cx0, cy0, _cw, ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);
    let width = composer_card_width(&host);
    let height = AIChatPlaceholder::from_editor(host.editor_state()).composer_only_height(width);
    let card = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("composer card placed");

    assert_eq!(card.origin.x, cx0 + AICHAT_INSET_LEFT);
    assert_eq!(card.origin.y + card.size.y, cy0 + ch - AICHAT_INSET_BOTTOM);
    assert_eq!(card.size.x, width);
    assert_eq!(card.size.y, height);

    // Contrast: with the rail on the chat's home there is no card —
    // the same getter resolves the tall pinned column instead.
    let mut pinned_host = WidgetHostNative::new();
    pinned_host.editor_state_mut().editor_ui.enter_chat_tab();
    let pinned = pinned_host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("pinned panel placed");
    assert!(pinned.size.y > card.size.y);
    assert_ne!(pinned.origin, card.origin);
}

/// RETIRED BEHAVIOUR (the dragged bar): the bar honoured the horizontal
/// half of a position left over from dragging the floating panel. The
/// composer card honours NEITHER half — it is a dock, not a window, so
/// a stale drag position cannot lift it off the canvas floor.
#[test]
fn a_stale_drag_position_cannot_move_the_composer_card_off_the_canvas_floor() {
    let mut host = WidgetHostNative::new();
    {
        let state = host.editor_state_mut();
        state.chat.panel_position = Some((600.0, 100.0));
        state.chat.anchor = op_editor_core::ChatAnchor::TopRight;
    }
    let (cx0, cy0, _cw, ch) = host.canvas_region(VIEWPORT.0, VIEWPORT.1);
    let card = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("card placed");

    assert_eq!(card.origin.x, cx0 + AICHAT_INSET_LEFT);
    assert_eq!(card.origin.y + card.size.y, cy0 + ch - AICHAT_INSET_BOTTOM);
    assert_ne!(card.origin.x, 600.0, "the stale dragged x must be ignored");
}

/// RETIRED BEHAVIOUR (expand-on-click): clicking the minimized bar used
/// to expand the panel and hand over the caret, because the bar read as
/// an input. The caret half of that promise lives on in the card: a
/// click in the composer's text box focuses it — without taking the
/// rail anywhere (only a send or a header glyph opens the Agent tab).
#[test]
fn clicking_the_composer_cards_input_hands_over_the_caret() {
    let mut host = WidgetHostNative::new();
    let card = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("card placed");
    let text = AIChatPlaceholder::from_editor(host.editor_state()).input_text_rect(card);
    let point = Point2D::new(
        text.origin.x + text.size.x / 2.0,
        text.origin.y + text.size.y / 2.0,
    );

    assert!(host.apply_click(point.x, point.y, VIEWPORT.0, VIEWPORT.1));

    assert!(
        host.editor_state().chat.focused,
        "the card reads as an input, so a click hands over the caret"
    );
    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
        LeftPanelTab::Layers,
        "focusing the composer must not enter the chat tab"
    );
    // The focused card grows its header; the unfocused one does not.
    // Focused, the header's two glyphs both route to the Agent tab
    // (the old maximize glyph's spot is one of them).
    let focused = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("focused card placed");
    assert_eq!(focused.size.y - card.size.y, 30.0);
    let header_point = Point2D::new(
        focused.origin.x + focused.size.x / 2.0,
        focused.origin.y + 15.0,
    );
    assert_eq!(
        AIChatPlaceholder::from_editor(host.editor_state()).hit_test(focused, header_point),
        Some(AIChatHit::ToggleMaximize),
        "the focused card's header glyphs are the launcher, not a window control"
    );
    // The same absolute point is ABOVE the unfocused card: no header,
    // no launcher — the glyph only exists once the card is focused.
    assert_ne!(
        AIChatPlaceholder::from_editor(host.editor_state()).hit_test(card, header_point),
        Some(AIChatHit::ToggleMaximize)
    );
}

/// RETIRED BEHAVIOUR (the vacated footprint): the minimized form had to
/// give the expanded panel's footprint back to the canvas. The card
/// never had a bigger footprint, but the guarantee it stood for is
/// pinned here for its successor: everything above the card belongs to
/// the canvas, and pressing there must not open or focus the chat.
#[test]
fn a_press_above_the_composer_card_reaches_the_canvas() {
    let mut host = WidgetHostNative::new();
    let card = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("card placed");
    let above_y = card.origin.y - 120.0;

    assert!(host.apply_press(card.origin.x + 40.0, above_y, VIEWPORT.0, VIEWPORT.1));

    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
        LeftPanelTab::Layers,
        "a canvas press must not enter the chat tab"
    );
    assert!(
        !host.editor_state().chat.focused,
        "a canvas press must not focus the composer"
    );
}

/// Minimizing changes the pinned panel's HEIGHT in no way at all, so
/// the width cannot drift either. (Guards the same invariant the two
/// `minimizing_keeps…` tests below do, from the geometry side.)
#[test]
fn minimizing_keeps_the_panel_x_and_width_to_the_pixel() {
    for anchor in [
        op_editor_core::ChatAnchor::BottomLeft,
        op_editor_core::ChatAnchor::BottomRight,
        op_editor_core::ChatAnchor::TopRight,
    ] {
        for width in [360.0_f32, 520.0, 288.0] {
            let mut host = WidgetHostNative::new();
            // The expanded chat panel lives in the rail's Agent tab; anywhere
            // else the chat is composer-only, so put the rail on its home.
            host.editor_state_mut().editor_ui.enter_chat_tab();
            {
                let state = host.editor_state_mut();
                state.chat.anchor = anchor;
                state.chat.panel_width = width;
            }

            let (expanded, minimized) = expanded_then_minimized(&mut host);

            assert_eq!(
                minimized.size.x, expanded.size.x,
                "{anchor:?} @ {width}: the pinned panel must stay exactly as wide"
            );
            assert_eq!(
                minimized.origin.x, expanded.origin.x,
                "{anchor:?} @ {width}: the pinned panel's left edge must not move"
            );
        }
    }
}

/// The same guarantee after a drag: the pinned panel ignores a stored
/// floating position entirely, so neither edge can jump sideways.
#[test]
fn minimizing_a_dragged_panel_keeps_its_x_and_width() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    {
        let state = host.editor_state_mut();
        state.chat.panel_position = Some((600.0, 100.0));
        state.chat.anchor = op_editor_core::ChatAnchor::TopRight;
    }

    let (expanded, minimized) = expanded_then_minimized(&mut host);

    assert_eq!(minimized.origin.x, expanded.origin.x);
    assert_eq!(minimized.size.x, expanded.size.x);
}

/// The expanded rect, then the same host's minimized rect.
fn expanded_then_minimized(
    host: &mut WidgetHostNative,
) -> (op_editor_ui::Rect, op_editor_ui::Rect) {
    let expanded = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("panel placed");
    host.editor_state_mut().chat.minimize();
    let minimized = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("bar placed");
    (expanded, minimized)
}
