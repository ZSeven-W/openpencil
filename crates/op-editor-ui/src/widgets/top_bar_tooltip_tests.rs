//! Tooltip behaviour for the TopBar's chrome buttons: the dwell gate,
//! the repaint the dwell needs to be scheduled, suppression under an
//! open menu, viewport clamping, and label coverage of every button.

use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::TopBarButton;

use crate::widgets::editor_state_ext::translate;
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::top_bar_tooltip::{
    self, paint_top_bar_tooltip, ALL_TOP_BAR_BUTTONS, TOOLTIP_DWELL_MS,
};
use crate::widgets::{PaintCx, TopBar, TOP_BAR_HEIGHT};
use crate::{Point2D, Rect};

const VIEWPORT_W: f32 = 1400.0;

fn top_bar_rect() -> Rect {
    Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(VIEWPORT_W, TOP_BAR_HEIGHT),
    }
}

/// A UI state hovering `button`, having entered the bar at `since_ms`.
fn hovering(button: TopBarButton, since_ms: u64) -> EditorUiState {
    let mut ui = EditorUiState::default();
    ui.set_topbar_button_hover(Some(button), since_ms);
    ui
}

fn paint_at(ui: &EditorUiState, now_ms: u64) -> Option<Rect> {
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let bar = TopBar::for_editor_ui(ui);
    paint_top_bar_tooltip(&mut cx, ui, &bar, top_bar_rect(), VIEWPORT_W, now_ms)
}

#[test]
fn entering_the_bar_stamps_the_dwell_clock_once_per_visit() {
    let mut ui = EditorUiState::default();
    assert!(ui.set_topbar_button_hover(Some(TopBarButton::ToggleTheme), 1_000));
    assert_eq!(ui.topbar_hover_since_ms, Some(1_000));

    // Sliding to the next button inside the same visit must NOT restart
    // the wait — that is what makes a settled tooltip follow the cursor.
    assert!(ui.set_topbar_button_hover(Some(TopBarButton::ToggleLocale), 1_200));
    assert_eq!(ui.topbar_hover_since_ms, Some(1_000));

    // Leaving and coming back is a new visit, so it waits again.
    assert!(ui.set_topbar_button_hover(None, 1_300));
    assert!(ui.set_topbar_button_hover(Some(TopBarButton::ToggleTheme), 5_000));
    assert_eq!(ui.topbar_hover_since_ms, Some(5_000));
}

#[test]
fn no_change_reports_no_repaint() {
    let mut ui = hovering(TopBarButton::ToggleTheme, 1_000);
    assert!(!ui.set_topbar_button_hover(Some(TopBarButton::ToggleTheme), 2_000));
    assert_eq!(ui.topbar_hover_since_ms, Some(1_000));
}

#[test]
fn nothing_paints_before_the_dwell_elapses() {
    let ui = hovering(TopBarButton::OpenAssetCenter, 1_000);
    assert_eq!(
        top_bar_tooltip::visible_button(&ui, 1_000 + TOOLTIP_DWELL_MS - 1),
        None
    );
    assert!(paint_at(&ui, 1_000 + TOOLTIP_DWELL_MS - 1).is_none());
}

#[test]
fn the_tooltip_paints_once_the_dwell_elapses() {
    let ui = hovering(TopBarButton::OpenAssetCenter, 1_000);
    let now = 1_000 + TOOLTIP_DWELL_MS;
    assert_eq!(
        top_bar_tooltip::visible_button(&ui, now),
        Some(TopBarButton::OpenAssetCenter)
    );
    let rect = paint_at(&ui, now).expect("the dwell has elapsed, so the tooltip paints");
    assert!(rect.size.x > 0.0 && rect.size.y > 0.0);
}

/// The load-bearing one: the dwell expiring is not an input event, so
/// the due instant must reach the animation scheduler. Without this the
/// tooltip only ever appears if the user jiggles the mouse afterwards.
#[test]
fn a_pending_tooltip_schedules_the_repaint_that_will_draw_it() {
    let mut state = op_editor_core::EditorState::default();
    state.editor_ui = hovering(TopBarButton::OpenExportMenu, 1_000);

    let deadline =
        crate::widgets::host_frame_bookkeeping::base_animation_deadline_ms(&state, None, 1_000);
    assert_eq!(deadline, Some(1_000 + TOOLTIP_DWELL_MS));

    // Once it is showing there is nothing further to wake up for.
    assert_eq!(
        top_bar_tooltip::next_deadline_ms(&state.editor_ui, 1_000 + TOOLTIP_DWELL_MS),
        None
    );
}

#[test]
fn an_unhovered_bar_schedules_nothing() {
    let ui = EditorUiState::default();
    assert_eq!(top_bar_tooltip::next_deadline_ms(&ui, 9_000), None);
}

#[test]
fn an_open_menu_suppresses_its_own_buttons_tooltip() {
    let mut ui = hovering(TopBarButton::ToggleFileMenu, 1_000);
    let now = 1_000 + TOOLTIP_DWELL_MS;
    assert!(paint_at(&ui, now).is_some());

    ui.file_menu_open = true;
    assert!(top_bar_tooltip::suppressed(
        &ui,
        TopBarButton::ToggleFileMenu
    ));
    assert_eq!(top_bar_tooltip::visible_button(&ui, now), None);
    assert!(paint_at(&ui, now).is_none());
    // A suppressed tooltip must not keep the scheduler awake either.
    assert_eq!(top_bar_tooltip::next_deadline_ms(&ui, 1_000), None);
}

/// Opens the surface a given button owns.
type OpenSurface = fn(&mut EditorUiState);

#[test]
fn every_open_surface_suppresses_exactly_its_own_button() {
    let cases: [(TopBarButton, OpenSurface); 9] = [
        (TopBarButton::ToggleFileMenu, |ui| ui.file_menu_open = true),
        (TopBarButton::OpenImportMenu, |ui| {
            ui.import_menu_open = true
        }),
        (TopBarButton::ToggleLocale, |ui| {
            ui.locale_picker.open = true
        }),
        (TopBarButton::OpenExportMenu, |ui| {
            ui.export_quick_menu_open = true
        }),
        (TopBarButton::OpenAssetCenter, |ui| {
            ui.scene_template_center.open = true
        }),
        (TopBarButton::OpenAgentSettings, |ui| {
            ui.agent_settings_open = true
        }),
        (TopBarButton::OpenCollaboration, |ui| {
            ui.collab.panel.open = true
        }),
        (TopBarButton::ToggleGitPanel, |ui| ui.git_panel.open = true),
        (TopBarButton::OpenAccount, |ui| ui.account_menu_open = true),
    ];
    for (button, open) in cases {
        let mut ui = EditorUiState::default();
        open(&mut ui);
        assert!(
            top_bar_tooltip::suppressed(&ui, button),
            "{button:?} should be suppressed while its own surface is open"
        );
        for other in ALL_TOP_BAR_BUTTONS {
            if other != button {
                assert!(
                    !top_bar_tooltip::suppressed(&ui, other),
                    "{button:?}'s open surface must not suppress {other:?}"
                );
            }
        }
    }
}

#[test]
fn home_button_tooltip_reuses_file_menu_home_label() {
    let ui = EditorUiState::default();
    assert_eq!(
        top_bar_tooltip::tooltip_for(&ui, TopBarButton::Home).label,
        translate(&ui, "fileMenu.home"),
    );
}

#[test]
fn every_button_has_a_non_empty_label() {
    let ui = EditorUiState::default();
    for button in ALL_TOP_BAR_BUTTONS {
        let tip = top_bar_tooltip::tooltip_for(&ui, button);
        assert!(
            !tip.label.trim().is_empty(),
            "{button:?} has no tooltip label"
        );
        assert!(
            !tip.label.starts_with("tooltip."),
            "{button:?} fell through to its raw i18n key: {}",
            tip.label
        );
    }
}

#[test]
fn toggles_name_the_state_they_will_move_to() {
    let mut ui = EditorUiState {
        sidebar_open: true,
        ..Default::default()
    };
    let open_label = top_bar_tooltip::tooltip_for(&ui, TopBarButton::ToggleSidebar).label;
    ui.sidebar_open = false;
    let closed_label = top_bar_tooltip::tooltip_for(&ui, TopBarButton::ToggleSidebar).label;
    assert_ne!(open_label, closed_label);

    ui.window_fullscreen = false;
    let windowed = top_bar_tooltip::tooltip_for(&ui, TopBarButton::ToggleFullscreen).label;
    ui.window_fullscreen = true;
    let full = top_bar_tooltip::tooltip_for(&ui, TopBarButton::ToggleFullscreen).label;
    assert_ne!(windowed, full);
}

/// Shortcut hints must be earned by a real binding, so exactly the three
/// bound actions carry one. Anything else claiming a shortcut is a lie
/// to the user.
#[test]
fn only_actually_bound_actions_advertise_a_shortcut() {
    let ui = EditorUiState::default();
    let with_shortcut: Vec<TopBarButton> = ALL_TOP_BAR_BUTTONS
        .into_iter()
        .filter(|b| top_bar_tooltip::tooltip_for(&ui, *b).shortcut.is_some())
        .collect();
    let expected: Vec<TopBarButton> = if cfg!(target_arch = "wasm32") {
        // The browser host binds none of them.
        Vec::new()
    } else if cfg!(target_os = "macos") {
        vec![
            TopBarButton::OpenAgentSettings,
            TopBarButton::ToggleFullscreen,
            TopBarButton::TogglePreview,
        ]
    } else {
        // Fullscreen rides the macOS-only menu accelerator.
        vec![TopBarButton::OpenAgentSettings, TopBarButton::TogglePreview]
    };
    assert_eq!(with_shortcut, expected);
    assert_eq!(
        top_bar_tooltip::tooltip_for(&ui, TopBarButton::ToggleSidebar).shortcut,
        None
    );
}

#[test]
fn the_tooltip_hangs_below_its_button_and_stays_in_the_viewport() {
    let ui = hovering(TopBarButton::ToggleSidebar, 0);
    let bar = TopBar::for_editor_ui(&ui);
    let anchor = bar
        .button_rect(top_bar_rect(), TopBarButton::ToggleSidebar)
        .expect("the sidebar toggle always paints");
    let rect = paint_at(&ui, TOOLTIP_DWELL_MS).expect("dwell elapsed");
    assert!(
        rect.origin.y >= anchor.origin.y + anchor.size.y,
        "tooltip must hang below the button, not over it"
    );
    assert!(
        rect.origin.x >= 0.0 && rect.origin.x + rect.size.x <= VIEWPORT_W,
        "tooltip {rect:?} escaped the {VIEWPORT_W}px viewport"
    );
}

#[test]
fn the_rightmost_button_clamps_its_tooltip_inside_the_viewport() {
    // Fullscreen is the rightmost button, so an uncentred tooltip would
    // run off the window's right edge.
    let ui = hovering(TopBarButton::ToggleFullscreen, 0);
    let rect = paint_at(&ui, TOOLTIP_DWELL_MS).expect("dwell elapsed");
    assert!(
        rect.origin.x + rect.size.x <= VIEWPORT_W,
        "tooltip {rect:?} escaped the {VIEWPORT_W}px viewport"
    );
}

#[test]
fn every_visible_button_resolves_an_anchor_rect() {
    let mut ui = EditorUiState {
        account_ui_available: true,
        ..Default::default()
    };
    ui.collab.availability = op_editor_core::CollabAvailability::Ready;
    let bar = TopBar::for_editor_ui(&ui);
    for button in ALL_TOP_BAR_BUTTONS {
        // Buttons compiled out of this build legitimately have no rect;
        // the ones that paint must all be anchorable.
        if let Some(rect) = bar.button_rect(top_bar_rect(), button) {
            assert!(
                rect.size.x > 0.0 && rect.size.y > 0.0,
                "{button:?} anchored on an empty rect"
            );
        }
    }
}

/// The bubble sizes itself from a measurement, so the measurement has to
/// name the family the label is drawn in. Painted into a backend whose
/// named-family advances are 40% wider than its default face — the shape
/// of the real macOS gap between the bundled Roboto and `.AppleSystemUIFont`
/// — a family-blind `tooltip_width` builds a bubble too narrow to hold its
/// own label, and the label runs out past the rounded edge.
///
/// Every locale, every button, both the label and the shortcut hint.
#[test]
fn no_tooltip_label_overflows_its_own_bubble() {
    use crate::widgets::test_family_gap_backend::FamilyGapBackend;
    use crate::widgets::tooltip::TOOLTIP_PAD_X;

    for locale in op_i18n::Locale::ALL {
        for button in ALL_TOP_BAR_BUTTONS {
            let mut ui = hovering(button, 0);
            ui.locale = locale;
            ui.account_ui_available = true;
            ui.collab.availability = op_editor_core::CollabAvailability::Ready;

            let mut backend = FamilyGapBackend::default();
            let bar = TopBar::for_editor_ui(&ui);
            let Some(rect) = ({
                let mut cx = PaintCx {
                    backend: &mut backend,
                };
                paint_top_bar_tooltip(
                    &mut cx,
                    &ui,
                    &bar,
                    top_bar_rect(),
                    VIEWPORT_W,
                    TOOLTIP_DWELL_MS,
                )
            }) else {
                continue;
            };

            assert!(
                !backend.runs.is_empty(),
                "{locale:?}/{button:?} painted a bubble with no text in it"
            );
            for run in &backend.runs {
                assert!(
                    run.origin.x >= rect.origin.x + TOOLTIP_PAD_X - 0.01,
                    "{locale:?}/{button:?} tooltip run {:?} starts left of its bubble padding",
                    run.text
                );
                assert!(
                    run.right_edge() <= rect.origin.x + rect.size.x - TOOLTIP_PAD_X + 0.01,
                    "{locale:?}/{button:?} tooltip run {:?} is {}px wide and overflows the \
                     {}px bubble",
                    run.text,
                    run.width_in_paint_family(),
                    rect.size.x,
                );
            }
        }
    }
}
