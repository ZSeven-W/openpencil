use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_core::{EditorState, Locale};

mod header;
mod paint;

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
pub const VARIABLES_PANEL_WIDTH: f32 = 820.0;

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
    NameCell(usize),
    ValueCell { row: usize, variant: usize },
    Row(usize),
    AxisChip(usize),
    AxisDropdownItem { axis: String, value: String },
}

#[derive(Debug, Clone)]
struct VarRow {
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
    editing_name_row: Option<usize>,
    editing_value_cell: Option<(usize, usize)>,
    editing_draft: String,
    caret_pos: usize,
    caret_anchor_ms: u64,
    now_ms: u64,
}

impl VariablesPanel {
    pub fn for_editor(state: &EditorState) -> Self {
        Self::for_editor_at(state, 0)
    }

    pub fn for_editor_at(state: &EditorState, now_ms: u64) -> Self {
        // Variable rows — keyed by BTreeMap order so paint is stable.
        let rows: Vec<VarRow> = state
            .doc
            .variables
            .as_ref()
            .map(|vars| {
                vars.iter()
                    .map(|(name, def)| VarRow {
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
        Self {
            rows,
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
            editing_name_row: state.editor_ui.variable_row_focus.and_then(|f| match f {
                VariableRowFocus::Name(i) => Some(i),
                VariableRowFocus::Number(_)
                | VariableRowFocus::String(_)
                | VariableRowFocus::NumberCell { .. }
                | VariableRowFocus::StringCell { .. } => None,
            }),
            editing_value_cell: state.editor_ui.variable_row_focus.and_then(|f| match f {
                VariableRowFocus::Number(i) | VariableRowFocus::String(i) => Some((i, 0)),
                VariableRowFocus::NumberCell { row, variant }
                | VariableRowFocus::StringCell { row, variant } => Some((row, variant)),
                VariableRowFocus::Name(_) => None,
            }),
            editing_draft: state.ui.property_input_draft.clone(),
            caret_pos: state.ui.property_caret_pos,
            caret_anchor_ms: state.ui.property_caret_anchor_ms,
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
            + FOOTER_HEIGHT
            + (self.row_count() as f32) * ROW_HEIGHT
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
        (label_width(&self.editing_draft, 13.0) + 28.0).max(96.0)
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
        if self.editing_name_row == Some(idx)
            && jian_core::anim::blink_visible(self.now_ms, self.caret_anchor_ms, 500)
        {
            Some(self.caret_pos.min(self.editing_draft.len()))
        } else {
            None
        }
    }

    fn rename_text_caret(&self, target: RenameTarget<'_>) -> Option<usize> {
        let is_active = match target {
            RenameTarget::Theme(axis) => self.renaming_theme.as_deref() == Some(axis),
            RenameTarget::Variant(value) => self.renaming_variant.as_deref() == Some(value),
        };
        if is_active && jian_core::anim::blink_visible(self.now_ms, self.caret_anchor_ms, 500) {
            Some(self.caret_pos.min(self.editing_draft.len()))
        } else {
            None
        }
    }

    pub fn value_caret_for_cell(&self, row: usize, variant: usize) -> Option<usize> {
        if self.editing_value_cell == Some((row, variant))
            && jian_core::anim::blink_visible(self.now_ms, self.caret_anchor_ms, 500)
        {
            Some(self.caret_pos.min(self.editing_draft.len()))
        } else {
            None
        }
    }

    /// Map a screen-space point to the row or chip it falls in.
    /// Honors `dropdown_open` first so a click on a value row of
    /// the open dropdown overlay wins over the chip / row beneath.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<VariablesPanelHit> {
        if !rect_contains(rect, point) {
            return None;
        }
        if rect_contains(close_rect(rect), point) {
            return Some(VariablesPanelHit::Close);
        }
        if let Some(axis) = self.theme_menu_open.as_deref() {
            if let Some(hit) = theme_menu_hit(
                self.theme_menu_rect(rect, axis),
                point,
                axis,
                self.theme_tab_labels().len(),
            ) {
                return Some(hit);
            }
        }
        if let Some(value) = self.variant_menu_open.as_deref() {
            if let Some(hit) = variant_menu_hit(
                self.variant_menu_rect(rect, value),
                point,
                value,
                self.variant_column_labels().len(),
            ) {
                return Some(hit);
            }
        }
        for (idx, axis) in self.theme_tab_labels().iter().enumerate() {
            if rect_contains(self.theme_tab_rect(rect, idx), point) {
                if *axis == self.active_axis_label() {
                    return Some(VariablesPanelHit::ToggleThemeMenu((*axis).to_string()));
                }
                return Some(VariablesPanelHit::ThemeTab((*axis).to_string()));
            }
        }
        if self.add_menu_open {
            if let Some(hit) = add_variable_menu_hit(rect, point) {
                return Some(hit);
            }
        }
        if self.preset_menu_open && rect_contains(self.preset_menu_rect(rect), point) {
            return Some(VariablesPanelHit::TogglePresetMenu);
        }
        if rect_contains(self.add_theme_rect(rect), point) {
            return Some(VariablesPanelHit::AddTheme);
        }
        if rect_contains(self.preset_rect(rect), point) {
            return Some(VariablesPanelHit::TogglePresetMenu);
        }
        if rect_contains(add_variant_rect(rect), point) {
            return Some(VariablesPanelHit::AddVariant);
        }
        for (idx, value) in self.variant_column_labels().iter().enumerate() {
            if rect_contains(self.variant_header_rect(rect, idx), point) {
                return Some(VariablesPanelHit::ToggleVariantMenu((*value).to_string()));
            }
        }
        if rect_contains(add_variable_rect(rect), point) {
            return Some(VariablesPanelHit::ToggleAddVariableMenu);
        }
        // Dropdown overlay — top-most. Paints when the host
        // marked an axis open AND that axis is one of the
        // active-theme chips.
        if let Some(open_axis) = self.dropdown_open.as_deref() {
            if let Some((chip_idx, _chip)) = self
                .chips
                .iter()
                .enumerate()
                .find(|(_, c)| c.axis == open_axis)
            {
                if let Some(values) = self.axis_values(open_axis) {
                    let chip_rect = self.chip_rect(rect, chip_idx);
                    let menu_y_start = chip_rect.origin.y + chip_rect.size.y + 4.0;
                    let menu_rect = Rect {
                        origin: Point2D::new(chip_rect.origin.x, menu_y_start),
                        size: Point2D::new(
                            DROPDOWN_WIDTH,
                            DROPDOWN_ROW_HEIGHT * (values.len() as f32),
                        ),
                    };
                    if rect_contains(menu_rect, point) {
                        let row = ((point.y - menu_y_start) / DROPDOWN_ROW_HEIGHT).floor();
                        if row >= 0.0 {
                            let r = row as usize;
                            if r < values.len() {
                                return Some(VariablesPanelHit::AxisDropdownItem {
                                    axis: open_axis.to_string(),
                                    value: values[r].clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        // Theme value chip in the column header.
        if !self.chips.is_empty() {
            for (i, _chip) in self.chips.iter().enumerate() {
                if rect_contains(self.chip_rect(rect, i), point) {
                    return Some(VariablesPanelHit::AxisChip(i));
                }
            }
        }
        // Variable rows.
        let y = rows_start_y(rect);
        let footer_y = rect.origin.y + rect.size.y - FOOTER_HEIGHT;
        if point.y >= footer_y {
            return None;
        }
        let idx = ((point.y - y) / ROW_HEIGHT).floor();
        if idx >= 0.0 {
            let i = idx as usize;
            if i < self.row_count() {
                if rect_contains(name_cell_rect(rect, i), point) {
                    return Some(VariablesPanelHit::NameCell(i));
                }
                let variant_count = self.variant_column_labels().len().max(1);
                for variant in 0..variant_count {
                    if rect_contains(value_cell_rect(rect, i, variant, variant_count), point) {
                        return Some(VariablesPanelHit::ValueCell { row: i, variant });
                    }
                }
                return Some(VariablesPanelHit::Row(i));
            }
        }
        None
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
}

fn close_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 42.0, rect.origin.y + 9.0),
        size: Point2D::new(26.0, 26.0),
    }
}

fn add_variant_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - PAD_X - 28.0,
            rect.origin.y + HEADER_HEIGHT + 4.0,
        ),
        size: Point2D::new(28.0, 28.0),
    }
}

fn add_variable_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            rect.origin.x + 8.0,
            rect.origin.y + rect.size.y - FOOTER_HEIGHT + 5.0,
        ),
        size: Point2D::new(164.0, 30.0),
    }
}

fn add_variable_menu_rect(rect: Rect) -> Rect {
    let height = ADD_VARIABLE_MENU_ROW_HEIGHT * ADD_VARIABLE_MENU_ROWS;
    Rect {
        origin: Point2D::new(
            rect.origin.x + PAD_X,
            rect.origin.y + rect.size.y - FOOTER_HEIGHT - height - 6.0,
        ),
        size: Point2D::new(ADD_VARIABLE_MENU_WIDTH, height),
    }
}

fn add_variable_menu_hit(rect: Rect, point: Point2D) -> Option<VariablesPanelHit> {
    let menu = add_variable_menu_rect(rect);
    if !rect_contains(menu, point) {
        return None;
    }
    let row = ((point.y - menu.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::AddVariableColor),
        1 => Some(VariablesPanelHit::AddVariableNumber),
        2 => Some(VariablesPanelHit::AddVariableString),
        _ => None,
    }
}

fn menu_rows_height(sibling_count: usize) -> f32 {
    let rows = if sibling_count > 1 { 2.0 } else { 1.0 };
    ADD_VARIABLE_MENU_ROW_HEIGHT * rows
}

fn theme_menu_hit(
    rect: Rect,
    point: Point2D,
    axis: &str,
    theme_count: usize,
) -> Option<VariablesPanelHit> {
    if !rect_contains(rect, point) {
        return None;
    }
    let row = ((point.y - rect.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::ThemeMenuRename(axis.to_string())),
        1 if theme_count > 1 => Some(VariablesPanelHit::ThemeMenuDelete(axis.to_string())),
        _ => None,
    }
}

fn variant_menu_hit(
    rect: Rect,
    point: Point2D,
    value: &str,
    variant_count: usize,
) -> Option<VariablesPanelHit> {
    if !rect_contains(rect, point) {
        return None;
    }
    let row = ((point.y - rect.origin.y) / ADD_VARIABLE_MENU_ROW_HEIGHT).floor() as usize;
    match row {
        0 => Some(VariablesPanelHit::VariantMenuRename(value.to_string())),
        1 if variant_count > 1 => Some(VariablesPanelHit::VariantMenuDelete(value.to_string())),
        _ => None,
    }
}

fn rows_start_y(rect: Rect) -> f32 {
    rect.origin.y + HEADER_HEIGHT + COLUMN_HEADER_HEIGHT
}

fn name_cell_rect(rect: Rect, idx: usize) -> Rect {
    let y = rows_start_y(rect) + ROW_HEIGHT * idx as f32;
    Rect {
        origin: Point2D::new(rect.origin.x + PAD_X + 28.0, y + 7.0),
        size: Point2D::new(NAME_COLUMN_WIDTH - 36.0, 30.0),
    }
}

fn value_column_x(rect: Rect) -> f32 {
    rect.origin.x + PAD_X + NAME_COLUMN_WIDTH
}

fn value_cell_rect(rect: Rect, row: usize, variant: usize, variant_count: usize) -> Rect {
    let width = variant_column_width(rect, variant_count);
    Rect {
        origin: Point2D::new(
            value_column_x(rect) + width * variant as f32,
            rows_start_y(rect) + ROW_HEIGHT * row as f32,
        ),
        size: Point2D::new(width, ROW_HEIGHT),
    }
}

fn variant_column_width(rect: Rect, count: usize) -> f32 {
    let count = count.max(1) as f32;
    let available = rect.size.x - PAD_X * 2.0 - NAME_COLUMN_WIDTH - ACTION_COLUMN_WIDTH;
    (available / count).max(156.0)
}

fn label_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x < r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y < r.origin.y + r.size.y
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
