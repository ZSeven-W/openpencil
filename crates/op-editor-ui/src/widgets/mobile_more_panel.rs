//! Responsive overflow surface for native touch editors.
//!
//! Compact windows use a short bottom sheet; tablet windows use a
//! top-right popover. Geometry and hit-testing live together so neither
//! platform can stretch the phone menu across an iPad again.

use super::editor_state_ext::translate;
use super::host_canvas_geometry;
use super::mobile_chrome::{paint_touch_icon, sheet_close_rect, sheet_handle_rect};
use super::text_metrics;
use super::{icons::Icon, PaintCx};
use crate::{Color, Point2D, Rect, Theme};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::EditorState;

const HEADER_HEIGHT: f32 = 56.0;
const PANEL_PADDING: f32 = 12.0;
const GRID_GAP: f32 = 8.0;
const TILE_HEIGHT: f32 = 76.0;
const PORTRAIT_COLUMN_COUNT: usize = 3;
// Eleven visible actions fit in two rows on a landscape phone. Keeping the
// portrait column count would grow the sheet to the full 320pt viewport and
// leave no useful canvas context behind the modal surface.
const LANDSCAPE_COLUMN_COUNT: usize = 6;
const PHONE_BOTTOM_PADDING: f32 = 16.0;
const TABLET_PANEL_WIDTH: f32 = 320.0;
const TABLET_BOTTOM_PADDING: f32 = 20.0;
const LABEL_FONT_SIZE: f32 = 13.0;
const LABEL_SIDE_PADDING: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileMoreEntry {
    NewFile,
    OpenFile,
    Templates,
    Assets,
    Ai,
    SignIn,
    Account,
    Language,
    Collaboration,
    Settings,
    Variables,
    Preview,
    Export,
}

impl MobileMoreEntry {
    /// Exhaustive semantic entries. Paint and hit-test use [`Self::visible`]
    /// because Sign in and Account are mutually exclusive states of one tile.
    pub const ALL: [MobileMoreEntry; 13] = [
        MobileMoreEntry::NewFile,
        MobileMoreEntry::OpenFile,
        MobileMoreEntry::Templates,
        MobileMoreEntry::Assets,
        MobileMoreEntry::Ai,
        MobileMoreEntry::SignIn,
        MobileMoreEntry::Account,
        MobileMoreEntry::Collaboration,
        MobileMoreEntry::Language,
        MobileMoreEntry::Settings,
        MobileMoreEntry::Variables,
        MobileMoreEntry::Preview,
        MobileMoreEntry::Export,
    ];

    /// Entries visible for the current account state. This is the canonical
    /// list for layout, paint and hit-testing so a signed-in account cannot
    /// leave an invisible Sign-in target behind (or vice versa).
    pub fn visible(state: &EditorState) -> Vec<MobileMoreEntry> {
        vec![
            MobileMoreEntry::NewFile,
            MobileMoreEntry::OpenFile,
            MobileMoreEntry::Templates,
            MobileMoreEntry::Assets,
            MobileMoreEntry::Ai,
            MobileMoreEntry::Collaboration,
            if state.editor_ui.account.is_signed_in() {
                MobileMoreEntry::Account
            } else {
                MobileMoreEntry::SignIn
            },
            MobileMoreEntry::Language,
            MobileMoreEntry::Settings,
            MobileMoreEntry::Variables,
            MobileMoreEntry::Preview,
            MobileMoreEntry::Export,
        ]
    }

    fn label(self, ui: &EditorUiState) -> &'static str {
        let key = match self {
            MobileMoreEntry::NewFile => "fileMenu.newFile",
            MobileMoreEntry::OpenFile => "fileMenu.openFile",
            MobileMoreEntry::Templates => "sceneTemplate.title",
            MobileMoreEntry::Assets => "assetCenter.title",
            MobileMoreEntry::Ai => "a11y.aiChat",
            MobileMoreEntry::SignIn => "settings.account.signIn",
            MobileMoreEntry::Account => "settings.account.title",
            MobileMoreEntry::Collaboration => "collab.topbar.collaborate",
            MobileMoreEntry::Language => "tooltip.topbar.language",
            MobileMoreEntry::Settings => "settings.title",
            MobileMoreEntry::Variables => "toolbar.variables",
            MobileMoreEntry::Preview => "tooltip.topbar.preview",
            MobileMoreEntry::Export => "common.export",
        };
        let label = translate(ui, key);
        if self == MobileMoreEntry::OpenFile {
            label.trim_end_matches(['.', '…'])
        } else {
            label
        }
    }

    fn icon(self) -> Icon {
        match self {
            MobileMoreEntry::NewFile => Icon::FilePlus,
            MobileMoreEntry::OpenFile => Icon::from_name("folder-open").unwrap_or(Icon::FolderOpen),
            MobileMoreEntry::Templates => Icon::LayoutDashboard,
            MobileMoreEntry::Assets => Icon::Palette,
            MobileMoreEntry::Ai => Icon::from_name("sparkles").unwrap_or(Icon::Sparkles),
            MobileMoreEntry::SignIn | MobileMoreEntry::Account => Icon::User,
            MobileMoreEntry::Collaboration => Icon::Users,
            MobileMoreEntry::Language => Icon::Globe,
            MobileMoreEntry::Settings => Icon::from_name("settings").unwrap_or(Icon::Settings),
            MobileMoreEntry::Variables => Icon::from_name("braces").unwrap_or(Icon::Braces),
            MobileMoreEntry::Preview => Icon::from_name("play").unwrap_or(Icon::Play),
            MobileMoreEntry::Export => Icon::from_name("download").unwrap_or(Icon::Download),
        }
    }
}

fn row_count(item_count: usize, columns: usize) -> usize {
    item_count.div_ceil(columns)
}

fn panel_height(item_count: usize, columns: usize, bottom_padding: f32) -> f32 {
    let rows = row_count(item_count, columns);
    HEADER_HEIGHT
        + rows as f32 * TILE_HEIGHT
        + rows.saturating_sub(1) as f32 * GRID_GAP
        + bottom_padding
}

fn compact_column_count(item_count: usize, viewport_w: f32, viewport_h: f32) -> usize {
    let portrait_height = panel_height(item_count, PORTRAIT_COLUMN_COUNT, PHONE_BOTTOM_PADDING);
    if viewport_w >= viewport_h || portrait_height > (viewport_h - 8.0).max(0.0) {
        LANDSCAPE_COLUMN_COUNT
    } else {
        PORTRAIT_COLUMN_COUNT
    }
}

fn column_count(state: &EditorState, panel: Rect) -> usize {
    if !state.editor_ui.compact_layout() {
        return PORTRAIT_COLUMN_COUNT;
    }
    let viewport_h = panel.origin.y + panel.size.y;
    compact_column_count(
        MobileMoreEntry::visible(state).len(),
        panel.size.x,
        viewport_h,
    )
}

pub fn more_panel_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    if state.editor_ui.compact_layout() {
        let item_count = MobileMoreEntry::visible(state).len();
        let columns = compact_column_count(item_count, viewport_w, viewport_h);
        let height = panel_height(item_count, columns, PHONE_BOTTOM_PADDING)
            .min((viewport_h - 8.0).max(0.0));
        return Rect {
            origin: Point2D::new(0.0, viewport_h - height),
            size: Point2D::new(viewport_w, height),
        };
    }
    let width = TABLET_PANEL_WIDTH.min((viewport_w - 24.0).max(0.0));
    let top = host_canvas_geometry::touch_app_bar_height(state) + 8.0;
    let height = panel_height(
        MobileMoreEntry::visible(state).len(),
        PORTRAIT_COLUMN_COUNT,
        TABLET_BOTTOM_PADDING,
    )
    .min((viewport_h - top - 12.0).max(0.0));
    Rect {
        origin: Point2D::new(viewport_w - width - 12.0, top),
        size: Point2D::new(width, height),
    }
}

pub fn more_entry_rect(state: &EditorState, panel: Rect, index: usize) -> Rect {
    let item_count = MobileMoreEntry::visible(state).len();
    let columns = column_count(state, panel);
    let content_w = (panel.size.x - PANEL_PADDING * 2.0).max(0.0);
    let tile_w = ((content_w - GRID_GAP * (columns as f32 - 1.0)) / columns as f32).max(0.0);
    let row = index / columns;
    let col = index % columns;
    let last_row = item_count.saturating_sub(1) / columns;
    let row_offset = if row == last_row {
        let last_row_items = item_count - last_row * columns;
        let row_width =
            tile_w * last_row_items as f32 + GRID_GAP * last_row_items.saturating_sub(1) as f32;
        (content_w - row_width) / 2.0
    } else {
        0.0
    };
    Rect {
        origin: Point2D::new(
            panel.origin.x + PANEL_PADDING + row_offset + (tile_w + GRID_GAP) * col as f32,
            panel.origin.y + HEADER_HEIGHT + (TILE_HEIGHT + GRID_GAP) * row as f32,
        ),
        size: Point2D::new(tile_w, TILE_HEIGHT),
    }
}

pub fn paint_more_panel(cx: &mut PaintCx<'_>, state: &EditorState, theme: &Theme, panel: Rect) {
    let compact = state.editor_ui.compact_layout();
    if compact {
        cx.backend
            .fill_round_rect_per_corner(panel, [20.0, 20.0, 0.0, 0.0], theme.card);
        cx.backend
            .fill_round_rect(sheet_handle_rect(panel), 2.0, theme.border);
    } else {
        cx.backend.fill_round_rect(panel, 18.0, theme.popover);
        cx.backend.stroke_round_rect(panel, 18.0, theme.border, 1.0);
    }

    let title = translate(&state.editor_ui, "git.header.overflowMoreActions");
    let title_layout = crate::TextLayout::single_run(
        title,
        "system-ui",
        16.0,
        theme.foreground.to_jian(),
        Point2D::ZERO,
    );
    let header = Rect {
        origin: panel.origin,
        size: Point2D::new(panel.size.x, HEADER_HEIGHT),
    };
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(
            panel.origin.x + 18.0,
            jian_widgets::centered_text_baseline_y(header, 16.0),
        ),
    );

    let close_target = sheet_close_rect(panel);
    let icon = Icon::from_name("x").unwrap_or(Icon::Close);
    paint_touch_icon(cx, close_target, icon, 18.0, theme.muted_foreground);

    cx.backend.save();
    cx.backend.clip_rect(panel);
    for (index, entry) in MobileMoreEntry::visible(state).into_iter().enumerate() {
        let tile = more_entry_rect(state, panel, index);
        cx.backend
            .fill_round_rect(tile, 14.0, if compact { theme.muted } else { theme.card });
        cx.backend.stroke_round_rect(tile, 14.0, theme.border, 1.0);

        let icon_target = Rect {
            origin: Point2D::new(tile.origin.x, tile.origin.y + 8.0),
            size: Point2D::new(tile.size.x, 32.0),
        };
        paint_touch_icon(cx, icon_target, entry.icon(), 20.0, theme.foreground);

        let label = entry.label(&state.editor_ui);
        let label = text_metrics::fit_chrome(
            cx.backend,
            label,
            (tile.size.x - LABEL_SIDE_PADDING * 2.0).max(0.0),
            LABEL_FONT_SIZE,
        );
        let layout = crate::TextLayout::single_run(
            &label,
            "system-ui",
            LABEL_FONT_SIZE,
            theme.foreground.to_jian(),
            Point2D::ZERO,
        );
        let label_rect = Rect {
            origin: Point2D::new(tile.origin.x, tile.origin.y + 46.0),
            size: Point2D::new(tile.size.x, 30.0),
        };
        let text_x = text_metrics::centered_text_x(cx.backend, &label, LABEL_FONT_SIZE, label_rect);
        cx.backend.save();
        cx.backend.clip_rect(tile);
        cx.backend.draw_text(
            &layout,
            Point2D::new(
                text_x,
                jian_widgets::centered_text_baseline_y(label_rect, LABEL_FONT_SIZE),
            ),
        );
        cx.backend.restore();
    }
    cx.backend.restore();
}

pub fn more_hit_test(state: &EditorState, panel: Rect, point: Point2D) -> Option<MobileMoreEntry> {
    if !panel.contains(point) {
        return None;
    }
    MobileMoreEntry::visible(state)
        .into_iter()
        .enumerate()
        .find_map(|(index, entry)| {
            more_entry_rect(state, panel, index)
                .contains(point)
                .then_some(entry)
        })
}

pub fn more_scrim_color() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_family_gap_backend::FamilyGapBackend;
    use op_editor_core::size_class::EditorSizeClass;

    fn touch_state(size_class: EditorSizeClass) -> EditorState {
        let mut state = EditorState::starter();
        state.editor_ui.touch = true;
        state.editor_ui.size_class = size_class;
        state
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_grid(
        state: &EditorState,
        viewport_w: f32,
        viewport_h: f32,
        expected_columns: usize,
        expected_bottom_padding: f32,
    ) {
        let panel = more_panel_rect(state, viewport_w, viewport_h);
        let entries = MobileMoreEntry::visible(state);
        assert_eq!(column_count(state, panel), expected_columns);
        assert_approx(
            panel.size.y,
            panel_height(entries.len(), expected_columns, expected_bottom_padding),
        );

        let rows = row_count(entries.len(), expected_columns);
        for (index, entry) in entries.iter().copied().enumerate() {
            let tile = more_entry_rect(state, panel, index);
            assert!(tile.size.x >= 44.0, "tile {index} is too narrow: {tile:?}");
            assert!(tile.size.y >= 44.0, "tile {index} is too short: {tile:?}");
            assert!(tile.origin.x >= panel.origin.x);
            assert!(tile.origin.y >= panel.origin.y);
            assert!(tile.origin.x + tile.size.x <= panel.origin.x + panel.size.x + 0.01);
            assert!(tile.origin.y + tile.size.y <= panel.origin.y + panel.size.y + 0.01);

            let center = Point2D::new(
                tile.origin.x + tile.size.x / 2.0,
                tile.origin.y + tile.size.y / 2.0,
            );
            assert_eq!(more_hit_test(state, panel, center), Some(entry));
        }

        let last_row_start = (rows - 1) * expected_columns;
        let first = more_entry_rect(state, panel, last_row_start);
        let last = more_entry_rect(state, panel, entries.len() - 1);
        let left_gap = first.origin.x - panel.origin.x - PANEL_PADDING;
        let right_gap = panel.origin.x + panel.size.x - PANEL_PADDING - last.origin.x - last.size.x;
        assert_approx(left_gap, right_gap);
    }

    #[test]
    fn compact_portrait_is_a_centered_three_column_sheet() {
        let state = touch_state(EditorSizeClass::Compact);
        assert_grid(&state, 320.0, 568.0, 3, PHONE_BOTTOM_PADDING);
        let panel = more_panel_rect(&state, 320.0, 568.0);
        assert_eq!(panel.origin.x, 0.0);
        assert_eq!(panel.size.x, 320.0);
        assert_eq!(panel.origin.y + panel.size.y, 568.0);
    }

    #[test]
    fn compact_landscape_uses_a_six_by_two_sheet() {
        let state = touch_state(EditorSizeClass::Compact);
        assert_grid(&state, 568.0, 320.0, 6, PHONE_BOTTOM_PADDING);
    }

    #[test]
    fn medium_and_expanded_keep_a_bounded_three_column_popover() {
        for (class, width, height) in [
            (EditorSizeClass::Medium, 600.0, 900.0),
            (EditorSizeClass::Expanded, 960.0, 600.0),
        ] {
            let state = touch_state(class);
            assert_grid(&state, width, height, 3, TABLET_BOTTOM_PADDING);
            let panel = more_panel_rect(&state, width, height);
            assert_eq!(panel.size.x, TABLET_PANEL_WIDTH);
            assert!(panel.origin.x + panel.size.x / 2.0 > width / 2.0);
            assert!(panel.origin.y >= host_canvas_geometry::TABLET_APP_BAR_HEIGHT);
        }
    }

    #[test]
    fn restored_entries_reuse_localized_labels_and_desktop_icons() {
        let mut state = EditorState::starter();
        assert_eq!(MobileMoreEntry::ALL.len(), 13);
        assert_eq!(MobileMoreEntry::visible(&state).len(), 12);
        assert_eq!(MobileMoreEntry::ALL[0], MobileMoreEntry::NewFile);
        assert_eq!(MobileMoreEntry::ALL[1], MobileMoreEntry::OpenFile);
        assert_eq!(MobileMoreEntry::NewFile.icon(), Icon::FilePlus);
        assert_eq!(MobileMoreEntry::OpenFile.icon(), Icon::FolderOpen);
        assert_eq!(MobileMoreEntry::Templates.icon(), Icon::LayoutDashboard);
        assert_eq!(MobileMoreEntry::Assets.icon(), Icon::Palette);

        for locale in op_i18n::Locale::ALL {
            state.editor_ui.locale = locale;
            assert_ne!(
                MobileMoreEntry::NewFile.label(&state.editor_ui),
                "fileMenu.newFile"
            );
            let label = MobileMoreEntry::OpenFile.label(&state.editor_ui);
            assert!(!label.ends_with('.'));
            assert!(!label.ends_with('…'));
            assert_ne!(label, "fileMenu.openFile");
            assert_ne!(
                MobileMoreEntry::Templates.label(&state.editor_ui),
                "sceneTemplate.title"
            );
            assert_ne!(
                MobileMoreEntry::Assets.label(&state.editor_ui),
                "assetCenter.title"
            );
        }
    }

    #[test]
    fn account_state_swaps_one_tile_without_moving_collaboration_or_changing_count() {
        let mut state = touch_state(EditorSizeClass::Compact);
        let anonymous = MobileMoreEntry::visible(&state);
        assert_eq!(anonymous.len(), 12);
        assert!(anonymous.contains(&MobileMoreEntry::SignIn));
        assert!(!anonymous.contains(&MobileMoreEntry::Account));
        assert_eq!(anonymous[5], MobileMoreEntry::Collaboration);
        assert_eq!(MobileMoreEntry::SignIn.icon(), Icon::User);
        assert_eq!(MobileMoreEntry::Collaboration.icon(), Icon::Users);
        assert_ne!(
            MobileMoreEntry::SignIn.label(&state.editor_ui),
            "settings.account.signIn"
        );
        assert_ne!(
            MobileMoreEntry::Collaboration.label(&state.editor_ui),
            "collab.topbar.collaborate"
        );

        state.editor_ui.account = op_editor_core::AccountState::SignedIn {
            display_name: "Fini".into(),
            username: "fini".into(),
        };
        let signed_in = MobileMoreEntry::visible(&state);
        assert_eq!(signed_in.len(), anonymous.len());
        assert!(!signed_in.contains(&MobileMoreEntry::SignIn));
        assert!(signed_in.contains(&MobileMoreEntry::Account));
        assert_eq!(signed_in[5], MobileMoreEntry::Collaboration);
        assert_eq!(
            signed_in
                .iter()
                .position(|entry| *entry == MobileMoreEntry::Account),
            anonymous
                .iter()
                .position(|entry| *entry == MobileMoreEntry::SignIn)
        );
    }

    #[test]
    fn every_locale_fits_labels_using_the_painted_font_family() {
        let mut state = touch_state(EditorSizeClass::Medium);
        let panel = more_panel_rect(&state, 600.0, 900.0);

        for locale in op_i18n::Locale::ALL {
            state.editor_ui.locale = locale;
            let mut backend = FamilyGapBackend::default();
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            paint_more_panel(&mut cx, &state, &Theme::dark(), panel);

            let visible_count = MobileMoreEntry::visible(&state).len();
            assert_eq!(backend.runs.len(), visible_count + 1);
            for index in 0..visible_count {
                let tile = more_entry_rect(&state, panel, index);
                let run = &backend.runs[index + 1];
                assert!(
                    run.origin.x >= tile.origin.x + LABEL_SIDE_PADDING - 0.01,
                    "{} label {index} starts outside its inset: {run:?}",
                    locale.code()
                );
                assert!(
                    run.right_edge() <= tile.origin.x + tile.size.x - LABEL_SIDE_PADDING + 0.01,
                    "{} label {index} overflows its tile: {run:?}",
                    locale.code()
                );
            }
        }
    }
}
