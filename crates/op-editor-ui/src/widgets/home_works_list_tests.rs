//! Tests for the phone 作品 page: what it lists (only what the state
//! has) and that its rects answer the hit-test the paint pass draws.

use super::*;
use op_editor_core::size_class::EditorSizeClass;

const W: f32 = 390.0;
const H: f32 = 844.0;

#[test]
fn phone_works_open_file_is_visible_clickable_and_separate_from_work_rows() {
    for (width, height) in [
        (280.0, 568.0),
        (320.0, 568.0),
        (390.0, 844.0),
        (430.0, 932.0),
    ] {
        for current in [false, true] {
            let mut state = EditorState::starter();
            phone(&mut state);
            if current {
                state
                    .editor_ui
                    .workspace
                    .open_for_reading(op_editor_core::HomeFamily::Presentation, 1);
            }
            with_recents(&mut state, 6);
            let surface = HomeSurface::for_editor(&state).unwrap();
            let works = surface.works_layout(width, height);
            let layout = surface.layout(width, height);
            assert_eq!(works.open_file, layout.open_file);
            assert!(works.open_file.size.x >= 44.0 && works.open_file.size.y >= 44.0);
            assert!(
                works.open_file.origin.x >= 0.0
                    && works.open_file.origin.x + works.open_file.size.x <= width
            );
            assert!(works.heading.origin.x + works.heading.size.x <= works.open_file.origin.x);
            assert_eq!(
                surface.hit_test(width, height, center(works.open_file)),
                Some(HomeHit::OpenFile)
            );
            if let Some(row) = works.current {
                assert!(works.open_file.origin.y + works.open_file.size.y < row.origin.y);
            }
        }
    }
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn phone(state: &mut EditorState) {
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.home.visible = true;
    state.editor_ui.home.works_open = true;
}

fn with_recents(state: &mut EditorState, count: usize) {
    state.editor_ui.recent_files = (0..count)
        .map(|i| op_editor_core::RecentFile {
            path: format!("/Users/me/Documents/作品 {i}.op"),
            modified_at: 0,
        })
        .collect();
}

#[test]
fn an_untouched_starter_is_not_a_work() {
    let mut state = EditorState::starter();
    phone(&mut state);
    assert!(CurrentWork::for_editor(&state).is_none());
    let home = HomeSurface::for_editor(&state).unwrap();
    let layout = home.works_layout(W, H);
    assert!(layout.current.is_none());
    assert!(layout.rows.is_empty());
    assert!(layout.empty.is_some(), "nothing to list says so");
}

#[test]
fn an_active_workspace_is_the_current_work() {
    let mut state = EditorState::starter();
    phone(&mut state);
    state
        .editor_ui
        .workspace
        .open_for_reading(op_editor_core::HomeFamily::Presentation, 1);
    state.editor_ui.workspace.brief = "产品介绍".into();
    let work = CurrentWork::for_editor(&state).expect("workspace work");
    assert_eq!(work.title, "产品介绍");
    let home = HomeSurface::for_editor(&state).unwrap();
    let layout = home.works_layout(W, H);
    assert!(layout.empty.is_none());
    assert_eq!(
        home.hit_test(W, H, center(layout.current.unwrap())),
        Some(HomeHit::WorksCurrent)
    );
}

#[test]
fn recent_rows_map_to_recent_file_indices_and_stop_above_the_nav() {
    let mut state = EditorState::starter();
    phone(&mut state);
    with_recents(&mut state, 20);
    let home = HomeSurface::for_editor(&state).unwrap();
    assert_eq!(home.works_recent.len(), WORKS_RECENT_CAP);
    assert_eq!(home.works_recent[3], "作品 3");
    let layout = home.works_layout(W, H);
    assert!(!layout.rows.is_empty());
    let nav_top = H - crate::widgets::home_surface::HOME_BOTTOM_NAV_H;
    for (index, row) in layout.rows.iter().enumerate() {
        assert!(row.size.y >= 44.0);
        assert!(
            row.origin.y + row.size.y <= nav_top,
            "row {index} hides under the nav"
        );
        assert_eq!(
            home.hit_test(W, H, center(*row)),
            Some(HomeHit::WorksRecent(index))
        );
    }
}

#[test]
fn the_pinned_chrome_still_answers_over_the_works_page() {
    let mut state = EditorState::starter();
    phone(&mut state);
    let home = HomeSurface::for_editor(&state).unwrap();
    let chrome = home.layout(W, H);
    assert_eq!(
        home.hit_test(W, H, center(chrome.nav_items[0])),
        Some(HomeHit::NavCreate)
    );
    assert_eq!(
        home.hit_test(W, H, center(chrome.professional)),
        Some(HomeHit::Professional)
    );
    // The 创作 page's composer is not under the works page.
    assert_eq!(home.hit_test(W, H, center(chrome.input_box)), None);
}

#[test]
fn the_works_page_is_phone_only() {
    let mut state = EditorState::starter();
    phone(&mut state);
    state.editor_ui.size_class = EditorSizeClass::Expanded;
    let home = HomeSurface::for_editor(&state).unwrap();
    assert!(!home.works_page());
}
