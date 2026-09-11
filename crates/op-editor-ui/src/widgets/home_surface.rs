//! The desktop-first 制图台 Home surface.
//!
//! Geometry and hit-testing live here; the immediate-mode paint pass is in
//! `home_surface_paint.rs`. The surface is intentionally document-agnostic:
//! it reads the same `EditorState` as the canvas and never creates a second
//! model.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, HomeDevice, HomeFamily, HomeHit, HomeState};

pub const HOME_TOPBAR_H: f32 = 64.0;
const CONTENT_MAX_W: f32 = 720.0;
const PAGE_PAD: f32 = 48.0;
const CARD_MAX_W: f32 = 280.0;
const CARD_GAP: f32 = 22.0;
const CARD_ROW_GAP: f32 = 18.0;
const FOOTER_H: f32 = 22.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomeLayout {
    pub headline: Rect,
    pub subtitle: Rect,
    pub sheet: Rect,
    pub sheet_text: Rect,
    pub screenshot: Rect,
    pub reference_link: Rect,
    pub figma: Rect,
    pub example: Rect,
    pub send: Rect,
    pub chips: [Rect; 4],
    pub expected: Rect,
    pub device_mobile: Rect,
    pub device_desktop: Rect,
    pub cards: [Rect; 4],
    pub footer: Rect,
    pub professional: Rect,
}

impl HomeLayout {
    pub fn cards_wrap(&self, viewport_width: f32) -> bool {
        viewport_width <= 1180.0
    }

    pub fn card_family(index: usize) -> Option<HomeFamily> {
        HomeFamily::ALL.get(index).copied()
    }
}

pub struct HomeSurface<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a HomeState,
    pub ui: &'a op_editor_core::EditorUiState,
    pub now_ms: u64,
}

impl<'a> HomeSurface<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<Self> {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        state.editor_ui.home.visible.then(|| Self {
            id: WidgetId::new(7600),
            theme: theme_for(&state.editor_ui),
            state: &state.editor_ui.home,
            ui: &state.editor_ui,
            now_ms,
        })
    }

    pub fn layout_for(
        viewport_width: f32,
        viewport_height: f32,
        bound: Option<HomeFamily>,
    ) -> HomeLayout {
        let width = viewport_width.max(1.0);
        let height = viewport_height.max(1.0);
        let narrow = width <= 1180.0;
        let headline_y = if height < 820.0 { 82.0 } else { 112.0 };
        let headline = Rect::xywh((width - 520.0) / 2.0, headline_y, 520.0, 58.0);
        let subtitle = Rect::xywh((width - 520.0) / 2.0, headline.origin.y + 67.0, 520.0, 22.0);
        let sheet_width = CONTENT_MAX_W.min((width - PAGE_PAD * 2.0).max(260.0));
        let sheet_height = if height < 820.0 { 132.0 } else { 150.0 };
        let sheet = Rect::xywh(
            (width - sheet_width) / 2.0,
            subtitle.origin.y + subtitle.size.y + 28.0,
            sheet_width,
            sheet_height,
        );
        let sheet_text = Rect::xywh(
            sheet.origin.x + 20.0,
            sheet.origin.y + 20.0,
            sheet.size.x - 40.0,
            (sheet.size.y - 68.0).max(32.0),
        );
        let refs_top = sheet.origin.y + sheet.size.y - 46.0;
        let screenshot = Rect::xywh(sheet.origin.x + 12.0, refs_top, 62.0, 28.0);
        let reference_link = Rect::xywh(screenshot.origin.x + 62.0, refs_top, 92.0, 28.0);
        let figma = Rect::xywh(reference_link.origin.x + 92.0, refs_top, 58.0, 28.0);
        let example = Rect::xywh(figma.origin.x + 58.0, refs_top, 112.0, 28.0);
        let send = Rect::xywh(
            sheet.origin.x + sheet.size.x - 54.0,
            sheet.origin.y + sheet.size.y - 48.0,
            38.0,
            38.0,
        );

        let chip_widths = [106.0, 106.0, 106.0, 106.0];
        let chip_gap = 8.0;
        let chips_width = chip_widths.iter().sum::<f32>() + chip_gap * 3.0;
        let chips_x = (width - chips_width) / 2.0;
        let chips_y = sheet.origin.y + sheet.size.y + 16.0;
        let chips = [
            Rect::xywh(chips_x, chips_y, chip_widths[0], 38.0),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap),
                chips_y,
                chip_widths[1],
                38.0,
            ),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap) * 2.0,
                chips_y,
                chip_widths[2],
                38.0,
            ),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap) * 3.0,
                chips_y,
                chip_widths[3],
                38.0,
            ),
        ];

        let expected = bound.map_or(Rect::ZERO, |_| {
            Rect::xywh((width - 470.0) / 2.0, chips_y + 48.0, 470.0, 30.0)
        });
        let hero_bottom = if bound.is_some() {
            expected.origin.y + expected.size.y
        } else {
            chips_y + 38.0
        };

        let columns = if narrow { 2 } else { 4 };
        let rows = if narrow { 2 } else { 1 };
        let card_width = if narrow {
            ((width - PAGE_PAD * 2.0 - CARD_GAP) / 2.0).min(CARD_MAX_W)
        } else {
            CARD_MAX_W
        };
        let card_height = if narrow {
            ((height - 44.0 - FOOTER_H - CARD_ROW_GAP - hero_bottom - 28.0) / 2.0)
                .clamp(104.0, 196.0)
        } else {
            196.0
        };
        let grid_width = card_width * columns as f32 + CARD_GAP * (columns - 1) as f32;
        let grid_top = (height
            - 44.0
            - FOOTER_H
            - card_height * rows as f32
            - CARD_ROW_GAP * (rows - 1) as f32)
            .max(hero_bottom + 28.0);
        let grid_x = (width - grid_width) / 2.0;
        let mut cards = [Rect::ZERO; 4];
        for (index, card) in cards.iter_mut().enumerate() {
            let column = index % columns;
            let row = index / columns;
            *card = Rect::xywh(
                grid_x + column as f32 * (card_width + CARD_GAP),
                grid_top + row as f32 * (card_height + CARD_ROW_GAP),
                card_width,
                card_height,
            );
        }
        let footer = Rect::xywh(
            PAGE_PAD,
            height - 14.0 - FOOTER_H,
            width - PAGE_PAD * 2.0,
            FOOTER_H,
        );
        let professional = Rect::xywh((width - 270.0).max(PAGE_PAD), 22.0, 222.0, 28.0);
        let device_mobile = if bound == Some(HomeFamily::AppUi) {
            Rect::xywh(
                expected.origin.x + 282.0,
                expected.origin.y + 1.0,
                76.0,
                28.0,
            )
        } else {
            Rect::ZERO
        };
        let device_desktop = if bound == Some(HomeFamily::AppUi) {
            Rect::xywh(
                expected.origin.x + 358.0,
                expected.origin.y + 1.0,
                76.0,
                28.0,
            )
        } else {
            Rect::ZERO
        };
        HomeLayout {
            headline,
            subtitle,
            sheet,
            sheet_text,
            screenshot,
            reference_link,
            figma,
            example,
            send,
            chips,
            expected,
            device_mobile,
            device_desktop,
            cards,
            footer,
            professional,
        }
    }

    pub fn layout(&self, viewport_width: f32, viewport_height: f32) -> HomeLayout {
        Self::layout_for(viewport_width, viewport_height, self.state.bound)
    }

    pub fn hit_test(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<HomeHit> {
        let layout = self.layout(viewport_width, viewport_height);
        if layout.professional.contains(point) {
            return Some(HomeHit::Professional);
        }
        if layout.send.contains(point) {
            return Some(HomeHit::Send);
        }
        for (index, rect) in layout.chips.into_iter().enumerate() {
            if rect.contains(point) {
                return Some(HomeHit::Chip(HomeFamily::ALL[index]));
            }
        }
        if self.state.bound == Some(HomeFamily::AppUi) {
            if layout.device_mobile.contains(point) {
                return Some(HomeHit::Device(HomeDevice::Mobile));
            }
            if layout.device_desktop.contains(point) {
                return Some(HomeHit::Device(HomeDevice::Desktop));
            }
        }
        if layout.screenshot.contains(point) {
            return Some(HomeHit::Attachment);
        }
        if layout.reference_link.contains(point) {
            return Some(HomeHit::ReferenceLink);
        }
        if layout.figma.contains(point) {
            return Some(HomeHit::Figma);
        }
        if layout.example.contains(point) {
            return Some(HomeHit::TryExample);
        }
        for (index, rect) in layout.cards.into_iter().enumerate() {
            if rect.contains(point) {
                return HomeLayout::card_family(index).map(HomeHit::Card);
            }
        }
        if layout.sheet.contains(point) {
            return Some(HomeHit::Sheet);
        }
        if layout.footer.contains(point) {
            let relative_x = point.x - layout.footer.origin.x;
            return Some(if relative_x < 76.0 {
                HomeHit::Recent
            } else if relative_x < 184.0 {
                HomeHit::NewCanvas
            } else {
                HomeHit::OpenFile
            });
        }
        None
    }

    pub fn focused_input_caret_rect(&self, viewport_width: f32, viewport_height: f32) -> Rect {
        let text = self.state.input.text();
        let layout = self.layout(viewport_width, viewport_height);
        let caret = jian_core::text_input::prev_char_boundary(
            text,
            self.state.input.caret().min(text.len()),
        );
        let x = layout.sheet_text.origin.x + text[..caret].chars().count() as f32 * 8.0;
        Rect::xywh(
            x.min(layout.sheet_text.origin.x + layout.sheet_text.size.x - 1.0),
            layout.sheet_text.origin.y,
            1.5,
            20.0,
        )
    }

    pub fn sheet_text_offset_at(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        point: Point2D,
    ) -> Option<usize> {
        let rect = self.layout(viewport_width, viewport_height).sheet_text;
        rect.contains(point)
            .then_some(self.state.input.text().len())
    }
}

impl Widget for HomeSurface<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, 900.0),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        self.paint_home(cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Main);
        node.set_label("制图台");
        node
    }
}

#[path = "home_surface_paint.rs"]
mod paint;

impl HomeSurface<'_> {
    fn paint_home(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_home(self, cx, rect);
    }
}

#[cfg(test)]
#[path = "home_surface_tests.rs"]
mod tests;
