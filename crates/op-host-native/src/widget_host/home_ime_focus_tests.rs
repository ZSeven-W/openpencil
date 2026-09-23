//! Host-level tests for the Home composer's software-keyboard contract:
//! the IME focus follows the input box on touch hosts (the shell lowers
//! the keyboard off the ABI bool flip), the desktop keeps its
//! always-focused reading, and the keyboard band never covers the
//! composer.

use super::WidgetHostNative;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_ui::widgets::{HomeLayout, HomeSurface};
use op_editor_ui::Point2D;

const W: f32 = 390.0;
const H: f32 = 844.0;
/// A representative iPhone software-keyboard band.
const KEYBOARD_H: f32 = 336.0;
const WIDE_W: f32 = 1440.0;
const WIDE_H: f32 = 900.0;
/// A shorter phone viewport whose compact composer sits INSIDE the
/// keyboard band (at full height 844 the input box clears 336 pt without
/// scrolling, so the reveal has nothing to do there).
const COVERED_H: f32 = 640.0;

/// A compact phone host with the Home takeover up (touch chrome on).
fn touch_compact_home() -> WidgetHostNative {
    let mut state = op_editor_core::EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.home.visible = true;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

/// A desktop host with Home up — no touch chrome, no software keyboard.
fn desktop_home() -> WidgetHostNative {
    let mut state = op_editor_core::EditorState::starter();
    state.editor_ui.home.visible = true;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host
}

fn center(rect: op_editor_ui::Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn home_layout(host: &WidgetHostNative, w: f32, h: f32) -> HomeLayout {
    HomeSurface::for_editor(host.editor_state())
        .expect("home visible")
        .layout(w, h)
}

/// A compact tap defers to release and replays through the ordinary
/// press ladder; drive the pair and return whether the replay ran.
fn tap(host: &mut WidgetHostNative, x: f32, y: f32) -> bool {
    tap_in(host, x, y, W, H)
}

fn tap_in(host: &mut WidgetHostNative, x: f32, y: f32, w: f32, h: f32) -> bool {
    host.apply_press(x, y, w, h) && host.apply_release_with_viewport(w, h)
}

#[test]
fn touch_home_keyboard_focus_follows_the_input_box() {
    let mut host = touch_compact_home();
    assert!(
        !host.text_input_focus_active(),
        "an untouched Home must not claim the software keyboard"
    );

    let input = center(home_layout(&host, W, H).input_box);
    assert!(tap(&mut host, input.x, input.y), "the deferred tap replays");
    assert!(
        host.text_input_focus_active(),
        "the input box owns the keyboard"
    );

    // The task grid releases it again.
    let tab = center(home_layout(&host, W, H).tabs[0]);
    assert!(tap(&mut host, tab.x, tab.y));
    assert!(
        !host.text_input_focus_active(),
        "a task-grid tap must release the keyboard"
    );

    // So does a tap on the hero copy (no Home target there).
    let hero = center(home_layout(&host, W, H).welcome);
    assert!(host.apply_press(hero.x, hero.y, W, H));
    host.apply_release_with_viewport(W, H);
    assert!(
        !host.text_input_focus_active(),
        "a hero tap must release the keyboard"
    );
}

#[test]
fn desktop_home_keeps_the_keyboard_claimed() {
    let mut host = desktop_home();
    assert!(
        host.text_input_focus_active(),
        "desktop Home IS the composer: focused before any press"
    );

    let input = center(home_layout(&host, WIDE_W, WIDE_H).input_box);
    assert!(host.apply_press(input.x, input.y, WIDE_W, WIDE_H));
    assert!(host.text_input_focus_active());

    let tab = center(home_layout(&host, WIDE_W, WIDE_H).tabs[0]);
    assert!(host.apply_press(tab.x, tab.y, WIDE_W, WIDE_H));
    assert!(
        !host.editor_state().editor_ui.home.composer_focused,
        "the press still updates the composer's own bit"
    );
    assert!(
        host.text_input_focus_active(),
        "desktop focus must not regress: there is no soft keyboard to dismiss"
    );
}

#[test]
fn keyboard_occlusion_scrolls_the_composer_clear_of_the_band() {
    let mut host = touch_compact_home();
    let input = center(home_layout(&host, W, H).input_box);
    assert!(tap_in(&mut host, input.x, input.y, W, H));
    assert!(host.text_input_focus_active());

    assert!(host.set_keyboard_occlusion(KEYBOARD_H));
    let layout = home_layout(&host, W, H);
    let visible_bottom = H - KEYBOARD_H;
    // The whole composer clears, not just the box being typed in: a
    // visible caret with 开始设计 still buried is a dead end on a phone.
    let composer_bottom = layout.send.origin.y + layout.send.size.y;
    assert!(
        host.editor_state().editor_ui.home.scroll_y > 0.0,
        "the covered composer must scroll"
    );
    assert!(
        composer_bottom + 0.01 <= visible_bottom,
        "the composer's send button must clear the keyboard band:          {composer_bottom} > {visible_bottom}"
    );

    // Zero occlusion no longer forces any scroll.
    let before = host.editor_state().editor_ui.home.scroll_y;
    assert!(host.set_keyboard_occlusion(0.0));
    assert_eq!(
        host.editor_state().editor_ui.home.scroll_y,
        before,
        "no keyboard, no forced scroll"
    );
}

#[test]
fn a_page_too_short_for_the_whole_composer_still_clears_the_input_box() {
    // 640 pt minus a 336 pt keyboard cannot hold the whole composer, and
    // the reveal is clamped by the page's own max scroll. The floor it
    // must still deliver: the box the caret is in.
    let mut host = touch_compact_home();
    let input = center(home_layout(&host, W, COVERED_H).input_box);
    assert!(tap_in(&mut host, input.x, input.y, W, COVERED_H));
    assert!(host.set_keyboard_occlusion(KEYBOARD_H));

    let layout = home_layout(&host, W, COVERED_H);
    let input_bottom = layout.input_box.origin.y + layout.input_box.size.y;
    let visible_bottom = COVERED_H - KEYBOARD_H;
    assert!(
        input_bottom + 0.01 <= visible_bottom,
        "the input box must clear the keyboard band: {input_bottom} > {visible_bottom}"
    );
}

#[test]
fn focusing_after_the_keyboard_raised_scrolls_the_composer_into_view() {
    // Keyboard first, focus second: the Sheet press itself must do the
    // reveal or the just-tapped composer stays buried.
    let mut host = touch_compact_home();
    assert!(host.set_keyboard_occlusion(KEYBOARD_H));

    let input = center(home_layout(&host, W, H).input_box);
    assert!(tap_in(&mut host, input.x, input.y, W, H));
    assert!(host.text_input_focus_active());
    assert!(
        host.editor_state().editor_ui.home.scroll_y > 0.0,
        "the Sheet press itself must do the reveal"
    );

    let layout = home_layout(&host, W, H);
    let visible_bottom = H - KEYBOARD_H;
    let composer_bottom = layout.send.origin.y + layout.send.size.y;
    assert!(
        composer_bottom + 0.01 <= visible_bottom,
        "the just-focused composer must be revealed: {composer_bottom} > {visible_bottom}"
    );
}

#[test]
fn an_already_visible_composer_is_left_where_the_reader_put_it() {
    // The reveal is one-way: it pulls a covered box up, it never pulls an
    // already-visible page back down to recover slack (the agent-settings
    // reveal leaves an in-view field alone for the same reason).
    let mut host = touch_compact_home();
    let max_scroll = op_editor_ui::widgets::home_surface::max_scroll_for_mode(
        W,
        COVERED_H,
        host.editor_state().editor_ui.home.task,
        op_editor_ui::widgets::home_surface::model_chip_width(""),
        true,
    );
    assert!(max_scroll > 0.0, "precondition: the compact page scrolls");
    let parked = max_scroll / 2.0;
    host.editor_state_mut().editor_ui.home.scroll_y = parked;

    let input = center(home_layout(&host, W, COVERED_H).input_box);
    assert!(tap_in(&mut host, input.x, input.y, W, COVERED_H));
    assert!(host.text_input_focus_active());
    assert_eq!(
        host.editor_state().editor_ui.home.scroll_y,
        parked,
        "the tap alone must not move the page"
    );

    // A band far too short to reach the composer at this scroll.
    assert!(host.set_keyboard_occlusion(8.0));
    let layout = home_layout(&host, W, COVERED_H);
    let input_bottom = layout.input_box.origin.y + layout.input_box.size.y;
    assert!(
        input_bottom < COVERED_H - 8.0,
        "precondition: the composer already clears the band"
    );
    assert_eq!(
        host.editor_state().editor_ui.home.scroll_y,
        parked,
        "a visible composer must not move the page"
    );
}
