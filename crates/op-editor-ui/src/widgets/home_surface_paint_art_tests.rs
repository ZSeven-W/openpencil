//! The App task's example art speaks the UI locale.

use super::super::art::paint_app_art;
use super::super::StudioPalette;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::{HomeDevice, ThemeMode};
use op_i18n::Locale;

/// Records every text run and answers measurements with a fixed advance
/// per char, so the fit logic runs without a font stack.
#[derive(Default)]
struct TextCapture {
    texts: Vec<String>,
}

impl RenderBackend for TextCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        for run in layout.runs() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
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

fn painted(locale: Locale, device: HomeDevice) -> Vec<String> {
    let mut backend = TextCapture::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_app_art(
        &mut cx,
        Rect::xywh(0.0, 0.0, 560.0, 260.0),
        StudioPalette::for_mode(ThemeMode::Light),
        device,
        1.0,
        locale,
    );
    backend.texts
}

fn has_cjk(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3000}'..='\u{30FF}').contains(&c))
}

#[test]
fn english_art_paints_no_chinese() {
    for device in [HomeDevice::Mobile, HomeDevice::Desktop] {
        let texts = painted(Locale::EnUs, device);
        assert!(!texts.is_empty(), "{device:?} painted no text");
        let leaked: Vec<_> = texts.iter().filter(|text| has_cjk(text)).collect();
        assert!(leaked.is_empty(), "{device:?} leaked CJK: {leaked:?}");
    }
    let phones = painted(Locale::EnUs, HomeDevice::Mobile);
    assert!(phones.iter().any(|text| text == "Classic Latte"));
    assert!(phones.iter().any(|text| text == "Orders"));
}

#[test]
fn chinese_art_keeps_the_prototype_copy() {
    let phones = painted(Locale::ZhCn, HomeDevice::Mobile);
    for expected in ["经典拿铁", "菜单", "你的订单很快就好", "首页"] {
        assert!(
            phones.iter().any(|text| text == expected),
            "missing {expected}"
        );
    }
    let counter = painted(Locale::ZhCn, HomeDevice::Desktop);
    assert!(counter.iter().any(|text| text == "#1028  经典拿铁 × 2"));
    assert!(counter.iter().any(|text| text == "1 分钟前 · 到店自取"));
}

#[test]
fn a_latin_subtitle_only_shows_when_it_adds_something() {
    use super::latin_subtitle;
    assert_eq!(latin_subtitle("美式咖啡", "Americano"), Some("Americano"));
    assert_eq!(latin_subtitle("Americano", "Americano"), None);
    let english_menu = painted(Locale::EnUs, HomeDevice::Mobile);
    assert_eq!(
        english_menu
            .iter()
            .filter(|text| text.as_str() == "Americano")
            .count(),
        1,
        "the English menu names Americano once"
    );
}
