//! Tests for the template card's two actions and the basis chip they set.
//!
//! Kept beside the geometry rather than folded into the panel's tests: the
//! question these answer — "which of the two things a card can do did the
//! user just ask for" — is the one the double action introduced, and a reader
//! chasing a card that opened the wrong door should land in one file.

use super::{card_action_rects, card_add_hover_token};
use crate::widgets::press_flow::press_scene_template_center;
use crate::widgets::scene_template_panel::test_rects::MEDIUM as PANEL;
use crate::widgets::scene_template_panel::{SceneTemplateHit, SceneTemplatePanel};
use crate::{Point2D, Rect};

use op_editor_core::scene_template_catalog::{scene_template_catalogue, TemplateScene};
use op_editor_core::{EditorState, SceneFilter, SceneTemplateFocus};

/// A host that can run a generation, which is what the second button needs.
fn capable_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.scene_template_generate_supported = true;
    state.editor_ui.open_scene_template_center(0);
    state
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// Focus the grid on one template and return its card rect and index.
fn only_card(state: &mut EditorState, template_id: &str) -> (usize, Rect) {
    let title = scene_template_catalogue()
        .iter()
        .find(|t| t.id == template_id)
        .expect("the template ships")
        .title_fallback
        .clone();
    state
        .editor_ui
        .scene_template_center
        .search
        .set_text(&title);
    let panel = SceneTemplatePanel::for_editor(state).expect("open");
    assert_eq!(
        panel.filtered().len(),
        1,
        "the fixture needs exactly one card to aim at"
    );
    let (index, rect) = panel.card_rects(PANEL).into_iter().next().expect("a card");
    (index, rect)
}

/// The mapping is data, and a renamed guide in the corpus would turn it into
/// a pin the pipeline logs and ignores. This is the only place both the
/// catalogue and the registry are visible, so it is the only place the two
/// can be held to each other.
#[test]
fn every_catalogue_style_guide_names_a_real_registry_entry() {
    let registry = op_ai_skills::style_guide::style_guide_registry();
    for template in scene_template_catalogue() {
        let Some(name) = template.style_guide.as_deref() else {
            continue;
        };
        assert!(
            registry.iter().any(|guide| guide.name == name),
            "{} pins `{name}`, which is not in the style-guide registry",
            template.id
        );
    }
}

/// Generation is offered exactly where the pipeline can build the thing the
/// card is showing. A card template carrying a guide would paint a button
/// that returns a 400 px component.
#[test]
fn only_templates_the_pipeline_can_build_offer_generation() {
    let mut offered = 0;
    for template in scene_template_catalogue() {
        if template.generate_style_guide().is_some() {
            offered += 1;
            assert!(
                matches!(template.scene, TemplateScene::Slides | TemplateScene::Web),
                "{} offers generation for a scene with no design type",
                template.id
            );
        }
    }
    assert_eq!(
        offered, 16,
        "the thirteen 16:9 deck templates plus the three web pages carry a style guide"
    );
}

/// The picture, the title, and the primary button all mean the same thing —
/// the default action must not be something the user has to aim for.
#[test]
fn pressing_anywhere_but_the_second_button_adds_to_the_canvas() {
    let mut state = capable_state();
    let (index, card) = only_card(&mut state, "slide-deck");
    state.editor_ui.scene_template_center.hover = Some(index);

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let (add, generate) = card_action_rects(card, true);
    let generate = generate.expect("a deck template offers generation");

    let expected = Some(SceneTemplateHit::AddTemplateToCanvas("slide-deck".into()));
    for point in [
        centre(add),
        // The title block, below the preview and clear of the strip.
        Point2D::new(card.origin.x + 20.0, card.origin.y + card.size.y - 20.0),
    ] {
        assert_eq!(panel.hit_test(PANEL, point), expected);
    }
    assert_eq!(
        panel.hit_test(PANEL, centre(generate)),
        Some(SceneTemplateHit::GenerateFromTemplate("slide-deck".into()))
    );
}

/// Without hover there is no strip, so the whole card is the default action.
/// Paint and hit-test agree on this or the card grows an invisible dead zone.
#[test]
fn an_unhovered_card_has_no_button_to_hit() {
    let mut state = capable_state();
    let (_, card) = only_card(&mut state, "slide-deck");
    state.editor_ui.scene_template_center.hover = None;

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let (_, generate) = card_action_rects(card, true);
    assert_eq!(
        panel.hit_test(PANEL, centre(generate.expect("rect"))),
        Some(SceneTemplateHit::AddTemplateToCanvas("slide-deck".into()))
    );
}

/// A host with no generation chain — the web bundle — gets one button, and
/// the press that would have hit the second one still adds to the canvas.
#[test]
fn a_host_without_generation_paints_one_full_width_button() {
    let mut state = capable_state();
    state.editor_ui.scene_template_generate_supported = false;
    let (index, card) = only_card(&mut state, "slide-deck");
    state.editor_ui.scene_template_center.hover = Some(index);

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let template = panel.filtered()[index];
    assert!(!panel.card_offers_generate(template));

    let (add, generate) = card_action_rects(card, false);
    assert!(generate.is_none());
    assert_eq!(
        panel.hit_test(PANEL, centre(add)),
        Some(SceneTemplateHit::AddTemplateToCanvas("slide-deck".into()))
    );
}

/// A template with no style guide is the same shape as an incapable host.
#[test]
fn a_template_without_a_style_guide_offers_no_second_button() {
    let mut state = capable_state();
    let (index, _) = only_card(&mut state, "before-after");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert!(!panel.card_offers_generate(panel.filtered()[index]));
}

/// The strip must not blink out from under the pointer on its way to a
/// button — hovering a button is still hovering its card.
#[test]
fn hovering_a_button_keeps_its_own_strip_up() {
    let mut state = capable_state();
    let (index, _) = only_card(&mut state, "slide-deck");
    state.editor_ui.scene_template_center.hover = Some(card_add_hover_token(index));

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert!(panel.card_actions_visible(index));
}

/// The secondary button sets up the next request and touches nothing else:
/// the pin the pipeline reads, the filter, the focus, and a chip saying so.
#[test]
fn generating_from_a_template_pins_its_style_and_aims_the_row() {
    let mut state = capable_state();
    let (index, card) = only_card(&mut state, "minimal-keynote");
    state.editor_ui.scene_template_center.hover = Some(index);
    let (_, generate) = card_action_rects(card, true);
    let revision_before = state.revision;

    assert_eq!(
        press_scene_template_center(&mut state, PANEL, centre(generate.expect("rect")), 0),
        Some(true)
    );

    let ui = &state.editor_ui;
    assert_eq!(
        ui.pinned_style_guide.as_deref(),
        Some("portfolio-minimal-light")
    );
    assert_eq!(
        ui.scene_template_center.filter,
        SceneFilter::Scene(TemplateScene::Slides)
    );
    assert_eq!(ui.scene_template_center.focus, SceneTemplateFocus::Generate);
    assert_eq!(
        ui.scene_template_center.generate_basis.as_deref(),
        Some("minimal-keynote")
    );
    assert!(
        ui.scene_template_center.open,
        "the panel stays open — the row it points at is inside it"
    );
    assert!(
        ui.scene_template_center.pending_open.is_none(),
        "choosing a style must not also bring the template in"
    );
    assert_eq!(state.revision, revision_before, "the document is untouched");
}

/// Dismissing the chip has to clear the pin too. A chip that vanished while
/// the guide stayed pinned would steer every later generation invisibly.
#[test]
fn dismissing_the_basis_chip_unpins_the_style() {
    let mut state = capable_state();
    let (index, card) = only_card(&mut state, "gradient-tech");
    state.editor_ui.scene_template_center.hover = Some(index);
    let (_, generate) = card_action_rects(card, true);
    press_scene_template_center(&mut state, PANEL, centre(generate.expect("rect")), 0);
    assert!(state.editor_ui.pinned_style_guide.is_some());

    // Clear the search so the row is measured the way the user sees it.
    state.editor_ui.scene_template_center.search.set_text("");
    let dismiss = {
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        panel
            .basis_chip_dismiss_rect(PANEL)
            .expect("the chip is showing")
    };
    assert_eq!(
        press_scene_template_center(&mut state, PANEL, centre(dismiss), 0),
        Some(true)
    );

    assert!(state.editor_ui.pinned_style_guide.is_none());
    assert!(state
        .editor_ui
        .scene_template_center
        .generate_basis
        .is_none());
}

/// The chip shares the generate row with the topic field. It may narrow that
/// field; it may not sit on top of it or squeeze it out of existence.
#[test]
fn the_basis_chip_narrows_the_topic_field_without_overlapping_it() {
    let mut state = capable_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let bare_input = panel.generate_input_rect(PANEL).expect("the row paints");
    assert!(panel.basis_chip_rect(PANEL).is_none());

    state.editor_ui.scene_template_center.generate_basis = Some("minimal-keynote".into());
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let chip = panel.basis_chip_rect(PANEL).expect("the chip is showing");
    let input = panel.generate_input_rect(PANEL).expect("the row paints");
    let button = panel.generate_button_rect(PANEL).expect("the row paints");

    assert!(
        chip.origin.x + chip.size.x <= input.origin.x,
        "chip overlaps"
    );
    assert!(input.size.x > 0.0, "no room left to type");
    assert!(input.size.x < bare_input.size.x, "the chip took no room");
    assert!(
        input.origin.x + input.size.x <= button.origin.x,
        "the field runs into the generate button"
    );
}

/// A basis naming a template the catalogue no longer has reads as no basis:
/// the chip's job is to say which template, and it cannot.
#[test]
fn an_unresolvable_basis_paints_no_chip() {
    let mut state = capable_state();
    state.editor_ui.scene_template_center.generate_basis = Some("not-a-template".into());
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert!(panel.basis_chip_rect(PANEL).is_none());
}
