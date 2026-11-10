//! Dropdown menus of the Motion section: presets to add or stagger, an
//! animation row's trigger / loop / order menu, and the easing and fill
//! pickers. They open in the Interactions popover
//! (`InteractionMenuTarget::Motion`), so one property-panel menu is ever open.

use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_interactions::tr;
use crate::widgets::property_panel_motion::edit;
use jian_ops_schema::motion::{MotionTrigger, NodeAnimationFillMode};
use op_editor_core::motion_panel_edit::MotionPanelEdit;
use op_editor_core::motion_presets::MotionPreset;
use op_editor_core::motion_summary::{
    easing_label, fill_label, trigger_label, MotionMenuTarget, MotionSummary, MOTION_EASINGS,
    MOTION_TRIGGERS,
};

/// `✓ ` marks the current choice in a menu.
fn choice(current: bool, label: &str) -> String {
    if current {
        format!("✓ {label}")
    } else {
        label.to_owned()
    }
}

/// Rows of the open Motion menu, in paint order.
pub fn motion_menu_rows(
    locale: op_editor_core::Locale,
    target: MotionMenuTarget,
    summary: &MotionSummary,
) -> Vec<(PropertyPanelAction, String)> {
    let easings =
        |current: Option<&jian_ops_schema::motion::Easing>,
         make: &dyn Fn(jian_ops_schema::motion::Easing) -> MotionPanelEdit| {
            MOTION_EASINGS
                .iter()
                .map(|easing| {
                    let label = choice(current == Some(easing), tr(locale, easing_label(easing)));
                    (edit(make(easing.clone())), label)
                })
                .collect::<Vec<_>>()
        };
    match target {
        MotionMenuTarget::AddPreset => MotionPreset::ALL
            .iter()
            .map(|preset| {
                (
                    edit(MotionPanelEdit::AddPreset(*preset)),
                    tr(locale, preset.label()).to_owned(),
                )
            })
            .collect(),
        MotionMenuTarget::StaggerPreset => MotionPreset::ALL
            .iter()
            .filter(|preset| **preset != MotionPreset::Custom)
            .map(|preset| {
                (
                    edit(MotionPanelEdit::StaggerPreset(*preset)),
                    tr(locale, preset.label()).to_owned(),
                )
            })
            .collect(),
        MotionMenuTarget::Animation(index) => {
            let Some(row) = summary.rows.get(index) else {
                return Vec::new();
            };
            let mut rows: Vec<_> = MOTION_TRIGGERS
                .iter()
                .map(|trigger| {
                    (
                        edit(MotionPanelEdit::SetTrigger(index, *trigger)),
                        choice(row.trigger == *trigger, tr(locale, trigger_label(*trigger))),
                    )
                })
                .collect();
            rows.push((
                edit(MotionPanelEdit::ToggleLoop(index)),
                choice(
                    row.iterations.is_infinite(),
                    tr(locale, ("motion.field.loop", "Loop forever")),
                ),
            ));
            if row.trigger == MotionTrigger::InView {
                rows.push((
                    edit(MotionPanelEdit::ToggleOnce(index)),
                    choice(
                        row.once,
                        tr(locale, ("motion.field.once", "Only the first time")),
                    ),
                ));
            }
            if index > 0 {
                rows.push((
                    edit(MotionPanelEdit::MoveUp(index)),
                    tr(locale, ("interactions.moveUp", "Move up")).to_owned(),
                ));
            }
            if index + 1 < summary.rows.len() {
                rows.push((
                    edit(MotionPanelEdit::MoveDown(index)),
                    tr(locale, ("interactions.moveDown", "Move down")).to_owned(),
                ));
            }
            rows
        }
        MotionMenuTarget::Easing(index) => match summary.rows.get(index) {
            Some(row) => easings(Some(&row.easing), &|easing| {
                MotionPanelEdit::SetEasing(index, easing)
            }),
            None => Vec::new(),
        },
        MotionMenuTarget::Fill(index) => match summary.rows.get(index) {
            Some(row) => [NodeAnimationFillMode::Forwards, NodeAnimationFillMode::None]
                .into_iter()
                .map(|fill| {
                    (
                        edit(MotionPanelEdit::SetFill(index, fill)),
                        choice(row.fill_mode == fill, tr(locale, fill_label(fill))),
                    )
                })
                .collect(),
            None => Vec::new(),
        },
        MotionMenuTarget::TransitionEasing => match &summary.transition {
            Some(transition) => easings(Some(&transition.easing), &|easing| {
                MotionPanelEdit::SetTransitionEasing(easing)
            }),
            None => Vec::new(),
        },
    }
}
