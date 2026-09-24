//! Layout-contract tests for the compact (phone) Home branch chosen by
//! `EditorUiState::compact_layout()`: the 2×4 task grid, the 普通 /
//! 专业 top bar switch, the compact composer, the single featured
//! example, and the pinned bottom nav — every touch target at the 44 pt
//! floor. The desktop composition's contracts live in
//! `home_surface_layout_tests.rs` and must keep passing untouched.

use super::{HomeSurface, HOME_BOTTOM_NAV_H, HOME_TOPBAR_H};
use crate::{Point2D, Rect};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{EditorState, HomeFamily, HomeHit};

const W: f32 = 390.0;
const H: f32 = 844.0;
const CHIP_W: f32 = 120.0;

fn compact_state() -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.home.visible = true;
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state
}

fn compact_layout(state: &EditorState) -> super::HomeLayout {
    let home = HomeSurface::for_editor(state).expect("home visible");
    home.layout(W, H)
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn assert_min_touch(rect: Rect, label: &str) {
    assert!(
        rect.size.x >= 44.0 && rect.size.y >= 44.0,
        "{label} must keep the 44 pt touch floor, got {rect:?}"
    );
}

#[test]
fn compact_grid_is_two_rows_of_four_with_a_weakened_blank_tile() {
    let layout = compact_layout(&compact_state());
    // Two rows × four columns inside the content column.
    assert_close(layout.tabs_row.size.x, W - 34.0, 0.5);
    assert_close(layout.tabs_row.size.y, 63.0 * 2.0 + 5.0, 0.5);
    for (index, rect) in layout.tabs.iter().enumerate() {
        let (row, col) = (index / 4, index % 4);
        assert_close(
            rect.origin.y,
            layout.tabs_row.origin.y + row as f32 * 68.0,
            0.5,
        );
        assert_close(
            rect.origin.x,
            layout.tabs_row.origin.x + col as f32 * (85.25 + 5.0),
            0.5,
        );
        assert_min_touch(*rect, "task tile");
        assert_close(rect.size.y, 63.0, 0.5);
    }
    // The eighth slot (row 1, col 3) is the weakened blank tile.
    let blank = layout.new_canvas;
    assert_close(
        blank.origin.x,
        layout.tabs[4].origin.x + 3.0 * (85.25 + 5.0),
        0.5,
    );
    assert_close(blank.origin.y, layout.tabs_row.origin.y + 68.0, 0.5);
    assert_close(blank.size.y, 63.0, 0.5);
    // No 更多 affordance on the phone: all seven tasks are in the grid.
    assert_eq!(layout.more_button, Rect::ZERO);
    assert_eq!(layout.more_popover, Rect::ZERO);
}

#[test]
fn compact_top_bar_carries_the_two_way_mode_switch() {
    let state = compact_state();
    let layout = compact_layout(&state);
    // Both segments live inside the 56 pt bar and clear the 44 pt floor.
    for rect in [layout.mode_switch, layout.mode_normal, layout.professional] {
        assert!(rect.origin.y >= 0.0);
        assert!(rect.origin.y + rect.size.y <= HOME_TOPBAR_H);
        assert_min_touch(rect, "mode switch segment");
    }
    assert_close(layout.mode_normal.size.y, 44.0, 0.5);
    assert_close(layout.professional.size.y, 44.0, 0.5);
    assert!(
        layout.mode_normal.origin.x + layout.mode_normal.size.x < layout.professional.origin.x,
        "普通 sits left of 专业"
    );
    // The settings gear shares NavSettings and keeps the floor.
    assert_min_touch(layout.settings, "settings gear");
    assert!(
        layout.settings.origin.x > layout.professional.origin.x + layout.professional.size.x,
        "gear sits right of the switch"
    );
    // The desktop-only top bar targets are absent on the phone.
    assert_eq!(layout.open_file, Rect::ZERO);
    assert_eq!(layout.account, Rect::ZERO);

    // Hit-test: both halves answer, 专业 through the shared variant.
    let home = HomeSurface::for_editor(&state).expect("home visible");
    assert_eq!(
        home.hit_test(W, H, center(layout.mode_normal)),
        Some(HomeHit::ModeNormal)
    );
    assert_eq!(
        home.hit_test(W, H, center(layout.professional)),
        Some(HomeHit::Professional)
    );
    assert_eq!(
        home.hit_test(W, H, center(layout.settings)),
        Some(HomeHit::NavSettings)
    );
}

#[test]
fn compact_composer_keeps_every_target_at_the_touch_floor() {
    let layout = compact_layout(&compact_state());
    // The composer card spans the content column with prototype
    // metrics: 10 + 44 + 5 + 85 + 6 + 44 + 44 + 12 = 250 tall.
    // The card spans the content column; its 13 px padding is inner.
    assert_close(layout.composer.size.x, W - 34.0, 0.5);
    assert_close(layout.composer.size.y, 250.0, 0.5);
    assert_min_touch(layout.send, "开始设计");
    assert_min_touch(layout.model_chip, "model chip");
    assert_min_touch(layout.screenshot, "加图片");
    assert_min_touch(layout.reference_link, "加链接");
    assert_min_touch(layout.label_row, "label row");
    assert_close(layout.input_box.size.y, 85.0, 0.5);
    // The App task's segmented options (手机 / 电脑) fill the row.
    assert!(layout.segment.size.x > 0.0);
    for option in layout.segment_options.into_iter().take(2) {
        assert_min_touch(option, "segment option");
    }
    // No Figma tool on the phone, and no 3-directions toggle: the phone
    // reader pages one board at a time and has no side-by-side stage.
    assert_eq!(layout.figma, Rect::ZERO);
    assert_eq!(layout.variants, Rect::ZERO);
}

#[test]
fn compact_page_shows_one_featured_example_and_no_desktop_sections() {
    let layout = compact_layout(&compact_state());
    // ONE featured example card under its heading; the whole card is
    // the tap target.
    assert_close(layout.preview.size.y, 133.0, 0.5);
    assert_close(layout.preview.size.x, W - 34.0, 0.5);
    assert_eq!(layout.use_example, layout.preview);
    assert!(layout.preview.origin.y > layout.composer.origin.y + layout.composer.size.y);
    // The desktop-only lower sections are absent.
    for card in layout.explore_cards {
        assert_eq!(card, Rect::ZERO);
    }
    assert_eq!(layout.recent, Rect::ZERO);
    for chip in layout.recent_chips {
        assert_eq!(chip, Rect::ZERO);
    }
    // The featured card hits UseExample (or BackToWorkspace while a
    // workspace is active — same rect, same arm).
    let state = compact_state();
    let home = HomeSurface::for_editor(&state).expect("home visible");
    assert_eq!(
        home.hit_test(W, H, center(layout.preview)),
        Some(HomeHit::UseExample)
    );
    // The blank tile hits NewCanvas.
    assert_eq!(
        home.hit_test(W, H, center(layout.new_canvas)),
        Some(HomeHit::NewCanvas)
    );
    // A grid tile selects its task.
    assert_eq!(
        home.hit_test(W, H, center(layout.tabs[5])),
        Some(HomeHit::Tab(HomeFamily::Infographic))
    );
}

#[test]
fn compact_bottom_nav_is_three_pinned_items() {
    let state = compact_state();
    let layout = compact_layout(&state);
    assert_close(layout.bottom_nav.size.y, HOME_BOTTOM_NAV_H, 0.5);
    assert_close(
        layout.bottom_nav.origin.y + layout.bottom_nav.size.y,
        H,
        0.5,
    );
    for rect in layout.nav_items {
        assert_min_touch(rect, "nav item");
    }
    let home = HomeSurface::for_editor(&state).expect("home visible");
    assert_eq!(
        home.hit_test(W, H, center(layout.nav_items[0])),
        Some(HomeHit::NavCreate)
    );
    assert_eq!(
        home.hit_test(W, H, center(layout.nav_items[1])),
        Some(HomeHit::NavProjects)
    );
    assert_eq!(
        home.hit_test(W, H, center(layout.nav_items[2])),
        Some(HomeHit::NavSettings)
    );
    // The nav is pinned chrome: it wins over page content at the same
    // point.
    let bottom_of_page = Point2D::new(W / 2.0, H - 10.0);
    assert!(home.hit_test(W, H, bottom_of_page).is_some());
}

#[test]
fn compact_page_reports_scroll_only_when_content_overflows() {
    let state = compact_state();
    // 390×844: the page fits above the nav.
    let home = HomeSurface::for_editor(&state).expect("home visible");
    let fits = home.layout(W, H);
    let visible_bottom = H - HOME_BOTTOM_NAV_H;
    assert!(
        fits.preview.origin.y + fits.preview.size.y + 24.0 <= visible_bottom,
        "the compact page fits a 844 pt phone"
    );
    // A short viewport scrolls, bounded by the featured card's bottom.
    let short = home.layout(430.0, 360.0);
    assert!(short.preview.origin.y + short.preview.size.y > 360.0 - HOME_BOTTOM_NAV_H);
}

#[test]
fn the_desktop_composition_is_unchanged_by_the_compact_branch() {
    // The same HomeSurface API with the compact flag off must keep the
    // wide rect set (the untouched `home_surface_layout_tests.rs` prove
    // the numbers; here we prove the branch does not leak).
    let mut state = EditorState::new();
    state.editor_ui.home.visible = true;
    let home = HomeSurface::for_editor(&state).expect("home visible");
    let layout = home.layout(1440.0, 900.0);
    assert_eq!(layout.mode_switch, Rect::ZERO);
    assert_eq!(layout.mode_normal, Rect::ZERO);
    assert_eq!(layout.settings, Rect::ZERO);
    assert_eq!(layout.bottom_nav, Rect::ZERO);
    for item in layout.nav_items {
        assert_eq!(item, Rect::ZERO);
    }
    assert!(layout.recent.size.y > 0.0);
    assert!(layout.open_file.size.x > 0.0);
    // The static entry points stay wide even at phone widths: only
    // compact_layout() (touch + compact size class) picks the branch.
    let wide_at_390 = super::HomeSurface::layout_for(W, H, HomeFamily::AppUi, CHIP_W);
    assert!(wide_at_390.tabs_row.size.y > 0.0);
    assert_eq!(wide_at_390.bottom_nav, Rect::ZERO);
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn a_focused_composer_folds_the_page_around_the_input() {
    // Mobile spec: 聚焦输入时收起用途宫格、精选示例和主导航. The compact page
    // never read the focus flag, so the software keyboard covered Send while
    // the grid and the bottom nav kept their space.
    let resting = compact_layout(&compact_state());
    let mut state = compact_state();
    state.editor_ui.home.composer_focused = true;
    let focused = compact_layout(&state);

    assert_eq!(focused.tabs_row, Rect::ZERO, "the task grid folds away");
    assert!(focused.tabs.iter().all(|tab| *tab == Rect::ZERO));
    assert_eq!(focused.preview, Rect::ZERO, "the example folds away");
    assert_eq!(focused.bottom_nav, Rect::ZERO, "the bottom nav folds away");
    assert!(
        focused.composer.origin.y < resting.composer.origin.y,
        "the composer lifts toward the top"
    );
    assert!(focused.composer.origin.y >= resting.welcome_sub.origin.y + resting.welcome_sub.size.y);
    assert_eq!(
        focused.input_box.size, resting.input_box.size,
        "same composer, just higher"
    );

    let home = HomeSurface::for_editor(&state).expect("home visible");
    assert_eq!(
        home.hit_test(W, H, center(focused.send)),
        Some(HomeHit::Send),
        "Send hit-tests where it is painted"
    );
    assert_ne!(
        home.hit_test(W, H, center(resting.tabs[0])),
        Some(HomeHit::Tab(HomeFamily::AppUi)),
        "a folded tile can no longer be pressed"
    );
}

#[test]
fn the_phone_connect_card_fits_the_screen_and_offers_no_local_cli() {
    let layout = compact_layout(&compact_state());
    let card = layout.connect_card;
    assert!(
        card.origin.x >= 0.0 && card.origin.x + card.size.x <= W,
        "the card must fit a {W} pt screen: {card:?}"
    );
    assert_eq!(
        layout.connect_rows[2],
        Rect::ZERO,
        "a phone cannot run a local CLI agent"
    );
    let mut state = compact_state();
    state.editor_ui.home.connect_card_open = true;
    let home = HomeSurface::for_editor(&state).expect("home visible");
    assert_eq!(
        home.hit_test(W, H, center(layout.connect_rows[1])),
        Some(HomeHit::ConnectApiKey),
        "the remaining rows still answer"
    );
}

#[test]
fn a_host_without_a_variants_runner_offers_no_toggle() {
    let mut state = EditorState::new();
    state.editor_ui.home.visible = true;
    let wide = HomeSurface::for_editor(&state)
        .expect("home visible")
        .layout(1440.0, 900.0);
    assert!(wide.variants.size.x > 0.0, "desktop offers the toggle");
    state.editor_ui.home.variants_unavailable = true;
    let web = HomeSurface::for_editor(&state)
        .expect("home visible")
        .layout(1440.0, 900.0);
    assert_eq!(web.variants, Rect::ZERO);
}
