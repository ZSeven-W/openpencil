//! `TopBar` — full-width application title bar (Step 4 visual lift).
//!
//! Mirrors `apps/web/src/components/editor/top-bar.tsx`: panel-toggle
//! + folder + brand on the left, file name centered, theme +
//! agent-status + i18n + fullscreen on the right. Click handling is
//! a P6 follow-up; Step 4 paints only.

use crate::document::Document;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const TOP_BAR_HEIGHT: f32 = 40.0;
const ICON_SIZE: f32 = 16.0;
const ICON_BUTTON: f32 = 28.0;
/// Globe locale-picker button — wider than a normal icon button so a
/// chevron-down sits next to the globe glyph (signals the dropdown).
const GLOBE_BUTTON_WIDTH: f32 = 44.0;
/// File-menu compound button — folder + a smaller chevron-down sit
/// inside a single round-rect background. Tighter gap than two
/// separate icon buttons (4 px between glyphs vs ICON_BUTTON + 4).
const FILE_MENU_BUTTON_WIDTH: f32 = 46.0;
const CHEVRON_SIZE: f32 = 12.0;
const PAD: f32 = 12.0;

/// What a click in the top bar resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarHit {
    /// PanelLeft icon — toggle sidebar (LayerPanel) visibility.
    ToggleSidebar,
    /// Folder + chevron compound — toggle the file menu dropdown.
    ToggleFileMenu,
    /// Figma logo — open the .fig import modal.
    OpenFigmaImport,
    /// Sun icon — flip theme dark↔light.
    ToggleTheme,
    /// Globe icon — cycle through UI locales.
    ToggleLocale,
    /// Agents 与 MCP chip — open the agent settings modal.
    OpenAgentSettings,
}

pub struct TopBar {
    pub id: WidgetId,
    /// Centred file name. `String` rather than `&'static str` so an
    /// opened doc can show its basename without leaking a static
    /// slice on every Open.
    pub file_name: String,
    pub agent_count: u32,
    pub theme: Theme,
    pub label_agents_and_mcp: &'static str,
    pub label_agent_singular: &'static str,
}

impl TopBar {
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(5000),
            file_name: file_name.into(),
            agent_count: 1,
            theme: Theme::dark(),
            label_agents_and_mcp: "Agents & MCP",
            label_agent_singular: "agent",
        }
    }

    pub fn untitled() -> Self {
        Self::new("未命名")
    }

    pub fn for_document(doc: &Document) -> Self {
        let file_name = doc
            .ui
            .file_name_display
            .clone()
            .unwrap_or_else(|| doc.t("common.untitled").to_string());
        Self {
            id: WidgetId::new(5000),
            file_name,
            agent_count: 0,
            theme: doc.theme(),
            label_agents_and_mcp: doc.t("topbar.agentsAndMcp"),
            label_agent_singular: doc.t("topbar.agentSingular"),
        }
    }

    /// Returns the on-screen rect of the Globe-plus-chevron locale
    /// button. Used by the host to anchor the LocalePicker dropdown
    /// directly underneath when `Document.ui.locale_picker_open ==
    /// true`. The button itself is wider than a normal icon button
    /// so the chevron-down has room to render.
    /// Anchor rect for the file-menu dropdown overlay (folder +
    /// chevron compound). Host anchors the dropdown directly under
    /// this rect when `Document.ui.file_menu_open == true`.
    pub fn file_menu_rect(top_bar_rect: Rect) -> Rect {
        let file_menu_x = top_bar_rect.origin.x + PAD + ICON_BUTTON + 4.0;
        Rect {
            origin: Point2D::new(file_menu_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    pub fn globe_rect(top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        // Right-cluster layout (right → left): Maximize | Sun | Globe.
        // Maximize + Sun are normal ICON_BUTTON wide; Globe is the
        // wider GLOBE_BUTTON_WIDTH so the chevron fits inside it.
        let globe_x = right - PAD - ICON_BUTTON * 2.0 - GLOBE_BUTTON_WIDTH;
        Rect {
            origin: Point2D::new(globe_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(GLOBE_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// Hit-test the title bar at `point`. Recognised buttons:
    ///   - PanelLeft (left edge) → ToggleSidebar
    ///   - Sun (third from right) → ToggleTheme
    ///   - Globe (fourth from right) → ToggleLocale
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<TopBarHit> {
        if !rect_contains(rect, point) {
            return None;
        }
        let icon_y = rect.origin.y + 8.0;
        let panel_left_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(panel_left_rect, point) {
            return Some(TopBarHit::ToggleSidebar);
        }
        let file_menu_x = rect.origin.x + PAD + ICON_BUTTON + 4.0;
        let file_menu_rect = Rect {
            origin: Point2D::new(file_menu_x, icon_y),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        };
        if rect_contains(file_menu_rect, point) {
            return Some(TopBarHit::ToggleFileMenu);
        }
        let figma_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + 13.0;
        let figma_rect = Rect {
            origin: Point2D::new(figma_x, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(figma_rect, point) {
            return Some(TopBarHit::OpenFigmaImport);
        }
        // Right cluster: Maximize / Sun / Globe-with-chevron (right→left).
        // Maximize + Sun are normal ICON_BUTTON wide; Globe is
        // GLOBE_BUTTON_WIDTH wide because it carries a chevron.
        let right = rect.origin.x + rect.size.x;
        let sun_x = right - PAD - ICON_BUTTON * 2.0;
        let icon_y = rect.origin.y + 8.0;
        let sun_rect = Rect {
            origin: Point2D::new(sun_x, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(sun_rect, point) {
            return Some(TopBarHit::ToggleTheme);
        }
        let globe = Self::globe_rect(rect);
        if rect_contains(globe, point) {
            return Some(TopBarHit::ToggleLocale);
        }
        // Agent chip hit area — slightly larger than the painted
        // chip so off-by-a-few-pixels clicks still register. The
        // exact paint geometry uses skia text measurement which the
        // hit-test can't replicate without a backend; an over-wide
        // CJK glyph estimate (12 px / char) plus 8 px padding on
        // each side keeps the click target slightly looser than
        // the visible chip so the first press always lands.
        let chip_chars = if self.agent_count == 0 {
            self.label_agents_and_mcp.chars().count()
        } else {
            // "{N} {label}" without allocating a String — counts the
            // count digits + 1 space + label chars.
            let digits = (self.agent_count as f32).log10().floor() as usize + 1;
            digits + 1 + self.label_agent_singular.chars().count()
        };
        let dot_w = if self.agent_count == 0 {
            0.0
        } else {
            8.0 + 6.0
        };
        let approx_text_w = chip_chars as f32 * 12.0;
        let chip_w = 8.0 + ICON_SIZE + 6.0 + dot_w + approx_text_w + 12.0 + 16.0;
        let chip_rect = Rect {
            origin: Point2D::new(globe.origin.x - chip_w - 6.0, rect.origin.y + 4.0),
            size: Point2D::new(chip_w, rect.size.y - 8.0),
        };
        if rect_contains(chip_rect, point) {
            return Some(TopBarHit::OpenAgentSettings);
        }
        None
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

impl Widget for TopBar {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, TOP_BAR_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, self.theme.background);
        // Bottom hairline.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + rect.size.y - 1.0),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        // ── Left cluster ───────────────────────────────────────
        let center_y = rect.origin.y + rect.size.y / 2.0;
        let panel_left_x = rect.origin.x + PAD;
        paint_icon_button(cx, &self.theme, panel_left_x, center_y, Icon::PanelLeft);
        // File-menu compound: folder + tight chevron in one button.
        let file_menu_x = panel_left_x + ICON_BUTTON + 4.0;
        paint_file_menu_button(cx, &self.theme, file_menu_x, center_y);
        // Vertical divider before the Figma import affordance.
        let divider_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + 6.0;
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(divider_x, center_y - 8.0),
                size: Point2D::new(1.0, 16.0),
            },
            self.theme.border,
        );
        // Figma import button.
        let figma_x = divider_x + 6.0;
        paint_figma_button(cx, &self.theme, figma_x, center_y);

        // ── Centered file name ─────────────────────────────────
        let name = TextLayout::single_run(
            &self.file_name,
            "system-ui",
            13.0,
            to_jian_color(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        // Approximate text width; skia textlayout would tell us the
        // exact pixel width but for Step 4 we don't pull that in.
        let approx_w = self.file_name.chars().count() as f32 * 9.0;
        cx.backend.draw_text(
            &name,
            Point2D::new(
                rect.origin.x + (rect.size.x - approx_w) / 2.0,
                center_y + 5.0,
            ),
        );

        // ── Right cluster ──────────────────────────────────────
        // Right → left: Maximize | Sun | Globe+Chevron. Globe is a
        // wider compound button (signals the dropdown affordance).
        let mut rx = rect.origin.x + rect.size.x - PAD - ICON_BUTTON;

        // Fullscreen.
        paint_icon_button(cx, &self.theme, rx, center_y, Icon::Maximize);
        rx -= ICON_BUTTON;

        // Theme toggle.
        paint_icon_button(cx, &self.theme, rx, center_y, Icon::Sun);
        rx -= GLOBE_BUTTON_WIDTH;

        // i18n globe + chevron-down (single hit-target).
        let globe_button = Rect {
            origin: Point2D::new(rx, center_y - ICON_BUTTON / 2.0),
            size: Point2D::new(GLOBE_BUTTON_WIDTH, ICON_BUTTON),
        };
        // Globe glyph at the left half.
        draw_icon(
            cx.backend,
            Icon::Globe,
            Point2D::new(globe_button.origin.x + 4.0, center_y - ICON_SIZE / 2.0),
            ICON_SIZE,
            self.theme.muted_foreground,
            1.4,
        );
        // Chevron-down at the right side, smaller.
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(
                globe_button.origin.x + 4.0 + ICON_SIZE + 4.0,
                center_y - CHEVRON_SIZE / 2.0,
            ),
            CHEVRON_SIZE,
            self.theme.muted_foreground,
            1.4,
        );
        // `rx` now points at the LEFT edge of the globe button —
        // the chip anchors immediately to its left (small gap).

        // Agent chip — two states:
        //   - empty (agent_count == 0): LayoutGrid icon + "Agents
        //     与 MCP" label. Affordance for "set up agents/MCP".
        //   - active (≥ 1): Sparkles + green dot + "N agent" text.
        // Anchored just left of the globe icon button (+small gap).
        let count_label;
        let (chip_text, leading_icon, show_dot): (&str, Icon, bool) = if self.agent_count == 0 {
            (self.label_agents_and_mcp, Icon::LayoutGrid, false)
        } else {
            count_label = format!("{} {}", self.agent_count, self.label_agent_singular);
            (count_label.as_str(), Icon::Sparkles, true)
        };
        let dot_w = if show_dot { 8.0 + 6.0 } else { 0.0 };
        let text_w = cx.backend.measure_text(chip_text, 12.0);
        let chip_w = 8.0 + ICON_SIZE + 6.0 + dot_w + text_w + 12.0;
        let chip_rect = Rect {
            origin: Point2D::new(rx - chip_w - 6.0, center_y - 13.0),
            size: Point2D::new(chip_w, 26.0),
        };
        // No border ring — TS empty-state chip has no outline.
        draw_icon(
            cx.backend,
            leading_icon,
            Point2D::new(chip_rect.origin.x + 8.0, chip_rect.origin.y + 4.0),
            ICON_SIZE,
            self.theme.foreground,
            1.4,
        );
        let mut text_x = chip_rect.origin.x + 8.0 + ICON_SIZE + 6.0;
        if show_dot {
            let dot_color = Color {
                r: 0.0,
                g: 0.85,
                b: 0.4,
                a: 1.0,
            };
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(text_x, chip_rect.origin.y + chip_rect.size.y / 2.0 - 4.0),
                    size: Point2D::new(8.0, 8.0),
                },
                4.0,
                dot_color,
            );
            text_x += 8.0 + 6.0;
        }
        let chip_label = TextLayout::single_run(
            chip_text,
            "system-ui",
            12.0,
            to_jian_color(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&chip_label, Point2D::new(text_x, chip_rect.origin.y + 18.0));
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Header);
        node.set_label("Title bar");
        node
    }
}

fn paint_icon_button(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, center_y: f32, icon: Icon) {
    let icon_origin = Point2D::new(
        x + (ICON_BUTTON - ICON_SIZE) / 2.0,
        center_y - ICON_SIZE / 2.0,
    );
    draw_icon(
        cx.backend,
        icon,
        icon_origin,
        ICON_SIZE,
        theme.muted_foreground,
        1.4,
    );
}

/// File-menu compound: folder glyph + tighter chevron, both inside
/// a single 46×28 hit-target. The chevron gap is ~4 px instead of
/// ICON_BUTTON-wide as it used to render.
fn paint_file_menu_button(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, center_y: f32) {
    draw_icon(
        cx.backend,
        Icon::FolderOpen,
        Point2D::new(x + 6.0, center_y - ICON_SIZE / 2.0),
        ICON_SIZE,
        theme.muted_foreground,
        1.4,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(x + 6.0 + ICON_SIZE + 4.0, center_y - CHEVRON_SIZE / 2.0),
        CHEVRON_SIZE,
        theme.muted_foreground,
        1.4,
    );
}

fn paint_figma_button(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, center_y: f32) {
    crate::widgets::brand_icons::paint_figma_logo(
        cx.backend,
        Point2D::new(x + (ICON_BUTTON - ICON_SIZE) / 2.0, center_y - ICON_SIZE / 2.0),
        ICON_SIZE,
        theme.muted_foreground,
    );
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untitled_carries_default_chinese_label() {
        let bar = TopBar::untitled();
        assert_eq!(bar.file_name, "未命名");
    }

    #[test]
    fn layout_reports_full_width_and_top_bar_height() {
        let bar = TopBar::untitled();
        let cx = LayoutCx {
            available_width: 1000.0,
            dpi: 1.0,
        };
        let lb = bar.layout(&cx);
        assert_eq!(lb.rect.size.x, 1000.0);
        assert_eq!(lb.rect.size.y, TOP_BAR_HEIGHT);
    }

    #[test]
    fn access_node_advertises_header_role() {
        let node = TopBar::untitled().access_node();
        assert_eq!(node.role(), accesskit::Role::Header);
    }
}
