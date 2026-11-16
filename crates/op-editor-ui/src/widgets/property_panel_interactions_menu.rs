//! Dropdown menus of the Interactions section: trigger picker, trigger
//! row menu, action-kind menu, and the screen / layer parameter pickers.
//! One popover at a time, chosen by `InteractionMenuTarget`.

use crate::theme::Theme;
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::INPUT_RADIUS;
use crate::widgets::property_panel_interactions::{kind_label, tr, InteractionSummary};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_core::motion::screen_transition::ScreenTransitionKind;
use jian_core::motion::MotionControlMode;
use jian_ops_schema::motion::Easing;
use op_editor_core::interaction_edit::{
    InteractionActionKind, InteractionActionView, InteractionParamKind, InteractionTrigger,
};
use op_editor_core::interaction_edit_apply::{
    InteractionContext, InteractionEdit, InteractionMenuTarget, InteractionOption,
};

pub(crate) const INTERACTION_MENU_ROW_H: f32 = 28.0;
pub(crate) const INTERACTION_MENU_MIN_W: f32 = 200.0;
const MAX_LABEL_CHARS: usize = 30;

/// Hit-result for a click on the open Interactions menu.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionMenuHit {
    Row(PropertyPanelAction),
    Inside,
    Outside,
}

fn edit(e: InteractionEdit) -> PropertyPanelAction {
    PropertyPanelAction::EditInteraction(e)
}

fn truncate(label: String) -> String {
    if label.chars().count() <= MAX_LABEL_CHARS {
        label
    } else {
        let mut s: String = label.chars().take(MAX_LABEL_CHARS - 1).collect();
        s.push('…');
        s
    }
}

fn present(summary: &InteractionSummary, trigger: InteractionTrigger) -> bool {
    summary.triggers.iter().any(|t| t.trigger == Some(trigger))
}

/// Menu rows in paint order for `target`.
pub fn interaction_menu_rows(
    locale: op_editor_core::Locale,
    target: &InteractionMenuTarget,
    summary: &InteractionSummary,
    ctx: &InteractionContext,
) -> Vec<(PropertyPanelAction, String)> {
    let mut rows = Vec::new();
    match target {
        InteractionMenuTarget::Motion(motion) => {
            return crate::widgets::property_panel_motion::motion_menu_rows(
                locale,
                *motion,
                &summary.motion,
            );
        }
        InteractionMenuTarget::AddTrigger => {
            for trigger in InteractionTrigger::ALL {
                if !present(summary, trigger) {
                    rows.push((
                        edit(InteractionEdit::AddTrigger(trigger)),
                        tr(locale, trigger.label()).to_string(),
                    ));
                }
            }
        }
        InteractionMenuTarget::Trigger(key) => {
            if let Some(from) = InteractionTrigger::from_key(key) {
                rows.push((
                    edit(InteractionEdit::AddAction(from)),
                    tr(locale, ("interactions.addAction", "+ Add action")).to_string(),
                ));
                for to in InteractionTrigger::ALL {
                    if to != from && !present(summary, to) {
                        rows.push((
                            edit(InteractionEdit::ChangeTrigger { from, to }),
                            format!(
                                "{} {}",
                                tr(locale, ("interactions.changeTo", "Change to")),
                                tr(locale, to.label())
                            ),
                        ));
                    }
                }
            }
            rows.push((
                edit(InteractionEdit::RemoveTrigger(key.clone())),
                tr(locale, ("interactions.remove", "Remove")).to_string(),
            ));
        }
        InteractionMenuTarget::Action { trigger, index } => {
            let current = summary.action(*trigger, *index).and_then(|a| a.kind);
            for kind in InteractionActionKind::ALL {
                if kind == InteractionActionKind::PlayMotion && ctx.motion_targets.is_empty() {
                    rows.push((
                        edit(InteractionEdit::CreateManualMotion {
                            trigger: *trigger,
                            index: *index,
                        }),
                        tr(
                            locale,
                            ("interactions.motion.create", "Create and play a fade-in"),
                        )
                        .to_string(),
                    ));
                    continue;
                }
                let mark = if current == Some(kind) { "✓ " } else { "   " };
                rows.push((
                    edit(InteractionEdit::SetActionKind {
                        trigger: *trigger,
                        index: *index,
                        kind,
                    }),
                    format!("{mark}{}", kind_label(locale, kind)),
                ));
            }
            if *index > 0 {
                rows.push((
                    edit(InteractionEdit::MoveAction {
                        trigger: *trigger,
                        index: *index,
                        up: true,
                    }),
                    format!("   {}", tr(locale, ("interactions.moveUp", "Move up"))),
                ));
            }
            if index + 1 < summary.action_count(*trigger) {
                rows.push((
                    edit(InteractionEdit::MoveAction {
                        trigger: *trigger,
                        index: *index,
                        up: false,
                    }),
                    format!("   {}", tr(locale, ("interactions.moveDown", "Move down"))),
                ));
            }
            rows.push((
                edit(InteractionEdit::RemoveAction {
                    trigger: *trigger,
                    index: *index,
                }),
                format!("   {}", tr(locale, ("interactions.remove", "Remove"))),
            ));
        }
        InteractionMenuTarget::Param { trigger, index } => {
            let Some(action) = summary.action(*trigger, *index) else {
                return rows;
            };
            let set = |value: String| {
                edit(InteractionEdit::SetActionParam {
                    trigger: *trigger,
                    index: *index,
                    value,
                })
            };
            let param_kind = action.kind.map(|k| k.param_kind());
            match param_kind {
                Some(InteractionParamKind::Screen) => {
                    for screen in &ctx.screens {
                        let mark = if *screen == action.param {
                            "✓ "
                        } else {
                            "   "
                        };
                        rows.push((set(screen.clone()), format!("{mark}{screen}")));
                    }
                    if ctx.screens.is_empty() {
                        rows.push((
                            PropertyPanelAction::CloseInteractionMenu,
                            tr(
                                locale,
                                ("interactions.noScreens", "No screens on this page"),
                            )
                            .to_string(),
                        ));
                    }
                }
                Some(InteractionParamKind::Node | InteractionParamKind::MotionTarget) => {
                    let nodes = if param_kind == Some(InteractionParamKind::MotionTarget) {
                        &ctx.motion_targets
                    } else {
                        &ctx.nodes
                    };
                    let hidden = tr(locale, ("interactions.hiddenSuffix", "(hidden)"));
                    for node in nodes
                        .iter()
                        .take(op_editor_core::interaction_edit_apply::MAX_TARGETS)
                    {
                        let mark = if node.id == action.param {
                            "✓ "
                        } else {
                            "   "
                        };
                        let suffix = if node.hidden {
                            format!(" {hidden}")
                        } else {
                            String::new()
                        };
                        rows.push((
                            set(node.id.clone()),
                            truncate(format!("{mark}{}{suffix}", node.name)),
                        ));
                    }
                    if nodes.is_empty() {
                        rows.push((
                            PropertyPanelAction::CloseInteractionMenu,
                            tr(
                                locale,
                                ("interactions.noLayers", "No other layers in this screen"),
                            )
                            .to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }
        InteractionMenuTarget::Option {
            trigger,
            index,
            option,
        } => {
            let Some(action) = summary.action(*trigger, *index) else {
                return rows;
            };
            let selected = option_value(*option, action);
            let choices = if *option == InteractionOption::Clip {
                ctx.manual_clips
                    .get(&action.param)
                    .into_iter()
                    .flatten()
                    .take(op_editor_core::interaction_edit_apply::MAX_TARGETS)
                    .map(|id| (id.clone(), id.clone()))
                    .collect()
            } else {
                option_choices(locale, *option)
            };
            for (value, label) in choices {
                let mark = if value == selected { "✓ " } else { "   " };
                rows.push((
                    edit(InteractionEdit::SetActionOption {
                        trigger: *trigger,
                        index: *index,
                        option: *option,
                        value,
                    }),
                    truncate(format!("{mark}{label}")),
                ));
            }
            if rows.is_empty() && *option == InteractionOption::Clip {
                rows.push((
                    edit(InteractionEdit::CreateManualMotion {
                        trigger: *trigger,
                        index: *index,
                    }),
                    tr(
                        locale,
                        ("interactions.motion.create", "Create and play a fade-in"),
                    )
                    .to_string(),
                ));
            }
        }
    }
    rows
}

type Label = (&'static str, &'static str);

const TRANSITIONS: &[(ScreenTransitionKind, Label)] = &[
    (
        ScreenTransitionKind::Default,
        ("interactions.transition.default", "Default"),
    ),
    (
        ScreenTransitionKind::SlideLeft,
        ("motion.preset.slideLeft", "Slide left"),
    ),
    (
        ScreenTransitionKind::SlideRight,
        ("motion.preset.slideRight", "Slide right"),
    ),
    (
        ScreenTransitionKind::SlideUp,
        ("motion.preset.slideUp", "Slide up"),
    ),
    (
        ScreenTransitionKind::SlideDown,
        ("motion.preset.slideDown", "Slide down"),
    ),
    (
        ScreenTransitionKind::Fade,
        ("interactions.transition.fade", "Fade"),
    ),
    (
        ScreenTransitionKind::None,
        ("interactions.transition.none", "No transition"),
    ),
];

const MODES: &[(MotionControlMode, Label)] = &[
    (MotionControlMode::Play, ("motion.timeline.play", "Play")),
    (
        MotionControlMode::Restart,
        ("interactions.motion.restart", "Restart"),
    ),
    (
        MotionControlMode::Reverse,
        ("interactions.motion.reverse", "Reverse"),
    ),
    (
        MotionControlMode::Stop,
        ("interactions.motion.stop", "Stop"),
    ),
];

const EASINGS: &[(Easing, Label)] = &[
    (Easing::Linear, ("motion.easing.linear", "Linear")),
    (Easing::Ease, ("motion.easing.ease", "Ease")),
    (Easing::EaseIn, ("motion.easing.easeIn", "Ease in")),
    (Easing::EaseOut, ("motion.easing.easeOut", "Ease out")),
    (
        Easing::EaseInOut,
        ("motion.easing.easeInOut", "Ease in-out"),
    ),
    (Easing::Standard, ("motion.easing.standard", "Standard")),
    (
        Easing::Emphasized,
        ("motion.easing.emphasized", "Emphasized"),
    ),
    (
        Easing::EmphasizedDecelerate,
        (
            "motion.easing.emphasizedDecelerate",
            "Emphasized decelerate",
        ),
    ),
    (
        Easing::EmphasizedAccelerate,
        (
            "motion.easing.emphasizedAccelerate",
            "Emphasized accelerate",
        ),
    ),
];

fn wire_name(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn option_value(option: InteractionOption, action: &InteractionActionView) -> String {
    use InteractionOption as O;
    match option {
        O::Clip => action
            .motion
            .as_ref()
            .map(|motion| motion.animation.clone())
            .unwrap_or_default(),
        O::Mode => wire_name(
            action
                .motion
                .as_ref()
                .map(|motion| motion.mode)
                .unwrap_or_default(),
        ),
        O::TransitionKind => wire_name(
            action
                .transition
                .as_ref()
                .map(|spec| spec.kind)
                .unwrap_or_default(),
        ),
        O::TransitionEasing => action
            .transition
            .as_ref()
            .and_then(|spec| spec.easing.as_ref())
            .map_or_else(String::new, |easing| {
                if matches!(easing, Easing::CubicBezier(..)) {
                    "cubicBezier".into()
                } else {
                    wire_name(easing)
                }
            }),
        O::TransitionDuration => String::new(),
    }
}

fn option_choices(
    locale: op_editor_core::Locale,
    option: InteractionOption,
) -> Vec<(String, String)> {
    use InteractionOption as O;
    match option {
        O::Mode => MODES
            .iter()
            .map(|(value, label)| (wire_name(value), tr(locale, *label).into()))
            .collect(),
        O::TransitionKind => TRANSITIONS
            .iter()
            .map(|(value, label)| (wire_name(value), tr(locale, *label).into()))
            .collect(),
        O::TransitionEasing => std::iter::once((
            String::new(),
            tr(locale, ("interactions.transition.default", "Default")).into(),
        ))
        .chain(
            EASINGS
                .iter()
                .map(|(value, label)| (wire_name(value), tr(locale, *label).into())),
        )
        .collect(),
        O::Clip | O::TransitionDuration => Vec::new(),
    }
}

pub(super) fn option_label(
    locale: op_editor_core::Locale,
    option: InteractionOption,
    action: &InteractionActionView,
) -> String {
    use InteractionOption as O;
    let value = option_value(option, action);
    let label = match option {
        O::Clip if value.is_empty() => {
            return tr(locale, ("motion.timeline.pickId", "Choose a motion id")).into()
        }
        O::Clip => return value,
        O::Mode => ("interactions.motion.mode", "Playback"),
        O::TransitionKind => ("motion.transition.title", "Transition"),
        O::TransitionEasing => ("motion.field.easing", "Easing"),
        O::TransitionDuration => return String::new(),
    };
    let choice = option_choices(locale, option)
        .into_iter()
        .find_map(|(key, label)| (key == value).then_some(label))
        .unwrap_or_else(|| tr(locale, ("motion.easing.custom", "Custom curve")).into());
    format!("{} · {choice}", tr(locale, label))
}

/// The popover rect: right-aligned under `anchor`, flipped above it
/// when it would overflow `bounds` and there is more room above, then
/// clamped into `bounds` vertically.
pub fn interaction_menu_rect(anchor: Rect, row_count: usize, bounds: Rect) -> Rect {
    let h = row_count as f32 * INTERACTION_MENU_ROW_H + 8.0;
    let w = anchor.size.x.max(INTERACTION_MENU_MIN_W);
    let right = anchor.origin.x + anchor.size.x;
    let below = anchor.origin.y + anchor.size.y;
    let bounds_bottom = bounds.origin.y + bounds.size.y;
    let mut top = below;
    if below + h > bounds_bottom {
        let room_above = anchor.origin.y - bounds.origin.y;
        let room_below = bounds_bottom - below;
        top = if room_above > room_below {
            anchor.origin.y - h
        } else {
            below
        };
        top = top.min(bounds_bottom - h).max(bounds.origin.y);
    }
    Rect {
        origin: Point2D::new(right - w, top),
        size: Point2D::new(w, h),
    }
}

/// `(action, row_rect)` for each menu row, given the menu rect.
pub fn interaction_menu_row_rects(
    menu: Rect,
    rows: &[(PropertyPanelAction, String)],
) -> Vec<(PropertyPanelAction, Rect)> {
    rows.iter()
        .enumerate()
        .map(|(i, (action, _))| {
            let ry = menu.origin.y + 4.0 + i as f32 * INTERACTION_MENU_ROW_H;
            (
                action.clone(),
                Rect {
                    origin: Point2D::new(menu.origin.x, ry),
                    size: Point2D::new(menu.size.x, INTERACTION_MENU_ROW_H),
                },
            )
        })
        .collect()
}

/// Paint the menu at `menu` with `hover` highlighting a row index.
pub fn paint_interaction_menu(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    menu: Rect,
    rows: &[(PropertyPanelAction, String)],
    hover: Option<usize>,
) {
    cx.backend
        .fill_round_rect(menu, INPUT_RADIUS, theme.popover);
    cx.backend
        .stroke_round_rect(menu, INPUT_RADIUS, theme.border, 1.0);
    for (i, (action, label)) in rows.iter().enumerate() {
        let ry = menu.origin.y + 4.0 + i as f32 * INTERACTION_MENU_ROW_H;
        let inert = matches!(action, PropertyPanelAction::CloseInteractionMenu);
        if hover == Some(i) && !inert {
            let row = Rect {
                origin: Point2D::new(menu.origin.x + 4.0, ry),
                size: Point2D::new(menu.size.x - 8.0, INTERACTION_MENU_ROW_H),
            };
            cx.backend.fill_round_rect(row, 6.0, theme.muted);
        }
        let color = if inert {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let text = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(
                menu.origin.x + 10.0,
                ry + INTERACTION_MENU_ROW_H / 2.0 + 4.0,
            ),
        );
    }
}
