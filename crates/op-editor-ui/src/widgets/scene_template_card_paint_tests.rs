use super::*;
use op_editor_core::scene_template_catalog::scene_template_by_id;
use op_editor_core::EditorState;

use crate::widgets::scene_template_panel::test_rects::MEDIUM as PANEL;
use crate::widgets::test_capture_backend::CaptureBackend;

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.open_scene_template_center(0);
    state
}

fn first_card(state: &EditorState) -> Rect {
    SceneTemplatePanel::for_editor(state)
        .expect("open")
        .card_rects(PANEL)[0]
        .1
}

/// Picture, band, and caption tile the card exactly — no overlap, no gap, and
/// nothing outside the rounded frame.
#[test]
fn the_preview_block_and_caption_tile_the_card() {
    let state = open_state();
    let card = first_card(&state);
    let picture = SceneTemplatePanel::card_preview_rect(card);
    let band = SceneTemplatePanel::card_palette_rect(card);
    let block = SceneTemplatePanel::card_preview_block_rect(card);

    assert_eq!(band.origin.x, picture.origin.x);
    assert_eq!(band.size.x, picture.size.x);
    assert_eq!(
        band.origin.y,
        picture.origin.y + picture.size.y,
        "the band sits flush under the picture, not floating below it"
    );
    assert_eq!(block.origin.y + block.size.y, band.origin.y + band.size.y);
    assert_eq!(
        SceneTemplatePanel::card_caption_top(card),
        block.origin.y + block.size.y
    );
    assert!(
        block.origin.x >= card.origin.x
            && block.origin.x + block.size.x <= card.origin.x + card.size.x
            && block.origin.y + block.size.y < card.origin.y + card.size.y,
        "the preview block must stay inside its card"
    );
}

/// The picture is the card. A caption that grows until it competes with the
/// preview is the shape this redesign replaced.
#[test]
fn the_preview_block_dominates_the_card_at_every_breakpoint() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    for rect in [
        crate::widgets::scene_template_panel::test_rects::NARROW,
        PANEL,
        crate::widgets::scene_template_panel::test_rects::WIDE,
    ] {
        let (_, card_w, card_h) = panel.grid_metrics(rect);
        let card = Rect::xywh(0.0, 0.0, card_w, card_h);
        let block = SceneTemplatePanel::card_preview_block_rect(card);
        let share = block.size.y / card_h;
        assert!(
            share > 0.7,
            "the preview block is only {:.0}% of a {card_w}x{card_h} card",
            share * 100.0
        );
    }
}

/// The band paints the template's own colours, one stripe each, spanning the
/// full width — a seam at the right edge shows the card through the palette.
#[test]
fn the_palette_band_paints_one_stripe_per_declared_colour() {
    let state = open_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let template = scene_template_by_id("minimal-keynote").expect("shipped");
    let card = Rect::xywh(0.0, 0.0, 420.0, 320.0);
    let band = SceneTemplatePanel::card_palette_rect(card);
    let palette = scene_template_palette(&template.id);

    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint_card_palette(&mut cx, band, template);

    let stripes: Vec<_> = backend
        .rect_fills
        .iter()
        .filter(|(rect, _)| rect.origin.y == band.origin.y)
        .collect();
    assert_eq!(stripes.len(), palette.len());
    let (first, _) = stripes[0];
    let (last, _) = stripes[stripes.len() - 1];
    assert_eq!(first.origin.x, band.origin.x);
    assert_eq!(
        last.origin.x + last.size.x,
        band.origin.x + band.size.x,
        "the last stripe must reach the band's right edge"
    );
    let expected = parse_swatch(&palette[0]).expect("a canonical hex");
    assert_eq!(
        stripes[0].1, expected,
        "the first stripe is the first colour"
    );
}

/// The long summary is hover-only: it is the one piece of per-card prose, and
/// paying for it on all forty resting cards is what made the old caption four
/// lines tall.
#[test]
fn the_summary_paints_only_while_the_card_is_hovered() {
    // The grid is index-based and the registry is process-global: another
    // test's saved template would claim index 0 and this shipped card's
    // hover strip would never paint.
    let _guard = crate::widgets::asset_center_template_cards::template_test_support::exclusive_user_templates();
    let template = scene_template_by_id("minimal-keynote").expect("shipped");
    let card = Rect::xywh(0.0, 0.0, 420.0, 320.0);

    let resting = open_state();
    let painted_at_rest = summary_scrim_count(&resting, card, template);
    assert_eq!(
        painted_at_rest, 0,
        "a resting card must not carry its description"
    );

    let mut hovered = open_state();
    hovered.editor_ui.scene_template_center.hover = Some(0);
    assert!(
        summary_scrim_count(&hovered, card, template) > 0,
        "hovering must reveal it"
    );
}

/// How many fills the hover scrim contributed — it is the only fill painted
/// in that colour, so counting it is the cheapest way to see the summary.
fn summary_scrim_count(
    state: &EditorState,
    card: Rect,
    template: &'static op_editor_core::scene_template_catalog::SceneTemplateDefinition,
) -> usize {
    let panel = SceneTemplatePanel::for_editor(state).expect("open");
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint_card(&mut cx, card, template, 0);
    backend
        .rect_fills
        .iter()
        .filter(|(_, color)| *color == HOVER_SCRIM)
        .count()
}
