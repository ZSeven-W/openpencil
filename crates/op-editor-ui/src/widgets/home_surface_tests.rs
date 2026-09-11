use super::HomeSurface;
use crate::{Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

#[test]
fn home_layout_is_compact_and_non_overlapping_at_reference_desktop_size() {
    let layout = HomeSurface::layout_for(1440.0, 900.0, None);
    assert_eq!(layout.sheet.size.x, 720.0);
    assert!(layout.headline.origin.y < layout.sheet.origin.y);
    assert!(!overlaps(layout.sheet, layout.chips[0]));
    assert!(!layout
        .cards
        .iter()
        .any(|card| overlaps(*card, layout.sheet)));
    assert_eq!(layout.cards.len(), 4);
    assert!(layout.cards[0].origin.y == layout.cards[1].origin.y);
}

#[test]
fn home_cards_wrap_two_by_two_below_1180() {
    let layout = HomeSurface::layout_for(1180.0, 760.0, Some(HomeFamily::AppUi));
    assert!(layout.cards[0].origin.y == layout.cards[1].origin.y);
    assert!(layout.cards[2].origin.y > layout.cards[0].origin.y);
    assert!(layout.cards[0].origin.x < layout.cards[1].origin.x);
    for card in layout.cards {
        assert!(!overlaps(card, layout.sheet));
        assert!(!overlaps(card, layout.expected));
    }
}

#[test]
fn home_hit_test_resolves_chip_and_send() {
    let state = op_editor_core::EditorState::new();
    let mut state = state;
    state.editor_ui.home.visible = true;
    let home = HomeSurface::for_editor(&state).expect("home");
    let layout = home.layout(1440.0, 900.0);
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.chips[0])),
        Some(HomeHit::Chip(HomeFamily::AppUi))
    );
    assert_eq!(
        home.hit_test(1440.0, 900.0, center(layout.send)),
        Some(HomeHit::Send)
    );
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}
