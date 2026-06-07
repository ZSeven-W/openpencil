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
use crate::{Color, Point2D, Rect};
use op_editor_core::editor_ui_state::EditorUiState;

pub const TOP_BAR_HEIGHT: f32 = 40.0;
// Top-bar glyph size — 14 px (a touch smaller than the old 16 and
// matching the TS `size-[15px]` chrome). Local to the top bar; other
// widgets keep their own `ICON_SIZE`.
pub(super) const ICON_SIZE: f32 = 14.0;
pub(super) const ICON_BUTTON: f32 = 28.0;
/// Corner radius of a top-bar button's hover background. Smaller than
/// the floating toolbar's 8 px to suit the 28 px-tall chrome buttons.
pub(super) const BUTTON_RADIUS: f32 = 6.0;
/// Globe locale-picker button — wider than a normal icon button so a
/// chevron-down sits next to the globe glyph (signals the dropdown).
pub(super) const GLOBE_BUTTON_WIDTH: f32 = 44.0;
/// File-menu compound button — folder + a smaller chevron-down sit
/// inside a single round-rect background. Tighter gap than two
/// separate icon buttons (4 px between glyphs vs ICON_BUTTON + 4).
pub(super) const FILE_MENU_BUTTON_WIDTH: f32 = 46.0;
pub(super) const CHEVRON_SIZE: f32 = 10.0;
pub(super) const COMPOUND_GLYPH_GAP: f32 = 4.0;
pub(super) const GIT_BUTTON_PAD_X: f32 = (ICON_BUTTON - ICON_SIZE) / 2.0;
pub(super) const PAD: f32 = 12.0;
/// Top-bar vertical divider geometry (TS `w-px h-3.5 bg-border/60
/// mx-1`): 1 px wide, 14 px tall, 4 px gap on each side.
pub(super) const DIVIDER_W: f32 = 1.0;
pub(super) const DIVIDER_H: f32 = 14.0;
pub(super) const DIVIDER_GAP: f32 = 4.0;
/// The git button needs a desktop git backend (`op-git` via the
/// desktop host) to populate + paint the panel; the web/wasm build
/// has none, so the button is compiled out there — otherwise it would
/// toggle an invisible panel (Codex stop-time review).
pub(super) const GIT_BUTTON_AVAILABLE: bool = !cfg!(target_arch = "wasm32");
/// Gap between the stacked per-agent brand icons in the chip.
pub(super) const AGENT_ICON_GAP: f32 = 4.0;
/// Diameter of a macOS-style window-control dot.
pub(super) const TRAFFIC_DOT: f32 = 12.0;
/// Centre-to-centre spacing of the 3 window-control dots.
pub(super) const TRAFFIC_STEP: f32 = 20.0;
/// Horizontal span the window-control cluster reserves at the
/// TopBar's left edge before the app's own icons. macOS uses the
/// *native* traffic-light buttons — they sit at the system
/// position and end ~x=70, so reserve enough that the app icons
/// clear them with a comfortable gap. Windows / Linux paint the
/// custom dots from `PAD`, so the reservation is just the dot
/// cluster + a small gap.
pub(super) const TRAFFIC_CLUSTER_W: f32 = if cfg!(target_os = "macos") {
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
    /// Git-branch button next to the file name — toggle the git panel.
    ToggleGitPanel,
    /// Maximize icon (rightmost of the right cluster) — toggle window
    /// fullscreen.
    ToggleFullscreen,
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
    /// Current git branch when the open document is in a repo — shown
    /// beside the file name. `None` = no branch (the button still
    /// paints, icon-only, as a toggle for the git panel).
    pub git_branch: Option<String>,
    /// Which chrome button the cursor is over — drives the per-button
    /// `theme.button_hover` wash. `None` = no hover.
    pub hover: Option<op_editor_core::TopBarButton>,
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
            git_branch: None,
            hover: None,
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
            git_branch: ui.git_panel.branch.clone(),
            hover: ui.topbar_button_hover,
        }
    }

    /// True when the cursor is resting on `button` — drives the
    /// per-button hover wash in `paint_chrome`.
    pub(super) fn is_hovered(&self, button: op_editor_core::TopBarButton) -> bool {
        self.hover == Some(button)
    }

    /// Left-edge reservation for the window controls. Collapses to
    /// `0` in fullscreen on macOS — the native traffic lights hide
    /// then, so the gap would be dead space. Other platforms keep
    /// the custom-dot cluster's inset in every mode.
    pub(super) fn left_inset_for(fullscreen: bool) -> f32 {
        if fullscreen && cfg!(target_os = "macos") {
            0.0
        } else {
            TRAFFIC_CLUSTER_W
        }
    }

    pub(super) fn left_inset(&self) -> f32 {
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
    pub(super) fn agent_icons_width(&self) -> f32 {
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
    pub(super) fn chip_status_text(&self) -> Option<String> {
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

    pub(super) fn locale_glyph_left(globe_button: Rect) -> f32 {
        let group_w = ICON_SIZE + COMPOUND_GLYPH_GAP + CHEVRON_SIZE;
        globe_button.origin.x + (globe_button.size.x - group_w) / 2.0
    }

    /// Git-panel toggle button — sits just right of the centred file
    /// name. Width holds the branch glyph plus an optional branch
    /// label. Shared by paint + hit-test so they can't drift.
    pub(super) fn git_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let center_y = top_bar_rect.origin.y + top_bar_rect.size.y / 2.0;
        // The name is *centred* using the 9 px/char heuristic, but a
        // CJK glyph renders ~14 px wide, so the real right edge is
        // further out — use a CJK-aware estimate so the button clears
        // the (often CJK) file name instead of overlapping it.
        let center_approx = self.file_name.chars().count() as f32 * 9.0;
        let render_w: f32 = self
            .file_name
            .chars()
            .map(|c| if is_wide_glyph(c) { 14.0 } else { 7.5 })
            .sum();
        let filename_left = top_bar_rect.origin.x + (top_bar_rect.size.x - center_approx) / 2.0;
        let filename_right = filename_left + render_w;
        let branch_w = self
            .git_branch
            .as_deref()
            .map(|b| 6.0 + b.chars().count() as f32 * 7.0)
            .unwrap_or(0.0);
        Rect {
            origin: Point2D::new(filename_right + 10.0, center_y - ICON_BUTTON / 2.0),
            size: Point2D::new(GIT_BUTTON_PAD_X * 2.0 + ICON_SIZE + branch_w, ICON_BUTTON),
        }
    }

    pub(super) fn git_icon_left(git_button: Rect) -> f32 {
        git_button.origin.x + GIT_BUTTON_PAD_X
    }

    /// Center-x of the Git-panel toggle button when it is shown
    /// (desktop only — see `GIT_BUTTON_AVAILABLE`). The floating Git
    /// panel anchors its caret here so it reads as a popover hanging
    /// off the button (TS parity); `None` when the button is hidden.
    pub fn git_button_center_x(&self, top_bar_rect: Rect) -> Option<f32> {
        if !GIT_BUTTON_AVAILABLE {
            return None;
        }
        let r = self.git_button_rect(top_bar_rect);
        Some(r.origin.x + r.size.x / 2.0)
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
        // Git-panel toggle, just right of the centred file name
        // (desktop only — see `GIT_BUTTON_AVAILABLE`).
        if GIT_BUTTON_AVAILABLE && rect_contains(self.git_button_rect(rect), point) {
            return Some(TopBarHit::ToggleGitPanel);
        }
        // Right cluster: Maximize / Sun / Globe-with-chevron (right→left).
        // Maximize + Sun are normal ICON_BUTTON wide; Globe is
        // GLOBE_BUTTON_WIDTH wide because it carries a chevron.
        let right = rect.origin.x + rect.size.x;
        let sun_x = right - PAD - ICON_BUTTON * 2.0;
        let icon_y = rect.origin.y + 8.0;
        // Maximize (rightmost icon, painted at `right - PAD - ICON_BUTTON`)
        // → toggle fullscreen.
        let maximize_rect = Rect {
            origin: Point2D::new(right - PAD - ICON_BUTTON, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if rect_contains(maximize_rect, point) {
            return Some(TopBarHit::ToggleFullscreen);
        }
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

pub(super) fn rect_contains(r: Rect, p: Point2D) -> bool {
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
        // Full composition lives in the `top_bar_paint` sibling (split
        // for the 800-line cap).
        self.paint_chrome(cx, rect);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Header);
        node.set_label("Title bar");
        node
    }
}

/// Paint the `theme.button_hover` background behind a chrome button
/// when the cursor rests on it, and return the glyph color to use:
/// `theme.foreground` while hovered (the wash lifts the icon), else
/// `theme.muted_foreground`. Centralises the hover treatment so every
/// top-bar button reads identically.
pub(super) fn paint_hover_bg(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hovered: bool,
) -> Color {
    if hovered {
        cx.backend
            .fill_round_rect(rect, BUTTON_RADIUS, theme.button_hover);
        theme.foreground
    } else {
        theme.muted_foreground
    }
}

pub(super) fn paint_icon_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    icon: Icon,
    hovered: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    let color = paint_hover_bg(cx, theme, button_rect, hovered);
    let icon_origin = Point2D::new(
        x + (ICON_BUTTON - ICON_SIZE) / 2.0,
        center_y - ICON_SIZE / 2.0,
    );
    draw_icon(cx.backend, icon, icon_origin, ICON_SIZE, color, 1.4);
}

/// File-menu compound: folder glyph + tighter chevron, both inside
/// a single 46×28 hit-target. The chevron gap is ~4 px instead of
/// ICON_BUTTON-wide as it used to render.
pub(super) fn paint_file_menu_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    hovered: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
    };
    let color = paint_hover_bg(cx, theme, button_rect, hovered);
    draw_icon(
        cx.backend,
        Icon::FolderOpen,
        Point2D::new(x + 6.0, center_y - ICON_SIZE / 2.0),
        ICON_SIZE,
        color,
        1.4,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(x + 6.0 + ICON_SIZE + 4.0, center_y - CHEVRON_SIZE / 2.0),
        CHEVRON_SIZE,
        color,
        1.4,
    );
}

pub(super) fn paint_figma_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    hovered: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    let color = paint_hover_bg(cx, theme, button_rect, hovered);
    crate::widgets::brand_icons::paint_figma_logo(
        cx.backend,
        Point2D::new(
            x + (ICON_BUTTON - ICON_SIZE) / 2.0,
            center_y - ICON_SIZE / 2.0,
        ),
        ICON_SIZE,
        color,
    );
}

/// Rough "is this a full-width (CJK/Hangul/full-width-form) glyph"
/// test — used only to estimate the rendered file-name width so the
/// git button clears it.
pub(super) fn is_wide_glyph(c: char) -> bool {
    let cp = c as u32;
    matches!(cp, 0x1100..=0x11FF | 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

/// Top-left y for a glyph of `size` vertically centred on `center_y`.
/// Every top-bar glyph routes through this so the whole bar shares
/// one center line.
pub(super) fn glyph_top(center_y: f32, size: f32) -> f32 {
    center_y - size / 2.0
}

/// Paint a top-bar vertical divider with its left edge at `x`,
/// centred on `center_y` (TS `w-px h-3.5 bg-border/60`).
pub(super) fn paint_divider(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, center_y: f32) {
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

pub(super) fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nearly_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

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

    #[test]
    fn for_editor_ui_picks_up_button_hover() {
        let ui = EditorUiState {
            topbar_button_hover: Some(op_editor_core::TopBarButton::ToggleTheme),
            ..Default::default()
        };
        let bar = TopBar::for_editor_ui(&ui);
        assert!(bar.is_hovered(op_editor_core::TopBarButton::ToggleTheme));
        assert!(!bar.is_hovered(op_editor_core::TopBarButton::ToggleSidebar));
    }

    #[test]
    fn maximize_button_hit_tests_to_toggle_fullscreen() {
        let bar = TopBar::untitled();
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
        };
        let cy = 8.0 + ICON_BUTTON / 2.0;
        // Rightmost icon (Maximize) → ToggleFullscreen.
        let fs_cx = 1000.0 - PAD - ICON_BUTTON / 2.0;
        assert_eq!(
            bar.hit_test(rect, Point2D::new(fs_cx, cy)),
            Some(TopBarHit::ToggleFullscreen),
        );
        // The neighbour to its left (Sun) still maps to ToggleTheme —
        // adjacency unbroken.
        let sun_cx = 1000.0 - PAD - ICON_BUTTON - ICON_BUTTON / 2.0;
        assert_eq!(
            bar.hit_test(rect, Point2D::new(sun_cx, cy)),
            Some(TopBarHit::ToggleTheme),
        );
    }

    #[test]
    fn locale_button_glyph_group_is_centered_in_hover_rect() {
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
        };
        let globe_rect = TopBar::globe_rect(rect);
        let glyph_group_left = TopBar::locale_glyph_left(globe_rect);
        let glyph_group_right = glyph_group_left + ICON_SIZE + COMPOUND_GLYPH_GAP + CHEVRON_SIZE;
        let glyph_group_center = (glyph_group_left + glyph_group_right) / 2.0;
        let hover_center = globe_rect.origin.x + globe_rect.size.x / 2.0;

        assert!(
            nearly_eq(glyph_group_center, hover_center),
            "locale glyph group should be centered in its hover rect"
        );
    }

    #[test]
    fn icon_only_git_button_centers_glyph_in_hover_rect() {
        let mut bar = TopBar::untitled();
        bar.git_branch = None;
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
        };
        let git_rect = bar.git_button_rect(rect);
        let icon_left = TopBar::git_icon_left(git_rect);
        let icon_center = icon_left + ICON_SIZE / 2.0;
        let hover_center = git_rect.origin.x + git_rect.size.x / 2.0;

        assert!(
            nearly_eq(icon_center, hover_center),
            "icon-only git button should center the branch glyph in its hover rect"
        );
    }
}
