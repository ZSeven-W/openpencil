//! Asset Center → Styles tab painting.
//!
//! Split from `scene_template_panel_paint.rs` so neither file has to carry
//! both card shapes. A template card is a picture with a caption; a style
//! card is a palette with a name, and until previews are baked (M2) the
//! colour band is the whole picture — it is what makes two guides
//! distinguishable at a glance.

use super::asset_center_style_cards::StyleGuideCard;
use super::asset_center_style_layout::STYLE_SECTION_HEADER_H;
use super::icons::{draw_icon, Icon};
use super::panel_control_metrics::{control_fill, CHIP_RADIUS};
use super::scene_template_panel::{SceneTemplatePanel, STYLE_SWATCH_H};
use super::scene_template_style_geometry::SCENE_TEMPLATE_IMPORT_HOVER;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::prompt_center_panel::estimated_text_width;
use crate::widgets::scene_template_panel_paint::truncate_to_width;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

const CARD_RADIUS: f32 = 12.0;
const NAME_SIZE: f32 = 13.5;
const SUMMARY_SIZE: f32 = 11.0;
const PIN_SIZE: f32 = 11.0;
const SECTION_SIZE: f32 = 11.5;
const IMPORT_LABEL_SIZE: f32 = 12.0;
const INSET: f32 = 14.0;

impl SceneTemplatePanel<'_> {
    pub(super) fn paint_style_cards(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        self.paint_style_import_button(cx, panel);
        let viewport = self.cards_viewport(panel);
        let cards = self.style_cards();
        if cards.is_empty() {
            self.paint_text(
                cx,
                self.t("assetCenter.style.empty", "没有匹配的风格"),
                Point2D::new(viewport.origin.x + 4.0, viewport.origin.y + 28.0),
                12.0,
                self.theme.muted_foreground,
            );
            return;
        }

        let layout = self.style_layout_for(panel, &cards);
        cx.backend.save();
        cx.backend.clip_rect(viewport);
        for header in &layout.headers {
            if !Self::style_row_visible(header.rect, viewport) {
                continue;
            }
            self.paint_text(
                cx,
                if header.is_user {
                    self.t("assetCenter.style.mine", "我的风格")
                } else {
                    self.t("assetCenter.style.builtIn", "内置风格")
                },
                Point2D::new(
                    header.rect.origin.x,
                    header.rect.origin.y + STYLE_SECTION_HEADER_H - 12.0,
                ),
                SECTION_SIZE,
                self.theme.muted_foreground,
            );
        }
        let pinned = self.pinned_style_guide();
        for (index, rect) in layout.cards {
            // Same cheap reject the template grid uses: rects for scrolled-out
            // rows are still computed so hover and paint agree on indices.
            if !Self::style_row_visible(rect, viewport) {
                continue;
            }
            self.paint_style_card(cx, rect, &cards[index], index, pinned);
        }
        cx.backend.restore();
    }

    fn style_row_visible(rect: Rect, viewport: Rect) -> bool {
        rect.origin.y <= viewport.origin.y + viewport.size.y
            && rect.origin.y + rect.size.y >= viewport.origin.y
    }

    /// The import button in the tab row.
    fn paint_style_import_button(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let Some(rect) = self.style_import_button_rect(panel) else {
            return;
        };
        // A chip, painted the way every other chip in the panel is: the same
        // surface, the same hairline, the same pointer ladder.
        let hovered =
            self.state.editor_ui.scene_template_center.hover == Some(SCENE_TEMPLATE_IMPORT_HOVER);
        let pressed = self.is_pressed(SCENE_TEMPLATE_IMPORT_HOVER);
        cx.backend.fill_round_rect(
            rect,
            CHIP_RADIUS,
            control_fill(self.theme.secondary, hovered, pressed),
        );
        cx.backend
            .stroke_round_rect(rect, CHIP_RADIUS, self.theme.border, 1.0);
        let label = self.style_import_label();
        let label_w = estimated_text_width(label, IMPORT_LABEL_SIZE);
        let glyph = 12.0;
        let start = rect.origin.x + ((rect.size.x - label_w - glyph - 6.0) / 2.0).max(8.0);
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(start, rect.origin.y + (rect.size.y - glyph) / 2.0),
            glyph,
            self.theme.foreground,
            1.5,
        );
        self.paint_text(
            cx,
            label,
            Point2D::new(
                start + glyph + 6.0,
                jian_widgets::centered_text_baseline_y(rect, IMPORT_LABEL_SIZE),
            ),
            IMPORT_LABEL_SIZE,
            self.theme.foreground,
        );
    }

    fn paint_style_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        card: &StyleGuideCard,
        index: usize,
        pinned: Option<&str>,
    ) {
        let is_pinned = card.is_pinned(pinned);
        cx.backend
            .fill_round_rect(rect, CARD_RADIUS, self.theme.card);
        // The pinned card is outlined in the primary colour at double width.
        // A pin survives the panel closing and steers every later generation,
        // so it has to read as state, not as hover.
        let (border, border_w) = if is_pinned {
            (self.theme.primary, 2.0)
        } else {
            (self.theme.border, 1.0)
        };
        cx.backend
            .stroke_round_rect(rect, CARD_RADIUS, border, border_w);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            self.state.editor_ui.scene_template_center.hover == Some(index),
            self.is_pressed(index),
        );

        let text_x = rect.origin.x + INSET;
        let text_w = rect.size.x - INSET * 2.0;

        let pin_label = self.t("assetCenter.style.pinned", "已钉住");
        let pin_w = if is_pinned {
            estimated_text_width(pin_label, PIN_SIZE) + 8.0
        } else {
            0.0
        };
        // The delete button lives in the same corner the pin badge does, so
        // the name has to clear whichever of them is showing.
        let delete_visible = card.is_user && self.style_delete_visible(index);
        let corner_w = if delete_visible {
            pin_w.max(self.style_delete_rect_for(rect).size.x + 8.0)
        } else {
            pin_w
        };
        self.paint_text(
            cx,
            &truncate_to_width(&card.name, (text_w - corner_w).max(0.0), NAME_SIZE),
            Point2D::new(text_x, rect.origin.y + 28.0),
            NAME_SIZE,
            self.theme.foreground,
        );
        if delete_visible {
            self.paint_style_delete_button(cx, self.style_delete_rect_for(rect), index);
        } else if is_pinned {
            self.paint_text(
                cx,
                pin_label,
                Point2D::new(
                    rect.origin.x + rect.size.x - INSET - pin_w + 8.0,
                    rect.origin.y + 27.0,
                ),
                PIN_SIZE,
                self.theme.primary,
            );
        }

        self.paint_swatch_band(cx, Self::swatch_band_rect(rect), card);

        self.paint_text(
            cx,
            &truncate_to_width(&card.summary, text_w, SUMMARY_SIZE),
            Point2D::new(text_x, rect.origin.y + rect.size.y - 16.0),
            SUMMARY_SIZE,
            self.theme.muted_foreground,
        );
    }

    /// The ✕ that forgets an imported style.
    ///
    /// Painted only while the card is hovered (see `style_delete_visible`),
    /// and only on imports: the shipped corpus is not the user's to remove,
    /// and a delete button that refuses to delete is worse than none.
    fn paint_style_delete_button(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize) {
        let hovered = self.state.editor_ui.scene_template_center.hover
            == Some(Self::style_delete_hover_token(index));
        cx.backend.fill_round_rect(
            rect,
            rect.size.x / 2.0,
            if hovered {
                self.theme.destructive
            } else {
                self.theme.muted
            },
        );
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(rect.origin.x + 6.0, rect.origin.y + 6.0),
            rect.size.x - 12.0,
            if hovered {
                self.theme.destructive_foreground
            } else {
                self.theme.muted_foreground
            },
            1.6,
        );
    }

    /// The colour band's rect. Shared with the tests so a band that stops
    /// sitting inside its card fails loudly instead of painting off-card.
    pub(super) fn swatch_band_rect(card: Rect) -> Rect {
        Rect::xywh(
            card.origin.x + INSET,
            card.origin.y + 40.0,
            (card.size.x - INSET * 2.0).max(0.0),
            STYLE_SWATCH_H,
        )
    }

    fn paint_swatch_band(&self, cx: &mut PaintCx<'_>, band: Rect, card: &StyleGuideCard) {
        if card.swatches.is_empty() {
            return;
        }
        let count = card.swatches.len() as f32;
        let width = band.size.x / count;
        for (index, color) in card.swatches.iter().enumerate() {
            let swatch = Rect::xywh(
                band.origin.x + index as f32 * width,
                band.origin.y,
                width,
                band.size.y,
            );
            cx.backend.fill_rect(swatch, *color);
        }
        // One hairline around the whole band rather than per swatch: a light
        // guide's near-white background is otherwise invisible on the card.
        cx.backend
            .stroke_round_rect(band, 3.0, self.theme.border, 1.0);
    }
}
