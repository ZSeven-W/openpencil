//! Motion section — the property-panel list of a node's animations and
//! its transition, painted directly above the Interactions section.
//!
//! Layout, one `INPUT_HEIGHT + 6` row each:
//!
//! ```text
//! [On load · Slide up · 400 ms       ] [×]   one row per animation; a click
//!   [On load                        ▾]       opens its fields:
//!   [Duration 400  ] [Delay 0        ]       trigger / loop / once / order,
//!   [Standard                       ▾]       timing, curve,
//!   [Keep last frame                ▾]       fill,
//!   [Repeat 1      ]                         repeat (finite runs),
//!   [Motion id intro                 ]       id (manual runs)
//! [+ Add motion                     ▾]       → preset menu
//! [Stagger children                 ▾]       containers only
//! [Transition · 150 ms · Standard    ] [×]   or "+ Add transition"
//!   [Duration 150                    ]
//! ```
//!
//! The model lives in `op_editor_core::motion_summary`; writes go through
//! `EditorCommand::Motion`. This file is only rows, geometry and paint;
//! menus live in `property_panel_motion_menu`.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    paint_dropdown, paint_input_with_prefix_focused_state, paint_section_divider,
    paint_section_label, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_interactions::tr;
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_ops_schema::motion::MotionTrigger;
use op_editor_core::interaction_edit_apply::InteractionMenuTarget;
use op_editor_core::motion_panel_edit::MotionPanelEdit;
use op_editor_core::motion_summary::{
    easing_label, fill_label, trigger_label, MotionField, MotionMenuTarget, MotionRow,
    MotionSummary,
};
use op_editor_core::{EditorState, PropertyFocus};

pub use crate::widgets::property_panel_motion_menu::motion_menu_rows;

pub(crate) const ROW_STEP: f32 = INPUT_HEIGHT + 6.0;
const ICON_BUTTON_W: f32 = 26.0;
const FIELD_INDENT: f32 = 12.0;
const COLUMN_GAP: f32 = 8.0;

/// One painted row of the section, in paint order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MotionRowKind {
    /// Animation row, index into `summary.rows`.
    Animation(usize),
    /// An open row's options dropdown: trigger, loop, once, order.
    Options(usize),
    /// Duration | delay inputs.
    Timing(usize),
    /// Easing dropdown.
    Easing(usize),
    /// Fill-mode dropdown. Full width like the easing row: fill labels
    /// are long in several locales.
    Fill(usize),
    /// Repeat count, for finite animations.
    Repeat(usize),
    /// The id `play_motion` uses, for manual animations.
    Id(usize),
    Add,
    Stagger,
    Transition,
    /// The transition's duration input.
    TransitionTiming,
}

impl MotionRowKind {
    fn is_field(self) -> bool {
        matches!(
            self,
            Self::Options(_)
                | Self::Timing(_)
                | Self::Easing(_)
                | Self::Fill(_)
                | Self::Repeat(_)
                | Self::Id(_)
                | Self::TransitionTiming
        )
    }

    fn has_two_columns(self) -> bool {
        matches!(self, Self::Timing(_))
    }
}

pub(crate) fn rows(summary: &MotionSummary) -> Vec<MotionRowKind> {
    let mut out = Vec::new();
    for (index, row) in summary.rows.iter().enumerate() {
        out.push(MotionRowKind::Animation(index));
        if summary.expanded == Some(index) {
            out.extend([
                MotionRowKind::Options(index),
                MotionRowKind::Timing(index),
                MotionRowKind::Easing(index),
                MotionRowKind::Fill(index),
            ]);
            if !row.iterations.is_infinite() {
                out.push(MotionRowKind::Repeat(index));
            }
            if row.trigger == MotionTrigger::Manual {
                out.push(MotionRowKind::Id(index));
            }
        }
    }
    out.push(MotionRowKind::Add);
    if summary.can_stagger {
        out.push(MotionRowKind::Stagger);
    }
    out.push(MotionRowKind::Transition);
    if summary.transition.is_some() {
        out.push(MotionRowKind::TransitionTiming);
    }
    out
}

/// Total vertical space the Motion section consumes.
pub fn motion_section_height(summary: &MotionSummary) -> f32 {
    SECTION_HEADER_HEIGHT + rows(summary).len() as f32 * ROW_STEP + 6.0 + SECTION_GAP
}

pub(crate) struct MotionRowGeom {
    pub row: MotionRowKind,
    /// Main (dropdown or input) rect; the left column of a two-column row.
    pub main: Rect,
    /// Right column of a two-column row.
    pub second: Option<Rect>,
    /// Trailing remove button.
    pub button: Option<Rect>,
}

pub(crate) fn row_geometry(
    summary: &MotionSummary,
    x: f32,
    y: f32,
    width: f32,
) -> Vec<MotionRowGeom> {
    let usable_w = width - PAD_X * 2.0;
    let mut row_y = y + SECTION_HEADER_HEIGHT;
    let mut out = Vec::new();
    for row in rows(summary) {
        let has_button = match row {
            MotionRowKind::Animation(_) => true,
            MotionRowKind::Transition => summary.transition.is_some(),
            _ => false,
        };
        let indent = if row.is_field() { FIELD_INDENT } else { 0.0 };
        let button_w = if has_button { ICON_BUTTON_W + 4.0 } else { 0.0 };
        let left = x + PAD_X + indent;
        let full_w = usable_w - indent - button_w;
        let (main, second) = if row.has_two_columns() {
            let half = (full_w - COLUMN_GAP) / 2.0;
            (
                Rect::xywh(left, row_y, half, INPUT_HEIGHT),
                Some(Rect::xywh(
                    left + half + COLUMN_GAP,
                    row_y,
                    half,
                    INPUT_HEIGHT,
                )),
            )
        } else {
            (Rect::xywh(left, row_y, full_w, INPUT_HEIGHT), None)
        };
        let button = has_button.then(|| {
            Rect::xywh(
                x + PAD_X + usable_w - ICON_BUTTON_W,
                row_y,
                ICON_BUTTON_W,
                INPUT_HEIGHT,
            )
        });
        out.push(MotionRowGeom {
            row,
            main,
            second,
            button,
        });
        row_y += ROW_STEP;
    }
    out
}

pub(crate) fn menu(target: MotionMenuTarget) -> PropertyPanelAction {
    PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::Motion(target))
}

pub(crate) fn edit(edit: MotionPanelEdit) -> PropertyPanelAction {
    PropertyPanelAction::EditMotion(edit)
}

/// Emit the section's clickable rects. Main rects first so
/// `hit_test_action`'s reverse walk lets a trailing button win.
pub fn push_motion_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    summary: &MotionSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    for geometry in row_geometry(summary, x, y, width) {
        let (main, second, button) = match geometry.row {
            MotionRowKind::Animation(index) => (
                Some(PropertyPanelAction::ToggleMotionRow(index)),
                None,
                Some(edit(MotionPanelEdit::Remove(index))),
            ),
            MotionRowKind::Options(index) => {
                (Some(menu(MotionMenuTarget::Animation(index))), None, None)
            }
            MotionRowKind::Easing(index) => {
                (Some(menu(MotionMenuTarget::Easing(index))), None, None)
            }
            MotionRowKind::Fill(index) => (Some(menu(MotionMenuTarget::Fill(index))), None, None),
            MotionRowKind::Add => (Some(menu(MotionMenuTarget::AddPreset)), None, None),
            MotionRowKind::Stagger => (Some(menu(MotionMenuTarget::StaggerPreset)), None, None),
            MotionRowKind::Transition if summary.transition.is_some() => (
                Some(menu(MotionMenuTarget::TransitionEasing)),
                None,
                Some(edit(MotionPanelEdit::RemoveTransition)),
            ),
            MotionRowKind::Transition => (Some(edit(MotionPanelEdit::AddTransition)), None, None),
            // Text inputs: see `push_motion_input_rects`.
            MotionRowKind::Timing(_)
            | MotionRowKind::Repeat(_)
            | MotionRowKind::Id(_)
            | MotionRowKind::TransitionTiming => (None, None, None),
        };
        for (action, rect) in [
            (main, Some(geometry.main)),
            (second, geometry.second),
            (button, geometry.button),
        ] {
            if let (Some(action), Some(rect)) = (action, rect) {
                out.push((action, rect));
            }
        }
    }
}

/// Emit the section's text inputs.
pub fn push_motion_input_rects(
    out: &mut Vec<(PropertyFocus, Rect)>,
    summary: &MotionSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    for geometry in row_geometry(summary, x, y, width) {
        let fields = match geometry.row {
            MotionRowKind::Timing(index) => [
                Some(PropertyFocus::MotionField(index, MotionField::Duration)),
                Some(PropertyFocus::MotionField(index, MotionField::Delay)),
            ],
            MotionRowKind::Repeat(index) => [
                Some(PropertyFocus::MotionField(index, MotionField::Repeat)),
                None,
            ],
            MotionRowKind::Id(index) => [
                Some(PropertyFocus::MotionField(index, MotionField::Id)),
                None,
            ],
            MotionRowKind::TransitionTiming => [Some(PropertyFocus::TransitionDuration), None],
            _ => [None, None],
        };
        for (focus, rect) in fields
            .into_iter()
            .zip([Some(geometry.main), geometry.second])
        {
            if let (Some(focus), Some(rect)) = (focus, rect) {
                out.push((focus, rect));
            }
        }
    }
}

/// `On load · Slide up · 400 ms`; an endless loop reads `Pulse ∞`, next to
/// the name, so the mark survives when a narrow row cuts the end off.
pub(crate) fn animation_label(locale: op_editor_core::Locale, row: &MotionRow) -> String {
    format!(
        "{} · {}{} · {} ms",
        tr(locale, trigger_label(row.trigger)),
        tr(locale, row.title()),
        if row.iterations.is_infinite() {
            " ∞"
        } else {
            ""
        },
        row.duration_ms
    )
}

pub(crate) fn transition_label(locale: op_editor_core::Locale, summary: &MotionSummary) -> String {
    match &summary.transition {
        Some(transition) => format!(
            "{} · {} ms · {}",
            tr(locale, ("motion.transition.title", "Transition")),
            transition.duration_ms,
            tr(locale, easing_label(&transition.easing))
        ),
        None => tr(locale, ("motion.transition.add", "+ Add transition")).to_owned(),
    }
}

/// The prefix label of a text field.
fn field_prefix(locale: op_editor_core::Locale, focus: PropertyFocus) -> &'static str {
    match focus {
        PropertyFocus::MotionField(_, MotionField::Delay) => {
            tr(locale, ("motion.field.delay", "Delay"))
        }
        PropertyFocus::MotionField(_, MotionField::Repeat) => {
            tr(locale, ("motion.field.repeat", "Repeat"))
        }
        PropertyFocus::MotionField(_, MotionField::Id) => {
            tr(locale, ("motion.field.id", "Motion id"))
        }
        _ => tr(locale, ("motion.field.duration", "Duration")),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_motion_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    summary: &MotionSummary,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    paint_section_label(
        cx,
        theme,
        tr(locale, ("motion.section.title", "Motion")),
        x,
        y,
        width,
    );
    let mut inputs = Vec::new();
    push_motion_input_rects(&mut inputs, summary, x, y, width);
    for (focus, rect) in inputs {
        let initial = field_initial(focus, summary);
        paint_input_with_prefix_focused_state(
            cx,
            theme,
            rect,
            field_prefix(locale, focus),
            edit.value_for(focus, &initial),
            edit.focus == Some(focus),
            edit.caret_at(focus),
            edit.select_all_at(focus),
            edit.input_at(focus),
            edit.now_ms,
        );
    }
    for geometry in row_geometry(summary, x, y, width) {
        let labels: [Option<String>; 2] = match geometry.row {
            MotionRowKind::Animation(index) => {
                [Some(animation_label(locale, &summary.rows[index])), None]
            }
            MotionRowKind::Options(index) => [
                Some(tr(locale, trigger_label(summary.rows[index].trigger)).to_owned()),
                None,
            ],
            MotionRowKind::Easing(index) => [
                Some(tr(locale, easing_label(&summary.rows[index].easing)).to_owned()),
                None,
            ],
            MotionRowKind::Fill(index) => [
                Some(tr(locale, fill_label(summary.rows[index].fill_mode)).to_owned()),
                None,
            ],
            MotionRowKind::Add => [
                Some(tr(locale, ("motion.add", "+ Add motion")).to_owned()),
                None,
            ],
            MotionRowKind::Stagger => [
                Some(tr(locale, ("motion.stagger", "Stagger children")).to_owned()),
                None,
            ],
            MotionRowKind::Transition => [Some(transition_label(locale, summary)), None],
            // Painted above as text inputs.
            MotionRowKind::Timing(_)
            | MotionRowKind::Repeat(_)
            | MotionRowKind::Id(_)
            | MotionRowKind::TransitionTiming => [None, None],
        };
        for (label, rect) in labels
            .into_iter()
            .zip([Some(geometry.main), geometry.second])
        {
            if let (Some(label), Some(rect)) = (label, rect) {
                // Summary rows are long in some locales: keep the text clear
                // of the chevron (12 px inset on the left, 26 px on the right).
                let label = crate::util::ellipsize_to_width(&label, rect.size.x - 38.0, |text| {
                    crate::widgets::text_metrics::measure_chrome(cx.backend, text, 12.0)
                });
                paint_dropdown(cx, theme, rect, &label);
            }
        }
        if let Some(button) = geometry.button {
            draw_icon(
                cx.backend,
                Icon::Close,
                Point2D::new(
                    button.origin.x + (button.size.x - 14.0) / 2.0,
                    button.origin.y + (button.size.y - 14.0) / 2.0,
                ),
                14.0,
                theme.muted_foreground,
                1.4,
            );
        }
    }
    let bottom = y + motion_section_height(summary) - SECTION_GAP;
    paint_section_divider(cx, theme, x, bottom, width);
    bottom + SECTION_GAP
}

/// The draft a freshly focused Motion text field starts from.
pub(crate) fn field_initial(focus: PropertyFocus, summary: &MotionSummary) -> String {
    match focus {
        PropertyFocus::MotionField(index, field) => {
            let Some(row) = summary.rows.get(index) else {
                return String::new();
            };
            match field {
                MotionField::Duration => row.duration_ms.to_string(),
                MotionField::Delay => row.delay_ms.to_string(),
                MotionField::Repeat => row.iterations.count_or(1).to_string(),
                MotionField::Id => row.id.clone().unwrap_or_default(),
            }
        }
        PropertyFocus::TransitionDuration => summary
            .transition
            .as_ref()
            .map(|transition| transition.duration_ms.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Write a committed Motion text field. Unparsable numbers are dropped,
/// like every other numeric panel field.
pub(crate) fn commit_field(state: &mut EditorState, focus: PropertyFocus, draft: &str) {
    let draft = draft.trim();
    let edit = match focus {
        PropertyFocus::MotionField(index, MotionField::Id) => {
            Some(MotionPanelEdit::SetId(index, draft.to_owned()))
        }
        PropertyFocus::MotionField(index, MotionField::Repeat) => draft
            .parse::<u32>()
            .ok()
            .map(|count| MotionPanelEdit::SetRepeat(index, count)),
        PropertyFocus::MotionField(index, MotionField::Duration) => draft
            .parse::<u64>()
            .ok()
            .map(|ms| MotionPanelEdit::SetDuration(index, ms)),
        PropertyFocus::MotionField(index, MotionField::Delay) => draft
            .parse::<u64>()
            .ok()
            .map(|ms| MotionPanelEdit::SetDelay(index, ms)),
        PropertyFocus::TransitionDuration => draft
            .parse::<u64>()
            .ok()
            .map(MotionPanelEdit::SetTransitionDuration),
        _ => None,
    };
    if let Some(edit) = edit {
        let _ = state.apply_selected_motion_edit_untracked(&edit);
    }
}

#[cfg(test)]
#[path = "property_panel_motion_tests.rs"]
mod tests;
