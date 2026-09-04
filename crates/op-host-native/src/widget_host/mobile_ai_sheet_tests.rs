//! Mobile AI-sheet regressions: the keyboard-shrunk sheet's internal
//! layout (empty-state suggestions vs. bottom-anchored composer) and the
//! header chevron closing the sheet coherently (sheet gone, input blurred,
//! IME released) instead of leaving a desktop-style minimized bar floating
//! inside an empty sheet.

use super::*;
use op_editor_core::Tool;
use op_editor_ui::widgets::{
    host_canvas_geometry, AIChatHit, AIChatPlaceholder, MobileAppBar, MobileDock,
};

const COMPACT_VIEWPORT: (f32, f32) = (390.0, 844.0);

fn compact_ai_host() -> WidgetHostNative {
    let mut host = touch_host(EditorSizeClass::Compact);
    host.toggle_mobile_sheet(MobileSheetKind::Ai);
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Ai)
    );
    assert!(host.editor_state().chat.focused);
    host
}

/// The chevron's hit target: `expanded_header_title_rect` spans
/// `x ∈ [rect.x + 16, rect.x + 34]`, `y ∈ [rect.y + 5, rect.y + 31]`.
/// The point is validated against the panel's own hit-test before use.
fn chevron_point(rect: Rect) -> Point2D {
    Point2D::new(rect.origin.x + 25.0, rect.origin.y + 18.0)
}

#[test]
fn keyboard_shrunk_ai_sheet_keeps_empty_state_above_composer() {
    let (width, height) = COMPACT_VIEWPORT;
    for keyboard in [0.0_f32, 300.0, 500.0] {
        let mut host = compact_ai_host();
        if keyboard > 0.0 {
            assert!(host.set_keyboard_occlusion(keyboard));
        }
        let rect = host.ai_chat_rect(width, height).expect("AI sheet rect");
        let panel = AIChatPlaceholder::from_editor(host.editor_state());
        let region = panel.empty_state_region(rect);
        let input = panel.input_rect(rect);
        let region_bottom = region.origin.y + region.size.y;
        assert!(region.size.y >= 0.0);
        assert!(
            region_bottom <= input.origin.y,
            "empty-state region bottom ({region_bottom}) must stay above the \
             composer top ({}) at keyboard height {keyboard}",
            input.origin.y
        );
        // Probe the composer band: no suggestion pill may claim a point
        // there — pills that no longer fit are dropped from hit-testing
        // exactly like they are dropped from paint.
        let cx = rect.origin.x + rect.size.x / 2.0;
        for y in [
            region_bottom + 2.0,
            input.origin.y + 8.0,
            rect.origin.y + rect.size.y - 20.0,
        ] {
            let point = Point2D::new(cx, y);
            assert!(
                !matches!(panel.hit_test(rect, point), Some(AIChatHit::Example { .. })),
                "no suggestion pill may own a composer-band point (y {y}) at \
                 keyboard height {keyboard}"
            );
            assert_eq!(panel.example_hover_at(rect, point), None);
        }
    }
}

#[test]
fn ai_sheet_chevron_with_keyboard_up_closes_sheet_and_blurs_input() {
    let (width, height) = COMPACT_VIEWPORT;
    let mut host = compact_ai_host();
    assert!(host.set_keyboard_occlusion(300.0));
    let rect = host.ai_chat_rect(width, height).expect("AI sheet rect");
    let point = chevron_point(rect);
    // Guard the geometry assumption: this point is the collapse chevron.
    {
        let panel = AIChatPlaceholder::from_editor(host.editor_state());
        assert_eq!(
            panel.hit_test(rect, point),
            Some(AIChatHit::ToggleCollapse),
            "test point must land on the header chevron"
        );
    }

    assert!(host.apply_press(point.x, point.y, width, height));

    let state = host.editor_state();
    assert_eq!(
        state.editor_ui.mobile_sheet, None,
        "the chevron closes the mobile AI sheet"
    );
    assert!(
        !state.chat.focused,
        "the chat input blurs so the shell ends the IME session"
    );
    assert_eq!(
        host.ai_chat_rect(width, height),
        None,
        "no leftover sheet rect — the canvas chrome is back"
    );
}

#[test]
fn toggling_ai_sheet_closed_blurs_the_focused_chat_input() {
    let mut host = compact_ai_host();
    assert!(host.editor_state().chat.focused);

    // Close via the same toggle the dock / More entry drives.
    host.toggle_mobile_sheet(MobileSheetKind::Ai);

    let state = host.editor_state();
    assert_eq!(state.editor_ui.mobile_sheet, None);
    assert!(
        !state.chat.focused,
        "closing the AI sheet must not leave the chat input owning the keyboard"
    );
}

#[test]
fn tablet_ai_panel_allows_same_tap_app_bar_action_and_releases_ime() {
    for (size_class, width, height) in [
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(size_class);
        host.toggle_mobile_sheet(MobileSheetKind::Ai);
        host.editor_state_mut().viewport.zoom = 2.75;
        host.editor_state_mut().viewport.pan_x = -731.0;
        host.editor_state_mut().viewport.pan_y = 419.0;
        assert!(host.apply_ime_preedit("zhong", Some((5, 5))));

        let before = host.editor_state().viewport;
        let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
        let point = center(MobileAppBar::fit_rect(bar));
        assert!(!host
            .mobile_sheet_rect(width, height, MobileSheetKind::Ai)
            .contains(point));

        assert!(host.apply_press(point.x, point.y, width, height));

        let state = host.editor_state();
        assert_eq!(state.editor_ui.mobile_sheet, Some(MobileSheetKind::Ai));
        assert_ne!(state.viewport, before, "Fit must run on the first tap");
        assert!(!state.chat.focused, "outside tap releases the keyboard");
        assert!(state.chat.input.composition().is_none());
    }
}

#[test]
fn tablet_ai_panel_keeps_exposed_dock_interactive() {
    for (size_class, width, height) in [
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(size_class);
        host.toggle_mobile_sheet(MobileSheetKind::Ai);
        host.editor_state_mut().tool = Tool::Pen;
        let dock_rect = host_canvas_geometry::touch_dock_rect(host.editor_state(), width, height);
        let dock = MobileDock::for_editor(host.editor_state());
        let point = center(MobileDock::slot_rect(dock_rect, 0, dock.slot_count()));
        assert!(!host
            .mobile_sheet_rect(width, height, MobileSheetKind::Ai)
            .contains(point));

        assert!(host.apply_press(point.x, point.y, width, height));

        assert_eq!(host.editor_state().tool, Tool::Select);
        assert_eq!(
            host.editor_state().editor_ui.mobile_sheet,
            Some(MobileSheetKind::Ai)
        );
    }
}

#[test]
fn tablet_ai_panel_allows_canvas_pan_outside_but_owns_its_bounds() {
    let (width, height) = (834.0, 1_112.0);
    let mut host = touch_host(EditorSizeClass::Medium);
    host.toggle_mobile_sheet(MobileSheetKind::Ai);
    let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::Ai);
    let outside = Point2D::new(200.0, 400.0);
    assert!(!panel.contains(outside));
    let before = host.editor_state().viewport;

    assert!(host.apply_pan_gesture(outside.x, outside.y, 25.0, -30.0, width, height));
    assert_ne!(host.editor_state().viewport, before);
    let moved = host.editor_state().viewport;

    let inside = center(panel);
    assert!(host.apply_pan_gesture(inside.x, inside.y, 25.0, -30.0, width, height));
    assert_eq!(host.editor_state().viewport, moved);
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Ai)
    );
}

#[test]
fn compact_ai_sheet_remains_modal_on_outside_app_bar_tap() {
    let (width, height) = COMPACT_VIEWPORT;
    let mut host = compact_ai_host();
    host.editor_state_mut().viewport.zoom = 2.75;
    host.editor_state_mut().viewport.pan_x = -731.0;
    host.editor_state_mut().viewport.pan_y = 419.0;
    let before = host.editor_state().viewport;
    let bar = host_canvas_geometry::touch_app_bar_rect(host.editor_state(), width);
    let point = center(MobileAppBar::fit_rect(bar));

    assert!(host.apply_press(point.x, point.y, width, height));

    assert_eq!(host.editor_state().viewport, before);
    assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    assert!(!host.editor_state().chat.focused);
}
