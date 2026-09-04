//! Responsive overflow surface for native touch editors.
//!
//! Compact windows use a short bottom sheet; tablet windows use a
//! top-right popover. Geometry and hit-testing live together so neither
//! platform can stretch the phone menu across an iPad again.

use super::editor_state_ext::translate;
use super::host_canvas_geometry;
use super::mobile_chrome::{paint_touch_icon, sheet_close_rect};
use super::panel_control_metrics::mix;
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
const PORTRAIT_FALLBACK_COLUMN_COUNT: usize = 4;
// Thirteen visible actions fit in two rows on a landscape phone. Keeping the
// portrait column count would grow the sheet to the full 320pt viewport and
// leave no useful canvas context behind the modal surface.
const LANDSCAPE_COLUMN_COUNT: usize = 7;
const PHONE_BOTTOM_PADDING: f32 = 16.0;
const TABLET_PANEL_WIDTH: f32 = 320.0;
const TABLET_BOTTOM_PADDING: f32 = 20.0;
const LABEL_FONT_SIZE: f32 = 13.0;
const LABEL_SIDE_PADDING: f32 = 6.0;

// Compact portrait uses a phone-native hierarchy instead of the legacy
// equal-weight grid: one file quick strip, one primary AI action, two
// creation cards, then a two-column utility group. Besides reading faster,
// every target stays at least 50pt and the sheet is shorter than the old
// five-row grid.
const PHONE_PORTRAIT_PANEL_HEIGHT: f32 = 448.0;
const PHONE_SIDE_PADDING: f32 = 16.0;
const PHONE_GROUP_GAP: f32 = 8.0;
const PHONE_FILE_HEIGHT: f32 = 64.0;
const PHONE_AI_HEIGHT: f32 = 58.0;
const PHONE_CREATIVE_HEIGHT: f32 = 56.0;
const PHONE_UTILITY_HEIGHT: f32 = 50.0;
const PHONE_FILE_TOP: f32 = HEADER_HEIGHT;
const PHONE_AI_TOP: f32 = PHONE_FILE_TOP + PHONE_FILE_HEIGHT + 10.0;
const PHONE_CREATIVE_TOP: f32 = PHONE_AI_TOP + PHONE_AI_HEIGHT + PHONE_GROUP_GAP;
const PHONE_UTILITY_TOP: f32 = PHONE_CREATIVE_TOP + PHONE_CREATIVE_HEIGHT + 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileMoreEntry {
    NewFile,
    OpenFile,
    /// Save into the app sandbox (first save prompts for a file name).
    SaveFile,
    /// Save a copy under a new name and switch the document to it.
    SaveAsFile,
    Templates,
    Assets,
    Ai,
    /// Selection-independent generated-code inspector (tablet only).
    Code,
    SignIn,
    Account,
    Language,
    Collaboration,
    Settings,
    Variables,
    Export,
}

impl MobileMoreEntry {
    /// Exhaustive semantic entries. Paint and hit-test use [`Self::visible`]
    /// because Sign in and Account are mutually exclusive states of one tile.
    pub const ALL: [MobileMoreEntry; 15] = [
        MobileMoreEntry::NewFile,
        MobileMoreEntry::OpenFile,
        MobileMoreEntry::SaveFile,
        MobileMoreEntry::SaveAsFile,
        MobileMoreEntry::Templates,
        MobileMoreEntry::Assets,
        MobileMoreEntry::Ai,
        MobileMoreEntry::Code,
        MobileMoreEntry::SignIn,
        MobileMoreEntry::Account,
        MobileMoreEntry::Collaboration,
        MobileMoreEntry::Language,
        MobileMoreEntry::Settings,
        MobileMoreEntry::Variables,
        MobileMoreEntry::Export,
    ];

    /// Entries visible for the current account state. This is the canonical
    /// list for layout, paint and hit-testing so a signed-in account cannot
    /// leave an invisible Sign-in target behind (or vice versa).
    ///
    /// Run (Preview) is deliberately absent: the mobile editor has no
    /// preview mode — see `press_mobile_more_sheet_tier`, which no longer
    /// carries a Preview arm either.
    pub fn visible(state: &EditorState) -> Vec<MobileMoreEntry> {
        let mut entries = vec![
            MobileMoreEntry::NewFile,
            MobileMoreEntry::OpenFile,
            MobileMoreEntry::SaveFile,
            MobileMoreEntry::SaveAsFile,
            MobileMoreEntry::Templates,
            MobileMoreEntry::Assets,
            MobileMoreEntry::Ai,
        ];
        // Compact phones deliberately omit the generated-code inspector: its
        // controls need tablet width. Medium and Expanded touch layouts expose
        // the same Code destination through the overflow surface.
        if state.editor_ui.touch_chrome() && state.editor_ui.code_property_tab_available() {
            entries.push(MobileMoreEntry::Code);
        }
        entries.extend([
            MobileMoreEntry::Collaboration,
            if state.editor_ui.account.is_signed_in() {
                MobileMoreEntry::Account
            } else {
                MobileMoreEntry::SignIn
            },
            MobileMoreEntry::Language,
            MobileMoreEntry::Settings,
            MobileMoreEntry::Variables,
            MobileMoreEntry::Export,
        ]);
        entries
    }

    fn label(self, ui: &EditorUiState) -> &'static str {
        let key = match self {
            MobileMoreEntry::NewFile => "fileMenu.newFile",
            MobileMoreEntry::OpenFile => "fileMenu.openFile",
            MobileMoreEntry::SaveFile => "fileMenu.save",
            MobileMoreEntry::SaveAsFile => "fileMenu.saveAs",
            MobileMoreEntry::Templates => "sceneTemplate.title",
            MobileMoreEntry::Assets => "assetCenter.title",
            MobileMoreEntry::Ai => "a11y.aiChat",
            MobileMoreEntry::Code => "rightPanel.code",
            MobileMoreEntry::SignIn => "settings.account.signIn",
            MobileMoreEntry::Account => "settings.account.title",
            MobileMoreEntry::Collaboration => "collab.topbar.collaborate",
            MobileMoreEntry::Language => "tooltip.topbar.language",
            MobileMoreEntry::Settings => "settings.title",
            MobileMoreEntry::Variables => "toolbar.variables",
            MobileMoreEntry::Export => "common.export",
        };
        let label = translate(ui, key);
        if matches!(
            self,
            MobileMoreEntry::OpenFile | MobileMoreEntry::SaveAsFile
        ) {
            label.trim_end_matches(['.', '…'])
        } else {
            label
        }
    }

    fn icon(self) -> Icon {
        match self {
            MobileMoreEntry::NewFile => Icon::FilePlus,
            MobileMoreEntry::OpenFile => Icon::from_name("folder-open").unwrap_or(Icon::FolderOpen),
            MobileMoreEntry::SaveFile => Icon::from_name("save").unwrap_or(Icon::Save),
            MobileMoreEntry::SaveAsFile => Icon::from_name("copy").unwrap_or(Icon::Copy),
            MobileMoreEntry::Templates => Icon::LayoutDashboard,
            MobileMoreEntry::Assets => Icon::Palette,
            MobileMoreEntry::Ai => Icon::from_name("sparkles").unwrap_or(Icon::Sparkles),
            MobileMoreEntry::Code => Icon::from_name("code").unwrap_or(Icon::Braces),
            MobileMoreEntry::SignIn | MobileMoreEntry::Account => Icon::User,
            MobileMoreEntry::Collaboration => Icon::Users,
            MobileMoreEntry::Language => Icon::Globe,
            MobileMoreEntry::Settings => Icon::from_name("settings").unwrap_or(Icon::Settings),
            MobileMoreEntry::Variables => Icon::from_name("braces").unwrap_or(Icon::Braces),
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
    if viewport_w >= viewport_h {
        LANDSCAPE_COLUMN_COUNT
    } else if portrait_height > (viewport_h - 8.0).max(0.0) {
        PORTRAIT_FALLBACK_COLUMN_COUNT
    } else {
        PORTRAIT_COLUMN_COUNT
    }
}

fn uses_phone_portrait_layout(state: &EditorState, viewport_w: f32, viewport_h: f32) -> bool {
    state.editor_ui.compact_layout()
        && viewport_w < viewport_h
        && PHONE_PORTRAIT_PANEL_HEIGHT <= (viewport_h - 8.0).max(0.0)
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

fn legacy_tile_height(state: &EditorState, panel: Rect, item_count: usize, columns: usize) -> f32 {
    let rows = row_count(item_count, columns).max(1);
    let bottom_padding = if state.editor_ui.compact_layout() {
        PHONE_BOTTOM_PADDING
    } else {
        TABLET_BOTTOM_PADDING
    };
    let gaps = rows.saturating_sub(1) as f32 * GRID_GAP;
    let available = (panel.size.y - HEADER_HEIGHT - gaps - bottom_padding).max(0.0);
    (available / rows as f32).clamp(44.0, TILE_HEIGHT)
}

pub fn more_panel_rect(state: &EditorState, viewport_w: f32, viewport_h: f32) -> Rect {
    if state.editor_ui.compact_layout() {
        if uses_phone_portrait_layout(state, viewport_w, viewport_h) {
            return Rect {
                origin: Point2D::new(0.0, viewport_h - PHONE_PORTRAIT_PANEL_HEIGHT),
                size: Point2D::new(viewport_w, PHONE_PORTRAIT_PANEL_HEIGHT),
            };
        }
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

fn phone_pair_rect(panel: Rect, top: f32, height: f32, column: usize) -> Rect {
    let content_w = (panel.size.x - PHONE_SIDE_PADDING * 2.0).max(0.0);
    let width = ((content_w - PHONE_GROUP_GAP) / 2.0).max(0.0);
    Rect {
        origin: Point2D::new(
            panel.origin.x + PHONE_SIDE_PADDING + column as f32 * (width + PHONE_GROUP_GAP),
            panel.origin.y + top,
        ),
        size: Point2D::new(width, height),
    }
}

fn phone_portrait_entry_rect(state: &EditorState, panel: Rect, index: usize) -> Rect {
    let Some(entry) = MobileMoreEntry::visible(state).get(index).copied() else {
        return Rect::xywh(panel.origin.x, panel.origin.y, 0.0, 0.0);
    };
    let content_w = (panel.size.x - PHONE_SIDE_PADDING * 2.0).max(0.0);
    match entry {
        MobileMoreEntry::NewFile
        | MobileMoreEntry::OpenFile
        | MobileMoreEntry::SaveFile
        | MobileMoreEntry::Export => {
            let file_index = match entry {
                MobileMoreEntry::NewFile => 0,
                MobileMoreEntry::OpenFile => 1,
                MobileMoreEntry::SaveFile => 2,
                MobileMoreEntry::Export => 3,
                _ => unreachable!("matched file quick action"),
            };
            let width = content_w / 4.0;
            Rect::xywh(
                panel.origin.x + PHONE_SIDE_PADDING + file_index as f32 * width,
                panel.origin.y + PHONE_FILE_TOP,
                width,
                PHONE_FILE_HEIGHT,
            )
        }
        MobileMoreEntry::Ai => Rect::xywh(
            panel.origin.x + PHONE_SIDE_PADDING,
            panel.origin.y + PHONE_AI_TOP,
            content_w,
            PHONE_AI_HEIGHT,
        ),
        MobileMoreEntry::Code => {
            unreachable!("Code is not visible in the Compact portrait More sheet")
        }
        MobileMoreEntry::Templates => {
            phone_pair_rect(panel, PHONE_CREATIVE_TOP, PHONE_CREATIVE_HEIGHT, 0)
        }
        MobileMoreEntry::Assets => {
            phone_pair_rect(panel, PHONE_CREATIVE_TOP, PHONE_CREATIVE_HEIGHT, 1)
        }
        MobileMoreEntry::Collaboration => {
            phone_pair_rect(panel, PHONE_UTILITY_TOP, PHONE_UTILITY_HEIGHT, 0)
        }
        MobileMoreEntry::SignIn | MobileMoreEntry::Account => {
            phone_pair_rect(panel, PHONE_UTILITY_TOP, PHONE_UTILITY_HEIGHT, 1)
        }
        MobileMoreEntry::SaveAsFile => phone_pair_rect(
            panel,
            PHONE_UTILITY_TOP + PHONE_UTILITY_HEIGHT + PHONE_GROUP_GAP,
            PHONE_UTILITY_HEIGHT,
            0,
        ),
        MobileMoreEntry::Variables => phone_pair_rect(
            panel,
            PHONE_UTILITY_TOP + PHONE_UTILITY_HEIGHT + PHONE_GROUP_GAP,
            PHONE_UTILITY_HEIGHT,
            1,
        ),
        MobileMoreEntry::Language => phone_pair_rect(
            panel,
            PHONE_UTILITY_TOP + (PHONE_UTILITY_HEIGHT + PHONE_GROUP_GAP) * 2.0,
            PHONE_UTILITY_HEIGHT,
            0,
        ),
        MobileMoreEntry::Settings => phone_pair_rect(
            panel,
            PHONE_UTILITY_TOP + (PHONE_UTILITY_HEIGHT + PHONE_GROUP_GAP) * 2.0,
            PHONE_UTILITY_HEIGHT,
            1,
        ),
    }
}

pub fn more_entry_rect(state: &EditorState, panel: Rect, index: usize) -> Rect {
    let viewport_h = panel.origin.y + panel.size.y;
    if uses_phone_portrait_layout(state, panel.size.x, viewport_h) {
        return phone_portrait_entry_rect(state, panel, index);
    }
    let item_count = MobileMoreEntry::visible(state).len();
    let columns = column_count(state, panel);
    let tile_h = legacy_tile_height(state, panel, item_count, columns);
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
            panel.origin.y + HEADER_HEIGHT + (tile_h + GRID_GAP) * row as f32,
        ),
        size: Point2D::new(tile_w, tile_h),
    }
}

fn paint_phone_label(
    cx: &mut PaintCx<'_>,
    label: &str,
    rect: Rect,
    font_size: f32,
    font_weight: u16,
    color: Color,
    centered: bool,
) {
    let fitted = text_metrics::fit_chrome(cx.backend, label, rect.size.x.max(0.0), font_size);
    let layout = crate::TextLayout::single_run(
        &fitted,
        "system-ui",
        font_size,
        color.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(font_weight);
    let x = if centered {
        text_metrics::centered_text_x(cx.backend, &fitted, font_size, rect)
    } else {
        rect.origin.x
    };
    cx.backend.save();
    cx.backend.clip_rect(rect);
    cx.backend.draw_text(
        &layout,
        Point2D::new(x, jian_widgets::centered_text_baseline_y(rect, font_size)),
    );
    cx.backend.restore();
}

#[derive(Clone, Copy)]
struct PhoneEntryStyle {
    fill: Color,
    icon: Color,
    text: Color,
    prominent: bool,
}

fn paint_phone_horizontal_entry(
    cx: &mut PaintCx<'_>,
    state: &EditorState,
    theme: &Theme,
    entry: MobileMoreEntry,
    rect: Rect,
    style: PhoneEntryStyle,
) {
    cx.backend.fill_round_rect(rect, 14.0, style.fill);
    if !style.prominent {
        cx.backend
            .stroke_round_rect(rect, 14.0, theme.border.with_alpha(0.72), 1.0);
    }
    let icon_target = Rect::xywh(
        rect.origin.x + 6.0,
        rect.origin.y + 7.0,
        44.0,
        rect.size.y - 14.0,
    );
    paint_touch_icon(
        cx,
        icon_target,
        entry.icon(),
        if style.prominent { 22.0 } else { 19.0 },
        style.icon,
    );
    let trailing = if style.prominent { 38.0 } else { 12.0 };
    let label_rect = Rect::xywh(
        rect.origin.x + 54.0,
        rect.origin.y,
        (rect.size.x - 54.0 - trailing).max(0.0),
        rect.size.y,
    );
    paint_phone_label(
        cx,
        entry.label(&state.editor_ui),
        label_rect,
        if style.prominent { 15.0 } else { 13.5 },
        if style.prominent { 600 } else { 400 },
        style.text,
        false,
    );
    if style.prominent {
        let chevron = Rect::xywh(
            rect.origin.x + rect.size.x - 40.0,
            rect.origin.y + (rect.size.y - 40.0) / 2.0,
            40.0,
            40.0,
        );
        paint_touch_icon(
            cx,
            chevron,
            Icon::ChevronRight,
            18.0,
            style.text.with_alpha(0.84),
        );
    }
}

fn paint_phone_portrait_entries(
    cx: &mut PaintCx<'_>,
    state: &EditorState,
    theme: &Theme,
    panel: Rect,
) {
    let visible = MobileMoreEntry::visible(state);
    let file_entries = [
        MobileMoreEntry::NewFile,
        MobileMoreEntry::OpenFile,
        MobileMoreEntry::SaveFile,
        MobileMoreEntry::Export,
    ];

    // File actions read as one compact command strip instead of four equal
    // cards competing with feature destinations.
    let file_first = phone_portrait_entry_rect(state, panel, 0);
    let file_group = Rect::xywh(
        file_first.origin.x,
        file_first.origin.y,
        (panel.size.x - PHONE_SIDE_PADDING * 2.0).max(0.0),
        PHONE_FILE_HEIGHT,
    );
    cx.backend
        .fill_round_rect(file_group, 16.0, mix(theme.card, theme.foreground, 0.055));
    cx.backend
        .stroke_round_rect(file_group, 16.0, theme.border.with_alpha(0.78), 1.0);
    for (file_index, entry) in file_entries.into_iter().enumerate() {
        let index = visible
            .iter()
            .position(|visible_entry| *visible_entry == entry)
            .expect("file quick action is visible");
        let slot = phone_portrait_entry_rect(state, panel, index);
        if file_index > 0 {
            cx.backend.fill_rect(
                Rect::xywh(slot.origin.x, slot.origin.y + 16.0, 1.0, slot.size.y - 32.0),
                theme.border.with_alpha(0.72),
            );
        }
        let icon_target = Rect::xywh(slot.origin.x, slot.origin.y + 5.0, slot.size.x, 30.0);
        paint_touch_icon(
            cx,
            icon_target,
            entry.icon(),
            19.0,
            theme.foreground.with_alpha(0.9),
        );
        paint_phone_label(
            cx,
            entry.label(&state.editor_ui),
            Rect::xywh(
                slot.origin.x + 4.0,
                slot.origin.y + 36.0,
                slot.size.x - 8.0,
                24.0,
            ),
            11.5,
            400,
            theme.foreground.with_alpha(0.88),
            true,
        );
    }

    for (index, entry) in visible.into_iter().enumerate() {
        if file_entries.contains(&entry) {
            continue;
        }
        let rect = phone_portrait_entry_rect(state, panel, index);
        match entry {
            MobileMoreEntry::Ai => paint_phone_horizontal_entry(
                cx,
                state,
                theme,
                entry,
                rect,
                PhoneEntryStyle {
                    fill: theme.primary,
                    icon: theme.primary_foreground,
                    text: theme.primary_foreground,
                    prominent: true,
                },
            ),
            MobileMoreEntry::Templates | MobileMoreEntry::Assets => paint_phone_horizontal_entry(
                cx,
                state,
                theme,
                entry,
                rect,
                PhoneEntryStyle {
                    fill: theme.row_selected_primary,
                    icon: theme.primary,
                    text: theme.foreground,
                    prominent: false,
                },
            ),
            _ => paint_phone_horizontal_entry(
                cx,
                state,
                theme,
                entry,
                rect,
                PhoneEntryStyle {
                    fill: mix(theme.card, theme.foreground, 0.035),
                    icon: theme.muted_foreground,
                    text: theme.foreground.with_alpha(0.92),
                    prominent: false,
                },
            ),
        }
    }
}

pub fn paint_more_panel(cx: &mut PaintCx<'_>, state: &EditorState, theme: &Theme, panel: Rect) {
    let compact = state.editor_ui.compact_layout();
    if compact {
        cx.backend
            .fill_round_rect_per_corner(panel, [24.0, 24.0, 0.0, 0.0], theme.card);
    } else {
        cx.backend.fill_round_rect(panel, 18.0, theme.popover);
        cx.backend.stroke_round_rect(panel, 18.0, theme.border, 1.0);
    }

    let title = translate(&state.editor_ui, "git.header.overflowMoreActions");
    let title_layout = crate::TextLayout::single_run(
        title,
        "system-ui",
        17.0,
        theme.foreground.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(600);
    let header = Rect {
        origin: panel.origin,
        size: Point2D::new(panel.size.x, HEADER_HEIGHT),
    };
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(
            panel.origin.x + 18.0,
            jian_widgets::centered_text_baseline_y(header, 17.0),
        ),
    );

    let close_target = sheet_close_rect(panel);
    if compact {
        cx.backend.fill_round_rect(
            Rect::xywh(
                close_target.origin.x + 4.0,
                close_target.origin.y + 4.0,
                close_target.size.x - 8.0,
                close_target.size.y - 8.0,
            ),
            18.0,
            theme.muted,
        );
    }
    let icon = Icon::from_name("x").unwrap_or(Icon::Close);
    paint_touch_icon(cx, close_target, icon, 18.0, theme.muted_foreground);

    cx.backend.save();
    cx.backend.clip_rect(panel);
    let viewport_h = panel.origin.y + panel.size.y;
    if uses_phone_portrait_layout(state, panel.size.x, viewport_h) {
        paint_phone_portrait_entries(cx, state, theme, panel);
    } else {
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
            let text_x =
                text_metrics::centered_text_x(cx.backend, &label, LABEL_FONT_SIZE, label_rect);
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
#[path = "mobile_more_panel_tests.rs"]
mod tests;
