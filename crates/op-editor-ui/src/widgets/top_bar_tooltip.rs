//! Hover tooltips for the TopBar's chrome buttons.
//!
//! The bar is almost all icon buttons, and an icon is not
//! self-explanatory: a palette glyph is the Asset Center, a download
//! glyph is the export menu. Resting on a button for a moment names it.
//!
//! Three rules make this feel like a desktop tooltip rather than a
//! flicker of labels:
//!
//! 1. **Dwell.** Nothing appears until the cursor has been in the button
//!    row for [`TOOLTIP_DWELL_MS`], so sweeping across the bar on the way
//!    somewhere else shows nothing. The clock lives on
//!    `EditorUiState::topbar_hover_since_ms` and starts on entry into the
//!    row, not per button — once the first tooltip has earned its way on
//!    screen, moving along the row swaps labels immediately.
//! 2. **A scheduled repaint.** The dwell expiring is not an input event,
//!    so [`next_deadline_ms`] reports the due instant to the hosts'
//!    animation scheduler. Without that the tooltip would only appear if
//!    the user happened to jiggle the mouse after the wait.
//! 3. **Suppression.** A button whose menu is open is already answering
//!    the question; a tooltip on top of its own dropdown is noise.

use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::TopBarButton;

use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::tooltip::{self, TooltipPlacement};
use crate::widgets::{PaintCx, TopBar};
use crate::Rect;

/// How long the cursor must rest in the button row before the first
/// tooltip appears. Long enough that crossing the bar is silent, short
/// enough that a deliberate pause is answered without a wait.
pub const TOOLTIP_DWELL_MS: u64 = 450;

/// What a top-bar button's tooltip says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopBarTooltip {
    /// What the button is — never how to use it.
    pub label: &'static str,
    /// Its keyboard shortcut, when one really exists in this build.
    pub shortcut: Option<&'static str>,
}

/// Whether this build binds the desktop-only `zoom_modifier` shortcuts
/// (`op-host-desktop`'s keyboard dispatch). The browser host ships none
/// of them, so promising `⌘P` there would be a lie.
const DESKTOP_SHORTCUTS: bool = !cfg!(target_arch = "wasm32");

/// Fullscreen is reachable only through the macOS menu bar's
/// `⌃⌘F` accelerator (`op-host-desktop/src/menu.rs`); the muda menu
/// backend is macOS-gated, so no other platform has the binding.
const MACOS_MENU_SHORTCUTS: bool = cfg!(target_os = "macos") && !cfg!(target_arch = "wasm32");

/// The label + shortcut for `button`, resolved against the live UI state
/// so toggles name the action they will perform.
pub fn tooltip_for(ui: &EditorUiState, button: TopBarButton) -> TopBarTooltip {
    let (label, shortcut) = match button {
        TopBarButton::ToggleSidebar => (
            if ui.sidebar_open {
                translate(ui, "topbar.hideLayers")
            } else {
                translate(ui, "topbar.showLayers")
            },
            None,
        ),
        TopBarButton::ToggleFileMenu => (translate(ui, "tooltip.topbar.file"), None),
        TopBarButton::OpenImportMenu => (translate(ui, "tooltip.topbar.import"), None),
        TopBarButton::ToggleTheme => (
            // The glyph shows the destination, so the label must too: a
            // Sun in dark mode means "go light".
            match ui.effective_theme_mode() {
                op_editor_core::ThemeMode::Dark => translate(ui, "topbar.lightMode"),
                op_editor_core::ThemeMode::Light => translate(ui, "topbar.darkMode"),
            },
            None,
        ),
        TopBarButton::ToggleLocale => (translate(ui, "tooltip.topbar.language"), None),
        TopBarButton::OpenExportMenu => (translate(ui, "export.title"), None),
        TopBarButton::OpenAssetCenter => (translate(ui, "assetCenter.title"), None),
        TopBarButton::OpenAgentSettings => (
            translate(ui, "topbar.agentsAndMcp"),
            // op-host-desktop/src/keyboard_input.rs — `"," =>
            // apply_toggle_agent_settings()` under the Cmd/Ctrl guard.
            DESKTOP_SHORTCUTS.then_some("⌘,"),
        ),
        TopBarButton::OpenCollaboration => (translate(ui, "tooltip.topbar.collaboration"), None),
        TopBarButton::ToggleGitPanel => (
            if ui.git_panel.open {
                translate(ui, "git.closePanel")
            } else {
                translate(ui, "git.openPanel")
            },
            None,
        ),
        TopBarButton::ToggleFullscreen => (
            if ui.window_fullscreen {
                translate(ui, "topbar.exitFullscreen")
            } else {
                translate(ui, "topbar.fullscreen")
            },
            // op-host-desktop/src/menu.rs — View ▸ Toggle Full Screen,
            // `Accelerator::new(META | CONTROL, Code::KeyF)`. Modifier
            // order follows the repo's existing labels (`⌘⇧S`, `⌘⇧F`),
            // which lead with ⌘ rather than the Apple HIG ordering.
            MACOS_MENU_SHORTCUTS.then_some("⌘⌃F"),
        ),
        TopBarButton::TogglePreview => (
            if ui.preview.mode {
                translate(ui, "tooltip.topbar.exitPreview")
            } else {
                translate(ui, "tooltip.topbar.preview")
            },
            // op-host-desktop/src/keyboard_input.rs — `"p" =>
            // toggle_preview_with_cached_viewport()`.
            DESKTOP_SHORTCUTS.then_some("⌘P"),
        ),
        TopBarButton::OpenAccount => (translate(ui, "tooltip.topbar.account"), None),
    };
    TopBarTooltip { label, shortcut }
}

/// Whether `button`'s own surface is already open. Its dropdown or modal
/// says more than a tooltip would, and the tooltip would paint over it.
pub fn suppressed(ui: &EditorUiState, button: TopBarButton) -> bool {
    match button {
        TopBarButton::ToggleFileMenu => ui.file_menu_open,
        TopBarButton::OpenImportMenu => ui.import_menu_open || ui.import_menu.open,
        TopBarButton::ToggleLocale => ui.locale_picker.open,
        TopBarButton::OpenExportMenu => ui.export_quick_menu_open,
        TopBarButton::OpenAssetCenter => ui.scene_template_center.open,
        TopBarButton::OpenAgentSettings => ui.agent_settings_open,
        TopBarButton::OpenCollaboration => ui.collab.panel.open,
        TopBarButton::ToggleGitPanel => ui.git_panel.open,
        TopBarButton::OpenAccount => ui.account_menu_open,
        // Sidebar / theme / fullscreen / preview act immediately — they
        // open no surface that could cover or repeat the tooltip.
        TopBarButton::ToggleSidebar
        | TopBarButton::ToggleTheme
        | TopBarButton::ToggleFullscreen
        | TopBarButton::TogglePreview => false,
    }
}

/// Which button's tooltip should be on screen at `now_ms`, if any.
pub fn visible_button(ui: &EditorUiState, now_ms: u64) -> Option<TopBarButton> {
    let button = ui.topbar_button_hover?;
    if suppressed(ui, button) {
        return None;
    }
    let due = ui.topbar_hover_since_ms?.saturating_add(TOOLTIP_DWELL_MS);
    (now_ms >= due).then_some(button)
}

/// The instant a pending tooltip becomes due, for the hosts' animation
/// scheduler. `None` once it is showing (or was never coming): the
/// tooltip needs exactly one wake-up, not a running clock.
pub fn next_deadline_ms(ui: &EditorUiState, now_ms: u64) -> Option<u64> {
    let button = ui.topbar_button_hover?;
    if suppressed(ui, button) {
        return None;
    }
    let due = ui.topbar_hover_since_ms?.saturating_add(TOOLTIP_DWELL_MS);
    (now_ms < due).then_some(due)
}

/// Paint the due tooltip, if any, under its button. Hosts call this
/// after the TopBar and the rails (so it can hang over the canvas) but
/// before every dropdown and modal.
///
/// Returns the painted rect, for tests.
pub fn paint_top_bar_tooltip(
    cx: &mut PaintCx<'_>,
    ui: &EditorUiState,
    top_bar: &TopBar,
    top_bar_rect: Rect,
    viewport_width: f32,
    now_ms: u64,
) -> Option<Rect> {
    let button = visible_button(ui, now_ms)?;
    let anchor = top_bar.button_rect_with_measure(top_bar_rect, button, |text, size| {
        crate::widgets::text_metrics::measure_chrome(cx.backend, text, size)
    })?;
    let TopBarTooltip { label, shortcut } = tooltip_for(ui, button);
    if label.is_empty() {
        return None;
    }
    Some(tooltip::paint_tooltip(
        cx,
        &theme_for(ui),
        anchor,
        label,
        shortcut,
        TooltipPlacement::Below,
        Some(tooltip::viewport_clamp(viewport_width)),
    ))
}

/// Every button the bar can hover, for exhaustive coverage checks.
pub const ALL_TOP_BAR_BUTTONS: [TopBarButton; 13] = [
    TopBarButton::ToggleSidebar,
    TopBarButton::ToggleFileMenu,
    TopBarButton::OpenImportMenu,
    TopBarButton::ToggleTheme,
    TopBarButton::ToggleLocale,
    TopBarButton::OpenExportMenu,
    TopBarButton::OpenAssetCenter,
    TopBarButton::OpenAgentSettings,
    TopBarButton::OpenCollaboration,
    TopBarButton::ToggleGitPanel,
    TopBarButton::ToggleFullscreen,
    TopBarButton::TogglePreview,
    TopBarButton::OpenAccount,
];
