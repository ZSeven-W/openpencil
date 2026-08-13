//! The Asset Center's press and hover ladders.
//!
//! Split from the panel spine at the file-size cap. The two walk the same
//! order for the same reason the host press ladder does: hit-test runs in
//! reverse paint order, so the topmost layer answers first. Keeping them in
//! one file keeps that order readable as a single sequence — a control that
//! hovers but does not press, or presses but does not hover, is the failure
//! this adjacency exists to make obvious.

use op_editor_core::AssetCenterTab;

use super::scene_template_card_actions::SCENE_TEMPLATE_BASIS_CHIP_HOVER;
use super::scene_template_panel::{
    filter_hover_token, tab_hover_token, SceneTemplateHit, SceneTemplatePanel,
    GENERATE_INPUT_PAD_X, GENERATE_TEXT_SIZE, SCENE_TEMPLATE_CLOSE_HOVER,
    SCENE_TEMPLATE_GENERATE_HOVER, SEARCH_PAD_X, SEARCH_TEXT_SIZE,
};
use super::scene_template_style_geometry::SCENE_TEMPLATE_IMPORT_HOVER;
use super::scene_template_style_import::{
    STYLE_IMPORT_CANCEL_HOVER, STYLE_IMPORT_CONFIRM_HOVER, STYLE_IMPORT_PICK_HOVER,
};
use crate::{Point2D, Rect};

impl SceneTemplatePanel<'_> {
    /// Resolve a pointer to a hover token shared with paint.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<usize> {
        if !panel.contains(point) {
            return None;
        }
        // The paste box covers the gallery: only its own controls light up.
        if self.style_import_open() {
            if self.style_import_confirm_rect(panel).contains(point) {
                return Some(STYLE_IMPORT_CONFIRM_HOVER);
            }
            if self.style_import_cancel_rect(panel).contains(point) {
                return Some(STYLE_IMPORT_CANCEL_HOVER);
            }
            if self
                .style_import_pick_rect(panel)
                .is_some_and(|rect| rect.contains(point))
            {
                return Some(STYLE_IMPORT_PICK_HOVER);
            }
            return None;
        }
        if self.close_rect_for(panel).contains(point) {
            return Some(SCENE_TEMPLATE_CLOSE_HOVER);
        }
        if self
            .style_import_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SCENE_TEMPLATE_IMPORT_HOVER);
        }
        if let Some((index, _)) = self.tab_at(panel, point) {
            return Some(tab_hover_token(index));
        }
        for (index, (rect, _)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            if rect.contains(point) {
                return Some(filter_hover_token(index));
            }
        }
        if self
            .basis_chip_dismiss_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SCENE_TEMPLATE_BASIS_CHIP_HOVER);
        }
        if self
            .generate_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SCENE_TEMPLATE_GENERATE_HOVER);
        }
        // A card scrolled out of the viewport must not hover: its rect is
        // still computed (paint clips it), so the viewport check is what
        // keeps a pointer below the panel from lighting up a hidden row.
        let viewport = self.cards_viewport(panel);
        if !viewport.contains(point) {
            return None;
        }
        if self.tab() == AssetCenterTab::Styles {
            let cards = self.style_cards();
            let (index, card) = self
                .style_layout_for(panel, &cards)
                .cards
                .into_iter()
                .find(|(_, rect)| rect.contains(point))?;
            return Some(self.style_card_hover_token(index, card, cards[index].is_user, point));
        }
        let (index, card) = self
            .card_rects(panel)
            .into_iter()
            .find(|(_, rect)| rect.contains(point))?;
        Some(self.card_hover_token(index, card, point))
    }

    /// Hit-test panel chrome and cards. Outside presses return `None` so the
    /// caller can treat them as dismiss.
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<SceneTemplateHit> {
        if !panel.contains(point) {
            return None;
        }
        // Topmost layer first: while the paste box is up it owns every press
        // inside the panel, including the ones that land on chrome it covers.
        if let Some(hit) = self.style_import_hit(panel, point) {
            return Some(hit);
        }
        if self.close_rect_for(panel).contains(point) {
            return Some(SceneTemplateHit::Close);
        }
        if self
            .style_import_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::ImportStyleGuide);
        }
        if let Some((_, tab)) = self.tab_at(panel, point) {
            return Some(SceneTemplateHit::SelectTab(tab));
        }
        let search = self.search_rect_for(panel);
        if search.contains(point) {
            let caret = self.caret_at(
                &self.state.editor_ui.scene_template_center.search,
                search,
                SEARCH_PAD_X,
                SEARCH_TEXT_SIZE,
                point,
            );
            return Some(SceneTemplateHit::FocusSearch(caret));
        }
        for (rect, filter) in self.filter_chip_rects(panel) {
            if rect.contains(point) {
                return Some(SceneTemplateHit::SelectFilter(filter));
            }
        }
        // Above the topic field: the chip sits in the same row and its
        // dismiss target must not read as a click into the text.
        if self
            .basis_chip_dismiss_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::ClearGenerateBasis);
        }
        if let Some(input) = self.generate_input_rect(panel) {
            if input.contains(point) {
                let caret = self.caret_at(
                    &self.state.editor_ui.scene_template_center.generate,
                    input,
                    GENERATE_INPUT_PAD_X,
                    GENERATE_TEXT_SIZE,
                    point,
                );
                return Some(SceneTemplateHit::FocusGenerate(caret));
            }
        }
        if self
            .generate_button_rect(panel)
            .is_some_and(|rect| rect.contains(point))
        {
            return Some(SceneTemplateHit::Generate);
        }
        let viewport = self.cards_viewport(panel);
        if viewport.contains(point) {
            match self.tab() {
                AssetCenterTab::Templates => {
                    let user_cards = self.user_cards();
                    let cards = self.filtered();
                    let total = user_cards.len() + cards.len();
                    for (index, rect) in self.card_rects_for_count(panel, total) {
                        if !rect.contains(point) {
                            continue;
                        }
                        if index < user_cards.len() {
                            // The ✕ sits on the card, so it has to be tested
                            // before the card itself — otherwise deleting a
                            // saved template would open it instead.
                            if self.template_delete_visible(index)
                                && Self::template_delete_rect(rect).contains(point)
                            {
                                return Some(SceneTemplateHit::DeleteTemplate(
                                    user_cards[index].id.clone(),
                                ));
                            }
                            return Some(SceneTemplateHit::AddTemplateToCanvas(
                                user_cards[index].id.clone(),
                            ));
                        }
                        return Some(self.template_card_hit(
                            cards[index - user_cards.len()],
                            index,
                            rect,
                            point,
                        ));
                    }
                }
                AssetCenterTab::Styles => {
                    let cards = self.style_cards();
                    for (index, rect) in self.style_layout_for(panel, &cards).cards {
                        if !rect.contains(point) {
                            continue;
                        }
                        let card = &cards[index];
                        // The ✕ sits on the card, so it has to be tested
                        // before the card itself — otherwise deleting an
                        // import would pin it instead.
                        if card.is_user
                            && self.style_delete_visible(index)
                            && self.style_delete_rect_for(rect).contains(point)
                        {
                            return Some(SceneTemplateHit::DeleteStyleGuide(card.id.clone()));
                        }
                        return Some(SceneTemplateHit::ToggleStyleGuide(card.id.clone()));
                    }
                }
            }
        }
        Some(SceneTemplateHit::Inside)
    }
}
