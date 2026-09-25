//! Touch-tablet Home: the wide composition with tablet margins, the 作品
//! grid in place of the chip row, the touch slop, and the scroll range.

use super::super::HomeSurface;
use super::*;
use op_editor_core::size_class::size_class;
use op_editor_core::{EditorState, RecentFile};

const TABLETS: [(f32, f32); 6] = [
    (1024.0, 1366.0),
    (1366.0, 1024.0),
    (820.0, 1180.0),
    (800.0, 1280.0),
    (1280.0, 800.0),
    (768.0, 1024.0),
];

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn tablet_home(w: f32, h: f32, recents: usize) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class(w, h);
    state.editor_ui.home.visible = true;
    state.editor_ui.recent_files = (0..recents)
        .map(|i| RecentFile {
            path: format!("/Documents/work-{i}.op"),
            modified_at: 1_700_000_000 - i as u64,
        })
        .collect();
    state
}

#[test]
fn tablets_take_the_wide_page_with_real_margins() {
    for (w, h) in TABLETS {
        let state = tablet_home(w, h, 0);
        assert!(is_touch_tablet(&state.editor_ui), "{w}×{h}");
        let home = HomeSurface::for_editor(&state).expect("home");
        let layout = home.layout(w, h);
        assert_eq!(layout.bottom_nav, Rect::ZERO, "{w}×{h}: not the phone page");
        assert!(
            layout.composer.origin.x >= TABLET_PAD_X - 0.5,
            "{w}×{h}: content glued to the bezel at {}",
            layout.composer.origin.x
        );
        let right_edge = layout.explore_cards[2].origin.x + layout.explore_cards[2].size.x;
        assert!(right_edge <= w - TABLET_PAD_X + 0.5, "{w}×{h}");
    }
    // A desktop window of the same size keeps its own 8 px shell gutter.
    let mut desktop = tablet_home(1024.0, 1366.0, 0);
    desktop.editor_ui.touch = false;
    let home = HomeSurface::for_editor(&desktop).unwrap();
    assert!(home.layout(1024.0, 1366.0).composer.origin.x < TABLET_PAD_X);
}

#[test]
fn the_works_grid_replaces_the_chip_row_with_touch_sized_cards() {
    for (w, h) in TABLETS {
        let state = tablet_home(w, h, 7);
        let home = HomeSurface::for_editor(&state).unwrap();
        let layout = home.layout(w, h);
        assert!(layout.recent_chips.iter().all(|chip| *chip == Rect::ZERO));
        assert!(layout.new_canvas.size.y >= 44.0, "{w}×{h}: new canvas");
        let grid = home.works_grid(&layout).expect("tablet grid");
        assert_eq!(grid.cards.len(), 7, "{w}×{h}: every recent document");
        for (hit, card) in &grid.cards {
            assert!(card.size.x >= 44.0 && card.size.y >= 44.0);
            assert!(card.origin.y >= grid.heading.origin.y + grid.heading.size.y);
            assert_eq!(home.hit_test(w, h, center(*card)), Some(*hit), "{w}×{h}");
        }
        // The grid is the page's last band: the scroll range reaches it.
        let last = grid.cards.last().unwrap().1;
        let max = home.tablet_max_scroll(w, h);
        assert!(
            last.origin.y + last.size.y - max <= h,
            "{w}×{h}: last card reachable"
        );
    }
}

#[test]
fn the_grid_lists_the_current_work_first_and_a_note_when_empty() {
    let grid = works_grid(28.0, 900.0, 968.0, true, 3);
    let hits: Vec<HomeHit> = grid.cards.iter().map(|(hit, _)| *hit).collect();
    assert_eq!(
        hits,
        [
            HomeHit::WorksCurrent,
            HomeHit::WorksRecent(0),
            HomeHit::WorksRecent(1),
            HomeHit::WorksRecent(2),
        ]
    );
    // Three columns at this width: the fourth card wraps to row two.
    assert!(grid.cards[3].1.origin.y > grid.cards[0].1.origin.y);
    let empty = works_grid(28.0, 900.0, 968.0, false, 0);
    assert!(empty.cards.is_empty() && empty.empty.is_some());
    let wide = works_grid(28.0, 900.0, 1310.0, false, 8);
    assert_eq!(
        wide.cards[3].1.origin.y, wide.cards[0].1.origin.y,
        "4 columns"
    );
}

#[test]
fn small_desktop_targets_get_a_touch_slop_on_tablets() {
    let (w, h) = (1024.0, 1366.0);
    let state = tablet_home(w, h, 0);
    let home = HomeSurface::for_editor(&state).unwrap();
    let layout = home.layout(w, h);
    for (rect, hit) in [
        (layout.open_file, HomeHit::OpenFile),
        (layout.professional, HomeHit::Professional),
        (layout.model_chip, HomeHit::ModelChip),
        (layout.screenshot, HomeHit::Attachment),
    ] {
        assert!(rect.size.y < 44.0, "the painted target is desktop-sized");
        let c = center(rect);
        for dy in [-21.0, 21.0] {
            let probe = Point2D::new(c.x, c.y + dy);
            assert_eq!(home.hit_test(w, h, probe), Some(hit), "{hit:?} at dy {dy}");
        }
    }
    assert_eq!(touch_slop(Rect::xywh(10.0, 10.0, 60.0, 24.0)).size.y, 44.0);
    assert_eq!(touch_slop(Rect::xywh(10.0, 10.0, 60.0, 50.0)).size.y, 50.0);
}

#[test]
fn a_scrolled_wide_page_keeps_explore_below_the_panels() {
    // The explore / recent rows used to add the scroll twice, sliding up
    // over the composer as soon as the page scrolled at all.
    for touch in [true, false] {
        let mut state = tablet_home(1366.0, 700.0, 6);
        state.editor_ui.touch = touch;
        state.editor_ui.home.scroll_y = 120.0;
        let home = HomeSurface::for_editor(&state).unwrap();
        let layout = home.layout(1366.0, 700.0);
        let panels_bottom = (layout.composer.origin.y + layout.composer.size.y)
            .max(layout.preview.origin.y + layout.preview.size.y);
        assert!(
            layout.explore_heading.origin.y >= panels_bottom,
            "touch {touch}: explore {} over panels {panels_bottom}",
            layout.explore_heading.origin.y
        );
        let unscrolled = {
            let mut top = state.clone();
            top.editor_ui.home.scroll_y = 0.0;
            HomeSurface::for_editor(&top).unwrap().layout(1366.0, 700.0)
        };
        let moved = unscrolled.explore_heading.origin.y - layout.explore_heading.origin.y;
        let panels_moved = unscrolled.composer.origin.y - layout.composer.origin.y;
        assert!(
            (moved - panels_moved).abs() < 0.5,
            "touch {touch}: one scroll"
        );
    }
}
