//! System tab of the settings modal.
//!
//! Appearance, the auto-update preference plus its live release-probe
//! status, the experimental-features opt-in, and the pencil-cursor
//! picker — each on one borderless row in the modal's shared row
//! language (`agent_settings_rows`), separated by hairlines.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_metrics::CONTENT_TAIL_PAD;
use crate::widgets::agent_settings_rows::{
    fit_text, measure_settings_text, paint_footnote, paint_row_hairline, paint_row_label,
    paint_row_label_above_status, paint_row_status_line, paint_tab_intro, row_control_rect,
    row_rect_in, rows_block_height, tab_intro_height, RowLines, FOOTNOTE_H, SETTINGS_FONT_FAMILY,
};
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::{EditorUiState, ThemeMode, UpdateStatus};

/// System opens with the shared tab intro: title plus one muted line.
const HEADING_HAS_DESC: bool = true;
const SEGMENT_W: f32 = 78.0;
const SEGMENT_H: f32 = 30.0;
const SEGMENT_ICON: f32 = 13.0;
const SEGMENT_ICON_GAP: f32 = 6.0;
const SEGMENT_PAD_X: f32 = 6.0;
const CURSOR_SWATCH: f32 = 38.0;
const CURSOR_SWATCH_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHit {
    SelectThemeMode(ThemeMode),
    ToggleAutoUpdate,
    ToggleExperimental,
    /// Click on one of the pencil-cursor swatches.
    SelectPencilCursor(op_editor_core::PencilCursorStyle),
    None,
}

/// Host capability: the auto-update toggle drives the desktop
/// updater (`op-host-desktop/src/update_check.rs`). The web host has
/// no updater — hide the row there instead of painting a switch that
/// silently does nothing (same pattern as `GIT_BUTTON_AVAILABLE`).
pub(super) const AUTO_UPDATE_AVAILABLE: bool = !cfg!(target_arch = "wasm32");

/// Row order. Auto-update drops out on the web, and every row below it
/// moves up one slot — one table so paint and hit-test can't disagree
/// about which slot a row sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemRow {
    Appearance,
    AutoUpdate,
    Experimental,
    PencilCursor,
}

fn visible_rows() -> &'static [SystemRow] {
    if AUTO_UPDATE_AVAILABLE {
        &[
            SystemRow::Appearance,
            SystemRow::AutoUpdate,
            SystemRow::Experimental,
            SystemRow::PencilCursor,
        ]
    } else {
        &[
            SystemRow::Appearance,
            SystemRow::Experimental,
            SystemRow::PencilCursor,
        ]
    }
}

impl SystemRow {
    /// How many lines the row carries — and therefore how tall its box
    /// is. Auto-update rides a probe-status line and Experimental a
    /// description; the other two are label-only.
    fn lines(self) -> RowLines {
        match self {
            SystemRow::AutoUpdate | SystemRow::Experimental => RowLines::Two,
            SystemRow::Appearance | SystemRow::PencilCursor => RowLines::One,
        }
    }
}

fn row_kinds() -> Vec<RowLines> {
    visible_rows().iter().map(|row| row.lines()).collect()
}

/// Row boxes for this tab, paired with their line count. The traversal
/// layout test walks this.
#[cfg(test)]
pub(super) fn row_boxes(content: Rect) -> Vec<(Rect, RowLines)> {
    let kinds = row_kinds();
    (0..kinds.len())
        .map(|i| (row_rect_in(content, body_top(content), &kinds, i), kinds[i]))
        .collect()
}

fn body_top(content: Rect) -> f32 {
    content.origin.y + tab_intro_height(HEADING_HAS_DESC)
}

fn rect_for(content: Rect, row: SystemRow) -> Rect {
    let index = visible_rows()
        .iter()
        .position(|candidate| *candidate == row)
        .unwrap_or(0);
    row_rect_in(content, body_top(content), &row_kinds(), index)
}

pub(super) fn content_height() -> f32 {
    tab_intro_height(HEADING_HAS_DESC)
        + rows_block_height(&row_kinds())
        + FOOTNOTE_H
        + CONTENT_TAIL_PAD
}

/// The two-segment Light / Dark control on the Appearance row.
fn segment_rect(content: Rect, index: usize) -> Rect {
    let group = row_control_rect(
        rect_for(content, SystemRow::Appearance),
        SEGMENT_W * 2.0,
        SEGMENT_H,
    );
    Rect {
        origin: Point2D::new(group.origin.x + index as f32 * SEGMENT_W, group.origin.y),
        size: Point2D::new(SEGMENT_W, SEGMENT_H),
    }
}

const SEGMENT_MODES: [ThemeMode; 2] = [ThemeMode::Light, ThemeMode::Dark];

/// The Auto-update row's switch, exported for the hosts' hover/press
/// probes so they target the real control instead of a copied offset.
pub(super) fn auto_update_switch_rect(content: Rect) -> Rect {
    switch_rect_for(content, SystemRow::AutoUpdate)
}

fn switch_rect_for(content: Rect, row: SystemRow) -> Rect {
    row_control_rect(rect_for(content, row), SETTINGS_SWITCH_W, SETTINGS_SWITCH_H)
}

/// Swatch rect for cursor style index `i`, right-aligned on its row.
fn cursor_swatch_rect(content: Rect, i: usize) -> Rect {
    let count = op_editor_core::PencilCursorStyle::ALL.len() as f32;
    let group_w = count * CURSOR_SWATCH + (count - 1.0) * CURSOR_SWATCH_GAP;
    let group = row_control_rect(
        rect_for(content, SystemRow::PencilCursor),
        group_w,
        CURSOR_SWATCH,
    );
    Rect {
        origin: Point2D::new(
            group.origin.x + i as f32 * (CURSOR_SWATCH + CURSOR_SWATCH_GAP),
            group.origin.y,
        ),
        size: Point2D::new(CURSOR_SWATCH, CURSOR_SWATCH),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> SystemHit {
    for (i, mode) in SEGMENT_MODES.iter().enumerate() {
        if segment_rect(content, i).contains(scrolled) {
            return SystemHit::SelectThemeMode(*mode);
        }
    }
    if AUTO_UPDATE_AVAILABLE && switch_rect_for(content, SystemRow::AutoUpdate).contains(scrolled) {
        return SystemHit::ToggleAutoUpdate;
    }
    if switch_rect_for(content, SystemRow::Experimental).contains(scrolled) {
        return SystemHit::ToggleExperimental;
    }
    for (i, style) in op_editor_core::PencilCursorStyle::ALL.iter().enumerate() {
        if cursor_swatch_rect(content, i).contains(scrolled) {
            return SystemHit::SelectPencilCursor(*style);
        }
    }
    SystemHit::None
}

pub(super) fn paint_system_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
) {
    paint_tab_intro(
        cx,
        theme,
        content,
        t_settings(ui, "settings.system.heroTitle"),
        Some(t_settings(ui, "settings.system.heroSubtitle")),
    );

    let rows = visible_rows();
    let kinds = row_kinds();
    let last = rows.len() - 1;
    for (index, row) in rows.iter().enumerate() {
        let rect = row_rect_in(content, body_top(content), &kinds, index);
        match row {
            SystemRow::Appearance => paint_appearance_row(cx, theme, ui, content, rect),
            SystemRow::AutoUpdate => {
                paint_auto_update_row(cx, theme, ui, content, rect, settings.auto_update_enabled)
            }
            SystemRow::Experimental => {
                paint_row_label(
                    cx,
                    theme,
                    rect,
                    t_settings(ui, "settings.experimental"),
                    Some(t_settings(ui, "settings.experimentalDesc")),
                    SETTINGS_SWITCH_W + 16.0,
                );
                paint_settings_switch(
                    cx,
                    theme,
                    switch_rect_for(content, SystemRow::Experimental),
                    settings.experimental_features_enabled,
                );
            }
            SystemRow::PencilCursor => paint_cursor_row(cx, theme, ui, content, rect),
        }
        if index != last {
            paint_row_hairline(cx, theme, rect);
        }
    }

    if AUTO_UPDATE_AVAILABLE {
        let (_, _, description_key) = status_view(theme, &ui.update_status);
        paint_footnote(
            cx,
            theme,
            content,
            body_top(content) + rows_block_height(&kinds),
            &format!(
                "{}: v{} — {}",
                t_settings(ui, "settings.system.currentVersion"),
                env!("CARGO_PKG_VERSION"),
                t_settings(ui, description_key)
            ),
        );
    }
}

fn paint_appearance_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    row: Rect,
) {
    paint_row_label(
        cx,
        theme,
        row,
        t_settings(ui, "settings.system.appearance"),
        None,
        SEGMENT_W * 2.0 + 16.0,
    );
    let group = row_control_rect(row, SEGMENT_W * 2.0, SEGMENT_H);
    cx.backend.stroke_round_rect(group, 8.0, theme.border, 1.0);
    for (i, mode) in SEGMENT_MODES.iter().enumerate() {
        let seg = segment_rect(content, i);
        let active = ui.effective_theme_mode() == *mode;
        if active {
            cx.backend.fill_round_rect(seg, 7.0, theme.muted);
        }
        let color = if active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let (icon, label) = match mode {
            ThemeMode::Light => (Icon::Sun, t_settings(ui, "settings.system.appearanceLight")),
            ThemeMode::Dark => (Icon::Moon, t_settings(ui, "settings.system.appearanceDark")),
        };
        // The segment is fixed-width and sits flush against the content
        // column's right edge, so a long localisation ("Тёмное") centred
        // on its untrimmed width hangs off the modal. Fit it first.
        let label = fit_text(
            cx,
            label,
            SEGMENT_W - SEGMENT_ICON - SEGMENT_ICON_GAP - SEGMENT_PAD_X * 2.0,
            12.0,
        );
        let label_w = measure_settings_text(cx, &label, 12.0);
        let group_w = SEGMENT_ICON + SEGMENT_ICON_GAP + label_w;
        let icon_x = seg.origin.x + (SEGMENT_W - group_w) / 2.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(icon_x, seg.origin.y + (SEGMENT_H - SEGMENT_ICON) / 2.0),
            SEGMENT_ICON,
            color,
            1.6,
        );
        let layout = TextLayout::single_run(
            &label,
            SETTINGS_FONT_FAMILY,
            12.0,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(
                icon_x + SEGMENT_ICON + SEGMENT_ICON_GAP,
                seg.origin.y + SEGMENT_H / 2.0 + 4.0,
            ),
        );
    }
}

/// Auto-update row. The release probe's state rides the row's own
/// description line — coloured, with a leading dot — instead of the
/// separate status block the tinted card used to carry.
fn paint_auto_update_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    row: Rect,
    enabled: bool,
) {
    paint_row_label_above_status(
        cx,
        theme,
        row,
        t_settings(ui, "agents.autoUpdate"),
        SETTINGS_SWITCH_W + 16.0,
    );
    let (color, status_key, _) = status_view(theme, &ui.update_status);
    let status_text = match &ui.update_status {
        UpdateStatus::Available { version } => {
            format!("{} v{version}", t_settings(ui, status_key))
        }
        _ => t_settings(ui, status_key).to_string(),
    };
    paint_row_status_line(cx, row, &status_text, color);
    paint_settings_switch(
        cx,
        theme,
        switch_rect_for(content, SystemRow::AutoUpdate),
        enabled,
    );
}

/// Pencil-cursor picker: one swatch per style drawing its real
/// silhouette at half scale; the active style gets an accent ring.
fn paint_cursor_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    row: Rect,
) {
    let count = op_editor_core::PencilCursorStyle::ALL.len() as f32;
    let group_w = count * CURSOR_SWATCH + (count - 1.0) * CURSOR_SWATCH_GAP;
    paint_row_label(
        cx,
        theme,
        row,
        t_settings(ui, "settings.system.pencilCursor"),
        None,
        group_w + 16.0,
    );
    for (i, style) in op_editor_core::PencilCursorStyle::ALL.iter().enumerate() {
        let swatch = cursor_swatch_rect(content, i);
        let active = ui.pencil_cursor_style == *style;
        cx.backend.fill_round_rect(
            swatch,
            8.0,
            if active {
                theme.row_selected_primary
            } else {
                theme.button_hover
            },
        );
        if active {
            cx.backend
                .stroke_round_rect(swatch, 8.0, theme.primary, 1.5);
        }
        crate::widgets::canvas_agent_cursor::paint_cursor_swatch(
            cx,
            *style,
            Point2D::new(swatch.origin.x + 8.0, swatch.origin.y + 6.0),
            theme.primary,
        );
    }
}

fn status_view(theme: &Theme, status: &UpdateStatus) -> (Color, &'static str, &'static str) {
    match status {
        UpdateStatus::Idle => (
            theme.muted_foreground,
            "settings.system.idle",
            "settings.system.idleDescription",
        ),
        UpdateStatus::Checking => (
            theme.primary,
            "settings.system.checking",
            "settings.system.checkingDescription",
        ),
        UpdateStatus::UpToDate => (
            theme.status_success,
            "settings.system.upToDate",
            "settings.system.upToDateDescription",
        ),
        UpdateStatus::Available { .. } => (
            theme.status_warning,
            "settings.system.updateAvailable",
            "settings.system.updateAvailableDescription",
        ),
        UpdateStatus::Error => (
            theme.destructive,
            "settings.system.errorStatus",
            "settings.system.errorDescription",
        ),
    }
}
