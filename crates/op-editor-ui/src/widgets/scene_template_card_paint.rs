//! One template card: preview, palette band, and a two-line caption.
//!
//! Split from `scene_template_panel_paint.rs` (which now owns only the panel
//! chrome) because the card carries most of the panel's visual decisions and
//! all of its density trade-offs.
//!
//! The shape the card settled on, and why:
//!
//! - **The picture is the card.** Preview plus palette band is ~77% of the
//!   height at every breakpoint. A gallery is scanned, not read, and the
//!   thing being scanned is the design.
//! - **The palette band is part of the picture**, flush under it inside the
//!   same rounded clip. Two decks at thumbnail size are frequently
//!   indistinguishable in layout and obviously different in colour, so the
//!   band is often the discriminating information on the whole card.
//! - **The caption is one row plus a chip.** Title left, `N 页 · W×H` right,
//!   scene chip beneath. The two wrapped summary lines this replaced cost
//!   every card 32 px of permanent height to answer a question the user has
//!   about one card at a time — so the summary moved onto the preview, on
//!   hover, next to the buttons that act on it.
//!
//! Paint measures through [`crate::widgets::text_metrics`]: the caption
//! right-aligns its metadata and ellipsizes its title, and both are wrong by
//! the SF-Pro-versus-Roboto gap if measured family-blind.

use op_editor_core::scene_template_catalog::SceneTemplateDefinition;
use op_editor_core::scene_template_palette::scene_template_palette;
use op_util::hex_color;

use super::asset_center_template_cards::UserTemplateCard;
use super::icons::{draw_icon, Icon};
use super::panel_controls::{paint_accent_button, paint_neutral_button, ButtonSpec};
use super::scene_template_card_actions::{
    card_add_hover_token, card_generate_hover_token, ACTION_INSET, ACTION_LABEL_SIZE, ACTION_RADIUS,
};
use super::scene_template_panel::{
    preview_height, SceneTemplatePanel, CARD_PALETTE_H, CARD_PREVIEW_INSET,
};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::scene_template_previews::scene_template_preview;
use crate::widgets::text_metrics::{fit_chrome, measure_chrome};
use crate::widgets::PaintCx;
use crate::{Color, ImageDrawMode, Point2D, Rect};

pub(super) const CARD_RADIUS: f32 = 12.0;
const PREVIEW_RADIUS: f32 = 9.0;
const CARD_TITLE_SIZE: f32 = 14.0;
const META_SIZE: f32 = 11.0;
const SCENE_CHIP_SIZE: f32 = 11.0;
const SCENE_CHIP_H: f32 = 20.0;
const SUMMARY_SIZE: f32 = 11.5;
const SUMMARY_LINE_H: f32 = 16.0;
const SUMMARY_LINES: usize = 2;
/// Inset of the caption text from the card edge. Wider than the preview's so
/// the title reads as hanging off the card rather than off the picture.
const TEXT_INSET: f32 = 14.0;

/// The scrim behind the hover summary. Dark and near-opaque because it lands
/// on an arbitrary preview — a light wash is unreadable over a light deck and
/// a subtle one is unreadable over a busy one.
const HOVER_SCRIM: Color = Color {
    r: 0.05,
    g: 0.05,
    b: 0.06,
    a: 0.82,
};
const HOVER_SUMMARY_INK: Color = Color {
    r: 0.94,
    g: 0.94,
    b: 0.95,
    a: 1.0,
};

impl SceneTemplatePanel<'_> {
    pub(super) fn paint_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        template: &'static SceneTemplateDefinition,
        index: usize,
    ) {
        let hovered = self.card_actions_visible(index);
        // Hover elevates rather than outlines. A 1 px accent border on a card
        // whose top three-quarters is a photograph reads as an artefact of
        // the picture; a lifted surface behind it reads as the card coming
        // forward, which is what hovering it means.
        cx.backend.fill_round_rect(
            rect,
            CARD_RADIUS,
            if hovered {
                self.theme.accent
            } else {
                self.theme.card
            },
        );
        cx.backend.stroke_round_rect(
            rect,
            CARD_RADIUS,
            if hovered {
                self.theme.foreground.with_alpha(0.22)
            } else {
                self.theme.border
            },
            1.0,
        );
        // The wash goes on top of the lifted fill rather than instead of it.
        // `accent` sits only ~6% off `card` in both themes — enough to read
        // as a state, not enough on its own to read as elevation — and the
        // wash is the half of the effect that survives a theme where the two
        // tokens are closer than the palette intends.
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            hovered,
            self.is_pressed(index),
        );

        self.paint_card_preview(cx, rect, template);
        self.paint_card_caption(cx, rect, template);

        // Last, so both strips sit over the picture rather than under it.
        if hovered {
            self.paint_card_hover_summary(cx, rect, template);
            self.paint_card_actions(cx, rect, template, index);
        }
    }

    /// The preview block: picture on top, palette band flush beneath it, one
    /// rounded clip around both.
    fn paint_card_preview(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let block = Self::card_preview_block_rect(card);
        let picture = Self::card_preview_rect(card);
        cx.backend.save();
        cx.backend.clip_round_rect(block, PREVIEW_RADIUS);
        self.paint_card_picture(cx, picture, template);
        self.paint_card_palette(cx, Self::card_palette_rect(card), template);
        cx.backend.restore();
        // A hairline around the whole block: a template whose ground is
        // near-white otherwise has no edge against the card behind it.
        cx.backend
            .stroke_round_rect(block, PREVIEW_RADIUS, self.theme.border, 1.0);
    }

    fn paint_card_picture(
        &self,
        cx: &mut PaintCx<'_>,
        picture: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let Some(asset) = scene_template_preview(&template.id) else {
            cx.backend.fill_rect(picture, self.theme.muted);
            return;
        };
        let image_id = asset.image_id;
        let Some(encoded) = asset.bytes else {
            // Web only: not in the bundle and not fetched yet. Ask the host
            // and paint the plain block meanwhile — the card's title, palette
            // band and metadata are already readable, so a slow or failed
            // fetch costs the picture and nothing else.
            op_editor_core::web_assets::request(asset.route);
            cx.backend.fill_rect(picture, self.theme.muted);
            return;
        };
        // Same decode handshake the Prompt Center uses: register the bytes,
        // request the raster at the size actually painted, and fall back to a
        // plain block until the decode lands so the first frame never blocks.
        if !has_cached_image_bytes(image_id) {
            store_remote_image_bytes(image_id, encoded.to_vec());
        }
        let max_edge_px = required_raster_edge(picture, cx.backend.dpi_scale());
        let sharp = cx.backend.image_decoded(image_id, encoded, max_edge_px);
        if !sharp {
            note_pending_decode(image_id, max_edge_px);
        }
        if !sharp && !cx.backend.image_resident(image_id) {
            cx.backend.fill_rect(picture, self.theme.muted);
            return;
        }
        cx.backend
            .draw_image_with_mode(picture, image_id, encoded, ImageDrawMode::Fill);
    }

    /// Equal stripes of the template's own colours.
    ///
    /// A template that declares nothing readable paints no band rather than a
    /// placeholder one: the band's whole claim is "this is what the file is
    /// made of", and an invented stripe would make that claim falsely. The
    /// muted fill keeps the block's rounded bottom from showing the card
    /// through it.
    fn paint_card_palette(
        &self,
        cx: &mut PaintCx<'_>,
        band: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let palette = scene_template_palette(&template.id);
        if palette.is_empty() {
            cx.backend.fill_rect(band, self.theme.muted);
            return;
        }
        let stripe = band.size.x / palette.len() as f32;
        for (index, hex) in palette.iter().enumerate() {
            let Some(color) = parse_swatch(hex) else {
                continue;
            };
            // The last stripe runs to the band's right edge rather than to
            // `origin + stripe`: accumulated rounding otherwise leaves a
            // sub-pixel seam showing the card through the palette.
            let x = band.origin.x + index as f32 * stripe;
            let right = if index + 1 == palette.len() {
                band.origin.x + band.size.x
            } else {
                band.origin.x + (index + 1) as f32 * stripe
            };
            cx.backend
                .fill_rect(Rect::xywh(x, band.origin.y, right - x, band.size.y), color);
        }
    }

    /// Title on the left, `6 页 · 1920×1080` right-aligned on the same
    /// baseline, scene chip beneath.
    ///
    /// One row because the two answer different halves of the same question —
    /// what this is, and how big it is — and a user comparing cards reads
    /// down one column of each.
    fn paint_card_caption(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let top = Self::card_caption_top(card);
        let left = card.origin.x + TEXT_INSET;
        let right = card.origin.x + card.size.x - TEXT_INSET;
        let baseline = top + 24.0;

        let metadata = self.metadata(template);
        let meta_w = measure_chrome(cx.backend, &metadata, META_SIZE);
        self.paint_text(
            cx,
            &metadata,
            Point2D::new(right - meta_w, baseline),
            META_SIZE,
            self.theme.muted_foreground,
        );

        let title_w = (right - meta_w - 10.0 - left).max(0.0);
        let title = fit_chrome(
            cx.backend,
            template.title_for_locale(self.locale),
            title_w,
            CARD_TITLE_SIZE,
        );
        self.paint_text(
            cx,
            &title,
            Point2D::new(left, baseline),
            CARD_TITLE_SIZE,
            self.theme.foreground,
        );

        self.paint_scene_chip(cx, Point2D::new(left, top + 34.0), template);
    }

    /// "PPT" / "卡片" — which shelf of the catalogue this came off.
    ///
    /// The same vocabulary as the filter row directly above the grid, so a
    /// chip on a card and a chip in the row are the one word: a user who
    /// notices "信息图" on a card knows exactly which filter narrows to more
    /// like it.
    fn paint_scene_chip(
        &self,
        cx: &mut PaintCx<'_>,
        origin: Point2D,
        template: &'static SceneTemplateDefinition,
    ) {
        let label = self.filter_label(op_editor_core::SceneFilter::Scene(template.scene));
        let label_w = measure_chrome(cx.backend, label, SCENE_CHIP_SIZE);
        let chip = Rect::xywh(origin.x, origin.y, label_w + 18.0, SCENE_CHIP_H);
        cx.backend
            .fill_round_rect(chip, SCENE_CHIP_H / 2.0, self.theme.muted);
        self.paint_text(
            cx,
            label,
            Point2D::new(
                chip.origin.x + 9.0,
                jian_widgets::centered_text_baseline_y(chip, SCENE_CHIP_SIZE),
            ),
            SCENE_CHIP_SIZE,
            self.theme.muted_foreground,
        );
    }

    /// The long description, over the bottom of the preview, on hover only.
    ///
    /// It is the one piece of card text that is genuinely per-card prose, and
    /// also the one a user wants for exactly one card at a time. Keeping it
    /// off the resting card is what let the caption shrink from four lines to
    /// two; showing it here — under the pointer, above the buttons it
    /// explains — is what keeps it findable.
    fn paint_card_hover_summary(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &'static SceneTemplateDefinition,
    ) {
        let picture = Self::card_preview_rect(card);
        let (actions, _) = self.card_action_rects_for(card, self.card_offers_generate(template));
        let text_w = (picture.size.x - ACTION_INSET * 2.0).max(0.0);
        let lines = wrap_to_width(
            template.summary_for_locale(self.locale),
            text_w,
            SUMMARY_SIZE,
            SUMMARY_LINES,
        );
        if lines.is_empty() || text_w <= 0.0 {
            return;
        }
        let text_h = lines.len() as f32 * SUMMARY_LINE_H;
        let scrim_top = (actions.origin.y - 10.0 - text_h - 10.0).max(picture.origin.y);
        cx.backend.save();
        cx.backend.clip_rect(picture);
        cx.backend.fill_rect(
            Rect::xywh(
                picture.origin.x,
                scrim_top,
                picture.size.x,
                picture.origin.y + picture.size.y - scrim_top,
            ),
            HOVER_SCRIM,
        );
        let mut baseline = scrim_top + 10.0 + SUMMARY_SIZE;
        for line in lines {
            self.paint_text(
                cx,
                &line,
                Point2D::new(picture.origin.x + ACTION_INSET, baseline),
                SUMMARY_SIZE,
                HOVER_SUMMARY_INK,
            );
            baseline += SUMMARY_LINE_H;
        }
        cx.backend.restore();
    }

    /// The hover strip: what this card can do, spelled out.
    ///
    /// Only on hover, because the answer is the same for every card and a
    /// grid that repeated it forty times would be reading its own manual.
    /// The primary is filled and the secondary is a bordered surface, which
    /// is the same weight relationship the panel's other button pairs use —
    /// pressing the card does what the filled one says.
    fn paint_card_actions(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &'static SceneTemplateDefinition,
        index: usize,
    ) {
        let (add, generate) = self.card_action_rects_for(card, self.card_offers_generate(template));
        self.paint_card_action(
            cx,
            add,
            self.t("sceneTemplate.card.addToCanvas", "加入画布"),
            true,
            card_add_hover_token(index),
        );
        if let Some(rect) = generate {
            self.paint_card_action(
                cx,
                rect,
                self.t("sceneTemplate.card.generateFrom", "以此生成"),
                false,
                card_generate_hover_token(index),
            );
        }
    }

    /// A card's action button — the same accent/neutral pair the generate
    /// row uses, at the same radius and on the same pointer ladder.
    ///
    /// They were separately written before, so the panel had two "filled
    /// primary button" looks that differed in how they answered a hover.
    fn paint_card_action(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        label: &str,
        primary: bool,
        token: usize,
    ) {
        let spec = ButtonSpec {
            rect,
            radius: ACTION_RADIUS,
            label,
            label_size: ACTION_LABEL_SIZE,
            hovered: self.state.editor_ui.scene_template_center.hover == Some(token),
            pressed: self.is_pressed(token),
        };
        if primary {
            paint_accent_button(cx, &self.theme, spec);
        } else {
            paint_neutral_button(cx, &self.theme, spec);
        }
    }

    /// Picture plus palette band — the whole rounded block at the top of a
    /// card.
    pub(super) fn card_preview_block_rect(card: Rect) -> Rect {
        let picture = Self::card_preview_rect(card);
        Rect::xywh(
            picture.origin.x,
            picture.origin.y,
            picture.size.x,
            picture.size.y + CARD_PALETTE_H,
        )
    }

    /// The picture's rect inside its card.
    ///
    /// No clamp: the card height is derived from exactly this height (see
    /// `template_card_height`), so the preview always gets its full aspect and
    /// the band and caption below it always get the rest.
    pub(super) fn card_preview_rect(card: Rect) -> Rect {
        Rect::xywh(
            card.origin.x + CARD_PREVIEW_INSET,
            card.origin.y + CARD_PREVIEW_INSET,
            (card.size.x - CARD_PREVIEW_INSET * 2.0).max(0.0),
            preview_height(card.size.x),
        )
    }

    /// The palette band's rect, flush under the picture.
    pub(super) fn card_palette_rect(card: Rect) -> Rect {
        let picture = Self::card_preview_rect(card);
        Rect::xywh(
            picture.origin.x,
            picture.origin.y + picture.size.y,
            picture.size.x,
            CARD_PALETTE_H,
        )
    }

    /// Where the caption block starts — the bottom of the preview block.
    pub(super) fn card_caption_top(card: Rect) -> f32 {
        let block = Self::card_preview_block_rect(card);
        block.origin.y + block.size.y
    }

    /// "6 页 · 1920×1080" — what distinguishes a deck from a poster at a
    /// glance, which is the decision the card exists to support.
    pub(super) fn metadata(&self, template: &SceneTemplateDefinition) -> String {
        self.frames_metadata(template.frames, template.frame_width, template.frame_height)
    }

    /// "N 页 · W×H" — the same sentence for a saved template's card.
    fn frames_metadata(&self, frames: u16, width: u32, height: u32) -> String {
        let count = frames.to_string();
        let pages =
            op_i18n::translate_with(self.locale, "sceneTemplate.frames", &[("count", &count)]);
        let pages = if pages == "sceneTemplate.frames" {
            format!("{count} 页")
        } else {
            pages
        };
        format!("{pages} · {width}×{height}")
    }

    // ------------------------------------------------------------------
    // Saved-template cards ("My templates")
    // ------------------------------------------------------------------

    /// One saved-template card: the same surface language as the shipped
    /// cards — picture, caption, lift on hover — minus everything a saved
    /// template does not have: no palette band (none is declared), no scene
    /// chip (none is known), no generate action (no style guide to pin).
    /// The one extra control is the hover ✕ that forgets it, mirroring the
    /// style cards' delete.
    pub(super) fn paint_user_card(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        card: &UserTemplateCard,
        index: usize,
    ) {
        let hovered = self.state.editor_ui.scene_template_center.hover == Some(index)
            || self.template_delete_visible(index);
        cx.backend.fill_round_rect(
            rect,
            CARD_RADIUS,
            if hovered {
                self.theme.accent
            } else {
                self.theme.card
            },
        );
        cx.backend.stroke_round_rect(
            rect,
            CARD_RADIUS,
            if hovered {
                self.theme.foreground.with_alpha(0.22)
            } else {
                self.theme.border
            },
            1.0,
        );
        paint_button_feedback_wash(
            cx.backend,
            &self.theme,
            rect,
            CARD_RADIUS,
            hovered,
            self.is_pressed(index),
        );

        self.paint_user_preview_block(cx, rect, card);
        self.paint_user_caption(cx, rect, card);

        if self.template_delete_visible(index) {
            self.paint_user_delete_button(cx, Self::template_delete_rect(rect), index);
        }
    }

    /// The preview block: the runtime JPEG on top, the muted fill that the
    /// shipped palette band would occupy beneath it, one rounded clip around
    /// both.
    fn paint_user_preview_block(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        template: &UserTemplateCard,
    ) {
        let block = Self::card_preview_block_rect(card);
        let picture = Self::card_preview_rect(card);
        cx.backend.save();
        cx.backend.clip_round_rect(block, PREVIEW_RADIUS);
        self.paint_user_picture(cx, picture, template);
        // No declared palette, so the band is a plain fill — it keeps the
        // block's rounded bottom from showing the card through it.
        cx.backend
            .fill_rect(Self::card_palette_rect(card), self.theme.muted);
        cx.backend.restore();
        cx.backend
            .stroke_round_rect(block, PREVIEW_RADIUS, self.theme.border, 1.0);
    }

    fn paint_user_picture(&self, cx: &mut PaintCx<'_>, picture: Rect, template: &UserTemplateCard) {
        if template.preview_jpeg.is_empty() {
            cx.backend.fill_rect(picture, self.theme.muted);
            return;
        }
        // Same decode handshake the shipped previews use, but the bytes are
        // runtime registry bytes rather than a baked-in asset. The cache id is
        // tied to the immutable registry allocation, so search/reorder keeps
        // it stable while replacement preview bytes get a fresh slot.
        if !has_cached_image_bytes(template.image_id) {
            store_remote_image_bytes(template.image_id, template.preview_jpeg.clone());
        }
        let max_edge_px = required_raster_edge(picture, cx.backend.dpi_scale());
        let sharp =
            cx.backend
                .image_decoded(template.image_id, &template.preview_jpeg, max_edge_px);
        if !sharp {
            note_pending_decode(template.image_id, max_edge_px);
        }
        if !sharp && !cx.backend.image_resident(template.image_id) {
            cx.backend.fill_rect(picture, self.theme.muted);
            return;
        }
        cx.backend.draw_image_with_mode(
            picture,
            template.image_id,
            &template.preview_jpeg,
            ImageDrawMode::Fill,
        );
    }

    /// Title on the left, `N 页 · W×H` right-aligned on the same baseline.
    /// No scene chip beneath: a saved template carries no scene.
    fn paint_user_caption(&self, cx: &mut PaintCx<'_>, card: Rect, template: &UserTemplateCard) {
        let top = Self::card_caption_top(card);
        let left = card.origin.x + TEXT_INSET;
        let right = card.origin.x + card.size.x - TEXT_INSET;
        let baseline = top + 24.0;

        let metadata =
            self.frames_metadata(template.frames, template.frame_width, template.frame_height);
        let meta_w = measure_chrome(cx.backend, &metadata, META_SIZE);
        self.paint_text(
            cx,
            &metadata,
            Point2D::new(right - meta_w, baseline),
            META_SIZE,
            self.theme.muted_foreground,
        );

        let title_w = (right - meta_w - 10.0 - left).max(0.0);
        // The name is verbatim — the user's own word, deliberately not
        // translated — so it is truncated to the room the metadata leaves.
        let title = super::scene_template_panel_paint::truncate_to_width(
            &template.name,
            title_w,
            CARD_TITLE_SIZE,
        );
        self.paint_text(
            cx,
            &title,
            Point2D::new(left, baseline),
            CARD_TITLE_SIZE,
            self.theme.foreground,
        );
    }

    /// The ✕ that forgets a saved template — the same hover-gated corner
    /// button the imported style cards carry.
    fn paint_user_delete_button(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize) {
        let hovered = self.state.editor_ui.scene_template_center.hover
            == Some(Self::template_delete_hover_token(index));
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
}

fn parse_swatch(raw: &str) -> Option<Color> {
    let [r, g, b, a] = hex_color::parse_hex_rgba8(raw, hex_color::HexOptions::LENIENT)?;
    Some(Color::rgba_u8(r, g, b, a as f32 / 255.0))
}

/// Greedy wrap to at most `max_lines`, ellipsising the last line when the
/// text does not fit. Breaks per character, which is right for the CJK the
/// summaries are written in and acceptable for the Latin ones at this size.
fn wrap_to_width(text: &str, max_width: f32, size: f32, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0.0;
    for character in text.chars() {
        let advance =
            super::prompt_center_panel::estimated_text_width(&character.to_string(), size);
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
            lines.push(super::scene_template_panel_paint::truncate_to_width(
                &last, max_width, size,
            ));
        }
    }
    lines
}

#[cfg(test)]
#[path = "scene_template_card_paint_tests.rs"]
mod scene_template_card_paint_tests;
