//! Floating variables manager. This is the explicit `{}` toolbar surface:
//! a large modal-style panel for managing presets, theme axes, and document
//! variables. The compact `VariablesPanel` remains the automatic right-rail
//! fallback for documents that already contain variables.

mod presets;
#[cfg(test)]
mod tests;

pub use presets::{PresetMenuHit, ThemePresetMenu};

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId, TOP_BAR_HEIGHT};
use crate::{Color, Point2D, Rect, TextLayout};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::editor_ui_state::Locale;
use op_editor_core::{EditorState, VariablesPanelButton};

pub const VARIABLES_MODAL_MAX_W: f32 = 1510.0;
pub const VARIABLES_MODAL_MAX_H: f32 = 880.0;
pub const VARIABLES_MODAL_MIN_W: f32 = 720.0;
pub const VARIABLES_MODAL_MIN_H: f32 = 480.0;

const RADIUS: f32 = 18.0;
const HEADER_H: f32 = 78.0;
const AXIS_HEADER_H: f32 = 64.0;
const FOOTER_H: f32 = 68.0;
const PAD_X: f32 = 28.0;
const ROW_H: f32 = 44.0;
const ICON_BUTTON: f32 = 36.0;
const FOOTER_BUTTON_W: f32 = 164.0;
const FOOTER_BUTTON_H: f32 = 36.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariablesModalHit {
    Close,
    AddVariable,
    PresetMenu,
    HeaderAdd,
    Row(usize),
    AxisChip(usize),
    AxisDropdownItem {
        axis: String,
        value: String,
    },
    /// A row inside the open theme-preset dropdown (#20).
    Preset(PresetMenuHit),
    Inside,
    Outside,
}

#[derive(Debug, Clone)]
struct ModalRow {
    name: String,
    kind: VariableKind,
    resolved: Option<VariableScalar>,
}

#[derive(Debug, Clone)]
struct AxisChip {
    axis: String,
    value: String,
}

pub struct VariablesModal {
    pub id: WidgetId,
    theme: Theme,
    locale: Locale,
    rows: Vec<ModalRow>,
    chips: Vec<AxisChip>,
    themes: Vec<(String, Vec<String>)>,
    dropdown_open: Option<String>,
    hover: Option<VariablesPanelButton>,
    /// The open theme-preset dropdown, anchored under the header's
    /// preset button (#20). `None` while closed.
    preset_menu: Option<ThemePresetMenu>,
}

impl VariablesModal {
    pub fn for_editor(state: &EditorState) -> Self {
        let rows = state
            .doc
            .variables
            .as_ref()
            .map(|vars| {
                vars.iter()
                    .map(|(name, def)| ModalRow {
                        name: name.clone(),
                        kind: def.kind.clone(),
                        resolved: state.resolve_variable(name).cloned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let chips = state
            .ui
            .variables
            .active_theme
            .iter()
            .map(|(axis, value)| AxisChip {
                axis: axis.clone(),
                value: value.clone(),
            })
            .collect();
        let themes = state
            .doc
            .themes
            .as_ref()
            .map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        Self {
            id: WidgetId::new(5600),
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.locale,
            rows,
            chips,
            themes,
            dropdown_open: state.editor_ui.axis_dropdown_open.clone(),
            hover: state.editor_ui.variables_panel_hover,
            preset_menu: ThemePresetMenu::is_open(state)
                .then(|| ThemePresetMenu::for_editor(state)),
        }
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        let usable_w = (viewport_w - 376.0).max(VARIABLES_MODAL_MIN_W);
        let usable_h = (viewport_h - TOP_BAR_HEIGHT - 110.0).max(VARIABLES_MODAL_MIN_H);
        let w = usable_w.min(VARIABLES_MODAL_MAX_W);
        let h = usable_h.min(VARIABLES_MODAL_MAX_H);
        let x = ((viewport_w - w) / 2.0).max(16.0);
        let y = TOP_BAR_HEIGHT + ((viewport_h - TOP_BAR_HEIGHT - h) / 2.0).clamp(24.0, 72.0);
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn hover_at(&self, rect: Rect, point: Point2D) -> Option<VariablesPanelButton> {
        use VariablesPanelButton as B;
        match self.hit_test(rect, point) {
            VariablesModalHit::Close => Some(B::Close),
            VariablesModalHit::AddVariable => Some(B::AddVariable),
            VariablesModalHit::PresetMenu => Some(B::PresetMenu),
            VariablesModalHit::HeaderAdd => Some(B::HeaderAdd),
            VariablesModalHit::Row(i) => Some(B::Row(i)),
            VariablesModalHit::AxisChip(i) => Some(B::AxisChip(i)),
            VariablesModalHit::AxisDropdownItem { axis, value } => self
                .axis_values(&axis)
                .and_then(|vals| vals.iter().position(|v| *v == value))
                .map(B::DropdownItem),
            VariablesModalHit::Preset(_)
            | VariablesModalHit::Inside
            | VariablesModalHit::Outside => None,
        }
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> VariablesModalHit {
        // The preset dropdown overlays everything else in the modal —
        // test it first so its rows win over the header underneath.
        if let Some(menu) = &self.preset_menu {
            if let Some(hit) = menu.hit_test(menu.menu_rect(preset_button_rect(rect)), point) {
                return VariablesModalHit::Preset(hit);
            }
        }
        if !contains(rect, point) {
            return VariablesModalHit::Outside;
        }
        if let Some(open_axis) = self.dropdown_open.as_deref() {
            if let Some((chip_idx, _)) = self
                .chips
                .iter()
                .enumerate()
                .find(|(_, c)| c.axis == open_axis)
            {
                if let Some(values) = self.axis_values(open_axis) {
                    let chip = axis_chip_rect(rect, chip_idx);
                    let menu = Rect {
                        origin: Point2D::new(chip.origin.x, chip.origin.y + chip.size.y + 6.0),
                        size: Point2D::new(chip.size.x.max(160.0), values.len() as f32 * ROW_H),
                    };
                    if contains(menu, point) {
                        let idx = ((point.y - menu.origin.y) / ROW_H).floor() as usize;
                        if let Some(value) = values.get(idx) {
                            return VariablesModalHit::AxisDropdownItem {
                                axis: open_axis.to_string(),
                                value: value.clone(),
                            };
                        }
                    }
                }
            }
        }
        if contains(close_rect(rect), point) {
            return VariablesModalHit::Close;
        }
        if contains(header_add_rect(rect), point) {
            return VariablesModalHit::HeaderAdd;
        }
        if contains(preset_button_rect(rect), point) {
            return VariablesModalHit::PresetMenu;
        }
        if contains(footer_add_rect(rect), point) {
            return VariablesModalHit::AddVariable;
        }
        for idx in 0..self.chips.len() {
            if contains(axis_chip_rect(rect, idx), point) {
                return VariablesModalHit::AxisChip(idx);
            }
        }
        if let Some(idx) = self.row_index_at(rect, point) {
            return VariablesModalHit::Row(idx);
        }
        VariablesModalHit::Inside
    }

    fn row_index_at(&self, rect: Rect, point: Point2D) -> Option<usize> {
        let body = body_rect(rect);
        if !contains(body, point) {
            return None;
        }
        let idx = ((point.y - body.origin.y) / ROW_H).floor() as usize;
        (idx < self.rows.len()).then_some(idx)
    }

    fn axis_values(&self, axis: &str) -> Option<&[String]> {
        self.themes
            .iter()
            .find(|(name, _)| name == axis)
            .map(|(_, values)| values.as_slice())
    }
}

impl Widget for VariablesModal {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(VARIABLES_MODAL_MAX_W, VARIABLES_MODAL_MAX_H),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, RADIUS, self.theme.card);
        cx.backend
            .stroke_round_rect(rect, RADIUS, self.theme.border, 1.0);
        paint_header(cx, self.theme, self.locale, self.hover, rect);
        paint_axis_header(cx, self.theme, self.locale, self.hover, rect, &self.chips);
        paint_body(cx, self.theme, self.locale, self.hover, rect, &self.rows);
        paint_footer(cx, self.theme, self.locale, self.hover, rect);
        self.paint_dropdown(cx, rect);
        // Preset dropdown paints last — top-most modal overlay.
        if let Some(menu) = &self.preset_menu {
            menu.paint(cx, menu.menu_rect(preset_button_rect(rect)));
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(op_i18n::translate(self.locale, "toolbar.variables"));
        node
    }
}

impl VariablesModal {
    fn paint_dropdown(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let Some(axis) = self.dropdown_open.as_deref() else {
            return;
        };
        let Some((idx, _)) = self.chips.iter().enumerate().find(|(_, c)| c.axis == axis) else {
            return;
        };
        let Some(values) = self.axis_values(axis) else {
            return;
        };
        let chip = axis_chip_rect(rect, idx);
        let menu = Rect {
            origin: Point2D::new(chip.origin.x, chip.origin.y + chip.size.y + 6.0),
            size: Point2D::new(chip.size.x.max(160.0), values.len() as f32 * ROW_H),
        };
        cx.backend.fill_round_rect(menu, 8.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(menu, 8.0, self.theme.border, 1.0);
        for (value_idx, value) in values.iter().enumerate() {
            let row = Rect {
                origin: Point2D::new(menu.origin.x, menu.origin.y + value_idx as f32 * ROW_H),
                size: Point2D::new(menu.size.x, ROW_H),
            };
            if self.hover == Some(VariablesPanelButton::DropdownItem(value_idx)) {
                cx.backend
                    .fill_round_rect(inset(row, 4.0, 3.0), 6.0, self.theme.button_hover);
            }
            draw_text(
                cx,
                value,
                13.0,
                self.theme.foreground,
                row.origin.x + 14.0,
                row.origin.y + 27.0,
            );
        }
    }
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    locale: Locale,
    hover: Option<VariablesPanelButton>,
    rect: Rect,
) {
    let plus = header_add_rect(rect);
    icon_button(
        cx,
        theme,
        plus,
        Icon::Plus,
        hover == Some(VariablesPanelButton::HeaderAdd),
    );

    let preset = preset_button_rect(rect);
    if hover == Some(VariablesPanelButton::PresetMenu) {
        cx.backend.fill_round_rect(preset, 8.0, theme.button_hover);
    }
    let preset_label = op_i18n::translate(locale, "variables.presets");
    let preset_label_size = 15.0;
    let preset_label_x = preset.origin.x + 28.0;
    draw_icon(
        cx.backend,
        Icon::Save,
        Point2D::new(preset.origin.x, preset.origin.y + 8.0),
        18.0,
        theme.muted_foreground,
        1.6,
    );
    draw_text(
        cx,
        preset_label,
        preset_label_size,
        theme.foreground,
        preset_label_x,
        preset.origin.y + 23.0,
    );
    let preset_chevron_x = (preset_label_x + label_width(preset_label, preset_label_size) + 7.0)
        .min(preset.origin.x + preset.size.x - 22.0);
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(preset_chevron_x, preset.origin.y + 8.0),
        18.0,
        theme.muted_foreground,
        1.6,
    );

    let close = close_rect(rect);
    icon_button(
        cx,
        theme,
        close,
        Icon::Close,
        hover == Some(VariablesPanelButton::Close),
    );

    divider(cx, theme, rect.origin.y + HEADER_H, rect);
}

fn paint_axis_header(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    locale: Locale,
    hover: Option<VariablesPanelButton>,
    rect: Rect,
    chips: &[AxisChip],
) {
    let y = rect.origin.y + HEADER_H;
    draw_text(
        cx,
        op_i18n::translate(locale, "common.name"),
        15.0,
        theme.muted_foreground,
        rect.origin.x + PAD_X,
        y + 39.0,
    );

    let default_x = rect.origin.x + rect.size.x * 0.34;
    if chips.is_empty() {
        draw_text(
            cx,
            "Default",
            15.0,
            theme.muted_foreground,
            default_x,
            y + 39.0,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(default_x + 82.0, y + 24.0),
            16.0,
            theme.muted_foreground,
            1.5,
        );
    } else {
        for (idx, chip_info) in chips.iter().enumerate() {
            let chip = axis_chip_rect(rect, idx);
            let hovered = hover == Some(VariablesPanelButton::AxisChip(idx));
            cx.backend.fill_round_rect(
                chip,
                8.0,
                if hovered {
                    theme.button_hover
                } else {
                    theme.muted
                },
            );
            cx.backend.stroke_round_rect(chip, 8.0, theme.border, 1.0);
            let label = format!("{}: {}", chip_info.axis, chip_info.value);
            draw_text(
                cx,
                &label,
                13.0,
                theme.foreground,
                chip.origin.x + 12.0,
                chip.origin.y + 23.0,
            );
            draw_icon(
                cx.backend,
                Icon::ChevronDown,
                Point2D::new(chip.origin.x + chip.size.x - 24.0, chip.origin.y + 9.0),
                14.0,
                theme.muted_foreground,
                1.4,
            );
        }
    }

    let plus = header_column_add_rect(rect);
    if hover == Some(VariablesPanelButton::HeaderAdd) {
        cx.backend.fill_round_rect(plus, 8.0, theme.button_hover);
    }
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(plus.origin.x + 8.0, plus.origin.y + 8.0),
        18.0,
        theme.muted_foreground,
        1.6,
    );
    divider(cx, theme, y + AXIS_HEADER_H, rect);
}

fn paint_body(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    locale: Locale,
    hover: Option<VariablesPanelButton>,
    rect: Rect,
    rows: &[ModalRow],
) {
    let body = body_rect(rect);
    cx.backend.save();
    cx.backend.clip_rect(body);
    if rows.is_empty() {
        let label = op_i18n::translate(locale, "variables.noDefined");
        let w = cx.backend.measure_text(label, 14.0);
        draw_text(
            cx,
            label,
            14.0,
            theme.muted_foreground,
            body.origin.x + (body.size.x - w) / 2.0,
            body.origin.y + body.size.y / 2.0,
        );
        cx.backend.restore();
        return;
    }
    for (idx, row) in rows.iter().enumerate() {
        let row_rect = Rect {
            origin: Point2D::new(body.origin.x, body.origin.y + idx as f32 * ROW_H),
            size: Point2D::new(body.size.x, ROW_H),
        };
        if hover == Some(VariablesPanelButton::Row(idx)) {
            cx.backend
                .fill_round_rect(inset(row_rect, 12.0, 4.0), 8.0, theme.button_hover);
        }
        draw_text(
            cx,
            &row.name,
            14.0,
            theme.foreground,
            row_rect.origin.x + PAD_X,
            row_rect.origin.y + 28.0,
        );
        draw_variable_value(cx, theme, row, row_rect);
        divider(cx, theme, row_rect.origin.y + ROW_H, rect);
    }
    cx.backend.restore();
}

fn draw_variable_value(cx: &mut PaintCx<'_>, theme: Theme, row: &ModalRow, row_rect: Rect) {
    let x = row_rect.origin.x + row_rect.size.x * 0.34;
    match (&row.kind, &row.resolved) {
        (VariableKind::Color, Some(VariableScalar::Str(hex))) => {
            let swatch = Rect {
                origin: Point2D::new(x, row_rect.origin.y + 12.0),
                size: Point2D::new(20.0, 20.0),
            };
            cx.backend
                .fill_round_rect(swatch, 5.0, parse_hex(hex).unwrap_or(theme.muted));
            cx.backend.stroke_round_rect(swatch, 5.0, theme.border, 1.0);
            draw_text(
                cx,
                hex,
                13.0,
                theme.muted_foreground,
                x + 32.0,
                row_rect.origin.y + 28.0,
            );
        }
        (_, Some(value)) => draw_text(
            cx,
            &scalar_label(value),
            13.0,
            theme.muted_foreground,
            x,
            row_rect.origin.y + 28.0,
        ),
        _ => draw_text(
            cx,
            "-",
            13.0,
            theme.muted_foreground,
            x,
            row_rect.origin.y + 28.0,
        ),
    }
}

fn paint_footer(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    locale: Locale,
    hover: Option<VariablesPanelButton>,
    rect: Rect,
) {
    let footer_top = rect.origin.y + rect.size.y - FOOTER_H;
    divider(cx, theme, footer_top, rect);
    let btn = footer_add_rect(rect);
    if hover == Some(VariablesPanelButton::AddVariable) {
        cx.backend.fill_round_rect(btn, 8.0, theme.button_hover);
    }
    let add_label = op_i18n::translate(locale, "variables.addVariable");
    let add_label_size = 15.0;
    let add_label_x = btn.origin.x + 28.0;
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(btn.origin.x, btn.origin.y + 9.0),
        18.0,
        theme.muted_foreground,
        1.6,
    );
    draw_text(
        cx,
        add_label,
        add_label_size,
        theme.muted_foreground,
        add_label_x,
        btn.origin.y + 25.0,
    );
    let add_chevron_x = (add_label_x + label_width(add_label, add_label_size) + 7.0)
        .min(btn.origin.x + btn.size.x - 24.0);
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(add_chevron_x, btn.origin.y + 9.0),
        18.0,
        theme.muted_foreground,
        1.6,
    );
}

fn icon_button(cx: &mut PaintCx<'_>, theme: Theme, rect: Rect, icon: Icon, hovered: bool) {
    if hovered {
        cx.backend.fill_round_rect(rect, 8.0, theme.button_hover);
    }
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 8.0, rect.origin.y + 8.0),
        20.0,
        if hovered {
            theme.foreground
        } else {
            theme.muted_foreground
        },
        1.8,
    );
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        crate::widgets::property_panel_inputs::to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}

fn label_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.58
}

fn divider(cx: &mut PaintCx<'_>, theme: Theme, y: f32, rect: Rect) {
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(rect.origin.x, y),
            size: Point2D::new(rect.size.x, 1.0),
        },
        theme.border,
    );
}

fn close_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - PAD_X - ICON_BUTTON,
            rect.origin.y + (HEADER_H - ICON_BUTTON) / 2.0,
        ),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    }
}

fn header_add_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + PAD_X - 8.0, rect.origin.y + 21.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    }
}

fn header_column_add_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - PAD_X - ICON_BUTTON,
            rect.origin.y + HEADER_H + (AXIS_HEADER_H - ICON_BUTTON) / 2.0,
        ),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    }
}

fn preset_button_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + PAD_X + 60.0, rect.origin.y + 22.0),
        size: Point2D::new(142.0, 34.0),
    }
}

fn footer_add_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + PAD_X, rect.origin.y + rect.size.y - 52.0),
        size: Point2D::new(FOOTER_BUTTON_W, FOOTER_BUTTON_H),
    }
}

fn body_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + HEADER_H + AXIS_HEADER_H),
        size: Point2D::new(
            rect.size.x,
            rect.size.y - HEADER_H - AXIS_HEADER_H - FOOTER_H,
        ),
    }
}

fn axis_chip_rect(rect: Rect, idx: usize) -> Rect {
    let x = rect.origin.x + rect.size.x * 0.34 + idx as f32 * 180.0;
    Rect {
        origin: Point2D::new(x, rect.origin.y + HEADER_H + 15.0),
        size: Point2D::new(164.0, 34.0),
    }
}

fn inset(rect: Rect, x: f32, y: f32) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + x, rect.origin.y + y),
        size: Point2D::new(
            (rect.size.x - x * 2.0).max(0.0),
            (rect.size.y - y * 2.0).max(0.0),
        ),
    }
}

fn contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn scalar_label(value: &VariableScalar) -> String {
    match value {
        VariableScalar::Bool(v) => v.to_string(),
        VariableScalar::Num(v) => {
            let mut s = format!("{v}");
            if s.ends_with(".0") {
                s.truncate(s.len() - 2);
            }
            s
        }
        VariableScalar::Str(v) => v.clone(),
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    let s = hex.strip_prefix('#')?;
    let (r, g, b, a) = match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        6 | 8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = if s.len() == 8 {
                u8::from_str_radix(&s[6..8], 16).ok()?
            } else {
                255
            };
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(rgba(r, g, b, a as f32 / 255.0))
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}
