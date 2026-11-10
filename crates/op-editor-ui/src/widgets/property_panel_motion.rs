//! Motion section — the property-panel list of a node's animations and
//! its transition, painted directly above the Interactions section.
//!
//! Layout, one `INPUT_HEIGHT + 6` row each:
//!
//! ```text
//! [On load · Slide up · 400 ms       ] [×]   one row per animation
//! [+ Add motion                     ▾]        → preset menu
//! [Stagger children                 ▾]        containers only
//! [Transition · 150 ms · Standard    ] [×]   or "+ Add transition"
//! ```
//!
//! The model lives in `op_editor_core::motion_summary`; writes go through
//! `EditorCommand::Motion`. This file is only rows, geometry and paint.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    paint_dropdown, paint_section_divider, paint_section_label, INPUT_HEIGHT, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_interactions::tr;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::interaction_edit_apply::InteractionMenuTarget;
use op_editor_core::motion_panel_edit::MotionPanelEdit;
use op_editor_core::motion_summary::{
    easing_label, trigger_label, MotionField, MotionMenuTarget, MotionRow, MotionSummary,
};
use op_editor_core::{EditorState, PropertyFocus};

pub use crate::widgets::property_panel_motion_menu::motion_menu_rows;

pub(crate) const ROW_STEP: f32 = INPUT_HEIGHT + 6.0;
const ICON_BUTTON_W: f32 = 26.0;

/// One painted row of the section, in paint order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MotionRowKind {
    /// Animation row, index into `summary.rows`.
    Animation(usize),
    Add,
    Stagger,
    Transition,
}

pub(crate) fn rows(summary: &MotionSummary) -> Vec<MotionRowKind> {
    let mut out: Vec<_> = (0..summary.rows.len())
        .map(MotionRowKind::Animation)
        .collect();
    out.push(MotionRowKind::Add);
    if summary.can_stagger {
        out.push(MotionRowKind::Stagger);
    }
    out.push(MotionRowKind::Transition);
    out
}

/// Total vertical space the Motion section consumes.
pub fn motion_section_height(summary: &MotionSummary) -> f32 {
    SECTION_HEADER_HEIGHT + rows(summary).len() as f32 * ROW_STEP + 6.0 + SECTION_GAP
}

pub(crate) struct MotionRowGeom {
    pub row: MotionRowKind,
    /// Main (dropdown) rect.
    pub main: Rect,
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
            MotionRowKind::Add | MotionRowKind::Stagger => false,
        };
        let button_w = if has_button { ICON_BUTTON_W + 4.0 } else { 0.0 };
        let main = Rect {
            origin: Point2D::new(x + PAD_X, row_y),
            size: Point2D::new(usable_w - button_w, INPUT_HEIGHT),
        };
        let button = has_button.then(|| Rect {
            origin: Point2D::new(x + PAD_X + usable_w - ICON_BUTTON_W, row_y),
            size: Point2D::new(ICON_BUTTON_W, INPUT_HEIGHT),
        });
        out.push(MotionRowGeom { row, main, button });
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
        let (main, button) = match geometry.row {
            MotionRowKind::Animation(index) => (
                menu(MotionMenuTarget::Animation(index)),
                Some(edit(MotionPanelEdit::Remove(index))),
            ),
            MotionRowKind::Add => (menu(MotionMenuTarget::AddPreset), None),
            MotionRowKind::Stagger => (menu(MotionMenuTarget::StaggerPreset), None),
            MotionRowKind::Transition if summary.transition.is_some() => (
                menu(MotionMenuTarget::TransitionEasing),
                Some(edit(MotionPanelEdit::RemoveTransition)),
            ),
            MotionRowKind::Transition => (edit(MotionPanelEdit::AddTransition), None),
        };
        out.push((main, geometry.main));
        if let (Some(action), Some(rect)) = (button, geometry.button) {
            out.push((action, rect));
        }
    }
}

/// `On load · Slide up · 400 ms` (with `· ∞` for endless loops).
pub(crate) fn animation_label(locale: op_editor_core::Locale, row: &MotionRow) -> String {
    let mut label = format!(
        "{} · {} · {} ms",
        tr(locale, trigger_label(row.trigger)),
        tr(locale, row.title()),
        row.duration_ms
    );
    if row.iterations.is_infinite() {
        label.push_str(" · ∞");
    }
    label
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

pub fn paint_motion_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    summary: &MotionSummary,
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
    for geometry in row_geometry(summary, x, y, width) {
        let label = match geometry.row {
            MotionRowKind::Animation(index) => animation_label(locale, &summary.rows[index]),
            MotionRowKind::Add => tr(locale, ("motion.add", "+ Add motion")).to_owned(),
            MotionRowKind::Stagger => tr(locale, ("motion.stagger", "Stagger children")).to_owned(),
            MotionRowKind::Transition => transition_label(locale, summary),
        };
        paint_dropdown(cx, theme, geometry.main, &label);
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
