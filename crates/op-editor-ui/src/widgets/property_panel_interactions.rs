//! Interactions section — the property-panel editor for a node's
//! `events` block (what happens in Preview when the node is tapped,
//! hovered, long-pressed, changed, …).
//!
//! Layout, one `INPUT_HEIGHT + 6` row each:
//!
//! ```text
//! Screen: /home                       (top-level screen frames only)
//! [On tap               ▾] [+]        trigger row   → trigger menu
//!   [Navigate to        ▾] [×]        action row    → action-kind menu
//!     [/detail          ▾]            picker param  → screen / layer menu
//!     [Text  Saved!       ]           text param    → PropertyFocus input
//! [+ Add interaction    ▾]            → trigger picker
//! ```
//!
//! The structured model + JSON round-trip lives in
//! `op_editor_core::interaction_edit{,_apply}`; this file is only rows,
//! geometry and paint. Menus live in `property_panel_interactions_menu`.
//! Triggers and actions the panel cannot author stay visible read-only
//! (and removable), so opening a hand-written document never drops
//! behaviour.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{
    paint_dropdown, paint_input_with_prefix_focused_state, paint_section_divider,
    paint_section_label, INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_ops_schema::node::PenNode;
use op_editor_core::interaction_edit::{
    summarize_events, InteractionActionKind, InteractionParamKind, InteractionTrigger,
    InteractionTriggerView,
};
use op_editor_core::interaction_edit_apply::{
    InteractionContext, InteractionEdit, InteractionMenuTarget,
};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::PropertyFocus;

pub use crate::widgets::property_panel_interactions_menu::{
    interaction_menu_rect, interaction_menu_row_rects, interaction_menu_rows,
    paint_interaction_menu, InteractionMenuHit,
};

/// Interaction state for the selected node — always present (empty
/// when nothing is authored) so the section can paint its "+ Add
/// interaction" empty state. `screen` is `Some` only for a top-level
/// Frame carrying an authored `screen` route.
///
/// The Motion section rides along: it describes the same node, has the
/// same visibility and paints directly above, so its summary travels with
/// this one through the layout walkers instead of adding a parameter to
/// each of them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InteractionSummary {
    pub screen: Option<String>,
    pub triggers: Vec<InteractionTriggerView>,
    pub motion: op_editor_core::motion_summary::MotionSummary,
}

impl InteractionSummary {
    /// The current param value of `trigger`'s action `index`.
    pub fn param(&self, trigger: InteractionTrigger, index: usize) -> Option<&str> {
        self.action(trigger, index).map(|a| a.param.as_str())
    }

    pub fn action(
        &self,
        trigger: InteractionTrigger,
        index: usize,
    ) -> Option<&op_editor_core::interaction_edit::InteractionActionView> {
        self.triggers
            .iter()
            .find(|t| t.trigger == Some(trigger))
            .and_then(|t| t.actions.get(index))
    }

    pub fn action_count(&self, trigger: InteractionTrigger) -> usize {
        self.triggers
            .iter()
            .find(|t| t.trigger == Some(trigger))
            .map_or(0, |t| t.actions.len())
    }
}

/// Build the summary from a node's `events` + (when `is_top_level`)
/// its `screen` marker.
pub(crate) fn interactions_of(node: &PenNode, is_top_level: bool) -> InteractionSummary {
    InteractionSummary {
        screen: is_top_level
            .then(|| node.screen())
            .flatten()
            .map(str::to_string),
        triggers: summarize_events(node.events()),
        motion: op_editor_core::motion_summary::MotionSummary::of(node),
    }
}

// ── Rows ────────────────────────────────────────────────────────────

/// One painted row of the section, in paint order.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Screen,
    /// Trigger header, index into `summary.triggers`.
    Trigger(usize),
    /// Action row `(trigger_idx, action_idx)`.
    Action(usize, usize),
    /// Parameter row under an action.
    Param(usize, usize, InteractionParamKind),
    Add,
}

pub(crate) const ROW_STEP: f32 = INPUT_HEIGHT + 6.0;
const ACTION_INDENT: f32 = 12.0;
const PARAM_INDENT: f32 = 24.0;
const ICON_BUTTON_W: f32 = 26.0;

fn rows(summary: &InteractionSummary) -> Vec<Row> {
    let mut out = Vec::new();
    if summary.screen.is_some() {
        out.push(Row::Screen);
    }
    for (ti, trigger) in summary.triggers.iter().enumerate() {
        out.push(Row::Trigger(ti));
        for (ai, action) in trigger.actions.iter().enumerate() {
            out.push(Row::Action(ti, ai));
            // Read-only triggers / actions show no parameter editor.
            if let (Some(_), Some(kind)) = (trigger.trigger, action.kind) {
                let param = kind.param_kind();
                if param != InteractionParamKind::None {
                    out.push(Row::Param(ti, ai, param));
                }
            }
        }
    }
    out.push(Row::Add);
    out
}

fn is_text_param(kind: InteractionParamKind) -> bool {
    matches!(
        kind,
        InteractionParamKind::Text
            | InteractionParamKind::StateKey
            | InteractionParamKind::Duration
    )
}

/// Total vertical space the Interactions section consumes.
pub fn interactions_section_height(summary: &InteractionSummary) -> f32 {
    SECTION_HEADER_HEIGHT + rows(summary).len() as f32 * ROW_STEP + 6.0 + SECTION_GAP
}

struct RowGeom {
    row: Row,
    /// Main (dropdown / input) rect.
    main: Rect,
    /// Trailing icon button (`+` on triggers, `×` on actions).
    button: Option<Rect>,
}

fn row_geometry(summary: &InteractionSummary, x: f32, y: f32, width: f32) -> Vec<RowGeom> {
    let usable_w = width - PAD_X * 2.0;
    let mut row_y = y + SECTION_HEADER_HEIGHT;
    let mut out = Vec::new();
    for row in rows(summary) {
        let indent = match row {
            Row::Action(..) => ACTION_INDENT,
            Row::Param(..) => PARAM_INDENT,
            _ => 0.0,
        };
        let has_button = matches!(row, Row::Trigger(_) | Row::Action(..));
        let button_w = if has_button { ICON_BUTTON_W + 4.0 } else { 0.0 };
        let main = Rect {
            origin: Point2D::new(x + PAD_X + indent, row_y),
            size: Point2D::new(usable_w - indent - button_w, INPUT_HEIGHT),
        };
        let button = has_button.then(|| Rect {
            origin: Point2D::new(x + PAD_X + usable_w - ICON_BUTTON_W, row_y),
            size: Point2D::new(ICON_BUTTON_W, INPUT_HEIGHT),
        });
        out.push(RowGeom { row, main, button });
        row_y += ROW_STEP;
    }
    out
}

/// Emit the section's clickable rects (dropdowns + icon buttons).
/// Pushed main-rect first so `hit_test_action`'s reverse walk lets the
/// trailing icon button win where they touch.
pub fn push_interaction_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    summary: &InteractionSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    for g in row_geometry(summary, x, y, width) {
        match g.row {
            Row::Screen => {}
            Row::Trigger(ti) => {
                let view = &summary.triggers[ti];
                out.push((
                    PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::Trigger(
                        view.raw_key.clone(),
                    )),
                    g.main,
                ));
                let edit = match view.trigger {
                    Some(t) => InteractionEdit::AddAction(t),
                    None => InteractionEdit::RemoveTrigger(view.raw_key.clone()),
                };
                if let Some(button) = g.button {
                    out.push((PropertyPanelAction::EditInteraction(edit), button));
                }
            }
            Row::Action(ti, ai) => {
                let Some(trigger) = summary.triggers[ti].trigger else {
                    continue;
                };
                out.push((
                    PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::Action {
                        trigger,
                        index: ai,
                    }),
                    g.main,
                ));
                if let Some(button) = g.button {
                    out.push((
                        PropertyPanelAction::EditInteraction(InteractionEdit::RemoveAction {
                            trigger,
                            index: ai,
                        }),
                        button,
                    ));
                }
            }
            Row::Param(ti, ai, kind) if !is_text_param(kind) => {
                if let Some(trigger) = summary.triggers[ti].trigger {
                    out.push((
                        PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::Param {
                            trigger,
                            index: ai,
                        }),
                        g.main,
                    ));
                }
            }
            Row::Param(..) => {}
            Row::Add => out.push((
                PropertyPanelAction::ToggleInteractionMenu(InteractionMenuTarget::AddTrigger),
                g.main,
            )),
        }
    }
}

/// Emit the free-text parameter inputs (`PropertyFocus::InteractionParam`).
pub fn push_interaction_input_rects(
    out: &mut Vec<(PropertyFocus, Rect)>,
    summary: &InteractionSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    for g in row_geometry(summary, x, y, width) {
        if let Row::Param(ti, ai, kind) = g.row {
            if is_text_param(kind) {
                if let Some(trigger) = summary.triggers[ti].trigger {
                    out.push((PropertyFocus::InteractionParam(trigger, ai), g.main));
                }
            }
        }
    }
}

// ── Paint ───────────────────────────────────────────────────────────

/// Localized label with an English fallback.
pub(crate) fn tr(
    locale: op_editor_core::Locale,
    pair: (&'static str, &'static str),
) -> &'static str {
    let translated = crate::i18n::translate(locale, pair.0);
    if translated == pair.0 {
        pair.1
    } else {
        translated
    }
}

fn trigger_label(locale: op_editor_core::Locale, view: &InteractionTriggerView) -> String {
    match view.trigger {
        Some(t) => tr(locale, t.label()).to_string(),
        None => format!("{} ({})", view.raw_key, view.actions.len()),
    }
}

/// Label for a layer id: its name from the picker context, else the id.
pub(crate) fn node_label(ctx: &InteractionContext, id: &str) -> String {
    ctx.nodes
        .iter()
        .find(|n| n.id == id)
        .map(|n| n.name.clone())
        .unwrap_or_else(|| id.to_string())
}

pub(crate) fn kind_label(
    locale: op_editor_core::Locale,
    kind: InteractionActionKind,
) -> &'static str {
    tr(locale, kind.label())
}

#[allow(clippy::too_many_arguments)]
pub fn paint_interactions_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    summary: &InteractionSummary,
    ctx: &InteractionContext,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let start_y = y;
    paint_section_label(
        cx,
        theme,
        tr(locale, ("interactions.title", "Interactions")),
        x,
        y,
        width,
    );
    let geometry = row_geometry(summary, x, start_y, width);
    for g in &geometry {
        match &g.row {
            Row::Screen => paint_plain_row(
                cx,
                theme,
                g.main,
                &format!(
                    "{}: {}",
                    tr(locale, ("interactions.screen", "Screen")),
                    summary.screen.as_deref().unwrap_or_default()
                ),
            ),
            Row::Trigger(ti) => {
                let view = &summary.triggers[*ti];
                paint_dropdown(cx, theme, g.main, &trigger_label(locale, view));
                let icon = if view.trigger.is_some() {
                    Icon::Plus
                } else {
                    Icon::Close
                };
                if let Some(button) = g.button {
                    paint_icon_button(cx, theme, button, icon);
                }
            }
            Row::Action(ti, ai) => {
                let view = &summary.triggers[*ti];
                let action = &view.actions[*ai];
                let label = match action.kind {
                    Some(kind) => kind_label(locale, kind).to_string(),
                    None => format!("→ {}", action.raw_key),
                };
                if view.trigger.is_some() {
                    paint_dropdown(cx, theme, g.main, &label);
                    if let Some(button) = g.button {
                        paint_icon_button(cx, theme, button, Icon::Close);
                    }
                } else {
                    paint_plain_row(cx, theme, g.main, &label);
                }
            }
            Row::Param(ti, ai, kind) => {
                let Some(trigger) = summary.triggers[*ti].trigger else {
                    continue;
                };
                let param = &summary.triggers[*ti].actions[*ai].param;
                if is_text_param(*kind) {
                    let focus = PropertyFocus::InteractionParam(trigger, *ai);
                    let prefix = match kind {
                        InteractionParamKind::StateKey => {
                            tr(locale, ("interactions.param.state", "State"))
                        }
                        InteractionParamKind::Duration => {
                            tr(locale, ("interactions.param.ms", "ms"))
                        }
                        _ => tr(locale, ("interactions.param.text", "Text")),
                    };
                    paint_input_with_prefix_focused_state(
                        cx,
                        theme,
                        g.main,
                        prefix,
                        edit.value_for(focus, param),
                        edit.focus == Some(focus),
                        edit.caret_at(focus),
                        edit.select_all_at(focus),
                        edit.input_at(focus),
                        edit.now_ms,
                    );
                } else {
                    let label = match kind {
                        InteractionParamKind::Node if param.is_empty() => {
                            tr(locale, ("interactions.param.pickLayer", "Choose a layer"))
                                .to_string()
                        }
                        InteractionParamKind::Node => node_label(ctx, param),
                        _ => param.clone(),
                    };
                    paint_dropdown(cx, theme, g.main, &label);
                }
            }
            Row::Add => paint_dropdown(
                cx,
                theme,
                g.main,
                tr(locale, ("interactions.addInteraction", "+ Add interaction")),
            ),
        }
    }
    let y = start_y + SECTION_HEADER_HEIGHT + geometry.len() as f32 * ROW_STEP + 6.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

fn paint_icon_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon) {
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            rect.origin.x + (rect.size.x - 14.0) / 2.0,
            rect.origin.y + (rect.size.y - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.4,
    );
}

fn paint_plain_row(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, text: &str) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(rect.origin.x + 2.0, rect.origin.y + 19.0),
    );
}

#[cfg(test)]
#[path = "property_panel_interactions_tests.rs"]
mod tests;
