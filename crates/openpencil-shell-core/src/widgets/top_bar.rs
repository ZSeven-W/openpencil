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

pub const TOP_BAR_HEIGHT: f32 = 48.0;
const ICON_SIZE: f32 = 18.0;
const ICON_BUTTON: f32 = 32.0;
const PAD: f32 = 12.0;

/// What a click in the top bar resolved to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarHit {
    /// PanelLeft icon — toggle sidebar (LayerPanel) visibility.
    ToggleSidebar,
    /// Sun icon — flip theme dark↔light.
    ToggleTheme,
    /// Globe icon — cycle through UI locales.
    ToggleLocale,
}

pub struct TopBar {
    pub id: WidgetId,
    pub file_name: String,
    pub agent_count: u32,
    pub theme: Theme,
    /// Localised "Agents 与 MCP" label — rendered when
    /// `agent_count == 0` (empty state).
    pub label_agents_and_mcp: String,
    /// Localised "agent" / "agents" label — appended to the
    /// count when `agent_count > 0`. Falls back to "agent"
    /// when the locale doesn't ship the key.
    pub label_agent_singular: String,
}

impl TopBar {
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(5000),
            file_name: file_name.into(),
            agent_count: 1,
            theme: Theme::dark(),
            label_agents_and_mcp: "Agents & MCP".to_string(),
            label_agent_singular: "agent".to_string(),
        }
    }

    pub fn untitled() -> Self {
        Self::new("未命名")
    }

    /// Document-aware constructor — reads the active theme + the
    /// localised "untitled" label so the bar flips with theme +
    /// locale toggles. Default `agent_count = 0` matches the TS
    /// app's empty state ("Agents 与 MCP" affordance instead of
    /// the green-dot "1 agent" chip).
    pub fn for_document(doc: &Document) -> Self {
        Self {
            id: WidgetId::new(5000),
            file_name: doc.t("common.untitled").to_string(),
            agent_count: 0,
            theme: doc.theme(),
            label_agents_and_mcp: doc.t("topbar.agentsAndMcp").to_string(),
            label_agent_singular: doc.t("topbar.agentSingular").to_string(),
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
        let panel_left_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, rect.origin.y + 8.0),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(panel_left_rect, point) {
            return Some(TopBarHit::ToggleSidebar);
        }
        // Right cluster: Maximize / Sun / Globe (right→left).
        // Each ICON_BUTTON wide, no extra gap. Mirror of paint:
        //   rx = right - PAD - ICON_BUTTON  →  Maximize
        //   rx -= ICON_BUTTON              →  Sun
        //   rx -= ICON_BUTTON              →  Globe
        let right = rect.origin.x + rect.size.x;
        let sun_x = right - PAD - ICON_BUTTON * 2.0;
        let globe_x = right - PAD - ICON_BUTTON * 3.0;
        let icon_y = rect.origin.y + 8.0;
        let sun_rect = Rect {
            origin: Point2D::new(sun_x, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(sun_rect, point) {
            return Some(TopBarHit::ToggleTheme);
        }
        let globe_rect = Rect {
            origin: Point2D::new(globe_x, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(globe_rect, point) {
            return Some(TopBarHit::ToggleLocale);
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
        let mut x = rect.origin.x + PAD;
        let center_y = rect.origin.y + rect.size.y / 2.0;
        for icon in [Icon::PanelLeft, Icon::FolderOpen, Icon::ChevronDown] {
            paint_icon_button(cx, &self.theme, x, center_y, icon);
            x += ICON_BUTTON + 4.0;
        }

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
        // Tight spacing: icons sit close together (4 px) and the
        // agent chip sits flush against them (just 4 px gap, not
        // the previous 8 + ICON_BUTTON).
        let mut rx = rect.origin.x + rect.size.x - PAD - ICON_BUTTON;

        // Fullscreen.
        paint_icon_button(cx, &self.theme, rx, center_y, Icon::Maximize);
        rx -= ICON_BUTTON;

        // Theme toggle.
        paint_icon_button(cx, &self.theme, rx, center_y, Icon::Sun);
        rx -= ICON_BUTTON;

        // i18n globe.
        paint_icon_button(cx, &self.theme, rx, center_y, Icon::Globe);

        // Agent chip — two states:
        //   - empty (agent_count == 0): LayoutGrid icon + "Agents
        //     与 MCP" label. Affordance for "set up agents/MCP".
        //   - active (≥ 1): Sparkles + green dot + "N agent" text.
        // Anchored just left of the globe icon button (+small gap).
        let (chip_text, leading_icon, show_dot) = if self.agent_count == 0 {
            (self.label_agents_and_mcp.clone(), Icon::LayoutGrid, false)
        } else {
            (
                format!("{} {}", self.agent_count, self.label_agent_singular),
                Icon::Sparkles,
                true,
            )
        };
        let dot_w = if show_dot { 8.0 + 6.0 } else { 0.0 };
        let text_w = cx.backend.measure_text(&chip_text, 12.0);
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
            &chip_text,
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
    fn layout_reports_full_width_and_48_height() {
        let bar = TopBar::untitled();
        let cx = LayoutCx {
            available_width: 1000.0,
            dpi: 1.0,
        };
        let lb = bar.layout(&cx);
        assert_eq!(lb.rect.size.x, 1000.0);
        assert_eq!(lb.rect.size.y, 48.0);
    }

    #[test]
    fn access_node_advertises_header_role() {
        let node = TopBar::untitled().access_node();
        assert_eq!(node.role(), accesskit::Role::Header);
    }
}
