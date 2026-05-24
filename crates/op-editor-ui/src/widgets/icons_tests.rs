use crate::widgets::icons::{draw_icon, paint_icon_font_node, Icon};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct CountingBackend {
    paths: usize,
    fills: usize,
}

impl RenderBackend for CountingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
        self.paths += 1;
    }
    fn fill_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: f32, _: Color) {
        self.fills += 1;
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn paint_one(icon: Icon) -> CountingBackend {
    let mut b = CountingBackend::default();
    draw_icon(
        &mut b,
        icon,
        Point2D::new(0.0, 0.0),
        16.0,
        Color::WHITE,
        1.5,
    );
    b
}

#[test]
fn plus_renders_two_paths() {
    let b = paint_one(Icon::Plus);
    assert_eq!(b.paths, 2);
}

#[test]
fn minus_renders_one_path() {
    let b = paint_one(Icon::Minus);
    assert_eq!(b.paths, 1);
}

#[test]
fn sun_renders_disc_plus_eight_rays() {
    let b = paint_one(Icon::Sun);
    assert_eq!(b.paths, 9);
}

#[test]
fn every_variant_paints_at_least_one_primitive() {
    for icon in [
        Icon::Cursor,
        Icon::Square,
        Icon::ChevronDown,
        Icon::Type,
        Icon::Frame,
        Icon::Hand,
        Icon::Undo,
        Icon::Redo,
        Icon::Braces,
        Icon::BookOpen,
        Icon::Plus,
        Icon::Minus,
        Icon::Search,
        Icon::Sun,
        Icon::Globe,
        Icon::Maximize,
        Icon::Hash,
        Icon::PanelLeft,
        Icon::FolderOpen,
        Icon::Sparkles,
        Icon::Close,
        Icon::ChevronUp,
        Icon::MessageSquare,
        Icon::LayoutGrid,
        Icon::Rows3,
        Icon::Columns3,
        Icon::RotateCw,
        Icon::Diamond,
        Icon::Component,
        Icon::Unlink,
        Icon::Check,
        Icon::ArrowUpRight,
    ] {
        let b = paint_one(icon);
        assert!(b.paths > 0, "{:?} drew nothing", icon);
    }
}

#[test]
fn first_party_icon_font_names_all_resolve() {
    for name in [
        "calendar",
        "check",
        "chevron-down",
        "chevron-left",
        "chevron-right",
        "clock",
        "map-pin",
        "more-vertical",
        "play",
        "search",
        "star",
        "x",
        "arrow-right",
        "check-circle",
        "alert-triangle",
        "alert-octagon",
        "sticky-note",
        "bar-chart-2",
        "bold",
        "italic",
        "underline",
        "shopping-cart",
        "shopping-bag",
        "message-circle",
        "rocket",
        "menu",
        "credit-card",
        "trending-up",
        "trending-down",
        "compass",
        "refresh-cw",
        "layout-dashboard",
        "users",
        "package",
        "zap",
        "sliders-horizontal",
        "activity",
        "loader",
        "focus",
        "chart-line",
        "settings-2",
    ] {
        assert!(
            Icon::from_name(name).is_some(),
            "first-party iconFontName {:?} fell through to placeholder",
            name
        );
    }
}

#[test]
fn bundled_iconify_catalog_contains_requested_collections() {
    use crate::widgets::icon_catalog::{lookup_icon, IconRenderStyle};
    assert_eq!(
        lookup_icon("lucide", "airplay").map(|i| i.style),
        Some(IconRenderStyle::Stroke)
    );
    assert_eq!(
        lookup_icon("feather", "airplay").map(|i| i.style),
        Some(IconRenderStyle::Stroke)
    );
    assert_eq!(
        lookup_icon("simple-icons", "github").map(|i| i.style),
        Some(IconRenderStyle::Fill)
    );
}

#[test]
fn icon_font_node_paints_simple_icon_as_fill_path() {
    let mut b = CountingBackend::default();
    paint_icon_font_node(
        &mut b,
        "simple-icons",
        "github",
        Rect::xywh(0.0, 0.0, 24.0, 24.0),
        Some(Color::WHITE),
    );
    assert_eq!(b.fills, 1);
    assert_eq!(b.paths, 0);
}
