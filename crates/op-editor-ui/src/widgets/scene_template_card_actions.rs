//! The two things a template card can do, and the chip that remembers one.
//!
//! A template card used to have a single meaning: press it, get that document
//! instead of the one you had. That is the right answer exactly once — on an
//! empty canvas — and the wrong one every other time, because a user who
//! already has work open and clicks a template is asking to *use* it, not to
//! trade their document for it. So the card carries two actions:
//!
//! - **Add to canvas** — the default, and what a press anywhere on the card
//!   does. It brings the template's boards into the current document.
//! - **Generate from this** — pins the template's style guide and aims the
//!   generate row at it, so the next topic comes back in this aesthetic.
//!
//! Both live on a strip that floats over the bottom of the preview on hover,
//! rather than as permanent rows under it: at four cards across, two buttons
//! per card is eight buttons competing with the pictures they belong to.
//!
//! Geometry only. Paint reads these rects, and so does the hit-test, which is
//! the point — a button drawn from one set of numbers and pressed from
//! another is how a control ends up half a pixel away from where it looks.

use op_editor_core::scene_template_catalog::SceneTemplateDefinition;
use op_editor_core::AssetCenterTab;

use super::panel_control_metrics::{CHIP_H, CHIP_LABEL_SIZE};
use super::prompt_center_panel::estimated_text_width;
use super::scene_template_panel::{SceneTemplateHit, SceneTemplatePanel};
use crate::{Point2D, Rect};

/// Hover-token band for the "add to canvas" button of card `index`.
///
/// Cards hover on their bare index, so the action buttons need bands of
/// their own. They sit far below the chrome tokens (which count down from
/// `usize::MAX`) with room for more cards than a catalogue will ever hold.
pub(super) const CARD_ADD_HOVER_BASE: usize = usize::MAX - 12288;
/// Hover-token band for the "generate from this" button of card `index`.
pub(super) const CARD_GENERATE_HOVER_BASE: usize = usize::MAX - 16384;
/// Hover token for the generate row's basis chip dismiss target.
pub const SCENE_TEMPLATE_BASIS_CHIP_HOVER: usize = usize::MAX - 128;

/// How far each card action band reaches. A catalogue this long would have
/// stopped being a gallery well before it collided with the next band.
const CARD_ACTION_BAND: usize = 4096;

/// Hover-token band for a saved-template card's delete button.
///
/// Its own band, clear of the action-button bands ([`CARD_ADD_HOVER_BASE`] /
/// [`CARD_GENERATE_HOVER_BASE`]) and of the style-card delete band: a delete
/// button and the card it sits on are different targets, and sharing a token
/// would make hovering the ✕ light the whole card.
pub(super) const TEMPLATE_DELETE_HOVER_BASE: usize = usize::MAX - 8192;

impl SceneTemplatePanel<'_> {
    /// A hover token for the delete button of the saved-template card at
    /// `index`.
    pub(super) fn template_delete_hover_token(index: usize) -> usize {
        TEMPLATE_DELETE_HOVER_BASE + index.min(CARD_ACTION_BAND - 1)
    }

    /// The delete button on a saved-template card, in its top-right corner —
    /// the same geometry as the style cards' delete.
    pub(super) fn template_delete_rect(card: Rect) -> Rect {
        let side = super::scene_template_style_geometry::STYLE_DELETE_BTN;
        Rect::xywh(
            card.origin.x + card.size.x - side - 8.0,
            card.origin.y + 8.0,
            side,
            side,
        )
    }
}

/// A card's action buttons are chips — same height, same label size as the
/// filter row's — cut to the panel's control radius rather than to a pill,
/// because they sit in a rectangular strip over a rectangular picture.
pub(super) const ACTION_H: f32 = CHIP_H;
pub(super) const ACTION_INSET: f32 = 10.0;
pub(super) const ACTION_GAP: f32 = 8.0;
pub(super) const ACTION_LABEL_SIZE: f32 = CHIP_LABEL_SIZE;
pub(super) const ACTION_RADIUS: f32 = 7.0;

pub(super) const BASIS_CHIP_LABEL_SIZE: f32 = 11.5;
/// Room for the label's left padding plus the dismiss glyph and its gutter.
pub(super) const BASIS_CHIP_PAD_X: f32 = 10.0;
pub(super) const BASIS_CHIP_DISMISS_W: f32 = 22.0;
/// Longest a basis chip gets before its label is truncated. A template title
/// is short, but the chip shares a row with the topic field and must never be
/// the reason there is nowhere left to type.
pub(super) const BASIS_CHIP_MAX_W: f32 = 240.0;

/// A hover token for card `index`'s "add to canvas" button.
pub(super) fn card_add_hover_token(index: usize) -> usize {
    CARD_ADD_HOVER_BASE + index.min(CARD_ACTION_BAND - 1)
}

/// A hover token for card `index`'s "generate from this" button.
pub(super) fn card_generate_hover_token(index: usize) -> usize {
    CARD_GENERATE_HOVER_BASE + index.min(CARD_ACTION_BAND - 1)
}

/// The two action rects for a template card, `(add, generate)`.
///
/// `generate` is `None` when the template has no style guide to pin — see
/// `SceneTemplateDefinition::generate_style_guide`. In that case "add to
/// canvas" takes the whole strip rather than leaving a gap where a second
/// button would be: an empty slot reads as a control that failed to load.
pub(super) fn card_action_rects(card: Rect, with_generate: bool) -> (Rect, Option<Rect>) {
    let preview = SceneTemplatePanel::card_preview_rect(card);
    let row = Rect::xywh(
        preview.origin.x + ACTION_INSET,
        preview.origin.y + preview.size.y - ACTION_INSET - ACTION_H,
        (preview.size.x - ACTION_INSET * 2.0).max(0.0),
        ACTION_H,
    );
    if !with_generate {
        return (row, None);
    }
    let half = ((row.size.x - ACTION_GAP) / 2.0).max(0.0);
    (
        Rect::xywh(row.origin.x, row.origin.y, half, row.size.y),
        Some(Rect::xywh(
            row.origin.x + half + ACTION_GAP,
            row.origin.y,
            half,
            row.size.y,
        )),
    )
}

impl SceneTemplatePanel<'_> {
    /// Whether card `index`'s action strip is showing.
    ///
    /// Paint and hit-test share this predicate deliberately. Hit-testing a
    /// button that is not painted would give the card an invisible dead zone
    /// where a press means something other than "open me"; painting one that
    /// cannot be pressed is the same bug from the other side.
    ///
    /// The buttons count as their own card's hover, not as leaving it: the
    /// pointer moving from the picture onto a button rewrites the hover token
    /// from the card index to the button's, and a predicate that only knew
    /// the index would hide the strip on the way to pressing it — then show
    /// it again once hover fell back to the card. That oscillates once per
    /// mouse move.
    pub(super) fn card_actions_visible(&self, index: usize) -> bool {
        // Saved-template cards have no action strip: no style guide to pin,
        // so no "generate from this", and their one action is the whole card
        // (plus the hover delete button, which has its own predicate).
        if index < self.user_card_count() {
            return false;
        }
        // Touch layouts cannot rely on hover to reveal the action strip.
        if self.touch_density_active() {
            return true;
        }
        let hover = self.state.editor_ui.scene_template_center.hover;
        hover == Some(index)
            || hover == Some(card_add_hover_token(index))
            || hover == Some(card_generate_hover_token(index))
            || self.is_pressed(card_add_hover_token(index))
            || self.is_pressed(card_generate_hover_token(index))
    }

    /// Whether the saved-template card at `index` shows its delete button.
    ///
    /// Hover-gated, like the style cards' delete: a permanent ✕ on every
    /// saved card would turn the section into a list of things to remove. It
    /// stays live while the pointer is on the button itself as well as on the
    /// card, or moving toward it would make it vanish.
    pub(super) fn template_delete_visible(&self, index: usize) -> bool {
        let hover = self.state.editor_ui.scene_template_center.hover;
        hover == Some(index) || hover == Some(Self::template_delete_hover_token(index))
    }

    /// Whether this template offers the "generate from this" action here.
    ///
    /// Two gates. The template must name a style guide, and the host must be
    /// able to run a generation at all — the web bundle carries no generation
    /// chain, and its generate row is already hidden for that reason. A card
    /// button that ignored the second gate would be the one control in the
    /// panel that promises a pipeline the host does not have.
    pub(super) fn card_offers_generate(&self, template: &SceneTemplateDefinition) -> bool {
        self.state.editor_ui.scene_template_generate_supported
            && template.generate_style_guide().is_some()
    }

    /// Refine a card hover to one of its action buttons when the pointer is
    /// on one. Style cards have no action strip, so they always hover whole;
    /// saved-template cards have only the delete button.
    pub(super) fn card_hover_token(&self, index: usize, card: Rect, point: Point2D) -> usize {
        if self.tab() != AssetCenterTab::Templates {
            return index;
        }
        if index < self.user_card_count() {
            if Self::template_delete_rect(card).contains(point) {
                return Self::template_delete_hover_token(index);
            }
            return index;
        }
        if !self.card_actions_visible(index) {
            return index;
        }
        let Some(template) = self.filtered().get(index - self.user_card_count()).copied() else {
            return index;
        };
        let (add, generate) = self.card_action_rects_for(card, self.card_offers_generate(template));
        if generate.is_some_and(|rect| rect.contains(point)) {
            return card_generate_hover_token(index);
        }
        if add.contains(point) {
            return card_add_hover_token(index);
        }
        index
    }

    /// What a press inside template card `index` means.
    ///
    /// Only the secondary button changes the answer. Everywhere else on the
    /// card — the picture, the title, the primary button — means "add this to
    /// my canvas", so the default action is reachable without the user having
    /// to find a button at all, and finding it costs them nothing.
    pub(super) fn template_card_hit(
        &self,
        template: &'static SceneTemplateDefinition,
        index: usize,
        card: Rect,
        point: Point2D,
    ) -> SceneTemplateHit {
        if self.card_actions_visible(index) && self.card_offers_generate(template) {
            let (_, generate) = self.card_action_rects_for(card, true);
            if generate.is_some_and(|rect| rect.contains(point)) {
                return SceneTemplateHit::GenerateFromTemplate(template.id.clone());
            }
        }
        SceneTemplateHit::AddTemplateToCanvas(template.id.clone())
    }

    /// The basis chip's rect in the generate row, when one is set.
    pub fn basis_chip_rect(&self, panel: Rect) -> Option<Rect> {
        let label = self.basis_chip_label()?;
        let input_row_y = self.generate_row_top(panel) + 6.0;
        let chip_h = self.density().chip_h;
        Some(Rect::xywh(
            Self::content_rect(panel).origin.x,
            input_row_y + (self.generate_input_height_for() - chip_h) / 2.0,
            basis_chip_width(&label),
            chip_h,
        ))
    }

    /// The dismiss target inside the basis chip.
    pub(super) fn basis_chip_dismiss_rect(&self, panel: Rect) -> Option<Rect> {
        let chip = self.basis_chip_rect(panel)?;
        let dismiss_w = if self.touch_density_active() {
            self.density().chip_h
        } else {
            BASIS_CHIP_DISMISS_W
        };
        Some(Rect::xywh(
            chip.origin.x + chip.size.x - dismiss_w,
            chip.origin.y,
            dismiss_w,
            chip.size.y,
        ))
    }

    /// "基于：极简 Keynote" — the chip's full text, or `None` when the row is
    /// not working from a template.
    ///
    /// A basis whose template is no longer in the catalogue reads as no basis
    /// at all: the chip's whole job is to name what was chosen, and it cannot
    /// do that for an id it can no longer resolve.
    pub(super) fn basis_chip_label(&self) -> Option<String> {
        let id = self
            .state
            .editor_ui
            .scene_template_center
            .generate_basis
            .as_deref()?;
        let template = op_editor_core::scene_template_catalog::scene_template_by_id(id)?;
        Some(format!(
            "{}{}",
            self.t("sceneTemplate.generate.basis", "基于："),
            template.title_for_locale(self.locale)
        ))
    }
}

/// Chip width for `label`, capped so the topic field always has room.
pub(super) fn basis_chip_width(label: &str) -> f32 {
    (estimated_text_width(label, BASIS_CHIP_LABEL_SIZE)
        + BASIS_CHIP_PAD_X * 2.0
        + BASIS_CHIP_DISMISS_W)
        .min(BASIS_CHIP_MAX_W)
}

#[cfg(test)]
#[path = "scene_template_card_actions_tests.rs"]
mod scene_template_card_actions_tests;

/// Width the generate row loses to the basis chip, including its gap.
pub(super) fn basis_chip_reserved_width(chip: Option<Rect>, gap: f32) -> f32 {
    chip.map(|rect| rect.size.x + gap).unwrap_or(0.0)
}
