use super::{HomePalette, HomeSurface};
use crate::{Color, Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

#[test]
fn home_layout_matches_the_centered_reference_stack_at_1440x900() {
    let layout = HomeSurface::layout_for(1440.0, 900.0, None);
    assert_close(layout.headline.origin.y, 162.0);
    assert_close(layout.sheet.origin.y, 280.0);
    assert_eq!(layout.sheet.size, Point2D::new(720.0, 150.0));
    assert_close(layout.chips[0].origin.y, 448.0);
    assert_close(layout.expected.origin.y, 500.0);
    assert_close(layout.cards[0].origin.y, 566.0);
    assert_eq!(layout.cards[0].size, Point2D::new(280.0, 200.0));
    assert_eq!(layout.cards[0].origin.y, layout.cards[1].origin.y);
    assert!(
        layout.expected.size.y > 0.0,
        "empty expected row is reserved"
    );
    assert_no_overlaps(&layout);
}

#[test]
fn home_cards_wrap_two_by_two_below_1180() {
    let layout = HomeSurface::layout_for(1180.0, 760.0, Some(HomeFamily::AppUi));
    assert!(layout.cards[0].origin.y == layout.cards[1].origin.y);
    assert!(layout.cards[2].origin.y > layout.cards[0].origin.y);
    assert!(layout.cards[0].origin.x < layout.cards[1].origin.x);
    assert!(layout.cards[3].origin.y + layout.cards[3].size.y <= layout.footer.origin.y);
    assert_no_overlaps(&layout);
}

#[test]
fn home_stack_reports_scroll_when_the_narrow_viewport_is_short() {
    let max_scroll = HomeSurface::max_scroll_for(1180.0, 620.0, Some(HomeFamily::AppUi));
    assert!(max_scroll > 0.0);
    let scrolled =
        HomeSurface::layout_for_scrolled(1180.0, 620.0, Some(HomeFamily::AppUi), max_scroll);
    let unscrolled = HomeSurface::layout_for(1180.0, 620.0, Some(HomeFamily::AppUi));
    assert_eq!(scrolled.footer, unscrolled.footer);
    assert!(scrolled.cards[2].origin.y < unscrolled.cards[2].origin.y);
    assert!(scrolled.cards[3].origin.y + scrolled.cards[3].size.y <= scrolled.footer.origin.y);
}

#[test]
fn home_palette_owns_the_exact_light_and_dark_tokens() {
    let light = HomePalette::light();
    assert_hex(light.paper, "F4F1EA");
    assert_hex(light.paper_2, "EDE8DE");
    assert_hex(light.sheet, "FFFDF9");
    assert_hex(light.ink, "1B1A17");
    assert_hex(light.graphite, "5E5A52");
    assert_hex(light.ash, "9B958A");
    assert_hex(light.line, "D9D3C6");
    assert_hex(light.blue, "2F3FD1");
    assert_hex(light.blue_2, "2533A8");
    assert_hex(light.blue_soft, "E4E7FA");

    let dark = HomePalette::dark();
    assert_hex(dark.paper, "1E1C18");
    assert_hex(dark.paper_2, "262320");
    assert_hex(dark.sheet, "2B2925");
    assert_hex(dark.ink, "F1EDE4");
    assert_hex(dark.graphite, "B8B1A5");
    assert_hex(dark.ash, "7E786D");
    assert_hex(dark.line, "3A3630");
    assert_hex(dark.blue, "5B6CFF");
    assert_hex(dark.blue_2, "7482FF");
    assert_hex(dark.blue_soft, "2A2E52");
    assert_eq!(dark.dots.a, 0.10);
    assert_eq!(dark.margin_rule.a, 0.35);
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

#[test]
fn home_card_tag_label_distinguishes_the_app_flow_card() {
    assert_eq!(
        super::paint::card_tag_label(HomeFamily::AppUi),
        "示例 · 三屏"
    );
    for family in HomeFamily::ALL {
        if family != HomeFamily::AppUi {
            assert_eq!(super::paint::card_tag_label(family), "示例");
        }
    }
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
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

fn assert_no_overlaps(layout: &super::HomeLayout) {
    let ordered = [
        layout.headline,
        layout.subtitle,
        layout.sheet,
        layout.chips[0],
        layout.expected,
        layout.cards[0],
    ];
    for pair in ordered.windows(2) {
        assert!(
            !overlaps(pair[0], pair[1]),
            "blocks overlap: {:?} / {:?}",
            pair[0],
            pair[1]
        );
    }
    for card in layout.cards {
        assert!(!overlaps(card, layout.sheet));
        assert!(!overlaps(card, layout.expected));
        assert!(!overlaps(card, layout.footer));
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}
