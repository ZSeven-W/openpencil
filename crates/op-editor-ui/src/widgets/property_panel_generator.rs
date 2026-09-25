//! Generator section of the property panel.
//!
//! Shown when the selection is a generator frame (see
//! `op_editor_core::generator`): one editor per declared parameter —
//! a text field for number / text / color, a checkbox for boolean — plus
//! Regenerate and Detach. A two-line note states the ownership rule
//! (generated children are replaced on regenerate) and, when relevant, a
//! status line reports the last failure, hand-edited children, or a host
//! that cannot run programs.

use op_editor_core::generator::{
    generator_spec_of, installed_generator_runner, GeneratorParamKind,
};
use op_editor_core::{EditorState, NodeId, PropertyFocus};

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{NodeSnapshot, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused_state, paint_section_divider, paint_section_label,
    INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::{text_metrics, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};

const NOTE_LINE: f32 = 14.0;
const NOTE_HEIGHT: f32 = NOTE_LINE * 2.0 + 4.0;
const LABEL_HEIGHT: f32 = 16.0;
const ROW_GAP: f32 = 6.0;
const TOGGLE_HEIGHT: f32 = 24.0;
const BUTTON_HEIGHT: f32 = 28.0;
const NOTE_FONT: f32 = 11.0;

/// One parameter row as the panel shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorParamRow {
    pub label: String,
    pub kind: GeneratorParamKind,
    /// Display string for text fields; `"true"` / `"false"` for booleans.
    pub value: String,
}

/// Status line under the ownership note, most urgent first.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorStatus {
    /// The last run failed — its message.
    Failed(String),
    /// Children differ from the last generated output.
    Edited,
    /// This host has no generator runtime.
    ReadOnly,
}

/// Generator data the section paints, built from the selected node.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorSummary {
    pub node_id: NodeId,
    pub params: Vec<GeneratorParamRow>,
    pub status: Option<GeneratorStatus>,
}

/// The Copy-able row shape the layout walkers thread through
/// `VisibleSections` so paint and hit-testing agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorRows {
    pub count: usize,
    /// Bit `i` set when parameter `i` is a boolean (checkbox row).
    pub bool_mask: u32,
    pub status: bool,
}

impl GeneratorSummary {
    pub fn rows(&self) -> GeneratorRows {
        let mut bool_mask = 0u32;
        for (index, row) in self.params.iter().enumerate().take(32) {
            if row.kind == GeneratorParamKind::Boolean {
                bool_mask |= 1 << index;
            }
        }
        GeneratorRows {
            count: self.params.len().min(32),
            bool_mask,
            status: self.status.is_some(),
        }
    }
}

impl GeneratorRows {
    fn is_bool(&self, index: usize) -> bool {
        self.bool_mask & (1 << index) != 0
    }

    fn row_height(&self, index: usize) -> f32 {
        if self.is_bool(index) {
            TOGGLE_HEIGHT + ROW_GAP
        } else {
            LABEL_HEIGHT + INPUT_HEIGHT + ROW_GAP
        }
    }

    fn params_top(&self, y: f32) -> f32 {
        let mut y = y + SECTION_HEADER_HEIGHT + NOTE_HEIGHT + ROW_GAP;
        if self.status {
            y += NOTE_HEIGHT + ROW_GAP;
        }
        y
    }
}

/// Build the section summary for the selected `node`, or `None` when it
/// is not a generator.
pub fn summary_for(
    state: &EditorState,
    node: &jian_ops_schema::node::PenNode,
) -> Option<GeneratorSummary> {
    use op_editor_core::PenNodeExt;
    let parsed = generator_spec_of(node)?;
    let node_id = NodeId::new_opt(node.id_str())?;
    let params = parsed
        .as_ref()
        .map(|spec| {
            spec.params
                .iter()
                .map(|param| GeneratorParamRow {
                    label: param.display_label().to_string(),
                    kind: param.kind,
                    value: param.display_value(),
                })
                .collect()
        })
        .unwrap_or_default();
    let parked = state
        .ui
        .generator_error
        .as_ref()
        .filter(|error| error.node_id == node_id)
        .map(|error| GeneratorStatus::Failed(error.message.clone()));
    let status = match parsed {
        Err(error) => Some(GeneratorStatus::Failed(error.to_string())),
        Ok(_) => parked
            .or_else(|| {
                state
                    .generator_children_edited(&node_id)
                    .then_some(GeneratorStatus::Edited)
            })
            .or_else(|| {
                installed_generator_runner()
                    .is_none()
                    .then_some(GeneratorStatus::ReadOnly)
            }),
    };
    Some(GeneratorSummary {
        node_id,
        params,
        status,
    })
}

/// Height consumed by the section, including divider and trailing gap.
pub fn generator_section_height(rows: GeneratorRows) -> f32 {
    let mut y = rows.params_top(0.0);
    for index in 0..rows.count {
        y += rows.row_height(index);
    }
    y + BUTTON_HEIGHT + 12.0 + 1.0 + SECTION_GAP
}

/// Push one text-field rect per non-boolean parameter.
pub fn push_generator_input_rects(
    inputs: &mut Vec<(PropertyFocus, Rect)>,
    rows: GeneratorRows,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let mut y = rows.params_top(y);
    for index in 0..rows.count {
        if !rows.is_bool(index) {
            inputs.push((
                PropertyFocus::GeneratorParam(index),
                Rect::xywh(x + PAD_X, y + LABEL_HEIGHT, usable_w, INPUT_HEIGHT),
            ));
        }
        y += rows.row_height(index);
    }
}

/// Push the checkbox rects plus the Regenerate / Detach buttons.
pub fn push_generator_action_rects(
    actions: &mut Vec<(PropertyPanelAction, Rect)>,
    rows: GeneratorRows,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let mut y = rows.params_top(y);
    for index in 0..rows.count {
        if rows.is_bool(index) {
            actions.push((
                PropertyPanelAction::ToggleGeneratorParam(index),
                Rect::xywh(x + PAD_X, y, usable_w, TOGGLE_HEIGHT),
            ));
        }
        y += rows.row_height(index);
    }
    let (regenerate, detach) = button_rects(x, y, usable_w);
    actions.push((PropertyPanelAction::RegenerateGenerator, regenerate));
    actions.push((PropertyPanelAction::DetachGenerator, detach));
}

fn button_rects(x: f32, y: f32, usable_w: f32) -> (Rect, Rect) {
    let half = (usable_w - 8.0) / 2.0;
    (
        Rect::xywh(x + PAD_X, y, half, BUTTON_HEIGHT),
        Rect::xywh(x + PAD_X + half + 8.0, y, half, BUTTON_HEIGHT),
    )
}

/// Paint the section; returns the y below it.
#[allow(clippy::too_many_arguments)]
pub fn paint_generator_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let Some(summary) = snapshot.generator.as_ref() else {
        return y;
    };
    let rows = summary.rows();
    let t = |key| op_i18n::translate(locale, key);
    let usable_w = width - PAD_X * 2.0;
    let mut y = paint_section_label(cx, theme, t("generator.title"), x, y, width);
    paint_note(
        cx,
        t("generator.ownership"),
        theme.muted_foreground,
        x + PAD_X,
        y,
        usable_w,
    );
    y += NOTE_HEIGHT + ROW_GAP;
    if let Some(status) = &summary.status {
        let (text, color) = match status {
            GeneratorStatus::Failed(message) => (
                op_i18n::interpolate(t("generator.failed"), &[("error", message.as_str())]),
                theme.destructive,
            ),
            GeneratorStatus::Edited => (t("generator.edited").to_string(), theme.primary),
            GeneratorStatus::ReadOnly => {
                (t("generator.readOnly").to_string(), theme.muted_foreground)
            }
        };
        paint_note(cx, &text, color, x + PAD_X, y, usable_w);
        y += NOTE_HEIGHT + ROW_GAP;
    }
    for (index, row) in summary.params.iter().enumerate().take(rows.count) {
        if rows.is_bool(index) {
            paint_toggle(
                cx,
                theme,
                Rect::xywh(x + PAD_X, y, usable_w, TOGGLE_HEIGHT),
                &row.label,
                row.value == "true",
            );
        } else {
            paint_line(
                cx,
                &row.label,
                theme.muted_foreground,
                x + PAD_X,
                y + 12.0,
                usable_w,
            );
            let focus = PropertyFocus::GeneratorParam(index);
            paint_input_with_prefix_focused_state(
                cx,
                theme,
                Rect::xywh(x + PAD_X, y + LABEL_HEIGHT, usable_w, INPUT_HEIGHT),
                kind_prefix(row.kind),
                edit.value_for(focus, &row.value),
                edit.focus == Some(focus),
                edit.caret_at(focus),
                edit.select_all_at(focus),
                edit.input_at(focus),
                edit.now_ms,
            );
        }
        y += rows.row_height(index);
    }
    let (regenerate, detach) = button_rects(x, y, usable_w);
    paint_button(
        cx,
        theme,
        regenerate,
        Icon::RefreshCw,
        t("generator.regenerate"),
    );
    paint_button(cx, theme, detach, Icon::Unlink, t("generator.detach"));
    y += BUTTON_HEIGHT + 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + 1.0 + SECTION_GAP
}

fn kind_prefix(kind: GeneratorParamKind) -> &'static str {
    match kind {
        GeneratorParamKind::Number => "#",
        GeneratorParamKind::Text => "T",
        GeneratorParamKind::Color => "◐",
        GeneratorParamKind::Boolean => "",
    }
}

fn paint_line(cx: &mut PaintCx<'_>, text: &str, color: Color, x: f32, baseline: f32, w: f32) {
    let text = text_metrics::fit_chrome(cx.backend, text, w, NOTE_FONT);
    let layout = TextLayout::single_run(
        &text,
        "system-ui",
        NOTE_FONT,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline));
}

/// Word-wrap `text` into at most two lines of width `w`; the second line
/// is ellipsized.
fn paint_note(cx: &mut PaintCx<'_>, text: &str, color: Color, x: f32, y: f32, w: f32) {
    let mut first = String::new();
    let mut rest = text;
    for (index, _) in text
        .match_indices(' ')
        .chain(std::iter::once((text.len(), "")))
    {
        let candidate = &text[..index];
        if text_metrics::measure_chrome(cx.backend, candidate, NOTE_FONT) > w {
            break;
        }
        first = candidate.to_string();
        rest = text[index..].trim_start();
    }
    if first.is_empty() {
        first = text_metrics::fit_chrome(cx.backend, text, w, NOTE_FONT);
        rest = "";
    }
    paint_line(cx, &first, color, x, y + 11.0, w);
    if !rest.is_empty() {
        paint_line(cx, rest, color, x, y + 11.0 + NOTE_LINE, w);
    }
}

fn paint_toggle(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str, checked: bool) {
    let box_rect = Rect::xywh(
        rect.origin.x,
        rect.origin.y + (rect.size.y - 16.0) / 2.0,
        16.0,
        16.0,
    );
    jian_widgets::components::checkbox::Checkbox {
        checked,
        enabled: true,
    }
    .paint(
        cx.backend,
        box_rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
    paint_line(
        cx,
        label,
        theme.foreground,
        rect.origin.x + 22.0,
        rect.origin.y + rect.size.y / 2.0 + 4.0,
        (rect.size.x - 22.0).max(0.0),
    );
}

fn paint_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon, label: &str) {
    cx.backend.fill_round_rect(rect, 6.0, theme.card);
    cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
    let label = text_metrics::fit_chrome(cx.backend, label, rect.size.x - 30.0, NOTE_FONT);
    let label_w = text_metrics::measure_chrome(cx.backend, &label, NOTE_FONT);
    let start_x = rect.origin.x + (rect.size.x - label_w - 17.0) / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(start_x, rect.origin.y + 7.5),
        13.0,
        theme.foreground,
        1.5,
    );
    paint_line(
        cx,
        &label,
        theme.foreground,
        start_x + 17.0,
        rect.origin.y + rect.size.y / 2.0 + 4.0,
        label_w + 1.0,
    );
}

/// Apply a Generator-section button. Each mutator records its own single
/// undo step and parks failures for the status line.
pub fn apply_generator_action(state: &mut EditorState, action: &PropertyPanelAction) {
    let id = state.selection.anchor.clone();
    if !id.is_real() {
        return;
    }
    let runner = installed_generator_runner();
    match action {
        PropertyPanelAction::ToggleGeneratorParam(index) => {
            let _ = state.toggle_generator_bool_param(&id, *index, runner);
        }
        PropertyPanelAction::RegenerateGenerator => {
            let _ = state.regenerate_generator(&id, runner);
        }
        PropertyPanelAction::DetachGenerator => {
            let _ = state.detach_generator(&id);
        }
        _ => {}
    }
}

/// Seed the focused parameter field with its current value.
pub fn param_initial(panel_snapshot: &NodeSnapshot, index: usize) -> String {
    panel_snapshot
        .generator
        .as_ref()
        .and_then(|summary| summary.params.get(index))
        .map(|row| row.value.clone())
        .unwrap_or_default()
}

/// Commit a parameter draft: parse, write, regenerate. No history here —
/// `commit_property_focus` snapshots around it (one undo step).
pub fn commit_param(state: &mut EditorState, index: usize, draft: &str) {
    let id = state.selection.anchor.clone();
    if id.is_real() {
        let _ = state.set_generator_param_input(&id, index, draft, installed_generator_runner());
    }
}

#[cfg(test)]
#[path = "property_panel_generator_tests.rs"]
mod tests;
