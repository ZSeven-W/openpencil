//! Native Lucide icon picker used by the Toolbar shape dropdown.
//!
//! This is deliberately small compared with the TS Iconify dialog:
//! it lists the Lucide glyphs the Rust renderer already knows how to
//! paint as `icon_font` nodes. Search is owned by the host keyboard
//! router while the panel is open.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{EditorState, Locale};

pub const ICON_PICKER_PANEL_W: f32 = 320.0;
pub const ICON_PICKER_PANEL_H: f32 = 420.0;

const PAD: f32 = 14.0;
const HEADER_H: f32 = 40.0;
const SEARCH_H: f32 = 42.0;
const CLOSE_BTN: f32 = 24.0;
const ROW_H: f32 = 34.0;
const ICON_SIZE: f32 = 17.0;
const ROW_PAD_X: f32 = 10.0;
const CHAR_W: f32 = 6.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconPickerHit {
    Close,
    SelectIcon(String),
    Inside,
}

struct IconPickerItem {
    name: &'static str,
    label: &'static str,
    icon: Icon,
}

const ICONS: &[IconPickerItem] = &[
    item("search", "Search", Icon::Search),
    item("home", "Home", Icon::Home),
    item("user", "User", Icon::User),
    item("settings", "Settings", Icon::Settings),
    item("bell", "Bell", Icon::Bell),
    item("mail", "Mail", Icon::Mail),
    item("calendar", "Calendar", Icon::Calendar),
    item("clock", "Clock", Icon::Clock),
    item("heart", "Heart", Icon::Heart),
    item("star", "Star", Icon::Star),
    item("check", "Check", Icon::Check),
    item("x", "X", Icon::Close),
    item("plus", "Plus", Icon::Plus),
    item("minus", "Minus", Icon::Minus),
    item("arrow-right", "Arrow Right", Icon::ArrowRight),
    item("arrow-left", "Arrow Left", Icon::ArrowLeft),
    item("chevron-left", "Chevron Left", Icon::ChevronLeft),
    item("chevron-right", "Chevron Right", Icon::ChevronRight),
    item("chevron-down", "Chevron Down", Icon::ChevronDown),
    item("more-horizontal", "More Horizontal", Icon::MoreHorizontal),
    item("more-vertical", "More Vertical", Icon::MoreVertical),
    item("trash-2", "Trash", Icon::Trash),
    item("copy", "Copy", Icon::Copy),
    item("pencil", "Pencil", Icon::Pencil),
    item("download", "Download", Icon::Download),
    item("save", "Save", Icon::Save),
    item("image", "Image", Icon::Image),
    item("camera", "Camera", Icon::Camera),
    item("video", "Video", Icon::Video),
    item("music", "Music", Icon::Music),
    item("phone", "Phone", Icon::Phone),
    item("map-pin", "Map Pin", Icon::MapPin),
    item("info", "Info", Icon::Info),
    item("alert-circle", "Alert Circle", Icon::AlertCircle),
    item("help-circle", "Help Circle", Icon::HelpCircle),
    item("message-circle", "Message Circle", Icon::MessageCircle),
    item("message-square", "Message Square", Icon::MessageSquare),
    item("shopping-cart", "Shopping Cart", Icon::ShoppingCart),
    item("shopping-bag", "Shopping Bag", Icon::ShoppingBag),
    item("credit-card", "Credit Card", Icon::CreditCard),
    item("send", "Send", Icon::Send),
    item("rocket", "Rocket", Icon::Rocket),
    item("activity", "Activity", Icon::Activity),
    item("trending-up", "Trending Up", Icon::TrendingUp),
    item("trending-down", "Trending Down", Icon::TrendingDown),
    item("bar-chart-2", "Bar Chart", Icon::BarChart2),
    item("layout-dashboard", "Dashboard", Icon::LayoutDashboard),
    item("users", "Users", Icon::Users),
    item("package", "Package", Icon::Package),
    item("zap", "Zap", Icon::Zap),
    item("sliders-horizontal", "Sliders", Icon::SlidersHorizontal),
    item("lock", "Lock", Icon::Lock),
    item("unlock", "Unlock", Icon::LockOpen),
    item("eye", "Eye", Icon::Eye),
    item("eye-off", "Eye Off", Icon::EyeOff),
    item("github", "GitHub", Icon::Github),
    item("globe", "Globe", Icon::Globe),
    item("terminal", "Terminal", Icon::Terminal),
    item("bot", "Bot", Icon::Bot),
];

const fn item(name: &'static str, label: &'static str, icon: Icon) -> IconPickerItem {
    IconPickerItem { name, label, icon }
}

pub struct IconPickerPanel<'a> {
    state: &'a EditorState,
    theme: Theme,
    locale: Locale,
}

impl<'a> IconPickerPanel<'a> {
    pub fn for_editor(state: &'a EditorState) -> Option<IconPickerPanel<'a>> {
        if !state.editor_ui.icon_picker_open {
            return None;
        }
        Some(IconPickerPanel {
            state,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
        })
    }

    fn t(&self, key: &'static str) -> &'static str {
        crate::i18n::translate(self.locale, key)
    }

    fn filtered(&self) -> Vec<&'static IconPickerItem> {
        let query = self
            .state
            .editor_ui
            .icon_picker_search
            .trim()
            .to_lowercase();
        if query.is_empty() {
            return ICONS.iter().collect();
        }
        ICONS
            .iter()
            .filter(|item| item.name.contains(&query) || item.label.to_lowercase().contains(&query))
            .collect()
    }

    fn close_rect(panel: Rect) -> Rect {
        let y = panel.origin.y + (HEADER_H - CLOSE_BTN) / 2.0;
        Rect {
            origin: Point2D::new(panel.origin.x + panel.size.x - PAD - CLOSE_BTN, y),
            size: Point2D::new(CLOSE_BTN, CLOSE_BTN),
        }
    }

    fn search_rect(panel: Rect) -> Rect {
        Rect {
            origin: Point2D::new(panel.origin.x + PAD, panel.origin.y + HEADER_H + 6.0),
            size: Point2D::new(panel.size.x - PAD * 2.0, 28.0),
        }
    }

    fn list_top(panel: Rect) -> f32 {
        panel.origin.y + HEADER_H + SEARCH_H
    }

    fn visible_row_capacity(panel: Rect) -> usize {
        ((panel.size.y - HEADER_H - SEARCH_H - PAD) / ROW_H)
            .floor()
            .max(0.0) as usize
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<IconPickerHit> {
        if !rect_contains(panel, point) {
            return None;
        }
        if rect_contains(Self::close_rect(panel), point) {
            return Some(IconPickerHit::Close);
        }
        let list_top = Self::list_top(panel);
        if point.y >= list_top {
            let row = ((point.y - list_top) / ROW_H) as usize;
            let items = self.filtered();
            let capacity = Self::visible_row_capacity(panel);
            if row < capacity {
                if let Some(item) = items.get(row) {
                    return Some(IconPickerHit::SelectIcon(item.name.to_string()));
                }
            }
        }
        Some(IconPickerHit::Inside)
    }

    pub fn paint(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        cx.backend.fill_round_rect(panel, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(panel, 8.0, self.theme.border, 1.0);

        let title = TextLayout::single_run(
            self.t("icon.title"),
            "system-ui",
            13.0,
            to_jian(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(panel.origin.x + PAD, panel.origin.y + 25.0),
        );

        let close = Self::close_rect(panel);
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(close.origin.x + 5.0, close.origin.y + 5.0),
            14.0,
            self.theme.muted_foreground,
            1.4,
        );

        let search = Self::search_rect(panel);
        cx.backend.fill_round_rect(search, 6.0, self.theme.muted);
        draw_icon(
            cx.backend,
            Icon::Search,
            Point2D::new(search.origin.x + 8.0, search.origin.y + 6.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
        let query = self.state.editor_ui.icon_picker_search.trim();
        let search_text = if query.is_empty() {
            self.t("icon.searchIcons")
        } else {
            query
        };
        let search_color = if query.is_empty() {
            self.theme.muted_foreground
        } else {
            self.theme.foreground
        };
        let search_layout = TextLayout::single_run(
            search_text,
            "system-ui",
            12.0,
            to_jian(search_color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &search_layout,
            Point2D::new(search.origin.x + 30.0, search.origin.y + 18.0),
        );

        let filtered = self.filtered();
        if filtered.is_empty() {
            let empty = TextLayout::single_run(
                self.t("icon.noIconsFound"),
                "system-ui",
                12.0,
                to_jian(self.theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &empty,
                Point2D::new(panel.origin.x + PAD, Self::list_top(panel) + 28.0),
            );
            return;
        }

        let rows = Self::visible_row_capacity(panel);
        for (idx, item) in filtered.into_iter().take(rows).enumerate() {
            let y = Self::list_top(panel) + idx as f32 * ROW_H;
            let row = Rect {
                origin: Point2D::new(panel.origin.x + 6.0, y),
                size: Point2D::new(panel.size.x - 12.0, ROW_H),
            };
            cx.backend.fill_round_rect(row, 6.0, self.theme.popover);
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(row.origin.x + ROW_PAD_X, y + (ROW_H - ICON_SIZE) / 2.0),
                ICON_SIZE,
                self.theme.foreground,
                1.5,
            );
            let label = truncate(item.label, ((row.size.x - 54.0) / CHAR_W) as usize);
            let text = TextLayout::single_run(
                &label,
                "system-ui",
                12.0,
                to_jian(self.theme.foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&text, Point2D::new(row.origin.x + 38.0, y + 21.0));
        }
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push_str("...");
    out
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
