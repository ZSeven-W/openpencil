//! Layout-contract tests for the Studio Home surface at the reference
//! viewports: 1440×900 (the founder-approved screenshot), 1180×820, and
//! the 700 px compact tab row.

use super::{HomeLayout, HomeSurface, EXPLORE_FAMILIES, HOME_TOPBAR_H};
use crate::{Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

const CHIP_W: f32 = 120.0;

fn layout(w: f32, h: f32, task: HomeFamily) -> super::HomeLayout {
    HomeSurface::layout_for(w, h, task, CHIP_W)
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

#[test]
fn layout_at_1440_matches_the_approved_composition() {
    let layout = layout(1440.0, 900.0, HomeFamily::AppUi);
    // Content column: 1296 shell centred + 8 px padding → x 80, w 1280.
    assert_close(layout.tabs_row.origin.x, 80.0, 0.5);
    assert_close(layout.tabs_row.size.x, 1280.0, 0.5);
    // Seven equal 60 px tabs with 12 px gaps: (1280 − 72) / 7 ≈ 172.57.
    for rect in layout.tabs {
        assert_close(rect.size.x, 172.57, 0.5);
        assert_close(rect.size.y, 60.0, 0.5);
    }
    assert_close(
        layout.tabs[1].origin.x - layout.tabs[0].origin.x,
        184.57,
        0.5,
    );
    // Both panels: same y, both 370 tall, 18 px column gap, ≈1.05:1.
    assert_eq!(layout.composer.origin.y, layout.preview.origin.y);
    assert_close(layout.composer.size.y, 370.0, 0.5);
    assert_close(layout.preview.size.y, 370.0, 0.5);
    assert_close(
        layout.preview.origin.x - (layout.composer.origin.x + layout.composer.size.x),
        18.0,
        0.5,
    );
    let ratio = layout.composer.size.x / layout.preview.size.x;
    assert!(
        (ratio - 1.05).abs() < 0.01,
        "composer:preview = {} must be ≈1.05",
        ratio
    );
    // The composer internals stack inside the panel (the wide variant's
    // 41 px label row leaves a 199 px input box).
    assert!(inside(layout.label_row, layout.composer));
    assert!(inside(layout.input_box, layout.composer));
    assert!(inside(layout.tools_row, layout.composer));
    assert!(inside(layout.submit_row, layout.composer));
    assert!(inside(layout.send, layout.composer));
    assert!(inside(layout.model_chip, layout.composer));
    assert!(layout.input_box.size.y >= 130.0, "min input height");
    assert_close(layout.label_row.size.y, 41.0, 0.5);
    assert_close(
        layout.label_row.origin.y,
        layout.composer.origin.y + 16.0,
        0.5,
    );
    // The preview internals stay inside the preview panel (wide
    // heading is 59 tall).
    assert!(inside(layout.preview_heading, layout.preview));
    assert!(inside(layout.preview_art, layout.preview));
    assert!(inside(layout.preview_footer, layout.preview));
    assert!(inside(layout.use_example, layout.preview));
    assert_close(layout.preview_heading.size.y, 59.0, 0.5);
    // Three explore cards, 167 tall on short windows, 16 px gaps.
    for (index, card) in layout.explore_cards.iter().enumerate() {
        assert_close(card.size.y, 167.0, 0.5);
        if index > 0 {
            assert_close(
                card.origin.x
                    - layout.explore_cards[index - 1].origin.x
                    - layout.explore_cards[index - 1].size.x,
                16.0,
                0.5,
            );
            assert_eq!(card.origin.y, layout.explore_cards[index - 1].origin.y);
        }
    }
    assert_close(
        layout.explore_cards[2].origin.x + layout.explore_cards[2].size.x
            - layout.tabs_row.origin.x,
        1280.0,
        1.0,
    );
    // The recent row sits under the cards with its hairline + padding,
    // and the whole page fits under the 949 px reference window.
    assert!(
        layout.recent.origin.y > layout.explore_cards[0].origin.y + layout.explore_cards[0].size.y
    );
    assert_close(layout.recent.origin.y + layout.recent.size.y, 938.0, 1.0);
    assert!(
        layout.recent.origin.y + layout.recent.size.y <= 949.0,
        "explore + recent must fit under 949 px at 1440"
    );
    assert!(inside(layout.new_canvas, layout.recent_row_grown()));
    // No overlaps between the page's major blocks.
    let blocks = [
        layout.welcome,
        layout.tabs_row,
        layout.composer,
        layout.preview,
        layout.explore_heading,
        layout.explore_cards[0],
        layout.recent,
    ];
    for pair in blocks.windows(2) {
        assert!(
            !overlaps(pair[0], pair[1]),
            "{:?} overlaps {:?}",
            pair[0],
            pair[1]
        );
    }
    assert!(!overlaps(layout.composer, layout.preview));
    // The top bar buttons sit inside the 56 px bar.
    for button in [layout.open_file, layout.professional] {
        assert!(button.origin.y >= 0.0);
        assert!(button.origin.y + button.size.y <= HOME_TOPBAR_H);
        assert_eq!(button.size.y, 38.0);
    }
}

impl super::HomeLayout {
    // Test-only: the recent row's clickable band is the 50 px row plus
    // its 13 px padding above the hairline.
    fn recent_row_grown(&self) -> Rect {
        Rect::xywh(
            self.recent.origin.x,
            self.recent.origin.y - 13.0,
            self.recent.size.x,
            self.recent.size.y + 13.0,
        )
    }
}

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.size.x <= outer.origin.x + outer.size.x
        && inner.origin.y + inner.size.y <= outer.origin.y + outer.size.y
}

#[test]
fn layout_at_1180_keeps_the_same_contracts_on_a_1164_column() {
    let layout = layout(1180.0, 820.0, HomeFamily::AppUi);
    assert_close(layout.tabs_row.origin.x, 8.0, 0.5);
    assert_close(layout.tabs_row.size.x, 1164.0, 0.5);
    // The ≤1180 tab gap drops to 8: (1164 − 48) / 7 ≈ 159.4, tabs stay
    // 60 px tall.
    for rect in layout.tabs {
        assert_close(rect.size.x, 159.43, 0.5);
        assert_close(rect.size.y, 60.0, 0.5);
    }
    assert!(
        layout.more_button.size.x == 0.0,
        "no 更多 button on wide rows"
    );
    assert_eq!(layout.composer.origin.y, layout.preview.origin.y);
    assert_close(layout.composer.size.y, 370.0, 0.5);
    assert_close(layout.preview.size.y, 370.0, 0.5);
    let ratio = layout.composer.size.x / layout.preview.size.x;
    assert!((ratio - 1.05).abs() < 0.01);
    for card in layout.explore_cards {
        assert_close(card.size.y, 167.0, 0.5);
    }
}

#[test]
fn narrow_viewports_stack_the_panels() {
    let layout = layout(840.0, 900.0, HomeFamily::AppUi);
    // Composer over preview, both full-width, 335 + 400 tall (the wide
    // variant's stacked heights).
    assert!(layout.preview.origin.y > layout.composer.origin.y + layout.composer.size.y);
    assert_close(layout.composer.size.y, 335.0, 0.5);
    assert_close(layout.preview.size.y, 400.0, 0.5);
    assert_close(layout.composer.origin.x, layout.preview.origin.x, 0.5);
    assert_close(layout.composer.size.x, layout.preview.size.x, 0.5);
}

#[test]
fn compact_rows_show_four_tabs_and_a_more_button() {
    // 700 px viewport → 684 px content < 700 → compact row.
    let layout = layout(700.0, 900.0, HomeFamily::AppUi);
    assert!(layout.more_button.size.x > 0.0);
    for index in 0..4 {
        assert!(layout.tabs[index].size.x > 0.0, "tab {index} visible");
    }
    for index in 4..7 {
        assert!(layout.tabs[index].size.x == 0.0, "tab {index} hidden");
    }
    assert_eq!(
        HomeLayout::hidden_tasks(684.0, HomeFamily::AppUi).len(),
        3,
        "knowledge / tutorial / poster are in the popover"
    );
}

#[test]
fn a_hidden_selected_task_takes_the_fourth_slot() {
    let mut state = op_editor_core::EditorState::new();
    state.editor_ui.home.visible = true;
    state
        .editor_ui
        .home
        .set_task(HomeFamily::Infographic, 1_000);
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(700.0, 900.0);
    // The selected hidden task is visible in the fourth slot.
    assert!(layout.tabs[0].size.x > 0.0, "app");
    assert!(layout.tabs[1].size.x > 0.0, "web");
    assert!(layout.tabs[2].size.x > 0.0, "presentation");
    assert!(layout.tabs[5].size.x > 0.0, "infographic takes a slot");
    assert!(
        layout.tabs[3].size.x == 0.0,
        "knowledge bumped to the popover"
    );
    // Hitting the fourth visible tab selects the infographic task.
    let fourth = layout
        .tabs
        .iter()
        .find(|rect| rect.size.x > 0.0 && rect.origin.x > layout.tabs[2].origin.x)
        .copied()
        .expect("fourth visible tab");
    assert_eq!(
        home.hit_test(
            700.0,
            900.0,
            Point2D::new(
                fourth.origin.x + fourth.size.x / 2.0,
                fourth.origin.y + fourth.size.y / 2.0
            )
        ),
        Some(HomeHit::Tab(HomeFamily::Infographic))
    );
    // The popover rows hit their hidden families.
    let hidden = HomeLayout::hidden_tasks(684.0, HomeFamily::Infographic);
    assert_eq!(
        hidden,
        vec![
            HomeFamily::KnowledgeCards,
            HomeFamily::ScreenshotTutorial,
            HomeFamily::EventPoster
        ]
    );
    assert_eq!(
        EXPLORE_FAMILIES,
        [
            HomeFamily::KnowledgeCards,
            HomeFamily::ScreenshotTutorial,
            HomeFamily::EventPoster
        ]
    );
}

#[test]
fn the_stack_reports_scroll_when_the_viewport_is_short() {
    let max_scroll = HomeSurface::max_scroll_for(1180.0, 620.0, HomeFamily::AppUi, CHIP_W);
    assert!(max_scroll > 0.0);
    let scrolled =
        HomeSurface::layout_for_scrolled(1180.0, 620.0, HomeFamily::AppUi, max_scroll, CHIP_W);
    let unscrolled = HomeSurface::layout_for(1180.0, 620.0, HomeFamily::AppUi, CHIP_W);
    // The top bar rect is pinned; the recent row scrolls up.
    assert_eq!(scrolled.open_file, unscrolled.open_file);
    assert!(scrolled.recent.origin.y < unscrolled.recent.origin.y);
    assert!(scrolled.recent.origin.y + scrolled.recent.size.y <= 620.0);
}

/// Home is where a first-run user lands, so the account entry has to be
/// here and not only on the professional canvas. It sits left of 打开文件
/// as a square, clear of both buttons.
#[test]
fn the_top_bar_carries_the_account_entry_left_of_the_file_button() {
    let layout = layout(1440.0, 900.0, HomeFamily::AppUi);
    assert_close(layout.account.size.x, 38.0, 0.5);
    assert_close(layout.account.size.y, 38.0, 0.5);
    assert_eq!(layout.account.origin.y, layout.open_file.origin.y);
    assert!(
        layout.account.origin.x + layout.account.size.x < layout.open_file.origin.x,
        "the avatar clears 打开文件: {:?} vs {:?}",
        layout.account,
        layout.open_file
    );
    assert!(layout.open_file.origin.x + layout.open_file.size.x < layout.professional.origin.x);
    assert!(layout.account.origin.x > 0.0, "still on screen at 1440");
}
