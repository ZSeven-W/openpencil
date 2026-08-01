//! Scene Template Center painting, split from geometry to keep both compact.

use op_editor_core::scene_template_catalog::SceneTemplateDefinition;

use super::scene_template_panel::{
    filter_hover_token, SceneTemplatePanel, CARD_H, CARD_PREVIEW_ASPECT, CARD_PREVIEW_INSET,
    CHIP_H, CHIP_LABEL_SIZE, CLOSE_BTN, HEADER_H, PAD, SCENE_TEMPLATE_CLOSE_HOVER,
    SEARCH_TEXT_SIZE,
};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::prompt_center_panel::estimated_text_width;
use crate::widgets::property_panel_text_input::paint_text_input_view;
use crate::widgets::scene_template_previews::scene_template_preview;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, ImageDrawMode, Point2D, Rect, TextLayout};

const CARD_RADIUS: f32 = 9.0;
const TITLE_SIZE: f32 = 12.5;
const SUMMARY_SIZE: f32 = 11.0;
const META_SIZE: f32 = 10.5;

impl SceneTemplatePanel<'_> {
    /// Paint the complete non-modal panel.
    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 12.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, 12.0, self.theme.border, 1.0);
        self.paint_header(cx, panel);
        self.paint_search(cx, panel);
        self.paint_filter_chips(cx, panel);
        self.paint_cards(cx, panel);
    }

    fn paint_header(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        self.paint_text(
            cx,
            self.t("sceneTemplate.title", "场景模板"),
            Point2D::new(panel.origin.x + PAD, panel.origin.y + 29.0),
            15.0,
            self.theme.foreground,
        );

        let close = Self::close_rect(panel);
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: self.state.editor_ui.scene_template_center.hover
                == Some(SCENE_TEMPLATE_CLOSE_HOVER),
            pressed: self.is_pressed(SCENE_TEMPLATE_CLOSE_HOVER),
            active: false,
            enabled: true,
            icon_size: CLOSE_BTN - 11.0,
            stroke_width: 1.5,
        }
        .paint(
            cx.backend,
            close,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );

        cx.backend.fill_rect(
            Rect::xywh(panel.origin.x, panel.origin.y + HEADER_H, panel.size.x, 1.0),
            self.theme.border,
        );
    }

    fn paint_search(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let rect = Self::search_rect(panel);
        cx.backend.fill_round_rect(rect, 7.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(rect, 7.0, self.theme.border, 1.0);
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(rect.origin.x + 9.0, rect.origin.y + 7.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
        paint_text_input_view(
            cx,
            &self.theme,
            &self.state.editor_ui.scene_template_center.search,
            rect,
            SEARCH_TEXT_SIZE,
            32.0,
            rect.origin.y + 19.0,
            self.now_ms(),
            self.t("sceneTemplate.searchPlaceholder", "搜索场景或模板"),
            // The panel has one text field, so it owns the caret whenever the
            // panel is open — no focus enum needed.
            true,
        );
    }

    fn paint_filter_chips(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        for (index, (rect, filter)) in self.filter_chip_rects(panel).into_iter().enumerate() {
            let active = self.state.editor_ui.scene_template_center.filter == filter;
            let (fill, foreground) = if active {
                (self.theme.primary, self.theme.primary_foreground)
            } else {
                (self.theme.muted, self.theme.muted_foreground)
            };
            cx.backend.fill_round_rect(rect, CHIP_H / 2.0, fill);
            paint_button_feedback_wash(
                cx.backend,
                &self.theme,
                rect,
                CHIP_H / 2.0,
                self.state.editor_ui.scene_template_center.hover == Some(filter_hover_token(index)),
                self.is_pressed(filter_hover_token(index)),
            );
            let label = self.filter_label(filter);
            let label_w = estimated_text_width(label, CHIP_LABEL_SIZE);
            self.paint_text(
                cx,
                label,
                Point2D::new(
                    rect.origin.x + ((rect.size.x - label_w) / 2.0).max(5.0),
                    rect.origin.y + 16.0,
                ),
                CHIP_LABEL_SIZE,
                foreground,
            );
        }
    }

    fn paint_cards(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let viewport = self.cards_viewport(panel);
        let templates = self.filtered();
        if templates.is_empty() {
            self.paint_text(
                cx,
                self.t("sceneTemplate.empty", "没有匹配的模板"),
                Point2D::new(viewport.origin.x + 4.0, viewport.origin.y + 28.0),
                12.0,
                self.theme.muted_foreground,
            );
            return;
        }

        cx.backend.save();
        cx.backend.clip_rect(viewport);
        for (index, rect) in self.card_rects_for_count(panel, templates.len()) {
            // Cheap reject for rows scrolled out of view: their rects are
            // still computed so hover and paint agree on indices.
            if rect.origin.y > viewport.origin.y + viewport.size.y
                || rect.origin.y + rect.size.y < viewport.origin.y
            {
                continue;
            }
            self.paint_card(cx, rect, templates[index], index);
        }
        cx.backend.restore();
    }

    fn paint_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        template: &'static SceneTemplateDefinition,
        index: usize,
    ) {
        cx.backend
            .fill_round_rect(rect, CARD_RADIUS, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, CARD_RADIUS, self.theme.border, 1.0);
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            self.state.editor_ui.scene_template_center.hover == Some(index),
            self.is_pressed(index),
        );

        let preview = Self::card_preview_rect(rect);
        self.paint_card_preview(cx, preview, template);

        let text_x = rect.origin.x + CARD_PREVIEW_INSET + 4.0;
        let text_w = rect.size.x - (CARD_PREVIEW_INSET + 4.0) * 2.0;
        self.paint_text(
            cx,
            &truncate_to_width(template.title_for_locale(self.locale), text_w, TITLE_SIZE),
            Point2D::new(text_x, preview.origin.y + preview.size.y + 22.0),
            TITLE_SIZE,
            self.theme.foreground,
        );

        // Two summary lines, wrapped on the same width estimate the rest of
        // the panel uses.
        let summary = template.summary_for_locale(self.locale);
        let mut y = preview.origin.y + preview.size.y + 40.0;
        for line in wrap_to_width(summary, text_w, SUMMARY_SIZE, 2) {
            self.paint_text(
                cx,
                &line,
                Point2D::new(text_x, y),
                SUMMARY_SIZE,
                self.theme.muted_foreground,
            );
            y += 15.0;
        }

        self.paint_text(
            cx,
            &self.metadata(template),
            Point2D::new(text_x, rect.origin.y + rect.size.y - 12.0),
            META_SIZE,
            self.theme.muted_foreground,
        );
    }

    fn paint_card_preview(
        &self,
        cx: &mut PaintCx<'_>,
        preview: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let Some((image_id, encoded)) = scene_template_preview(&template.id) else {
            cx.backend.fill_round_rect(preview, 7.0, self.theme.muted);
            return;
        };
        // Same decode handshake the Prompt Center uses: register the bytes,
        // request the raster at the size actually painted, and fall back to a
        // plain block until the decode lands so the first frame never blocks.
        if !has_cached_image_bytes(image_id) {
            store_remote_image_bytes(image_id, encoded.to_vec());
        }
        let max_edge_px = required_raster_edge(preview, cx.backend.dpi_scale());
        let sharp = cx.backend.image_decoded(image_id, encoded, max_edge_px);
        if !sharp {
            note_pending_decode(image_id, max_edge_px);
        }
        if !sharp && !cx.backend.image_resident(image_id) {
            cx.backend.fill_round_rect(preview, 7.0, self.theme.muted);
            return;
        }
        cx.backend.save();
        cx.backend.clip_round_rect(preview, 7.0);
        cx.backend
            .draw_image_with_mode(preview, image_id, encoded, ImageDrawMode::Fill);
        cx.backend.restore();
    }

    fn card_preview_rect(card: Rect) -> Rect {
        let width = card.size.x - CARD_PREVIEW_INSET * 2.0;
        let height = (width / CARD_PREVIEW_ASPECT).min(CARD_H * 0.62);
        Rect::xywh(
            card.origin.x + CARD_PREVIEW_INSET,
            card.origin.y + CARD_PREVIEW_INSET,
            width,
            height,
        )
    }

    /// "6 页 · 1920×1080" — what distinguishes a deck from a poster at a
    /// glance, which is the decision the card exists to support.
    fn metadata(&self, template: &SceneTemplateDefinition) -> String {
        let count = template.frames.to_string();
        let pages =
            op_i18n::translate_with(self.locale, "sceneTemplate.frames", &[("count", &count)]);
        let pages = if pages == "sceneTemplate.frames" {
            format!("{count} 页")
        } else {
            pages
        };
        format!(
            "{pages} · {}×{}",
            template.frame_width, template.frame_height
        )
    }

    fn t(&self, key: &'static str, fallback: &'static str) -> &'static str {
        let translated = op_i18n::translate(self.locale, key);
        if translated == key {
            fallback
        } else {
            translated
        }
    }

    fn paint_text(
        &self,
        cx: &mut PaintCx<'_>,
        text: &str,
        position: Point2D,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            text,
            "system-ui",
            size,
            color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, position);
    }
}

fn truncate_to_width(text: &str, max_width: f32, size: f32) -> String {
    if estimated_text_width(text, size) <= max_width {
        return text.to_string();
    }
    let ellipsis_w = estimated_text_width("…", size);
    let mut out = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = estimated_text_width(&character.to_string(), size);
        if width + advance + ellipsis_w > max_width {
            break;
        }
        out.push(character);
        width += advance;
    }
    out.push('…');
    out
}

/// Greedy wrap to at most `max_lines`, ellipsising the last line when the
/// text does not fit. Breaks per character, which is right for the CJK the
/// summaries are written in and acceptable for the Latin ones at this size.
fn wrap_to_width(text: &str, max_width: f32, size: f32, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance = estimated_text_width(&character.to_string(), size);
        if width + advance > max_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            width = 0.0;
            if lines.len() == max_lines {
                break;
            }
        }
        current.push(character);
        width += advance;
    }
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    }
    // Anything past the cap is dropped, so the final line must show that.
    if lines.len() == max_lines {
        let consumed: usize = lines.iter().map(|line| line.chars().count()).sum();
        if text.chars().count() > consumed {
            let last = lines.pop().unwrap_or_default();
            lines.push(truncate_to_width(&last, max_width, size));
        }
    }
    lines
}
