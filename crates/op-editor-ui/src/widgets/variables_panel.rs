//! Variables panel — lists every variable in the document plus the
//! currently active theme axes. Mirrors the TS app's Variables panel
//! in the right rail (under the Themes header).
//!
//! v1 scope:
//!   - Header row showing each active theme axis as a small chip
//!     (`mode: dark`, `density: compact`, etc).
//!   - One row per variable: name on the left, a small preview on
//!     the right (resolved color swatch for `VariableKind::Color`;
//!     stringified scalar for other kinds).
//!   - Click hit-test returning `VariablesPanelHit::Row(idx)` /
//!     `AxisChip(idx)` / `AxisDropdownItem` so the host can wire
//!     row clicks to the color picker / theme switch.
//!
//! ## State source
//!
//! The panel reads the canonical document model on `EditorState`:
//!   - persisted variables — `doc.variables`
//!     (`Option<BTreeMap<String, VariableDefinition>>`)
//!   - persisted theme axes — `doc.themes`
//!     (`Option<BTreeMap<String, Vec<String>>>`, axis → value list)
//!   - transient active-theme selection —
//!     `ui.variables.active_theme` (`BTreeMap<String, String>`)
//!
//! Construction snapshots that state into owned value rows / chips so
//! paint + hit-test never re-walk the document.

use crate::theme::Theme;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_core::EditorState;

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
/// `rows` slice the panel was built from, so callers can map straight
/// back to the source variable name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariablesPanelHit {
    /// Click on a variable row.
    Row(usize),
    /// Click on a theme-axis chip in the header. Host toggles
    /// `EditorUiState.axis_dropdown_open` for that axis name.
    AxisChip(usize),
    /// Click on a value row inside an open axis dropdown.
    /// Carries the axis name + the picked value so the host can pin
    /// `ui.variables.active_theme[axis] = value`. The host is also
    /// responsible for clearing `axis_dropdown_open`.
    AxisDropdownItem { axis: String, value: String },
}

/// One variable row snapshot — owned so paint + hit-test never touch
/// the document after construction.
#[derive(Debug, Clone)]
struct VarRow {
    name: String,
    kind: VariableKind,
    /// Resolved scalar under the active theme; `None` when the
    /// variable has an empty themed list.
    resolved: Option<VariableScalar>,
}

#[derive(Debug, Clone)]
struct AxisChip {
    axis: String,
    value: String,
}

/// View model for the Variables panel. Holds owned snapshots derived
/// from `EditorState` at construction time.
pub struct VariablesPanel {
    rows: Vec<VarRow>,
    /// Axis chips painted in the header row.
    chips: Vec<AxisChip>,
    /// Axis → ordered value list, sourced from `doc.themes`.
    themes: Vec<(String, Vec<String>)>,
    /// If `Some(axis_name)` AND the axis matches one of `chips`,
    /// paint a dropdown overlay anchored to that chip.
    dropdown_open: Option<String>,
    /// Row index currently in inline-edit focus (Number / String
    /// variable). `None` = no row editing.
    editing_row: Option<usize>,
    /// Draft buffer for the row in edit focus.
    editing_draft: String,
    /// Which target the cursor is over — drives the hover wash.
    hover: Option<op_editor_core::VariablesPanelButton>,
}

impl VariablesPanel {
    pub fn for_editor(state: &EditorState) -> Self {
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
                        resolved: state.resolve_variable(name).cloned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Active-theme chips.
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
        let themes: Vec<(String, Vec<String>)> = state
            .doc
            .themes
            .as_ref()
            .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        Self {
            rows,
            chips,
            themes,
            dropdown_open: state.editor_ui.axis_dropdown_open.clone(),
            editing_row: state.editor_ui.variable_row_focus.map(|f| match f {
                VariableRowFocus::Number(i) => i,
                VariableRowFocus::String(i) => i,
            }),
            editing_draft: state.ui.property_input_draft.clone(),
            hover: state.editor_ui.variables_panel_hover,
        }
    }

    /// Number of variable rows the panel paints.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Resolve a pointer to a hoverable target (row / axis chip /
    /// dropdown item). Mirrors [`Self::hit_test`] but returns
    /// index-only [`op_editor_core::VariablesPanelButton`] for the wash.
    pub fn hover_at(
        &self,
        rect: Rect,
        point: Point2D,
    ) -> Option<op_editor_core::VariablesPanelButton> {
        use op_editor_core::VariablesPanelButton as B;
        match self.hit_test(rect, point)? {
            VariablesPanelHit::Row(i) => Some(B::Row(i)),
            VariablesPanelHit::AxisChip(i) => Some(B::AxisChip(i)),
            VariablesPanelHit::AxisDropdownItem { axis, value } => {
                // Recover the value's index within the open axis list so
                // the wash needs no owned strings.
                self.axis_values(&axis)
                    .and_then(|vals| vals.iter().position(|v| *v == value))
                    .map(B::DropdownItem)
            }
        }
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
                if let Some(values) = self.axis_values(open_axis) {
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
            for (chip_idx, chip) in self.chips.iter().enumerate() {
                let w = chip_width(chip);
                let chip_rect = Rect {
                    origin: Point2D::new(x, y),
                    size: Point2D::new(w, CHIP_HEIGHT),
                };
                cx.backend.fill_round_rect(chip_rect, 4.0, theme.muted);
                if self.hover == Some(op_editor_core::VariablesPanelButton::AxisChip(chip_idx)) {
                    cx.backend
                        .fill_round_rect(chip_rect, 4.0, theme.button_hover);
                }
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
        for (idx, var) in self.rows.iter().enumerate() {
            let row = Rect {
                origin: Point2D::new(rect.origin.x, y),
                size: Point2D::new(rect.size.x, ROW_HEIGHT),
            };
            // Hover wash on the row under the cursor.
            if self.hover == Some(op_editor_core::VariablesPanelButton::Row(idx)) {
                cx.backend.fill_rect(row, theme.button_hover);
            }
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
                // underline to signal active focus.
                let draft_layout = crate::TextLayout::single_run(
                    &self.editing_draft,
                    "system-ui",
                    11.0,
                    to_jian_color(theme.foreground),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &draft_layout,
                    Point2D::new(preview_x - 64.0, row.origin.y + 21.0),
                );
                let underline = Rect {
                    origin: Point2D::new(preview_x - 70.0, row.origin.y + 23.0),
                    size: Point2D::new(80.0, 1.0),
                };
                cx.backend.fill_rect(underline, theme.foreground);
            } else {
                paint_preview(cx, &theme, var, preview_x, row.origin.y + 7.0);
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
                if let Some(values) = self.axis_values(open_axis) {
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
                        .chips
                        .iter()
                        .find(|c| c.axis == open_axis)
                        .map(|c| c.value.clone())
                        .unwrap_or_default();
                    for (i, v) in values.iter().enumerate() {
                        let row_y = menu_y + (i as f32) * DROPDOWN_ROW_HEIGHT;
                        let is_active = *v == active_value;
                        let item_rect = Rect {
                            origin: Point2D::new(menu_rect.origin.x + 2.0, row_y),
                            size: Point2D::new(menu_rect.size.x - 4.0, DROPDOWN_ROW_HEIGHT),
                        };
                        if is_active {
                            cx.backend.fill_round_rect(item_rect, 4.0, theme.muted);
                        }
                        if self.hover == Some(op_editor_core::VariablesPanelButton::DropdownItem(i))
                        {
                            cx.backend
                                .fill_round_rect(item_rect, 4.0, theme.button_hover);
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

fn paint_preview(cx: &mut PaintCx<'_>, theme: &Theme, var: &VarRow, x: f32, y: f32) {
    match var.kind {
        VariableKind::Color => {
            let rgba = var
                .resolved
                .as_ref()
                .and_then(scalar_as_color)
                .unwrap_or(Color::WHITE);
            let swatch = Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
            };
            cx.backend.fill_round_rect(swatch, 3.0, rgba);
            cx.backend.stroke_round_rect(swatch, 3.0, theme.border, 1.0);
        }
        _ => {
            // Non-color: render the resolved scalar as a short text
            // label. Falls back to "—" when the variable doesn't
            // resolve under the active theme.
            let text = match var.resolved.as_ref() {
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

/// Parse a `Str` scalar as an `#rrggbb` colour swatch.
fn scalar_as_color(s: &VariableScalar) -> Option<Color> {
    let hex = match s {
        VariableScalar::Str(hex) => hex,
        _ => return None,
    };
    let (r, g, b) = op_editor_core::color_picker::parse_hex_rgb(hex)?;
    Some(Color { r, g, b, a: 1.0 })
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
    use jian_ops_schema::variable::VariableScalar;

    fn state_with_three_vars() -> EditorState {
        let mut s = EditorState::new();
        s.create_variable(
            "color-1",
            VariableKind::Color,
            VariableScalar::Str("#ff8800".into()),
        );
        s.create_variable(
            "spacing-md",
            VariableKind::Number,
            VariableScalar::Num(16.0),
        );
        s.create_variable("is-dark", VariableKind::Boolean, VariableScalar::Bool(true));
        s.ui.variables
            .active_theme
            .insert("mode".into(), "dark".into());
        s
    }

    #[test]
    fn row_count_matches_variable_count() {
        let s = state_with_three_vars();
        let p = VariablesPanel::for_editor(&s);
        assert_eq!(p.row_count(), 3);
    }

    #[test]
    fn axis_count_reflects_active_theme() {
        let s = state_with_three_vars();
        let p = VariablesPanel::for_editor(&s);
        assert_eq!(p.axis_count(), 1);
    }

    #[test]
    fn intrinsic_height_grows_with_rows_and_chips() {
        let s_empty = EditorState::new();
        let p = VariablesPanel::for_editor(&s_empty);
        let empty_h = p.intrinsic_height();
        assert!((empty_h - HEADER_HEIGHT).abs() < f32::EPSILON);
        let s = state_with_three_vars();
        let p2 = VariablesPanel::for_editor(&s);
        assert!(p2.intrinsic_height() > empty_h);
    }

    #[test]
    fn axis_dropdown_hit_routes_to_named_value() {
        let mut s = state_with_three_vars();
        s.doc.themes.get_or_insert_with(Default::default).insert(
            "mode".into(),
            vec!["light".into(), "dark".into(), "system".into()],
        );
        let mut p = VariablesPanel::for_editor(&s);
        p.dropdown_open = Some("mode".into());
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        let menu_y = HEADER_HEIGHT + CHIP_HEIGHT + 4.0;
        let click_y = menu_y + DROPDOWN_ROW_HEIGHT * 0.5;
        let click_x = PAD_X + 10.0;
        match p.hit_test(rect, Point2D::new(click_x, click_y)) {
            Some(VariablesPanelHit::AxisDropdownItem { axis, value }) => {
                assert_eq!(axis, "mode");
                assert_eq!(value, "light");
            }
            other => panic!("expected AxisDropdownItem for row 0, got {other:?}"),
        }
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
        let s = state_with_three_vars();
        let p = VariablesPanel::for_editor(&s);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        let chip_block = CHIP_HEIGHT + 8.0;
        let y = HEADER_HEIGHT + chip_block + ROW_HEIGHT * 1.0 + ROW_HEIGHT / 2.0;
        match p.hit_test(rect, Point2D::new(100.0, y)) {
            Some(VariablesPanelHit::Row(1)) => {}
            other => panic!("expected Row(1), got {other:?}"),
        }
    }

    #[test]
    fn hit_test_returns_axis_chip_for_chip_click() {
        let s = state_with_three_vars();
        let p = VariablesPanel::for_editor(&s);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
        };
        let y = HEADER_HEIGHT + CHIP_HEIGHT / 2.0;
        match p.hit_test(rect, Point2D::new(PAD_X + 4.0, y)) {
            Some(VariablesPanelHit::AxisChip(0)) => {}
            other => panic!("expected AxisChip(0), got {other:?}"),
        }
    }

    #[test]
    fn hit_test_returns_none_outside_rect() {
        let s = state_with_three_vars();
        let p = VariablesPanel::for_editor(&s);
        let rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(VARIABLES_PANEL_WIDTH, 200.0),
        };
        assert!(p.hit_test(rect, Point2D::new(-10.0, 50.0)).is_none());
        assert!(p.hit_test(rect, Point2D::new(50.0, 1000.0)).is_none());
    }

    #[test]
    fn axis_chip_table_mirrors_active_theme_btree_order() {
        let mut s = EditorState::new();
        s.ui.variables
            .active_theme
            .insert("z-axis".into(), "alpha".into());
        s.ui.variables
            .active_theme
            .insert("a-axis".into(), "omega".into());
        let p = VariablesPanel::for_editor(&s);
        // BTreeMap iterates in key order — a-axis first.
        assert_eq!(p.chips.len(), 2);
        assert_eq!(p.chips[0].axis, "a-axis");
        assert_eq!(p.chips[1].axis, "z-axis");
    }
}
