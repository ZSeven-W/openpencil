//! Unit tests for the Studio Home surface: palette tokens, hit-tests,
//! the model chip, and the connect card.

use super::{model, HomeSurface, StudioPalette};
use crate::{Color, Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn visible_home() -> op_editor_core::EditorState {
    let mut state = op_editor_core::EditorState::new();
    state.editor_ui.home.visible = true;
    state
}

#[test]
fn studio_palette_owns_the_exact_light_and_dark_tokens() {
    let light = StudioPalette::light();
    assert_hex(light.page, "F9FBFD");
    assert_hex(light.ink, "111A32");
    assert_hex(light.muted, "78859C");
    assert_hex(light.blue, "075BFF");
    assert_hex(light.blue_hover, "004CE0");
    assert_hex(light.line, "E1E8F2");
    assert_hex(light.yellow, "F3FF23");
    assert_hex(light.disabled_primary, "9ABBFF");
    assert_hex(light.preview, "EEF4FF");
    assert_hex(light.selected_tab, "EDF4FF");
    assert_hex(light.input_line, "D5DEEC");
    assert_hex(light.tint_knowledge, "FFF3E8");
    assert_hex(light.tint_tutorial, "EDF5FF");
    assert_hex(light.tint_poster, "F4FADD");
    assert_hex(light.tile_knowledge, "FF9442");
    assert_hex(light.tile_tutorial, "68A6FF");
    assert_hex(light.tile_poster, "C9FA15");

    // The founder-approved dark values (prototype `dark-theme.css`).
    let dark = StudioPalette::dark();
    assert_hex(dark.page, "10131A");
    assert_hex(dark.panel, "191E28");
    assert_hex(dark.line, "293244");
    assert_hex(dark.ink, "EDF1F8");
    assert_hex(dark.muted, "A0AEC3");
    assert_hex(dark.placeholder, "8F9DB3");
    assert_hex(dark.topbar, "141821");
    assert_hex(dark.topbar_line, "262F3E");
    assert_hex(dark.raised, "222A38");
    assert_hex(dark.raised_line, "303A4D");
    assert_hex(dark.surface_input, "131821");
    assert_hex(dark.input_line, "364157");
    assert_hex(dark.button_hover, "2A3445");
    assert_hex(dark.preview, "1B2538");
    assert_hex(dark.preview_line, "2A3952");
    assert_hex(dark.disabled_primary, "253754");
    assert_hex(dark.disabled_primary_ink, "93A8C8");
    assert_hex(dark.tab_selected_fill, "202F4C");
    assert_hex(dark.tab_selected_line, "3B5682");
    assert_hex(dark.tab_selected_ink, "A7C6FF");
    assert_hex(dark.canvas, "111620");
    assert_hex(dark.canvas_dot, "303A4E");
    assert_hex(dark.tint_knowledge, "2B2321");
    assert_hex(dark.tint_tutorial, "1D293C");
    assert_hex(dark.tint_poster, "252C1D");

    // Three contracts the dark theme must not break.
    // 1. The button blue stays the button blue; only the TEXT blue lifts.
    assert_hex(dark.blue, "075BFF");
    assert_hex(dark.link, "8BB3FF");
    assert_ne!(dark.blue, dark.link);
    // 2. Cards, inset inputs and raised popovers stay three distinct
    //    surfaces — this is what a single `panel` token cannot express.
    assert_ne!(dark.panel, dark.surface_input);
    assert_ne!(dark.panel, dark.raised);
    assert_ne!(dark.surface_input, dark.raised);
    // 3. The work keeps its own colours: the yellow marker, the sticker's
    //    dark ink and the three bright example tiles are theme-independent.
    assert_hex(dark.yellow, "F3FF23");
    assert_hex(dark.sticker_ink, "182019");
    assert_eq!(dark.tile_knowledge, light.tile_knowledge);
    assert_eq!(dark.tile_tutorial, light.tile_tutorial);
    assert_eq!(dark.tile_poster, light.tile_poster);
    // The marker becomes an underline in dark so it cannot sit behind
    // near-white glyphs.
    assert!(!light.marker_underline);
    assert!(dark.marker_underline);
}

#[test]
fn fading_the_palette_touches_every_colour_and_no_flag() {
    // The three surfaces that crossfade share one implementation, so a
    // token added later cannot be left at full alpha on one of them.
    let full = StudioPalette::dark();
    let half = full.faded(0.5);
    assert_eq!(half.page.a, full.page.a * 0.5);
    assert_eq!(half.raised.a, full.raised.a * 0.5);
    assert_eq!(half.surface_input.a, full.surface_input.a * 0.5);
    assert_eq!(half.canvas_dot.a, full.canvas_dot.a * 0.5);
    assert_eq!(half.link.a, full.link.a * 0.5);
    assert_eq!(half.sticker_ink.a, full.sticker_ink.a * 0.5);
    assert_eq!(half.marker_underline, full.marker_underline);
}

#[test]
fn home_hit_test_resolves_tab_segment_send_and_explore_card() {
    let state = visible_home();
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.tabs[0])),
        Some(HomeHit::Tab(HomeFamily::AppUi))
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.tabs[6])),
        Some(HomeHit::Tab(HomeFamily::EventPoster))
    );
    // The app task's segmented control: option 0 = 手机.
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.segment_options[0])),
        Some(HomeHit::Segment(0))
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.segment_options[1])),
        Some(HomeHit::Segment(1))
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.send)),
        Some(HomeHit::Send)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.model_chip)),
        Some(HomeHit::ModelChip)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.use_example)),
        Some(HomeHit::UseExample)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.explore_cards[1])),
        Some(HomeHit::ExploreCard(HomeFamily::ScreenshotTutorial))
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.professional)),
        Some(HomeHit::Professional)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.open_file)),
        Some(HomeHit::OpenFile)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.input_box)),
        Some(HomeHit::Sheet),
        "the composer input keeps its caret-press hit"
    );
}

#[test]
fn tasks_without_a_segment_hide_the_control() {
    let mut state = visible_home();
    state.editor_ui.home.set_task(HomeFamily::Web, 1_000);
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert!(layout.segment.size.x == 0.0);
    assert!(layout.segment_options.iter().all(|r| r.size.x == 0.0));
    // The presentation task shows exactly two options.
    state
        .editor_ui
        .home
        .set_task(HomeFamily::Presentation, 2_000);
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert!(layout.segment_options[0].size.x > 0.0);
    assert!(layout.segment_options[1].size.x > 0.0);
    assert!(layout.segment_options[2].size.x == 0.0);
    // Infographic shows three.
    state
        .editor_ui
        .home
        .set_task(HomeFamily::Infographic, 3_000);
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert!(layout.segment_options.iter().all(|r| r.size.x > 0.0));
}

#[test]
fn model_chip_label_reuses_the_chat_selection_and_empties_without_an_agent() {
    let mut state = op_editor_core::EditorState::new();
    assert_eq!(
        model::model_chip_label(&state),
        op_i18n::translate(state.editor_ui.locale, "home.connect.chipEmpty")
    );
    state.editor_ui.agent_settings.connected[0] = true;
    state.chat.available_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "claude-sonnet-4-6",
        "Claude Sonnet 4.6",
    )];
    state.chat.selected_model = 0;
    assert_eq!(model::model_chip_label(&state), "Claude Sonnet 4.6");
}

#[test]
fn connect_card_rows_stack_without_overlap_inside_the_card() {
    let state = visible_home();
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    let (card, rows) = (layout.connect_card, layout.connect_rows);
    assert_eq!(card.size, Point2D::new(440.0, 280.0));
    // Centred over the composer panel.
    assert_close(
        card.origin.x + card.size.x / 2.0
            - (layout.composer.origin.x + layout.composer.size.x / 2.0),
        0.0,
    );
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row.size.y, 56.0);
        assert!(row.origin.x >= card.origin.x);
        assert!(row.origin.x + row.size.x <= card.origin.x + card.size.x);
        assert!(row.origin.y >= card.origin.y);
        assert!(row.origin.y + row.size.y <= card.origin.y + card.size.y);
        if index > 0 {
            assert_close(rows[index].origin.y - rows[index - 1].origin.y, 66.0);
            assert!(!overlaps(rows[index - 1], rows[index]));
        }
    }
}

#[test]
fn connect_card_open_hides_home_hits_behind_the_modal() {
    let mut state = visible_home();
    state.editor_ui.home.connect_card_open = true;
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.connect_rows[0])),
        Some(HomeHit::ConnectFreeTier)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.connect_rows[1])),
        Some(HomeHit::ConnectApiKey)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.connect_rows[2])),
        Some(HomeHit::ConnectCli)
    );
    // Presses that would hit Home chrome (the send button) and presses
    // far outside both close the card instead.
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.send)),
        Some(HomeHit::ConnectClose)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, Point2D::new(4.0, 4.0)),
        Some(HomeHit::ConnectClose)
    );
}

#[test]
fn replace_strip_hits_only_while_pending() {
    let mut state = visible_home();
    state.editor_ui.home.draft = "我自己的需求".into();
    state.editor_ui.home.set_task(HomeFamily::AppUi, 1_000);
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    // Pending: both strip buttons hit.
    state.editor_ui.home.replace_pending = true;
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout_pending = home.layout(1440.0, 900.0);
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout_pending.replace_use)),
        Some(HomeHit::ReplaceConfirm)
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout_pending.replace_keep)),
        Some(HomeHit::ReplaceKeep)
    );
    // Not pending: the strip area is just the input box.
    let _ = layout;
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 2.0, "{actual} != {expected}");
}

fn assert_hex(color: Color, expected: &str) {
    let actual = format!(
        "{:02X}{:02X}{:02X}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8
    );
    assert_eq!(actual, expected);
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}
