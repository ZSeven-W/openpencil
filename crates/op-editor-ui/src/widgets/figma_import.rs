//! "Import from Figma" modal. Mirrors
//! `apps/web/src/components/shared/figma-import-dialog.tsx` —
//! 720×400 card with a dashed drop-zone, upload icon, headline,
//! subtitle, and footer hint. Drag-drop of .fig files isn't wired
//! yet; clicking inside the drop zone routes through a file dialog
//! to pick a .fig path.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::Locale;
use op_editor_core::EditorState;

pub const MODAL_WIDTH: f32 = 460.0;
pub const MODAL_HEIGHT: f32 = 260.0;
const PAD: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigmaImportHit {
    Close,
    DropZone,
    Outside,
    Inside,
}

pub struct FigmaImportModal {
    pub id: WidgetId,
    pub theme: Theme,
    /// Active UI locale — drives the modal's `t` copy lookup.
    locale: Locale,
}

impl FigmaImportModal {
    pub fn for_editor(state: &EditorState) -> Self {
        Self {
            id: WidgetId::new(5400),
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
        }
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let x = ((viewport_w - MODAL_WIDTH) / 2.0).max(16.0);
        let y = ((viewport_h - MODAL_HEIGHT) / 2.0).max(crate::widgets::TOP_BAR_HEIGHT + 16.0);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(MODAL_WIDTH, MODAL_HEIGHT),
        }
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> FigmaImportHit {
        if !rect_contains(panel, point) {
            return FigmaImportHit::Outside;
        }
        if rect_contains(close_rect(panel), point) {
            return FigmaImportHit::Close;
        }
        if rect_contains(drop_zone_rect(panel), point) {
            return FigmaImportHit::DropZone;
        }
        FigmaImportHit::Inside
    }
}

fn close_rect(panel: Rect) -> Rect {
    let s = 14.0;
    Rect {
        origin: Point2D::new(
            panel.origin.x + panel.size.x - 14.0 - s,
            panel.origin.y + 14.0,
        ),
        size: Point2D::new(s, s),
    }
}

fn drop_zone_rect(panel: Rect) -> Rect {
    let top = panel.origin.y + 44.0;
    let bottom = panel.origin.y + panel.size.y - 44.0;
    Rect {
        origin: Point2D::new(panel.origin.x + PAD, top),
        size: Point2D::new(panel.size.x - PAD * 2.0, bottom - top),
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn t(locale: Locale, key: &str) -> &'static str {
    match key {
        "title" => op_i18n::translate(locale, "figma.title"),
        "drop" => op_i18n::translate(locale, "figma.dropFile"),
        "browse" => op_i18n::translate(locale, "figma.orBrowse"),
        "footer" => op_i18n::translate(locale, "figma.exportTip"),
        _ => "",
    }
}

impl Widget for FigmaImportModal {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(MODAL_WIDTH, MODAL_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, 12.0, self.theme.border, 1.0);

        let title = TextLayout::single_run(
            t(self.locale, "title"),
            "system-ui",
            14.0,
            to_jian(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + 26.0),
        );

        // Close X — smaller stroke, tighter.
        let close = close_rect(rect);
        draw_icon(
            cx.backend,
            Icon::Close,
            close.origin,
            close.size.x,
            self.theme.muted_foreground,
            1.6,
        );

        // Compact info panel with a small Figma glyph for character.
        let drop = drop_zone_rect(rect);
        cx.backend.fill_round_rect(drop, 10.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(drop, 10.0, self.theme.border, 1.0);

        // Small Figma brand glyph centred above the headline.
        let glyph_size = 24.0;
        crate::widgets::brand_icons::paint_figma_logo(
            cx.backend,
            Point2D::new(
                drop.origin.x + drop.size.x / 2.0 - glyph_size / 2.0,
                drop.origin.y + drop.size.y / 2.0 - glyph_size - 16.0,
            ),
            glyph_size,
            self.theme.muted_foreground,
        );

        let headline = t(self.locale, "drop");
        let head_w = cx.backend.measure_text(headline, 13.0);
        let head_layout = TextLayout::single_run(
            headline,
            "system-ui",
            13.0,
            to_jian(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &head_layout,
            Point2D::new(
                drop.origin.x + (drop.size.x - head_w) / 2.0,
                drop.origin.y + drop.size.y / 2.0 + 12.0,
            ),
        );

        let sub = t(self.locale, "browse");
        let sub_w = cx.backend.measure_text(sub, 11.0);
        let sub_layout = TextLayout::single_run(
            sub,
            "system-ui",
            11.0,
            to_jian(self.theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &sub_layout,
            Point2D::new(
                drop.origin.x + (drop.size.x - sub_w) / 2.0,
                drop.origin.y + drop.size.y / 2.0 + 30.0,
            ),
        );

        let footer = TextLayout::single_run(
            t(self.locale, "footer"),
            "system-ui",
            11.0,
            to_jian(self.theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &footer,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + rect.size.y - 16.0),
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(op_i18n::translate(self.locale, "a11y.figmaImport"));
        node
    }
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_drop_zone_requests_figma_import() {
        let modal = FigmaImportModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        let drop = drop_zone_rect(panel);
        let point = Point2D::new(
            drop.origin.x + drop.size.x / 2.0,
            drop.origin.y + drop.size.y / 2.0,
        );

        assert_eq!(modal.hit_test(panel, point), FigmaImportHit::DropZone);
    }
}
