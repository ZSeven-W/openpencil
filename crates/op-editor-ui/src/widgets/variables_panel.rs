use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_core::{EditorState, Locale, VariablesPanelButton};

mod geometry;
mod header;
mod hit;
mod hover;
mod menus;
mod paint;

pub(self) use geometry::*;

const ROW_HEIGHT: f32 = 44.0;
const HEADER_HEIGHT: f32 = 44.0;
const COLUMN_HEADER_HEIGHT: f32 = 36.0;
const FOOTER_HEIGHT: f32 = 40.0;
const CHIP_HEIGHT: f32 = 20.0;
const PAD_X: f32 = 16.0;
const SWATCH_SIZE: f32 = 18.0;
const NAME_COLUMN_WIDTH: f32 = 220.0;
const ACTION_COLUMN_WIDTH: f32 = 44.0;
const DROPDOWN_WIDTH: f32 = 176.0;
const DROPDOWN_ROW_HEIGHT: f32 = 36.0;
const PANEL_RADIUS: f32 = 16.0;
const ADD_VARIABLE_MENU_WIDTH: f32 = 176.0;
const ADD_VARIABLE_MENU_ROW_HEIGHT: f32 = 30.0;
const ADD_VARIABLE_MENU_ROWS: f32 = 3.0;
/// Search row below the variant column header (TS `variables-panel.tsx`
/// px-4 py-2 + h-7 input ≈ 44 px).
const SEARCH_ROW_HEIGHT: f32 = 44.0;
/// Variable rows the panel shows before the search box appears (TS
/// `entries.length > 6`).
const SEARCH_VISIBLE_THRESHOLD: usize = 6;
/// Row `⋯` overflow menu (TS `variable-row.tsx` w-40).
const ROW_MENU_WIDTH: f32 = 160.0;
/// Edge strips are 6 px (TS w-1.5 / h-1.5), the corner grip 12 px (w-3 h-3).
const RESIZE_EDGE_PX: f32 = 6.0;
const RESIZE_CORNER_PX: f32 = 12.0;
pub const VARIABLES_PANEL_WIDTH: f32 = 820.0;
/// TS resize clamps (`variables-panel.tsx` MIN_WIDTH / MIN_HEIGHT).
pub const VARIABLES_PANEL_MIN_WIDTH: f32 = 480.0;
pub const VARIABLES_PANEL_MIN_HEIGHT: f32 = 240.0;
pub const VARIABLES_PANEL_DEFAULT_HEIGHT: f32 = 480.0;

/// Which resize affordance of the floating panel a press landed on
/// (TS right / bottom / corner pointer handles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariablesResizeEdge {
    Right,
    Bottom,
    Corner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariablesPanelHit {
    Close,
    ThemeTab(String),
    ToggleThemeMenu(String),
    ThemeMenuRename(String),
    ThemeMenuDelete(String),
    AddTheme,
    TogglePresetMenu,
    AddVariant,
    ToggleVariantMenu(String),
    VariantMenuRename(String),
    VariantMenuDelete(String),
    ToggleAddVariableMenu,
    AddVariableColor,
    AddVariableNumber,
    AddVariableString,
    /// Drag-resize start on a panel edge / the corner grip.
    Resize(VariablesResizeEdge),
    /// Focus the search filter input.
    SearchBox,
    /// Row indices below are UNFILTERED `doc.variables` positions so
    /// hosts can index `editor_state_var_table` directly even while a
    /// search filter narrows the painted list.
    NameCell(usize),
    ValueCell {
        row: usize,
        variant: usize,
    },
    /// The color swatch inside a Color value cell — opens the HSV
    /// picker targeted at that variant column (#19). The surrounding
    /// hex text region maps to `ValueCell` (inline hex editing).
    ColorSwatch {
        row: usize,
        variant: usize,
    },
    /// The row's `⋯` overflow button.
    RowMenuToggle(usize),
    /// Rename entry inside the open row menu.
    RowMenuRename(usize),
    /// Delete entry inside the open row menu.
    RowMenuDelete(usize),
    Row(usize),
    AxisChip(usize),
    AxisDropdownItem {
        axis: String,
        value: String,
    },
}

#[derive(Debug, Clone)]
struct VarRow {
    /// Position in the UNFILTERED `doc.variables` BTreeMap order —
    /// the index every focus / menu / host lookup is keyed by.
    source_idx: usize,
    name: String,
    kind: VariableKind,
    value: VariableValue,
    resolved: Option<VariableScalar>,
}

#[derive(Debug, Clone)]
struct AxisChip {
    axis: String,
    value: String,
}

pub struct VariablesPanel {
    rows: Vec<VarRow>,
    /// Unfiltered variable count — drives search-box visibility.
    total_rows: usize,
    theme: Theme,
    locale: Locale,
    chips: Vec<AxisChip>,
    themes: Vec<(String, Vec<String>)>,
    current_axis: Option<String>,
    dropdown_open: Option<String>,
    theme_menu_open: Option<String>,
    variant_menu_open: Option<String>,
    renaming_theme: Option<String>,
    renaming_variant: Option<String>,
    preset_menu_open: bool,
    add_menu_open: bool,
    search: String,
    search_focus: bool,
    search_input: jian_core::text_input::TextInputState,
    scroll: f32,
    /// Open `⋯` row menu, keyed by UNFILTERED row index.
    row_menu_open: Option<usize>,
    hover: Option<VariablesPanelButton>,
    editing_name_row: Option<usize>,
    editing_value_cell: Option<(usize, usize)>,
    header_input: jian_core::text_input::TextInputState,
    row_input: jian_core::text_input::TextInputState,
    now_ms: u64,
}

impl VariablesPanel {
    pub fn for_editor(state: &EditorState) -> Self {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &EditorState, now_ms: u64) -> Self {
        // Variable rows — keyed by BTreeMap order so paint is stable,
        // narrowed by the live search filter (TS filters with a
        // case-insensitive substring on the name). DIVERGENCE from TS
        // `variables-panel.tsx:106`: TS re-sorts with localeCompare;
        // the BTreeMap is byte-lexicographic, which matches for the
        // ASCII names the panel mints (`color-N` …).
        let needle = state.editor_ui.variables_search.to_lowercase();
        let total_rows = state
            .doc
            .variables
            .as_ref()
            .map(|vars| vars.len())
            .unwrap_or(0);
        let rows: Vec<VarRow> = state
            .doc
            .variables
            .as_ref()
            .map(|vars| {
                vars.iter()
                    .enumerate()
                    .filter(|(_, (name, _))| {
                        needle.is_empty() || name.to_lowercase().contains(&needle)
                    })
                    .map(|(source_idx, (name, def))| VarRow {
                        source_idx,
                        name: name.clone(),
                        kind: def.kind.clone(),
                        value: def.value.clone(),
                        resolved: state.resolve_variable(name).cloned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let chips: Vec<AxisChip> = state
            .ui
            .variables
            .active_theme
            .iter()
            .map(|(axis, value)| AxisChip {
                axis: axis.clone(),
                value: value.clone(),
            })
            .collect();
        // Theme axes + their value lists.
        let mut themes: Vec<(String, Vec<String>)> = state
            .doc
            .themes
            .as_ref()
            .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        if themes.is_empty() && !rows.is_empty() {
            themes.push(("Theme-1".to_string(), vec!["Default".to_string()]));
        }
        let current_axis = state
            .editor_ui
            .variables_current_axis
            .as_ref()
            .filter(|axis| themes.iter().any(|(name, _)| name == *axis))
            .cloned()
            .or_else(|| {
                state
                    .ui
                    .variables
                    .active_theme
                    .keys()
                    .find(|axis| themes.iter().any(|(name, _)| name == *axis))
                    .cloned()
            })
            .or_else(|| themes.first().map(|(axis, _)| axis.clone()));
        let mut search_input = jian_core::text_input::TextInputState::with_text(
            state.editor_ui.variables_search.clone(),
        );
        search_input.touch(state.ui.property_caret_anchor_ms);

        Self {
            rows,
            total_rows,
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            chips,
            themes,
            current_axis,
            dropdown_open: state.editor_ui.axis_dropdown_open.clone(),
            theme_menu_open: state.editor_ui.variables_theme_menu_axis.clone(),
            variant_menu_open: state.editor_ui.variables_variant_menu_value.clone(),
            renaming_theme: state.editor_ui.variables_theme_rename_axis.clone(),
            renaming_variant: state.editor_ui.variables_variant_rename_value.clone(),
            preset_menu_open: state.editor_ui.variables_preset_menu_open,
            add_menu_open: state.editor_ui.variables_add_menu_open,
            search: state.editor_ui.variables_search.clone(),
            search_focus: state.editor_ui.variables_search_focus,
            search_input,
            scroll: state.editor_ui.variables_scroll.offset,
            row_menu_open: state.editor_ui.variables_row_menu,
            hover: state.editor_ui.variables_panel_hover,
            editing_name_row: state.editor_ui.variable_row_focus.and_then(|f| match f {
                VariableRowFocus::Name(i) => Some(i),
                VariableRowFocus::Number(_)
                | VariableRowFocus::String(_)
                | VariableRowFocus::NumberCell { .. }
                | VariableRowFocus::StringCell { .. }
                | VariableRowFocus::ColorCell { .. } => None,
            }),
            editing_value_cell: state.editor_ui.variable_row_focus.and_then(|f| match f {
                VariableRowFocus::Number(i) | VariableRowFocus::String(i) => Some((i, 0)),
                VariableRowFocus::NumberCell { row, variant }
                | VariableRowFocus::StringCell { row, variant }
                | VariableRowFocus::ColorCell { row, variant } => Some((row, variant)),
                VariableRowFocus::Name(_) => None,
            }),
            header_input: state.editor_ui.variables_header_input.clone(),
            row_input: state.editor_ui.variable_row_input.clone(),
            now_ms,
        }
    }

    /// Number of variable rows the panel paints.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Number of axis chips in the header. May be zero — a document
    /// without themes shows only the variable list.
    pub fn axis_count(&self) -> usize {
        self.chips.len()
    }

    fn theme_tab_labels(&self) -> Vec<&str> {
        self.themes.iter().map(|(axis, _)| axis.as_str()).collect()
    }

    fn active_axis_label(&self) -> &str {
        self.current_axis
            .as_deref()
            .or_else(|| self.chips.first().map(|chip| chip.axis.as_str()))
            .or_else(|| self.themes.first().map(|(axis, _)| axis.as_str()))
            .unwrap_or("Theme-1")
    }

    fn variant_column_labels(&self) -> Vec<&str> {
        self.variant_column_labels_for_axis(self.active_axis_label())
    }

    fn variant_column_labels_for_axis(&self, axis: &str) -> Vec<&str> {
        self.axis_values(axis)
            .filter(|values| !values.is_empty())
            .map(|values| values.iter().map(String::as_str).collect())
            .unwrap_or_else(|| vec!["Default"])
    }

    pub fn variant_column_count(&self) -> usize {
        self.variant_column_labels().len()
    }

    fn variant_scalar_for<'a>(
        &self,
        var: &'a VarRow,
        axis: &str,
        value: &str,
    ) -> Option<&'a VariableScalar> {
        match &var.value {
            VariableValue::Scalar(s) => Some(s),
            VariableValue::Themed(entries) => entries
                .iter()
                .find(|entry| {
                    entry
                        .theme
                        .as_ref()
                        .and_then(|theme| theme.get(axis))
                        .is_some_and(|v| v == value)
                })
                .map(|entry| &entry.value)
                .or_else(|| {
                    entries
                        .iter()
                        .find(|entry| entry.theme.is_none())
                        .map(|entry| &entry.value)
                })
                .or_else(|| entries.first().map(|entry| &entry.value)),
        }
    }

    /// Total height (header + chips row + variable rows). Used by
    /// the right-rail host when computing layout.
    pub fn intrinsic_height(&self) -> f32 {
        HEADER_HEIGHT
            + COLUMN_HEADER_HEIGHT
            + self.search_row_height()
            + FOOTER_HEIGHT
            + (self.row_count() as f32) * ROW_HEIGHT
    }

    /// Whether the search filter row paints. TS shows it when more
    /// than 6 FILTERED entries exist (`variables-panel.tsx:153`).
    /// DIVERGENCE: we key on the unfiltered count OR an active
    /// search — the TS rule unmounts the box as soon as a filter
    /// narrows the list to ≤6, stranding the typed filter with no way
    /// to clear it.
    pub fn search_visible(&self) -> bool {
        self.total_rows > SEARCH_VISIBLE_THRESHOLD || !self.search.is_empty()
    }

    fn search_row_height(&self) -> f32 {
        if self.search_visible() {
            SEARCH_ROW_HEIGHT
        } else {
            0.0
        }
    }

    /// Full-width strip housing the search input.
    fn search_row_rect(&self, rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(
                rect.origin.x,
                rect.origin.y + HEADER_HEIGHT + COLUMN_HEADER_HEIGHT,
            ),
            size: Point2D::new(rect.size.x, self.search_row_height()),
        }
    }

    /// The inset input box inside the search strip.
    pub(in crate::widgets) fn search_input_rect(&self, rect: Rect) -> Rect {
        let strip = self.search_row_rect(rect);
        Rect {
            origin: Point2D::new(strip.origin.x + PAD_X, strip.origin.y + 8.0),
            size: Point2D::new((strip.size.x - PAD_X * 2.0).max(0.0), 28.0),
        }
    }

    /// Top of the (scrollable) rows region.
    fn rows_start_y(&self, rect: Rect) -> f32 {
        rect.origin.y + HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + self.search_row_height()
    }

    /// The clip viewport the rows scroll within.
    pub(in crate::widgets) fn rows_viewport(&self, rect: Rect) -> Rect {
        let top = self.rows_start_y(rect);
        let bottom = rect.origin.y + rect.size.y - FOOTER_HEIGHT;
        Rect {
            origin: Point2D::new(rect.origin.x, top),
            size: Point2D::new(rect.size.x, (bottom - top).max(0.0)),
        }
    }

    /// Largest valid scroll offset for the current row count.
    pub fn max_scroll(&self, rect: Rect) -> f32 {
        let viewport = self.rows_viewport(rect);
        ((self.rows.len() as f32) * ROW_HEIGHT - viewport.size.y).max(0.0)
    }

    /// Host-stored scroll clamped to the valid range — a stale offset
    /// after deletes / filtering self-corrects.
    pub fn effective_scroll(&self, rect: Rect) -> f32 {
        self.scroll.clamp(0.0, self.max_scroll(rect))
    }

    /// Screen-y of the painted row at DISPLAY position `display_idx`.
    fn row_y(&self, rect: Rect, display_idx: usize) -> f32 {
        self.rows_start_y(rect) - self.effective_scroll(rect) + ROW_HEIGHT * display_idx as f32
    }

    /// Painted (display) position of an UNFILTERED row index.
    fn display_index_of(&self, source_idx: usize) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.source_idx == source_idx)
    }

    /// The `⋯` overflow button in the row's actions column.
    fn row_menu_button_rect(&self, rect: Rect, display_idx: usize) -> Rect {
        let y = self.row_y(rect, display_idx);
        Rect {
            origin: Point2D::new(
                rect.origin.x + rect.size.x - PAD_X - 28.0,
                y + (ROW_HEIGHT - 26.0) / 2.0,
            ),
            size: Point2D::new(26.0, 26.0),
        }
    }

    /// Open row menu overlay (display index + rect), anchored under
    /// the owning row's `⋯` button, right-aligned (TS `right-0
    /// top-full mt-1 w-40`).
    fn row_menu_rect(&self, rect: Rect) -> Option<(usize, Rect)> {
        let source = self.row_menu_open?;
        let display_idx = self.display_index_of(source)?;
        let button = self.row_menu_button_rect(rect, display_idx);
        Some((
            display_idx,
            Rect {
                origin: Point2D::new(
                    button.origin.x + button.size.x - ROW_MENU_WIDTH,
                    button.origin.y + button.size.y + 4.0,
                ),
                size: Point2D::new(ROW_MENU_WIDTH, ADD_VARIABLE_MENU_ROW_HEIGHT * 2.0),
            },
        ))
    }

    /// Resize affordance under `point`, corner-first (TS pointer
    /// handles: 6 px right/bottom strips + a 12 px corner grip).
    pub fn resize_edge_at(&self, rect: Rect, point: Point2D) -> Option<VariablesResizeEdge> {
        if !(rect).contains(point) {
            return None;
        }
        let right = rect.origin.x + rect.size.x;
        let bottom = rect.origin.y + rect.size.y;
        if point.x >= right - RESIZE_CORNER_PX && point.y >= bottom - RESIZE_CORNER_PX {
            return Some(VariablesResizeEdge::Corner);
        }
        if point.x >= right - RESIZE_EDGE_PX {
            return Some(VariablesResizeEdge::Right);
        }
        if point.y >= bottom - RESIZE_EDGE_PX {
            return Some(VariablesResizeEdge::Bottom);
        }
        None
    }

    /// Value list for an axis name, sourced from `doc.themes`.
    fn axis_values(&self, axis: &str) -> Option<&[String]> {
        self.themes
            .iter()
            .find(|(name, _)| name == axis)
            .map(|(_, values)| values.as_slice())
    }

    /// Anchor rect of the chip at index `i` within `rect`. Mirrors
    /// the paint walk in `paint` so hit-test + dropdown anchoring
    /// stay aligned without re-measuring.
    fn chip_rect(&self, rect: Rect, idx: usize) -> Rect {
        let Some(chip) = self.chips.get(idx) else {
            return Rect {
                origin: Point2D::new(value_column_x(rect), rect.origin.y + HEADER_HEIGHT + 5.0),
                size: Point2D::new(0.0, CHIP_HEIGHT),
            };
        };
        let col_x = value_column_x(rect);
        let labels = self.variant_column_labels_for_axis(&chip.axis);
        let value_idx = labels
            .iter()
            .position(|label| *label == chip.value.as_str())
            .unwrap_or(0);
        let col_w = variant_column_width(rect, labels.len());
        Rect {
            origin: Point2D::new(
                col_x + col_w * value_idx as f32,
                rect.origin.y + HEADER_HEIGHT + 5.0,
            ),
            size: Point2D::new(
                (label_width(&chip.value, 13.0) + 22.0).min(col_w - 8.0),
                26.0,
            ),
        }
    }

    fn add_theme_rect(&self, rect: Rect) -> Rect {
        Rect {
            origin: Point2D::new(self.theme_tabs_end_x(rect) + 6.0, rect.origin.y + 8.0),
            size: Point2D::new(28.0, 28.0),
        }
    }

    fn preset_rect(&self, rect: Rect) -> Rect {
        let add = self.add_theme_rect(rect);
        Rect {
            origin: Point2D::new(add.origin.x + add.size.x + 8.0, rect.origin.y + 6.0),
            size: Point2D::new(122.0, 32.0),
        }
    }

    fn preset_menu_rect(&self, rect: Rect) -> Rect {
        let preset = self.preset_rect(rect);
        Rect {
            origin: Point2D::new(preset.origin.x, rect.origin.y + HEADER_HEIGHT + 4.0),
            size: Point2D::new(224.0, 144.0),
        }
    }

    fn theme_tabs_end_x(&self, rect: Rect) -> f32 {
        let mut x = rect.origin.x + PAD_X;
        for label in self.theme_tab_labels() {
            x += self.theme_tab_advance_width(label);
        }
        x
    }

    fn theme_tab_rect(&self, rect: Rect, idx: usize) -> Rect {
        let mut x = rect.origin.x + PAD_X;
        for (i, label) in self.theme_tab_labels().iter().enumerate() {
            let width = self.theme_tab_hit_width(label);
            if i == idx {
                return Rect {
                    origin: Point2D::new(x - 6.0, rect.origin.y + 6.0),
                    size: Point2D::new(width + 12.0, 32.0),
                };
            }
            x += self.theme_tab_advance_width(label);
        }
        Rect {
            origin: Point2D::new(rect.origin.x + PAD_X, rect.origin.y + 6.0),
            size: Point2D::new(0.0, 32.0),
        }
    }

    fn theme_rename_input_width(&self) -> f32 {
        (label_width(self.header_input.text(), 13.0) + 28.0).max(96.0)
    }

    fn theme_tab_hit_width(&self, label: &str) -> f32 {
        if self.renaming_theme.as_deref() == Some(label) {
            self.theme_rename_input_width()
        } else {
            label_width(label, 13.0) + 20.0
        }
    }

    fn theme_tab_advance_width(&self, label: &str) -> f32 {
        if self.renaming_theme.as_deref() == Some(label) {
            self.theme_rename_input_width() + 4.0
        } else {
            label_width(label, 13.0) + 24.0
        }
    }

    fn variant_header_rect(&self, rect: Rect, idx: usize) -> Rect {
        let variants = self.variant_column_labels();
        let col_w = variant_column_width(rect, variants.len());
        Rect {
            origin: Point2D::new(
                value_column_x(rect) + col_w * idx as f32 - 6.0,
                rect.origin.y + HEADER_HEIGHT + 4.0,
            ),
            size: Point2D::new(col_w.min(176.0), 30.0),
        }
    }

    fn theme_menu_rect(&self, rect: Rect, axis: &str) -> Rect {
        let idx = self
            .theme_tab_labels()
            .iter()
            .position(|label| *label == axis)
            .unwrap_or(0);
        let anchor = self.theme_tab_rect(rect, idx);
        Rect {
            origin: Point2D::new(anchor.origin.x, rect.origin.y + HEADER_HEIGHT + 4.0),
            size: Point2D::new(176.0, menu_rows_height(self.theme_tab_labels().len())),
        }
    }

    fn variant_menu_rect(&self, rect: Rect, value: &str) -> Rect {
        let variants = self.variant_column_labels();
        let idx = variants
            .iter()
            .position(|label| *label == value)
            .unwrap_or(0);
        let anchor = self.variant_header_rect(rect, idx);
        Rect {
            origin: Point2D::new(
                anchor.origin.x + 6.0,
                rect.origin.y + HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + 4.0,
            ),
            size: Point2D::new(176.0, menu_rows_height(variants.len())),
        }
    }

    pub fn name_caret_for_row(&self, idx: usize) -> Option<usize> {
        let input = self.name_input_for_row(idx)?;
        input
            .caret_visible(self.now_ms)
            .then(|| input.caret().min(input.text().len()))
    }

    fn name_input_for_row(&self, idx: usize) -> Option<&jian_core::text_input::TextInputState> {
        (self.editing_name_row == Some(idx)).then_some(&self.row_input)
    }

    fn rename_text_input(
        &self,
        target: RenameTarget<'_>,
    ) -> Option<&jian_core::text_input::TextInputState> {
        let is_active = match target {
            RenameTarget::Theme(axis) => self.renaming_theme.as_deref() == Some(axis),
            RenameTarget::Variant(value) => self.renaming_variant.as_deref() == Some(value),
        };
        is_active.then_some(&self.header_input)
    }

    pub fn value_caret_for_cell(&self, row: usize, variant: usize) -> Option<usize> {
        let input = self.value_input_for_cell(row, variant)?;
        input
            .caret_visible(self.now_ms)
            .then(|| input.caret().min(input.text().len()))
    }

    fn value_input_for_cell(
        &self,
        row: usize,
        variant: usize,
    ) -> Option<&jian_core::text_input::TextInputState> {
        (self.editing_value_cell == Some((row, variant))).then_some(&self.row_input)
    }

    /// Name pill of the row at DISPLAY position `display_idx`.
    fn name_cell_rect_at(&self, rect: Rect, display_idx: usize) -> Rect {
        let y = self.row_y(rect, display_idx);
        Rect {
            origin: Point2D::new(rect.origin.x + PAD_X + 28.0, y + 7.0),
            size: Point2D::new(NAME_COLUMN_WIDTH - 36.0, 30.0),
        }
    }

    /// Value cell of the row at DISPLAY position `display_idx`.
    fn value_cell_rect_at(
        &self,
        rect: Rect,
        display_idx: usize,
        variant: usize,
        variant_count: usize,
    ) -> Rect {
        let width = variant_column_width(rect, variant_count);
        Rect {
            origin: Point2D::new(
                value_column_x(rect) + width * variant as f32,
                self.row_y(rect, display_idx),
            ),
            size: Point2D::new(width, ROW_HEIGHT),
        }
    }

    fn labels(&self) -> VariablePanelLabels {
        let t = |key| crate::i18n::translate(self.locale, key);
        VariablePanelLabels {
            preset: t("variables.presets"),
            name: t("common.name"),
            empty: t("variables.noDefined"),
            add_variable: t("variables.addVariable"),
            color: t("variables.typeColor"),
            number: t("variables.typeNumber"),
            string: t("variables.typeString"),
            save_preset: t("variables.savePreset"),
            no_presets: t("variables.noPresets"),
            import: t("variables.importPreset"),
            export: t("variables.exportPreset"),
            rename: t("common.rename"),
            delete: t("common.delete"),
            search_placeholder: t("variables.searchVariables"),
            no_match: t("variables.noMatch"),
        }
    }
}

enum RenameTarget<'a> {
    Theme(&'a str),
    Variant(&'a str),
}

struct VariablePanelLabels {
    preset: &'static str,
    name: &'static str,
    empty: &'static str,
    add_variable: &'static str,
    color: &'static str,
    number: &'static str,
    string: &'static str,
    save_preset: &'static str,
    no_presets: &'static str,
    import: &'static str,
    export: &'static str,
    rename: &'static str,
    delete: &'static str,
    search_placeholder: &'static str,
    no_match: &'static str,
}

impl Widget for VariablesPanel {
    fn id(&self) -> WidgetId {
        // Stable id reserved at the host level; the table itself
        // has no id so we pick a constant in the chrome-widget
        // range. Mirrors `AlignToolbar::id`.
        WidgetId::new(0xA0_0007)
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("Variables");
        node
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, self.intrinsic_height()),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        paint::paint_panel(self, cx, rect);
    }
}

#[cfg(test)]
mod tests;
