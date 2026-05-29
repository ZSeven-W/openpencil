//! `TopBar` — full-width application title bar (Step 4 visual lift).
//!
//! Mirrors `apps/web/src/components/editor/top-bar.tsx`: panel-toggle
//! + folder + brand on the left, file name centered, theme +
//!   agent-status + i18n + fullscreen on the right. Click handling is
//!   a P6 follow-up; Step 4 paints only.

use crate::theme::Theme;
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

pub const TOP_BAR_HEIGHT: f32 = 40.0;
// Top-bar glyph size — 14 px (a touch smaller than the old 16 and
// matching the TS `size-[15px]` chrome). Local to the top bar; other
// widgets keep their own `ICON_SIZE`.
const ICON_SIZE: f32 = 14.0;
const ICON_BUTTON: f32 = 28.0;
/// Globe locale-picker button — wider than a normal icon button so a
/// chevron-down sits next to the globe glyph (signals the dropdown).
const GLOBE_BUTTON_WIDTH: f32 = 44.0;
/// File-menu compound button — folder + a smaller chevron-down sit
/// inside a single round-rect background. Tighter gap than two
/// separate icon buttons (4 px between glyphs vs ICON_BUTTON + 4).
const FILE_MENU_BUTTON_WIDTH: f32 = 46.0;
const CHEVRON_SIZE: f32 = 10.0;
const PAD: f32 = 12.0;
/// Top-bar vertical divider geometry (TS `w-px h-3.5 bg-border/60
/// mx-1`): 1 px wide, 14 px tall, 4 px gap on each side.
const DIVIDER_W: f32 = 1.0;
const DIVIDER_H: f32 = 14.0;
const DIVIDER_GAP: f32 = 4.0;
/// Gap between the stacked per-agent brand icons in the chip.
const AGENT_ICON_GAP: f32 = 4.0;
/// Diameter of a macOS-style window-control dot.
const TRAFFIC_DOT: f32 = 12.0;
/// Centre-to-centre spacing of the 3 window-control dots.
const TRAFFIC_STEP: f32 = 20.0;
/// Horizontal span the window-control cluster reserves at the
/// TopBar's left edge before the app's own icons. macOS uses the
/// *native* traffic-light buttons — they sit at the system
/// position and end ~x=70, so reserve enough that the app icons
/// clear them with a comfortable gap. Windows / Linux paint the
/// custom dots from `PAD`, so the reservation is just the dot
/// cluster + a small gap.
const TRAFFIC_CLUSTER_W: f32 = if cfg!(target_os = "macos") {
    66.0
} else {
    TRAFFIC_STEP * 2.0 + TRAFFIC_DOT + 16.0
};

/// A window-control dot in the TopBar's left cluster. Resolved by
/// [`TopBar::window_control_at`]; the desktop runner maps each onto
/// the matching winit `Window` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControl {
    /// Red dot — close the window / quit.
    Close,
    /// Yellow dot — minimise the window.
    Minimize,
    /// Green dot — toggle maximised.
    Maximize,
}

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
    /// Agents and MCP chip — open the agent settings modal.
    OpenAgentSettings,
}

pub struct TopBar {
    pub id: WidgetId,
    /// Centred file name. `String` rather than `&'static str` so an
    /// opened doc can show its basename without leaking a static
    /// slice on every Open.
    pub file_name: String,
    pub agent_count: u32,
    /// Per-provider connect state, indexed by `AgentProvider::ALL`.
    /// The active chip paints one brand icon per connected provider.
    pub connected: [bool; 5],
    /// Number of enabled MCP CLI integrations — the `· N MCP` half
    /// of the chip status.
    pub mcp_count: u32,
    pub theme: Theme,
    pub label_agents_and_mcp: &'static str,
    pub label_agent_singular: &'static str,
    pub label_agent_plural: &'static str,
    /// Cursor is over the window-control cluster — the 3 dots paint
    /// their close / minimise / maximise glyphs.
    pub traffic_hover: bool,
    /// Window is fullscreen — drops the left-edge window-control
    /// reservation on macOS (native traffic lights hide then).
    pub fullscreen: bool,
    /// Active theme mode — drives the toggle icon: a Sun glyph in
    /// Dark mode (click to go light), a Moon glyph in Light mode
    /// (click to go dark).
    pub theme_mode: op_editor_core::ThemeMode,
}

impl TopBar {
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(5000),
            file_name: file_name.into(),
            agent_count: 1,
            connected: [false; 5],
            mcp_count: 0,
            theme: Theme::dark(),
            label_agents_and_mcp: "Agents & MCP",
            label_agent_singular: "agent",
            label_agent_plural: "agents",
            traffic_hover: false,
            fullscreen: false,
            theme_mode: op_editor_core::ThemeMode::Dark,
        }
    }

    pub fn untitled() -> Self {
        Self::new("未命名")
    }

    pub fn for_editor_ui(ui: &EditorUiState) -> Self {
        let file_name = ui
            .file_name_display
            .clone()
            .unwrap_or_else(|| translate(ui, "common.untitled").to_string());
        // The chip reflects what the user set up in Settings: one
        // brand icon per connected provider, plus the enabled-MCP
        // count. All-zero paints the "Agents & MCP" set-up
        // affordance instead.
        let agent_count = ui.agent_settings.connected.iter().filter(|&&c| c).count() as u32;
        let mcp_count = ui
            .agent_settings
            .mcp_cli_enabled
            .iter()
            .filter(|&&e| e)
            .count() as u32;
        Self {
            id: WidgetId::new(5000),
            file_name,
            agent_count,
            connected: ui.agent_settings.connected,
            mcp_count,
            theme: theme_for(ui),
            label_agents_and_mcp: translate(ui, "topbar.agentsAndMcp"),
            label_agent_singular: translate(ui, "topbar.agentSingular"),
            label_agent_plural: translate(ui, "topbar.agentPlural"),
            traffic_hover: ui.topbar_traffic_hover,
            fullscreen: ui.window_fullscreen,
            theme_mode: ui.theme_mode,
        }
    }

    /// Left-edge reservation for the window controls. Collapses to
    /// `0` in fullscreen on macOS — the native traffic lights hide
    /// then, so the gap would be dead space. Other platforms keep
    /// the custom-dot cluster's inset in every mode.
    fn left_inset_for(fullscreen: bool) -> f32 {
        if fullscreen && cfg!(target_os = "macos") {
            0.0
        } else {
            TRAFFIC_CLUSTER_W
        }
    }

    fn left_inset(&self) -> f32 {
        Self::left_inset_for(self.fullscreen)
    }

    /// Bounds of the 3-dot window-control cluster — the host's
    /// cursor-move handler uses this to drive `topbar_traffic_hover`
    /// (the glyph reveal).
    pub fn traffic_cluster_rect(top_bar_rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(top_bar_rect.origin.x + PAD, top_bar_rect.origin.y),
            size: Point2D::new(TRAFFIC_STEP * 2.0 + TRAFFIC_DOT, top_bar_rect.size.y),
        }
    }

    /// Width of the chip's leading-icon cluster: the single
    /// `LayoutGrid` glyph in the empty state, or one brand icon per
    /// connected provider (with `AGENT_ICON_GAP` between them) in the
    /// active state. Paint + hit-test both size the chip off this.
    fn agent_icons_width(&self) -> f32 {
        if self.agent_count == 0 {
            ICON_SIZE
        } else {
            let n = self.agent_count.max(1) as f32;
            n * ICON_SIZE + (n - 1.0) * AGENT_ICON_GAP
        }
    }

    /// The chip's status text — `"{N} agent[s] · {M} MCP"`, each
    /// half present only when its count is non-zero (TS
    /// `top-bar.tsx` parity). `None` when nothing is set up, in
    /// which case the caller paints the `Agents & MCP` label.
    fn chip_status_text(&self) -> Option<String> {
        if self.agent_count == 0 && self.mcp_count == 0 {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();
        if self.agent_count > 0 {
            let word = if self.agent_count == 1 {
                self.label_agent_singular
            } else {
                self.label_agent_plural
            };
            parts.push(format!("{} {}", self.agent_count, word));
        }
        if self.mcp_count > 0 {
            parts.push(format!("{} MCP", self.mcp_count));
        }
        Some(parts.join(" · "))
    }

    /// Returns the on-screen rect of the Globe-plus-chevron locale
    /// button. Used by the host to anchor the LocalePicker dropdown
    /// directly underneath when `Document.ui.locale_picker_open ==
    /// true`. The button itself is wider than a normal icon button
    /// so the chevron-down has room to render.
    /// Anchor rect for the file-menu dropdown overlay (folder +
    /// chevron compound). Host anchors the dropdown directly under
    /// this rect when `Document.ui.file_menu_open == true`.
    pub fn file_menu_rect(top_bar_rect: Rect, fullscreen: bool) -> Rect {
        // Mirror the paint layout: panel button │ divider │ file-menu.
        // The divider span (gap + width + gap) pushes the file-menu
        // right of the sidebar toggle — keep this anchor in sync so
        // the dropdown opens under the folder button, not left of it.
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        let file_menu_x = top_bar_rect.origin.x
            + PAD
            + Self::left_inset_for(fullscreen)
            + ICON_BUTTON
            + divider_span;
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

    /// Resolve a press on the left-edge window-control dots.
    /// `None` for a press anywhere else (including the app's own
    /// buttons). The desktop runner consults this before its normal
    /// TopBar hit-test so a dot click drives the window, not the app.
    pub fn window_control_at(&self, rect: Rect, point: Point2D) -> Option<WindowControl> {
        // macOS uses the native traffic-light buttons — the custom
        // dots (and this hit-test) exist only for Windows / Linux.
        // Returning `None` here also avoids a false positive in
        // macOS fullscreen, where the left inset collapses and the
        // app's own icons would otherwise sit in the dot region.
        if cfg!(target_os = "macos") {
            return None;
        }
        if !rect_contains(rect, point) {
            return None;
        }
        let cy = rect.origin.y + rect.size.y / 2.0;
        let first_cx = rect.origin.x + PAD + TRAFFIC_DOT / 2.0;
        for (i, ctl) in [
            WindowControl::Close,
            WindowControl::Minimize,
            WindowControl::Maximize,
        ]
        .into_iter()
        .enumerate()
        {
            let dot_cx = first_cx + i as f32 * TRAFFIC_STEP;
            // Square slop around the dot — adjacent zones tile
            // without overlap (±TRAFFIC_STEP/2 in x).
            if (point.x - dot_cx).abs() <= TRAFFIC_STEP / 2.0
                && (point.y - cy).abs() <= rect.size.y / 2.0
            {
                return Some(ctl);
            }
        }
        None
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
            origin: Point2D::new(rect.origin.x + PAD + self.left_inset(), icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(panel_left_rect, point) {
            return Some(TopBarHit::ToggleSidebar);
        }
        // Reuse the canonical anchor so the hit area, paint, and
        // dropdown anchor can never drift (Codex caught a divider-span
        // drift here once).
        let file_menu_rect = Self::file_menu_rect(rect, self.fullscreen);
        let file_menu_x = file_menu_rect.origin.x;
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        if rect_contains(file_menu_rect, point) {
            return Some(TopBarHit::ToggleFileMenu);
        }
        let figma_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + divider_span;
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
        let status_text = self.chip_status_text();
        let chip_chars = match &status_text {
            Some(t) => t.chars().count(),
            None => self.label_agents_and_mcp.chars().count(),
        };
        let dot_w = if status_text.is_some() {
            8.0 + 6.0
        } else {
            0.0
        };
        let approx_text_w = chip_chars as f32 * 12.0;
        let chip_w = 8.0 + self.agent_icons_width() + 6.0 + dot_w + approx_text_w + 12.0 + 16.0;
        let chip_rect = Rect {
            origin: Point2D::new(
                globe.origin.x - chip_w - (DIVIDER_GAP * 2.0 + DIVIDER_W),
                rect.origin.y + 4.0,
            ),
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

        let center_y = rect.origin.y + rect.size.y / 2.0;
        // ── Window-control dots ────────────────────────────────
        // macOS keeps the *native* traffic-light buttons (the
        // transparent title bar preserves them), so the custom
        // dots paint only on Windows / Linux — whose
        // `decorations(false)` window ships none. The desktop
        // runner wires custom-dot clicks via `window_control_at`.
        if !cfg!(target_os = "macos") {
            let traffic = [
                Color {
                    r: 1.0,
                    g: 0.373,
                    b: 0.341,
                    a: 1.0,
                }, // #FF5F57
                Color {
                    r: 0.996,
                    g: 0.737,
                    b: 0.18,
                    a: 1.0,
                }, // #FEBC2E
                Color {
                    r: 0.157,
                    g: 0.784,
                    b: 0.251,
                    a: 1.0,
                }, // #28C840
            ];
            let first_dot_cx = rect.origin.x + PAD + TRAFFIC_DOT / 2.0;
            for (i, color) in traffic.into_iter().enumerate() {
                let dot_cx = first_dot_cx + i as f32 * TRAFFIC_STEP;
                cx.backend.fill_oval(
                    Rect {
                        origin: Point2D::new(
                            dot_cx - TRAFFIC_DOT / 2.0,
                            center_y - TRAFFIC_DOT / 2.0,
                        ),
                        size: Point2D::new(TRAFFIC_DOT, TRAFFIC_DOT),
                    },
                    color,
                );
                // Hovering the cluster reveals each dot's glyph
                // (macOS-style): ✕ close, − minimise, + maximise.
                if self.traffic_hover {
                    let glyph = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.55,
                    };
                    let r = 3.0_f32;
                    let lw = 1.3_f32;
                    match i {
                        0 => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y - r),
                                Point2D::new(dot_cx + r, center_y + r),
                                glyph,
                                lw,
                            );
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y + r),
                                Point2D::new(dot_cx + r, center_y - r),
                                glyph,
                                lw,
                            );
                        }
                        1 => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y),
                                Point2D::new(dot_cx + r, center_y),
                                glyph,
                                lw,
                            );
                        }
                        _ => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y),
                                Point2D::new(dot_cx + r, center_y),
                                glyph,
                                lw,
                            );
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx, center_y - r),
                                Point2D::new(dot_cx, center_y + r),
                                glyph,
                                lw,
                            );
                        }
                    }
                }
            }
        }
        // ── Left cluster ───────────────────────────────────────
        // sidebar toggle │ file-menu │ Figma — each group split by a
        // TS-style 1×14 divider (4 px gap each side).
        let panel_left_x = rect.origin.x + PAD + self.left_inset();
        paint_icon_button(cx, &self.theme, panel_left_x, center_y, Icon::PanelLeft);
        // Divider between the sidebar toggle and the file-menu.
        let divider1_x = panel_left_x + ICON_BUTTON + DIVIDER_GAP;
        paint_divider(cx, &self.theme, divider1_x, center_y);
        // File-menu compound: folder + tight chevron in one button.
        let file_menu_x = divider1_x + DIVIDER_W + DIVIDER_GAP;
        paint_file_menu_button(cx, &self.theme, file_menu_x, center_y);
        // Divider before the Figma import affordance.
        let divider2_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + DIVIDER_GAP;
        paint_divider(cx, &self.theme, divider2_x, center_y);
        // Figma import button.
        let figma_x = divider2_x + DIVIDER_W + DIVIDER_GAP;
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

        // Theme toggle — Sun in dark mode (click → light); Moon in
        // light mode (click → dark).
        let theme_icon = match self.theme_mode {
            op_editor_core::ThemeMode::Dark => Icon::Sun,
            op_editor_core::ThemeMode::Light => Icon::Moon,
        };
        paint_icon_button(cx, &self.theme, rx, center_y, theme_icon);
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
        //   - empty (agent_count == 0): LayoutGrid icon + an
        //     "Agents and MCP" label. Affordance for "set up agents/MCP".
        //   - active (≥ 1): one brand icon per connected provider +
        //     a green dot + "N agent" text.
        // Anchored just left of the globe icon button (+small gap).
        let status_text = self.chip_status_text();
        let show_dot = status_text.is_some();
        let chip_text: &str = status_text.as_deref().unwrap_or(self.label_agents_and_mcp);
        let dot_w = if show_dot { 6.0 + 6.0 } else { 0.0 };
        let icons_w = self.agent_icons_width();
        let text_w = cx.backend.measure_text(chip_text, 11.0);
        let chip_w = 8.0 + icons_w + 6.0 + dot_w + text_w + 12.0;
        // Leave room for the chip↔globe divider (4 px gap + 1 px + 4 px).
        let chip_rect = Rect {
            origin: Point2D::new(
                rx - chip_w - (DIVIDER_GAP * 2.0 + DIVIDER_W),
                center_y - 13.0,
            ),
            size: Point2D::new(chip_w, 26.0),
        };
        // Leading icons (no border ring — TS empty-state chip has no
        // outline). The empty state shows the single LayoutGrid
        // set-up affordance; the active chip stacks one brand logo
        // per connected provider so the user sees *which* agents
        // are on.
        let icons_y = glyph_top(center_y, ICON_SIZE);
        if self.agent_count == 0 {
            draw_icon(
                cx.backend,
                Icon::LayoutGrid,
                Point2D::new(chip_rect.origin.x + 8.0, icons_y),
                ICON_SIZE,
                self.theme.muted_foreground,
                1.4,
            );
        } else {
            let mut ix = chip_rect.origin.x + 8.0;
            for (i, provider) in op_editor_core::AgentProvider::ALL.iter().enumerate() {
                if !self.connected[i] {
                    continue;
                }
                crate::widgets::ai_chat_model_picker::paint_provider_logo(
                    cx,
                    *provider,
                    Point2D::new(ix, icons_y),
                    ICON_SIZE,
                    self.theme.foreground,
                );
                ix += ICON_SIZE + AGENT_ICON_GAP;
            }
        }
        let mut text_x = chip_rect.origin.x + 8.0 + icons_w + 6.0;
        if show_dot {
            // emerald-500 (#10b981) — matches the TS status dot.
            let dot_color = Color {
                r: 0.063,
                g: 0.725,
                b: 0.506,
                a: 1.0,
            };
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(text_x, chip_rect.origin.y + chip_rect.size.y / 2.0 - 3.0),
                    size: Point2D::new(6.0, 6.0),
                },
                3.0,
                dot_color,
            );
            text_x += 6.0 + 6.0;
        }
        let chip_label = TextLayout::single_run(
            chip_text,
            "system-ui",
            11.0,
            to_jian_color(self.theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        // 11 px text centred on the bar's center line (ascent ≈ 8 px).
        cx.backend
            .draw_text(&chip_label, Point2D::new(text_x, center_y + 4.0));

        // Divider between the agent chip and the globe button — groups
        // the status chip apart from the locale/theme/fullscreen controls.
        paint_divider(
            cx,
            &self.theme,
            globe_button.origin.x - DIVIDER_GAP - DIVIDER_W,
            center_y,
        );
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
        Point2D::new(
            x + (ICON_BUTTON - ICON_SIZE) / 2.0,
            center_y - ICON_SIZE / 2.0,
        ),
        ICON_SIZE,
        theme.muted_foreground,
    );
}

/// Top-left y for a glyph of `size` vertically centred on `center_y`.
/// Every top-bar glyph routes through this so the whole bar shares
/// one center line.
fn glyph_top(center_y: f32, size: f32) -> f32 {
    center_y - size / 2.0
}

/// Paint a top-bar vertical divider with its left edge at `x`,
/// centred on `center_y` (TS `w-px h-3.5 bg-border/60`).
fn paint_divider(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, center_y: f32) {
    let color = Color {
        a: theme.border.a * 0.6,
        ..theme.border
    };
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, center_y - DIVIDER_H / 2.0),
            size: Point2D::new(DIVIDER_W, DIVIDER_H),
        },
        color,
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
