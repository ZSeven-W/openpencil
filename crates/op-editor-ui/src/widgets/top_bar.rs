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
/// Globe locale-picker button — wider than a normal icon button so a
/// chevron-down sits next to the globe glyph (signals the dropdown).
pub(super) const GLOBE_BUTTON_WIDTH: f32 = 44.0;
/// File-menu compound button — folder + a smaller chevron-down sit
/// inside a single round-rect background. Tighter gap than two
/// separate icon buttons (4 px between glyphs vs ICON_BUTTON + 4).
pub(super) const FILE_MENU_BUTTON_WIDTH: f32 = 46.0;
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
/// Preview needs the host-side jian runtime owner. The current web/wasm host
/// has no equivalent runtime/measurement bridge, so hide the button there
/// rather than exposing a non-interactive preview flag.
pub(super) const PREVIEW_BUTTON_AVAILABLE: bool = !cfg!(target_arch = "wasm32");
/// Stacked agent-icon metrics — mirror TS `top-bar.tsx`
/// (`w-5 h-5 rounded-md bg-foreground/10 ring-1 ring-card` chips
/// overlapped by `-space-x-1.5`).
pub(super) const AGENT_ICON_CHIP: f32 = 20.0;
pub(super) const AGENT_ICON_LOGO: f32 = 12.0;
pub(super) const AGENT_ICON_OVERLAP: f32 = 6.0;
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
    /// Play / Stop icon — enter / exit canvas Preview (Play) mode.
    TogglePreview,
}

pub struct TopBar {
    pub id: WidgetId,
    /// Centred file name. `String` rather than `&'static str` so an
    /// opened doc can show its basename without leaking a static
    /// slice on every Open.
    pub file_name: String,
    /// Whether to paint the unsaved-edit label next to the file name.
    pub edited: bool,
    pub agent_count: u32,
    /// Per-provider connect state, indexed by `AgentProvider::ALL`.
    /// The active chip paints one brand icon per connected provider.
    pub connected: [bool; 7],
    /// Number of enabled MCP CLI integrations — the `· N MCP` half
    /// of the chip status.
    pub mcp_count: u32,
    pub theme: Theme,
    pub label_edited: &'static str,
    pub label_agents_and_mcp: &'static str,
    pub label_agent_singular: &'static str,
    pub label_agent_plural: &'static str,
    /// Cursor is over the window-control cluster — the 3 dots paint
    /// their close / minimise / maximise glyphs.
    pub traffic_hover: bool,
    /// Whether this host shows window controls in the top bar. Desktop
    /// hosts keep the reservation / custom dots; the web host has no
    /// window controls and starts app chrome at the left padding.
    pub show_traffic_controls: bool,
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
    /// Whether the canvas is in Preview (Play) mode — drives the
    /// Play/Stop toggle glyph (Square when active, Play when idle).
    pub preview_active: bool,
    /// Opt-in gate: the Preview (Play) button only paints / hit-tests
    /// when experimental features are enabled in Settings → System.
    pub experimental_enabled: bool,
    /// Which chrome button the cursor is over — drives the per-button
    /// `theme.button_hover` wash. `None` = no hover.
    pub hover: Option<op_editor_core::TopBarButton>,
    /// Which chrome button is held by the primary pointer.
    pub pressed: Option<op_editor_core::TopBarButton>,
    /// Host-measured width (px) of the chip's status text at the paint
    /// font size (11). The desktop host fills this via its `text_measure`
    /// backend so the agent-chip hit area matches the painted chip
    /// exactly. `None` (the wasm build, or before a measure pass) →
    /// `hit_test` falls back to a char-count estimate.
    pub chip_text_w: Option<f32>,
}

impl TopBar {
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            id: WidgetId::new(5000),
            file_name: file_name.into(),
            edited: false,
            agent_count: 1,
            connected: [false; 7],
            mcp_count: 0,
            theme: Theme::dark(),
            label_edited: "",
            label_agents_and_mcp: "Agents & MCP",
            label_agent_singular: "agent",
            label_agent_plural: "agents",
            traffic_hover: false,
            show_traffic_controls: true,
            fullscreen: false,
            theme_mode: op_editor_core::ThemeMode::Dark,
            git_branch: None,
            preview_active: false,
            experimental_enabled: false,
            hover: None,
            pressed: None,
            chip_text_w: None,
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
        // affordance instead. Built-in (API-key) agents count too —
        // a ready MiniMax/GLM entry is as much an agent as a
        // connected CLI provider (icons stay per-CLI-provider; the
        // count is the honest total).
        let connected = ui.agent_settings.verified_connected_mask();
        let builtin_ready = ui
            .agent_settings
            .builtin_agents
            .iter()
            .filter(|agent| agent.ready())
            .count() as u32;
        let agent_count = connected.iter().filter(|&&c| c).count() as u32 + builtin_ready;
        let mcp_count = ui
            .agent_settings
            .mcp_cli_enabled
            .iter()
            .filter(|&&e| e)
            .count() as u32;
        Self {
            id: WidgetId::new(5000),
            file_name,
            edited: ui.document_dirty,
            agent_count,
            connected,
            mcp_count,
            theme: theme_for(ui),
            label_edited: translate(ui, "topbar.edited"),
            label_agents_and_mcp: translate(ui, "topbar.agentsAndMcp"),
            label_agent_singular: translate(ui, "topbar.agentSingular"),
            label_agent_plural: translate(ui, "topbar.agentPlural"),
            traffic_hover: ui.topbar_traffic_hover,
            show_traffic_controls: true,
            fullscreen: ui.window_fullscreen,
            theme_mode: ui.theme_mode,
            git_branch: ui.git_panel.branch.clone(),
            preview_active: ui.preview_mode,
            experimental_enabled: ui.agent_settings.experimental_features_enabled,
            hover: ui.topbar_button_hover,
            pressed: match ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::TopBar(button)) => Some(button),
                _ => None,
            },
            chip_text_w: None,
        }
    }

    pub fn with_traffic_controls(mut self, show: bool) -> Self {
        self.show_traffic_controls = show;
        if !show {
            self.traffic_hover = false;
        }
        self
    }

    /// True when the cursor is resting on `button` — drives the
    /// per-button hover wash in `paint_chrome`.
    pub(super) fn is_hovered(&self, button: op_editor_core::TopBarButton) -> bool {
        self.hover == Some(button)
    }

    pub(super) fn is_pressed(&self, button: op_editor_core::TopBarButton) -> bool {
        self.pressed == Some(button)
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
        if self.show_traffic_controls {
            Self::left_inset_for(self.fullscreen)
        } else {
            0.0
        }
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
    /// `LayoutGrid` glyph in the empty state, or one brand chip per
    /// connected provider (overlapped by `AGENT_ICON_OVERLAP`) in the
    /// active state. Paint + hit-test both size the chip off this.
    pub(super) fn agent_icons_width(&self) -> f32 {
        if self.agent_count == 0 {
            return ICON_SIZE;
        }
        // Only CONNECTED CLI providers paint a brand chip — built-in
        // (API-key) agents count toward the label but have no logo. Sizing
        // off `agent_count` reserved a blank icon cluster whenever builtins
        // outnumbered CLI providers (measured: "4 agents · 2 MCP" with zero
        // CLI providers painted a chip half-full of empty space).
        let painted = self.connected.iter().filter(|&&c| c).count();
        if painted == 0 {
            return 0.0;
        }
        let n = painted as f32;
        AGENT_ICON_CHIP + (n - 1.0) * (AGENT_ICON_CHIP - AGENT_ICON_OVERLAP)
    }

    /// The icon cluster plus its trailing 6px gap — 0 when no icon paints,
    /// so the chip hugs `[pad][dot][text][pad]` with no phantom hole. Paint
    /// and hit-test both walk the chip off this single span.
    pub(super) fn agent_icons_span(&self) -> f32 {
        let icons = self.agent_icons_width();
        if icons <= 0.0 {
            0.0
        } else {
            icons + 6.0
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

    /// The exact text the chip paints — the status string when set up,
    /// else the empty-state `Agents & MCP` label. The desktop host
    /// measures this with its text backend and feeds the width back via
    /// [`Self::chip_text_w`] so the hit area can't drift from the paint.
    pub fn chip_text(&self) -> String {
        self.chip_status_text()
            .unwrap_or_else(|| self.label_agents_and_mcp.to_string())
    }

    /// Returns the on-screen rect of the Globe-plus-chevron locale
    /// button. Used by the host to anchor the LocalePicker dropdown
    /// directly underneath when `Document.ui.locale_picker.open ==
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

    pub fn file_menu_rect_for(&self, top_bar_rect: Rect) -> Rect {
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        let file_menu_x =
            top_bar_rect.origin.x + PAD + self.left_inset() + ICON_BUTTON + divider_span;
        Rect {
            origin: Point2D::new(file_menu_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// Whether the Preview (Play) button paints / hit-tests. Gated by
    /// the host capability (`PREVIEW_BUTTON_AVAILABLE`, desktop-only) AND
    /// the experimental-features opt-in. The right-cluster layout
    /// collapses when this is false, so paint, hit-test, and the
    /// globe-anchored locale picker all key off this one predicate.
    pub fn preview_button_visible(&self) -> bool {
        PREVIEW_BUTTON_AVAILABLE && self.experimental_enabled
    }

    pub fn globe_rect(&self, top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        // Right-cluster layout (right → left): Maximize | Play (native
        // only) | Sun | Globe. Icon buttons are normal ICON_BUTTON wide;
        // Globe is the wider GLOBE_BUTTON_WIDTH so the chevron fits.
        let icon_count = 2.0
            + if self.preview_button_visible() {
                1.0
            } else {
                0.0
            };
        let globe_x = right - PAD - ICON_BUTTON * icon_count - GLOBE_BUTTON_WIDTH;
        Rect {
            origin: Point2D::new(globe_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(GLOBE_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// Play / Stop toggle button — second from the right (just left of
    /// Maximize). Shared by paint + hit-test so they can't drift.
    pub(super) fn preview_button_rect(top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        let icon_y = top_bar_rect.origin.y + 8.0;
        Rect {
            origin: Point2D::new(right - PAD - ICON_BUTTON * 2.0, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        }
    }

    /// Theme toggle button. Its x-position shifts right in the web/wasm build
    /// where the Preview button is hidden.
    pub(super) fn theme_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        let right_icons = 1.0
            + if self.preview_button_visible() {
                1.0
            } else {
                0.0
            };
        Rect {
            origin: Point2D::new(
                right - PAD - ICON_BUTTON * (right_icons + 1.0),
                top_bar_rect.origin.y + 8.0,
            ),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        }
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
        let center_approx = self.title_approx_width();
        let render_w: f32 = self
            .file_name
            .chars()
            .map(|c| if is_wide_glyph(c) { 14.0 } else { 7.5 })
            .sum();
        let filename_left = top_bar_rect.origin.x + (top_bar_rect.size.x - center_approx) / 2.0;
        let filename_right = filename_left + render_w + self.edited_approx_width();
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

    pub(super) fn title_approx_width(&self) -> f32 {
        self.file_name.chars().count() as f32 * 9.0 + self.edited_approx_width()
    }

    pub(super) fn edited_approx_width(&self) -> f32 {
        if self.edited {
            8.0 + self.label_edited.chars().count() as f32 * 7.0
        } else {
            0.0
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
        if !self.show_traffic_controls || cfg!(target_os = "macos") {
            return None;
        }
        if !(rect).contains(point) {
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
        if !(rect).contains(point) {
            return None;
        }
        let icon_y = rect.origin.y + 8.0;
        let panel_left_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD + self.left_inset(), icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if (panel_left_rect).contains(point) {
            return Some(TopBarHit::ToggleSidebar);
        }
        // Reuse the canonical anchor so the hit area, paint, and
        // dropdown anchor can never drift (Codex caught a divider-span
        // drift here once). Use the INSTANCE method (`self.left_inset()`),
        // not the static `file_menu_rect` (which always reserves
        // `TRAFFIC_CLUSTER_W` via `left_inset_for`): on the web the traffic
        // controls are hidden (`show_traffic_controls = false` → inset 0), so
        // the static rect would shift the hover/hit area right of the painted
        // folder button by the traffic-cluster width.
        let file_menu_rect = self.file_menu_rect_for(rect);
        let file_menu_x = file_menu_rect.origin.x;
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        if (file_menu_rect).contains(point) {
            return Some(TopBarHit::ToggleFileMenu);
        }
        let figma_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + divider_span;
        let figma_rect = Rect {
            origin: Point2D::new(figma_x, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if (figma_rect).contains(point) {
            return Some(TopBarHit::OpenFigmaImport);
        }
        // Git-panel toggle, just right of the centred file name
        // (desktop only — see `GIT_BUTTON_AVAILABLE`).
        if GIT_BUTTON_AVAILABLE && (self.git_button_rect(rect)).contains(point) {
            return Some(TopBarHit::ToggleGitPanel);
        }
        // Right cluster: Maximize / Play / Sun / Globe-with-chevron
        // (right→left). Maximize + Play + Sun are normal ICON_BUTTON
        // wide; Globe is GLOBE_BUTTON_WIDTH wide (it carries a chevron).
        let right = rect.origin.x + rect.size.x;
        let icon_y = rect.origin.y + 8.0;
        // Maximize (rightmost icon, painted at `right - PAD - ICON_BUTTON`)
        // → toggle fullscreen.
        let maximize_rect = Rect {
            origin: Point2D::new(right - PAD - ICON_BUTTON, icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        };
        if (maximize_rect).contains(point) {
            return Some(TopBarHit::ToggleFullscreen);
        }
        // Play / Stop (second from right) → toggle Preview mode.
        if self.preview_button_visible() && Self::preview_button_rect(rect).contains(point) {
            return Some(TopBarHit::TogglePreview);
        }
        let sun_rect = self.theme_button_rect(rect);
        if (sun_rect).contains(point) {
            return Some(TopBarHit::ToggleTheme);
        }
        let globe = self.globe_rect(rect);
        if (globe).contains(point) {
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
        let dot_w = if status_text.is_some() {
            8.0 + 6.0
        } else {
            0.0
        };
        // Prefer the host-measured text width so the hit area matches the
        // painted chip exactly (paint uses skia `measure_text` @ 11 px).
        // Without a measure backend (wasm) fall back to a ~7 px/char
        // estimate — close to the 11 px-font advance, never the old
        // 12 px/char + 16 px slop that ballooned the target into the gap.
        let text_w = self.chip_text_w.unwrap_or_else(|| {
            let chip_chars = match &status_text {
                Some(t) => t.chars().count(),
                None => self.label_agents_and_mcp.chars().count(),
            };
            chip_chars as f32 * 7.0
        });
        // Same geometry as `paint_chrome`'s chip: 8 (lead pad) + icons + 6
        // (gap) + dot + text + 12 (trail pad). No extra slop.
        let chip_w = 8.0 + self.agent_icons_span() + dot_w + text_w + 12.0;
        let chip_rect = Rect {
            origin: Point2D::new(
                globe.origin.x - chip_w - (DIVIDER_GAP * 2.0 + DIVIDER_W),
                rect.origin.y + 4.0,
            ),
            size: Point2D::new(chip_w, rect.size.y - 8.0),
        };
        if (chip_rect).contains(point) {
            return Some(TopBarHit::OpenAgentSettings);
        }
        None
    }
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

/// Paint shared ghost-button feedback behind a chrome button and return
/// the glyph color to use.
pub(super) fn paint_hover_bg(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hovered: bool,
    pressed: bool,
) -> Color {
    crate::widgets::button::paint_ghost_button_feedback(cx.backend, theme, rect, hovered, pressed)
}

/// Folder / globe compound icon button — a leading glyph + a trailing
/// chevron-down inside one hit-target. Coloured like the sibling icon buttons
/// (`muted_foreground` at rest, `foreground` on hover/press) via the shared
/// ghost feedback, instead of jian `SelectTrigger`'s always-`foreground` icon.
/// Geometry mirrors `SelectTrigger` (PAD_X = 8, 14px chevron) so the glyphs
/// don't shift.
pub(super) fn paint_compound_icon_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    button_rect: Rect,
    icon: Icon,
    hovered: bool,
    pressed: bool,
) {
    const PAD_X: f32 = 8.0;
    const CHEVRON: f32 = 14.0;
    let color = paint_hover_bg(cx, theme, button_rect, hovered, pressed);
    // Leading glyph (SelectTrigger sized its icon `font_size + 1`), centred.
    let glyph = ICON_SIZE + 1.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            button_rect.origin.x + PAD_X,
            button_rect.origin.y + (button_rect.size.y - glyph) / 2.0,
        ),
        glyph,
        color,
        1.5,
    );
    // Trailing chevron-down, right-aligned.
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            button_rect.origin.x + button_rect.size.x - PAD_X - CHEVRON,
            button_rect.origin.y + (button_rect.size.y - CHEVRON) / 2.0,
        ),
        CHEVRON,
        color,
        1.5,
    );
}

/// File-menu compound: folder glyph + tighter chevron, both inside
/// a single 46×28 hit-target.
pub(super) fn paint_file_menu_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    hovered: bool,
    pressed: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
    };
    paint_compound_icon_button(cx, theme, button_rect, Icon::FolderOpen, hovered, pressed);
}

pub(super) fn paint_figma_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    hovered: bool,
    pressed: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    let color = paint_hover_bg(cx, theme, button_rect, hovered, pressed);
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
    jian_widgets::components::separator::Separator {
        orientation: jian_widgets::components::separator::Orientation::Vertical,
        thickness: DIVIDER_W,
    }
    .paint(
        cx.backend,
        Rect {
            origin: Point2D::new(x, center_y - DIVIDER_H / 2.0),
            size: Point2D::new(DIVIDER_W, DIVIDER_H),
        },
        color,
    );
}
