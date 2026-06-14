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
use crate::{Point2D, Rect, TextLayout};
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
    /// Which target the cursor is over — drives the hover wash.
    hover: Option<op_editor_core::FigmaImportButton>,
    /// Which target is currently pressed by the primary pointer.
    pressed: Option<op_editor_core::FigmaImportButton>,
}

impl FigmaImportModal {
    pub fn for_editor(state: &EditorState) -> Self {
        Self {
            id: WidgetId::new(5400),
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            hover: state.editor_ui.figma_import_hover,
            pressed: match state.editor_ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::FigmaImport(button)) => Some(button),
                _ => None,
            },
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
        if !(panel).contains(point) {
            return FigmaImportHit::Outside;
        }
        if (close_rect(panel)).contains(point) {
            return FigmaImportHit::Close;
        }
        if (drop_zone_rect(panel)).contains(point) {
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
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + 26.0),
        );

        // Close X — smaller stroke, tighter. Hover wash + foreground.
        let close = close_rect(rect);
        let close_hovered = self.hover == Some(op_editor_core::FigmaImportButton::Close);
        let close_pressed = self.pressed == Some(op_editor_core::FigmaImportButton::Close);
        // Pad the wash out a little so it reads as a button around
        // the 14 px glyph rather than hugging it.
        let pad = 5.0;
        let bg = Rect {
            origin: Point2D::new(close.origin.x - pad, close.origin.y - pad),
            size: Point2D::new(close.size.x + pad * 2.0, close.size.y + pad * 2.0),
        };
        crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            bg,
            close_hovered,
            close_pressed,
        );
        draw_icon(
            cx.backend,
            Icon::Close,
            close.origin,
            close.size.x,
            if close_hovered || close_pressed {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
            1.6,
        );

        // Compact info panel with a small Figma glyph for character.
        let drop = drop_zone_rect(rect);
        cx.backend.fill_round_rect(drop, 10.0, self.theme.muted);
        // Brighten the browse drop-zone on hover (it's clickable).
        crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            drop,
            self.hover == Some(op_editor_core::FigmaImportButton::DropZone),
            self.pressed == Some(op_editor_core::FigmaImportButton::DropZone),
        );
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
            (self.theme.foreground).to_jian(),
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
            (self.theme.muted_foreground).to_jian(),
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
            (self.theme.muted_foreground).to_jian(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct CaptureBackend {
        round_fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.round_fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn color_close(a: Color, b: Color) -> bool {
        (a.r - b.r).abs() < 1e-6
            && (a.g - b.g).abs() < 1e-6
            && (a.b - b.b).abs() < 1e-6
            && (a.a - b.a).abs() < 1e-6
    }

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

    #[test]
    fn pressed_close_uses_shared_button_feedback() {
        let mut state = EditorState::new();
        state.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::FigmaImport(
            op_editor_core::FigmaImportButton::Close,
        ));
        let modal = FigmaImportModal::for_editor(&state);
        let panel = modal.rect(800.0, 600.0);
        let close = close_rect(panel);
        let pad = 5.0;
        let bg = Rect {
            origin: Point2D::new(close.origin.x - pad, close.origin.y - pad),
            size: Point2D::new(close.size.x + pad * 2.0, close.size.y + pad * 2.0),
        };
        let expected = modal
            .theme
            .button_hover
            .with_alpha(modal.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        modal.paint(&mut cx, panel);

        assert!(
            backend.round_fills.iter().any(|(rect, radius, color)| {
                *rect == bg && (*radius - 6.0).abs() < 0.01 && color_close(*color, expected)
            }),
            "pressed close button should paint the shared pressed feedback token"
        );
    }
}
