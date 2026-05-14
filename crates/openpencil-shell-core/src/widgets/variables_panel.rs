//! Variables panel — lists every variable in `Document.var_table`
//! plus the currently active theme axes. Mirrors the TS app's
//! Variables panel in the right rail (under the Themes header).
//!
//! v1 scope (this commit):
//!   - Header row showing each active theme axis as a small chip
//!     (`mode: dark`, `density: compact`, etc).
//!   - One row per variable: name on the left, a small preview on
//!     the right (resolved color swatch for `VariableKind::Color`;
//!     stringified scalar for other kinds).
//!   - Click hit-test stub returning `VariablesPanelHit::Row(idx)`
//!     so the host can wire row clicks to the color picker / edit
//!     modal in a follow-up.
//!
//! Editing inputs (add / rename / delete a variable, toggle a theme
//! axis value) are deferred to a second commit once the active-theme
//! flow is exercised via a real `.op` file with themes.

use crate::document::{
    Document, ThemeAxis, Variable, VariableKind, VariableScalar, VariableTable, VariableValue,
};
use crate::theme::Theme;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};

const ROW_HEIGHT: f32 = 32.0;
const HEADER_HEIGHT: f32 = 28.0;
const CHIP_HEIGHT: f32 = 20.0;
const PAD_X: f32 = 12.0;
const SWATCH_SIZE: f32 = 18.0;
const DROPDOWN_WIDTH: f32 = 140.0;
const DROPDOWN_ROW_HEIGHT: f32 = 24.0;
/// Width of the Variables rail when it docks alongside the layer
/// panel. Matches the LAYER_PANEL_WIDTH default so the chrome reads
/// symmetrically; the host can resize via the existing panel-resize
/// gutter logic.
pub const VARIABLES_PANEL_WIDTH: f32 = 240.0;

/// Hit kinds for `VariablesPanel::hit_test`. Row index is into the
/// `variables` slice the panel was built from, so callers can map
/// straight back to the source [`Variable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariablesPanelHit {
    /// Click on a variable row.
    Row(usize),
    /// Click on a theme-axis chip in the header. Host toggles
    /// `Document.ui.axis_dropdown_open` for that axis name.
    AxisChip(usize),
    /// Click on a value row inside an open axis dropdown.
    /// Carries the axis name + the picked value so the host
    /// can pin `var_table.active_theme[axis] = value`. The host
    /// is also responsible for clearing `axis_dropdown_open`.
    AxisDropdownItem { axis: String, value: String },
}

/// View model for the Variables panel. Borrows from the document
/// (cheap; immutable across paint).
pub struct VariablesPanel<'a> {
    pub table: &'a VariableTable,
    /// Cached snapshot of axis chips painted in the header row.
    /// Stored so `hit_test` doesn't need to re-walk the table.
    chips: Vec<AxisChip>,
    /// If `Some(axis_name)` AND the axis matches one of `chips`,
    /// paint a dropdown overlay anchored to that chip. The list
    /// is sourced from `VariableTable::themes`.
    dropdown_open: Option<String>,
    /// Row index currently in inline-edit focus (Number / String
    /// variable). `None` = no row editing.
    editing_row: Option<usize>,
    /// Draft buffer for the row in edit focus. Shared with
    /// `Document.ui.property_input_draft` (reuses the caret +
    /// blink machinery).
    editing_draft: String,
}

#[derive(Debug, Clone)]
struct AxisChip {
    axis: String,
    value: String,
}

impl<'a> VariablesPanel<'a> {
    pub fn for_document(doc: &'a Document) -> Self {
        let mut panel = Self::for_table(&doc.var_table);
        panel.dropdown_open = doc.ui.axis_dropdown_open.clone();
        panel.editing_row = doc.ui.variable_row_focus.map(|f| match f {
            crate::document::VariableRowFocus::Number(i) => i,
            crate::document::VariableRowFocus::String(i) => i,
        });
        panel.editing_draft = doc.ui.property_input_draft.clone();
        panel
    }

    pub fn for_table(table: &'a VariableTable) -> Self {
        let chips: Vec<AxisChip> = table
            .active_theme
            .iter()
            .map(|(axis, value)| AxisChip {
                axis: axis.clone(),
                value: value.clone(),
            })
            .collect();
        Self {
            table,
            chips,
            dropdown_open: None,
            editing_row: None,
            editing_draft: String::new(),
        }
    }

    /// Number of variable rows the panel paints.
    pub fn row_count(&self) -> usize {
        self.table.variables.len()
    }

    /// Number of axis chips in the header. May be zero — a document
    /// without themes shows only the variable list.
    pub fn axis_count(&self) -> usize {
        self.chips.len()
    }

    /// Total height (header + chips row + variable rows). Used by
    /// the right-rail host when computing layout.
    pub fn intrinsic_height(&self) -> f32 {
        let chip_row = if self.chips.is_empty() {
            0.0
        } else {
            CHIP_HEIGHT + 8.0
        };
        HEADER_HEIGHT + chip_row + (self.row_count() as f32) * ROW_HEIGHT
    }

    /// Anchor rect of the chip at index `i` within `rect`. Mirrors
    /// the paint walk in `paint` so hit-test + dropdown anchoring
    /// stay aligned without re-measuring.
    fn chip_rect(&self, rect: Rect, idx: usize) -> Rect {
        let mut x = rect.origin.x + PAD_X;
        for (i, chip) in self.chips.iter().enumerate() {
            let w = chip_width(chip);
            if i == idx {
                return Rect {
                    origin: Point2D::new(x, rect.origin.y + HEADER_HEIGHT),
                    size: Point2D::new(w, CHIP_HEIGHT),
                };
            }
            x += w + 6.0;
        }
        Rect {
            origin: Point2D::new(rect.origin.x + PAD_X, rect.origin.y + HEADER_HEIGHT),
            size: Point2D::new(0.0, CHIP_HEIGHT),
        }
    }

    /// Map a screen-space point to the row or chip it falls in.
    /// Honors `dropdown_open` first so a click on a value row of
    /// the open dropdown overlay wins over the chip / row beneath.
    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<VariablesPanelHit> {
        if !rect_contains(rect, point) {
            return None;
        }
        let mut y = rect.origin.y + HEADER_HEIGHT;
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
                if let Some(values) = self
                    .table
                    .themes
                    .iter()
                    .find(|t| t.name == open_axis)
                    .map(|t| t.values.as_slice())
                {
                    let chip_rect = self.chip_rect(rect, chip_idx);
                    let menu_y_start = chip_rect.origin.y + CHIP_HEIGHT + 4.0;
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
        // Chip row.
        if !self.chips.is_empty() {
            let chip_y = y;
            if point.y >= chip_y && point.y < chip_y + CHIP_HEIGHT {
                let mut x = rect.origin.x + PAD_X;
                for (i, chip) in self.chips.iter().enumerate() {
                    let w = chip_width(chip);
                    if point.x >= x && point.x < x + w {
                        return Some(VariablesPanelHit::AxisChip(i));
                    }
                    x += w + 6.0;
                }
            }
            y += CHIP_HEIGHT + 8.0;
        }
        // Variable rows.
        let idx = ((point.y - y) / ROW_HEIGHT).floor();
        if idx >= 0.0 {
            let i = idx as usize;
            if i < self.row_count() {
                return Some(VariablesPanelHit::Row(i));
            }
        }
        None
    }
}

fn chip_width(chip: &AxisChip) -> f32 {
    // Approximate at 7 px per visible char + 16 px chrome. Real
    // measure_text would land once the panel is hosted; for now the
    // hit-test budget is generous enough that off-by-one is fine.
    let label_len = chip.axis.len() + 2 + chip.value.len(); // "axis: value"
    (label_len as f32) * 7.0 + 16.0
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x < r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y < r.origin.y + r.size.y
}

impl<'a> Widget for VariablesPanel<'a> {
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
        let theme = Theme::dark();
        // Background — `card` is the closest token to "right-rail
        // panel surface"; theme has no dedicated `panel` field today.
        cx.backend.fill_rect(rect, theme.card);
        // Section label.
        let label_layout = crate::TextLayout::single_run(
            "Variables",
            "system-ui",
            13.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label_layout,
            Point2D::new(rect.origin.x + PAD_X, rect.origin.y + 20.0),
        );
        let mut y = rect.origin.y + HEADER_HEIGHT;
        // Active theme chips.
        if !self.chips.is_empty() {
            let mut x = rect.origin.x + PAD_X;
            for chip in &self.chips {
                let w = chip_width(chip);
                let chip_rect = Rect {
                    origin: Point2D::new(x, y),
                    size: Point2D::new(w, CHIP_HEIGHT),
                };
                cx.backend.fill_round_rect(chip_rect, 4.0, theme.muted);
                let label = format!("{}: {}", chip.axis, chip.value);
                let layout = crate::TextLayout::single_run(
                    &label,
                    "system-ui",
                    11.0,
                    to_jian_color(theme.muted_foreground),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &layout,
                    Point2D::new(chip_rect.origin.x + 6.0, chip_rect.origin.y + 14.0),
                );
                x += w + 6.0;
            }
            y += CHIP_HEIGHT + 8.0;
        }
        // Variable rows.
        for (idx, var) in self.table.variables.iter().enumerate() {
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, ROW_HEIGHT),
            };
            // Name on the left.
            let name_layout = crate::TextLayout::single_run(
                &var.name,
                "system-ui",
                12.0,
                to_jian_color(theme.foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &name_layout,
                Point2D::new(row.origin.x + PAD_X, row.origin.y + 19.0),
            );
            // Preview on the right.
            let preview_x = row.origin.x + row.size.x - PAD_X - SWATCH_SIZE;
            if self.editing_row == Some(idx) {
                // Inline edit mode — paint the draft + a thin
                // underline to signal active focus. Aligned with
                // the same baseline the preview label would have
                // used. No caret blink today (the property panel's
                // caret machinery isn't yet shared); the underline
                // is enough affordance.
                let draft_layout = crate::TextLayout::single_run(
                    &self.editing_draft,
                    "system-ui",
                    11.0,
                    to_jian_color(theme.foreground),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend
                    .draw_text(&draft_layout, Point2D::new(preview_x - 64.0, row.origin.y + 21.0));
                let underline = Rect {
                    origin: Point2D::new(preview_x - 70.0, row.origin.y + 23.0),
                    size: Point2D::new(80.0, 1.0),
                };
                cx.backend.fill_rect(underline, theme.foreground);
            } else {
                paint_preview(cx, &theme, var, self.table, preview_x, row.origin.y + 7.0);
            }
            y += ROW_HEIGHT;
        }
        // Axis dropdown overlay — paints LAST so it covers the
        // chip row + variable rows beneath. Anchored under the
        // chip whose axis matches `dropdown_open`.
        if let Some(open_axis) = self.dropdown_open.as_deref() {
            if let Some((chip_idx, _)) = self
                .chips
                .iter()
                .enumerate()
                .find(|(_, c)| c.axis == open_axis)
            {
                if let Some(values) = self
                    .table
                    .themes
                    .iter()
                    .find(|t| t.name == open_axis)
                    .map(|t| t.values.as_slice())
                {
                    let chip_rect = self.chip_rect(rect, chip_idx);
                    let menu_y = chip_rect.origin.y + CHIP_HEIGHT + 4.0;
                    let menu_rect = Rect {
                        origin: Point2D::new(chip_rect.origin.x, menu_y),
                        size: Point2D::new(
                            DROPDOWN_WIDTH,
                            DROPDOWN_ROW_HEIGHT * (values.len() as f32),
                        ),
                    };
                    cx.backend.fill_round_rect(menu_rect, 6.0, theme.popover);
                    cx.backend
                        .stroke_round_rect(menu_rect, 6.0, theme.border, 1.0);
                    let active_value = self
                        .table
                        .active_theme
                        .get(open_axis)
                        .cloned()
                        .unwrap_or_default();
                    for (i, v) in values.iter().enumerate() {
                        let row_y = menu_y + (i as f32) * DROPDOWN_ROW_HEIGHT;
                        let is_active = *v == active_value;
                        if is_active {
                            let highlight = Rect {
                                origin: Point2D::new(menu_rect.origin.x + 2.0, row_y),
                                size: Point2D::new(menu_rect.size.x - 4.0, DROPDOWN_ROW_HEIGHT),
                            };
                            cx.backend.fill_round_rect(highlight, 4.0, theme.muted);
                        }
                        let label = crate::TextLayout::single_run(
                            v,
                            "system-ui",
                            11.0,
                            to_jian_color(theme.foreground),
                            Point2D::new(0.0, 0.0),
                        );
                        cx.backend.draw_text(
                            &label,
                            Point2D::new(menu_rect.origin.x + 10.0, row_y + 16.0),
                        );
                    }
                }
            }
        }
    }
}

fn paint_preview(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    var: &Variable,
    table: &VariableTable,
    x: f32,
    y: f32,
) {
    match var.kind {
        VariableKind::Color => {
            let rgba = table
                .resolve_color(&var.name)
                .unwrap_or(Color::WHITE);
            let swatch = Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
            };
            cx.backend.fill_round_rect(swatch, 3.0, rgba);
            cx.backend
                .stroke_round_rect(swatch, 3.0, theme.border, 1.0);
        }
        _ => {
            // Non-color: render the resolved scalar as a short text
            // label. Falls back to "—" when the variable doesn't
            // resolve under the active theme.
            let text = match table.resolve(&var.name) {
                Some(s) => scalar_to_label(s),
                None => "—".into(),
            };
            // Truncate long labels so they don't overflow.
            let display = truncate(&text, 12);
            let layout = crate::TextLayout::single_run(
                &display,
                "system-ui",
                11.0,
                to_jian_color(theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&layout, Point2D::new(x - 24.0, y + 14.0));
        }
    }
}

fn scalar_to_label(s: &VariableScalar) -> String {
    match s {
        VariableScalar::Str(s) => s.clone(),
        VariableScalar::Num(n) => format!("{n}"),
        VariableScalar::Bool(b) => if *b { "true" } else { "false" }.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
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
    use crate::document::{VariableKind, VariableScalar, VariableValue};
    use std::collections::BTreeMap;

    fn table_with_three_vars() -> VariableTable {
        let mut t = VariableTable::default();
        t.variables.push(Variable {
            name: "color-1".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff8800".into())),
        });
        t.variables.push(Variable {
            name: "spacing-md".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(16.0)),
        });
        t.variables.push(Variable {
            name: "is-dark".into(),
            kind: VariableKind::Boolean,
            value: VariableValue::Scalar(VariableScalar::Bool(true)),
        });
        t.active_theme.insert("mode".into(), "dark".into());
        t
    }

    #[test]
    fn row_count_matches_variable_count() {
        let t = table_with_three_vars();
        let p = VariablesPanel::for_table(&t);
        assert_eq!(p.row_count(), 3);
    }

    #[test]
    fn axis_count_reflects_active_theme() {
        let t = table_with_three_vars();
        let p = VariablesPanel::for_table(&t);
        assert_eq!(p.axis_count(), 1);
    }

    #[test]
    fn intrinsic_height_grows_with_rows_and_chips() {
        let mut t = VariableTable::default();
        let p = VariablesPanel::for_table(&t);
        let empty_h = p.intrinsic_height();
        assert!((empty_h - HEADER_HEIGHT).abs() < f32::EPSILON);
        t.variables.push(Variable {
            name: "x".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(1.0)),
        });
        let p2 = VariablesPanel::for_table(&t);
        assert!(p2.intrinsic_height() > empty_h);
    }

    #[test]
    fn axis_dropdown_hit_routes_to_named_value() {
        let mut t = table_with_three_vars();
        t.themes.push(crate::document::ThemeAxis {
            name: "mode".into(),
            values: vec!["light".into(), "dark".into(), "system".into()],
        });
        let mut p = VariablesPanel::for_table(&t);
        p.dropdown_open = Some("mode".into());
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        // Hit row 0 of the dropdown (== "light"). Chip anchor is
        // (PAD_X, HEADER_HEIGHT); dropdown starts CHIP_HEIGHT+4
        // below it.
        let menu_y = HEADER_HEIGHT + CHIP_HEIGHT + 4.0;
        let click_y = menu_y + DROPDOWN_ROW_HEIGHT * 0.5;
        let click_x = PAD_X + 10.0; // inside DROPDOWN_WIDTH
        match p.hit_test(rect, Point2D::new(click_x, click_y)) {
            Some(VariablesPanelHit::AxisDropdownItem { axis, value }) => {
                assert_eq!(axis, "mode");
                assert_eq!(value, "light");
            }
            other => panic!("expected AxisDropdownItem for row 0, got {other:?}"),
        }
        // Row 2 = "system".
        let click_y_sys = menu_y + DROPDOWN_ROW_HEIGHT * 2.5;
        match p.hit_test(rect, Point2D::new(click_x, click_y_sys)) {
            Some(VariablesPanelHit::AxisDropdownItem { axis, value }) => {
                assert_eq!(axis, "mode");
                assert_eq!(value, "system");
            }
            other => panic!("expected AxisDropdownItem for row 2, got {other:?}"),
        }
    }

    #[test]
    fn hit_test_returns_row_index_for_in_row_click() {
        let t = table_with_three_vars();
        let p = VariablesPanel::for_table(&t);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        // y for row 1 = HEADER + chip row + row_height * 1 + half.
        let chip_block = CHIP_HEIGHT + 8.0;
        let y = HEADER_HEIGHT + chip_block + ROW_HEIGHT * 1.0 + ROW_HEIGHT / 2.0;
        match p.hit_test(rect, Point2D::new(100.0, y)) {
            Some(VariablesPanelHit::Row(1)) => {}
            other => panic!("expected Row(1), got {other:?}"),
        }
    }

    #[test]
    fn hit_test_returns_axis_chip_for_chip_click() {
        let t = table_with_three_vars();
        let p = VariablesPanel::for_table(&t);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        // First chip starts at (PAD_X, HEADER_HEIGHT).
        let y = HEADER_HEIGHT + CHIP_HEIGHT / 2.0;
        match p.hit_test(rect, Point2D::new(PAD_X + 4.0, y)) {
            Some(VariablesPanelHit::AxisChip(0)) => {}
            other => panic!("expected AxisChip(0), got {other:?}"),
        }
    }

    #[test]
    fn hit_test_returns_none_outside_rect() {
        let t = table_with_three_vars();
        let p = VariablesPanel::for_table(&t);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, 200.0),
        };
        assert!(p.hit_test(rect, Point2D::new(-10.0, 50.0)).is_none());
        assert!(p.hit_test(rect, Point2D::new(50.0, 1000.0)).is_none());
    }

    #[test]
    fn axis_chip_table_mirrors_active_theme_btree_order() {
        let mut t = VariableTable::default();
        t.active_theme.insert("z-axis".into(), "alpha".into());
        t.active_theme.insert("a-axis".into(), "omega".into());
        let p = VariablesPanel::for_table(&t);
        // BTreeMap iterates in key order — a-axis first.
        assert_eq!(p.chips.len(), 2);
        assert_eq!(p.chips[0].axis, "a-axis");
        assert_eq!(p.chips[1].axis, "z-axis");
    }
}

// Suppress unused-import warning until tests are split into a
// sibling file. Test mod uses `BTreeMap` indirectly via the
// VariableTable default; keeping the import keeps that obvious.
#[allow(dead_code)]
fn _ensure_themed_value_path_compiles() {
    // Reference these types so `cargo check --lib` warns when they
    // move (the panel will need them once edit inputs land).
    let _t: Option<ThemeAxis> = None;
    let _v: Option<VariableValue> = None;
}
