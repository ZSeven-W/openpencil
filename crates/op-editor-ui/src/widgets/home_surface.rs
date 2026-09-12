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

#[path = "home_surface_palette.rs"]
mod palette;
pub use palette::HomePalette;

pub const HOME_TOPBAR_H: f32 = 64.0;
const CONTENT_MAX_W: f32 = 720.0;
const PAGE_PAD: f32 = 48.0;
const CARD_MAX_W: f32 = 280.0;
const CARD_GAP: f32 = 22.0;
const CARD_ROW_GAP: f32 = 18.0;
const FOOTER_H: f32 = 22.0;
const FOOTER_BOTTOM_GAP: f32 = 14.0;
const STACK_BOTTOM_GAP: f32 = 14.0;
const HEADLINE_H: f32 = 52.0;
const SUBTITLE_H: f32 = 18.0;
const SHEET_H: f32 = 150.0;
const CHIP_H: f32 = 38.0;
const EXPECTED_H: f32 = 26.0;
const CARDS_GAP: f32 = 40.0;
const CARD_H: f32 = 200.0;

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

    fn translated_stack(self, scroll_y: f32) -> Self {
        let translate = |rect: Rect| {
            Rect::xywh(
                rect.origin.x,
                rect.origin.y - scroll_y,
                rect.size.x,
                rect.size.y,
            )
        };
        Self {
            headline: translate(self.headline),
            subtitle: translate(self.subtitle),
            sheet: translate(self.sheet),
            sheet_text: translate(self.sheet_text),
            screenshot: translate(self.screenshot),
            reference_link: translate(self.reference_link),
            figma: translate(self.figma),
            example: translate(self.example),
            send: translate(self.send),
            chips: self.chips.map(translate),
            expected: translate(self.expected),
            device_mobile: translate(self.device_mobile),
            device_desktop: translate(self.device_desktop),
            cards: self.cards.map(translate),
            footer: self.footer,
            professional: self.professional,
        }
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
        Self::layout_for_scrolled(viewport_width, viewport_height, bound, 0.0)
    }

    pub fn layout_for_scrolled(
        viewport_width: f32,
        viewport_height: f32,
        bound: Option<HomeFamily>,
        scroll_y: f32,
    ) -> HomeLayout {
        let width = viewport_width.max(1.0);
        let height = viewport_height.max(1.0);
        let narrow = width <= 1180.0;
        let footer = Rect::xywh(
            PAGE_PAD,
            height - FOOTER_BOTTOM_GAP - FOOTER_H,
            width - PAGE_PAD * 2.0,
            FOOTER_H,
        );
        let top = HOME_TOPBAR_H;
        let bottom = if narrow {
            (footer.origin.y - STACK_BOTTOM_GAP).max(top)
        } else {
            footer.origin.y.max(top)
        };
        let pre_cards_h = HEADLINE_H
            + 14.0
            + SUBTITLE_H
            + 34.0
            + SHEET_H
            + 18.0
            + CHIP_H
            + 14.0
            + EXPECTED_H
            + CARDS_GAP;
        let columns = if narrow { 2 } else { 4 };
        let rows: usize = if narrow { 2 } else { 1 };
        let card_height = if narrow {
            (((bottom - top) - pre_cards_h - CARD_ROW_GAP) / 2.0).clamp(104.0, CARD_H)
        } else {
            CARD_H
        };
        let stack_height = pre_cards_h
            + card_height * rows as f32
            + CARD_ROW_GAP * (rows.saturating_sub(1) as f32);
        let stack_top = if narrow {
            top.max(top + ((bottom - top - stack_height) / 2.0).max(0.0))
        } else {
            top + ((bottom - top - stack_height) / 2.0).max(0.0)
        };
        let headline = Rect::xywh((width - 520.0) / 2.0, stack_top, 520.0, HEADLINE_H);
        let subtitle = Rect::xywh(
            (width - 520.0) / 2.0,
            headline.origin.y + HEADLINE_H + 14.0,
            520.0,
            SUBTITLE_H,
        );
        let sheet_width = CONTENT_MAX_W.min((width - PAGE_PAD * 2.0).max(260.0));
        let sheet = Rect::xywh(
            (width - sheet_width) / 2.0,
            subtitle.origin.y + SUBTITLE_H + 34.0,
            sheet_width,
            SHEET_H,
        );
        let sheet_text = Rect::xywh(
            sheet.origin.x + 24.0,
            sheet.origin.y + 20.0,
            sheet.size.x - 40.0,
            34.0,
        );
        let refs_top = sheet.origin.y + sheet.size.y - 45.0;
        let screenshot = Rect::xywh(sheet.origin.x + 24.0, refs_top, 70.0, 28.0);
        let reference_link = Rect::xywh(screenshot.origin.x + 70.0, refs_top, 108.0, 28.0);
        let figma = Rect::xywh(reference_link.origin.x + 108.0, refs_top, 68.0, 28.0);
        let example = Rect::xywh(figma.origin.x + 68.0, refs_top, 126.0, 28.0);
        let send = Rect::xywh(
            sheet.origin.x + sheet.size.x - 56.0,
            sheet.origin.y + sheet.size.y - 52.0,
            40.0,
            40.0,
        );

        let chip_widths = [106.0, 106.0, 106.0, 106.0];
        let chip_gap = 8.0;
        let chips_width = chip_widths.iter().sum::<f32>() + chip_gap * 3.0;
        let chips_x = (width - chips_width) / 2.0;
        let chips_y = sheet.origin.y + sheet.size.y + 18.0;
        let chips = [
            Rect::xywh(chips_x, chips_y, chip_widths[0], CHIP_H),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap),
                chips_y,
                chip_widths[1],
                CHIP_H,
            ),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap) * 2.0,
                chips_y,
                chip_widths[2],
                CHIP_H,
            ),
            Rect::xywh(
                chips_x + (chip_widths[0] + chip_gap) * 3.0,
                chips_y,
                chip_widths[3],
                CHIP_H,
            ),
        ];

        let expected = Rect::xywh(
            (width - 580.0) / 2.0,
            chips_y + CHIP_H + 14.0,
            580.0,
            EXPECTED_H,
        );
        let card_width = if narrow {
            ((width - PAGE_PAD * 2.0 - CARD_GAP) / 2.0).min(CARD_MAX_W)
        } else {
            CARD_MAX_W
        };
        let grid_width = card_width * columns as f32 + CARD_GAP * (columns - 1) as f32;
        let grid_top = expected.origin.y + EXPECTED_H + CARDS_GAP;
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
        let professional = Rect::xywh((width - 270.0).max(PAGE_PAD), 22.0, 222.0, 28.0);
        let device_mobile = if bound == Some(HomeFamily::AppUi) {
            Rect::xywh(
                expected.origin.x + 474.0,
                expected.origin.y,
                52.0,
                EXPECTED_H,
            )
        } else {
            Rect::ZERO
        };
        let device_desktop = if bound == Some(HomeFamily::AppUi) {
            Rect::xywh(
                expected.origin.x + 526.0,
                expected.origin.y,
                52.0,
                EXPECTED_H,
            )
        } else {
            Rect::ZERO
        };
        let layout = HomeLayout {
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
        };
        layout.translated_stack(scroll_y.max(0.0))
    }

    pub fn max_scroll_for(
        viewport_width: f32,
        viewport_height: f32,
        bound: Option<HomeFamily>,
    ) -> f32 {
        let layout = Self::layout_for(viewport_width, viewport_height, bound);
        let content_bottom = layout
            .cards
            .iter()
            .map(|card| card.origin.y + card.size.y)
            .fold(layout.expected.origin.y + layout.expected.size.y, f32::max);
        (content_bottom - layout.footer.origin.y + STACK_BOTTOM_GAP).max(0.0)
    }

    pub fn layout(&self, viewport_width: f32, viewport_height: f32) -> HomeLayout {
        let max_scroll = Self::max_scroll_for(viewport_width, viewport_height, self.state.bound);
        Self::layout_for_scrolled(
            viewport_width,
            viewport_height,
            self.state.bound,
            self.state.scroll_y.clamp(0.0, max_scroll),
        )
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
