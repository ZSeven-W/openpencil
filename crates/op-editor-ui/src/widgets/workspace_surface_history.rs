//! Visible recovery actions on the normal-mode toolbar.

use super::{header_button_width, StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Point2D, Rect};
use op_editor_core::WorkspaceHit;

pub(super) const MIN_HISTORY_SPACE: f32 = 76.0;

impl WorkspaceSurface<'_> {
    /// Fit between the view controls and pager, using the same geometry for
    /// paint, pointer and keyboard. Long translations shorten inside the button.
    pub fn history_buttons(&self, layout: &WorkspaceLayout) -> Vec<(WorkspaceHit, Rect, bool)> {
        let Some(last) = layout.view_segments.last() else {
            return Vec::new();
        };
        let mut x = last.origin.x + last.size.x + 12.0;
        let available = (layout.prev.unwrap_or(layout.zoom_out).origin.x - 12.0 - x).max(0.0);
        if available < 64.0 {
            return Vec::new();
        }
        let actions = [
            (WorkspaceHit::Undo, "toolbar.undo", self.history.undo),
            (WorkspaceHit::Redo, "toolbar.redo", self.history.redo),
        ];
        let widths =
            actions.map(|(_, key, _)| header_button_width(op_i18n::translate(self.ui.locale, key)));
        let scale = ((available - 6.0) / widths.iter().sum::<f32>()).min(1.0);
        actions
            .into_iter()
            .zip(widths)
            .map(|((hit, _, enabled), width)| {
                let button = Rect::xywh(x, last.origin.y - 1.0, width * scale, 28.0);
                x += button.size.x + 6.0;
                (hit, button, enabled)
            })
            .collect()
    }

    pub(super) fn history_hit(
        &self,
        layout: &WorkspaceLayout,
        point: Point2D,
    ) -> Option<WorkspaceHit> {
        self.history_buttons(layout)
            .into_iter()
            .find_map(|(hit, rect, enabled)| (enabled && rect.contains(point)).then_some(hit))
    }

    pub(super) fn paint_history(
        &self,
        cx: &mut PaintCx<'_>,
        layout: &WorkspaceLayout,
        palette: StudioPalette,
    ) {
        for (hit, rect, enabled) in self.history_buttons(layout) {
            let (key, icon) = match hit {
                WorkspaceHit::Undo => ("toolbar.undo", Icon::Undo),
                _ => ("toolbar.redo", Icon::Redo),
            };
            let color = if enabled {
                palette.ink
            } else {
                palette.muted.with_alpha(palette.muted.a * 0.4)
            };
            if enabled && self.state.hover == Some(hit) {
                cx.backend.fill_round_rect(rect, 6.0, palette.button_hover);
            }
            draw_icon(
                cx.backend,
                icon,
                Point2D::new(rect.origin.x + 7.0, rect.origin.y + 6.0),
                16.0,
                color,
                1.6,
            );
            if rect.size.x >= 54.0 {
                let text_rect = Rect::xywh(
                    rect.origin.x + 28.0,
                    rect.origin.y,
                    rect.size.x - 32.0,
                    rect.size.y,
                );
                let label = text_metrics::fit_chrome(
                    cx.backend,
                    op_i18n::translate(self.ui.locale, key),
                    text_rect.size.x,
                    12.0,
                );
                let text = crate::TextLayout::single_run(
                    &label,
                    "system-ui",
                    12.0,
                    color.to_jian(),
                    Point2D::ZERO,
                );
                cx.backend.save();
                cx.backend.clip_rect(text_rect);
                cx.backend.draw_text(
                    &text,
                    Point2D::new(
                        text_rect.origin.x,
                        jian_widgets::centered_text_baseline_y(text_rect, 12.0),
                    ),
                );
                cx.backend.restore();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        EditorCommand, EditorState, HomeFamily, NodeId, PenNodeExt, WorkspacePhase,
    };

    #[test]
    fn recovery_buttons_fit_translated_toolbars_and_skip_disabled_keyboard_targets() {
        for locale in op_i18n::Locale::ALL {
            for (width, dock) in [
                (800.0, 320.0),
                (900.0, 440.0),
                (960.0, 440.0),
                (1024.0, 440.0),
                (1440.0, 320.0),
            ] {
                let mut state = EditorState::starter();
                state.editor_ui.locale = locale;
                state.editor_ui.sidebar_open = true;
                state.editor_ui.layer_panel_width = dock;
                state
                    .editor_ui
                    .workspace
                    .open_for_reading(HomeFamily::AppUi, 1);
                state.editor_ui.workspace.sync_drawer_mode(width);
                state.with_history_group(|state| {
                    assert!(state.apply(EditorCommand::SetNodeName {
                        node_id: NodeId::new(state.active_children()[0].id_str()),
                        name: "Edited".into()
                    }));
                });
                let surface = WorkspaceSurface::for_editor(&state).unwrap();
                let layout = surface.layout(width, 900.0);
                let buttons = surface.history_buttons(&layout);
                assert_eq!(buttons.len(), 2, "{locale:?}, {width}");
                assert!(buttons[0].2);
                for (hit, rect, enabled) in &buttons {
                    let point = Point2D::new(
                        rect.origin.x + rect.size.x / 2.0,
                        rect.origin.y + rect.size.y / 2.0,
                    );
                    assert!(
                        rect.origin.x
                            >= layout.view_segments.last().unwrap().origin.x
                                + layout.view_segments.last().unwrap().size.x
                    );
                    assert!(rect.origin.x + rect.size.x <= layout.prev.unwrap().origin.x);
                    assert_eq!(
                        surface.hit_test_layout(&layout, point),
                        enabled.then_some(*hit)
                    );
                    assert_eq!(
                        surface
                            .focus_order(&layout)
                            .iter()
                            .any(|(target, _)| target == hit),
                        *enabled
                    );
                }
                state.editor_ui.workspace.phase = WorkspacePhase::Generating;
                let surface = WorkspaceSurface::for_editor(&state).unwrap();
                assert!(surface
                    .history_buttons(&surface.layout(width, 900.0))
                    .iter()
                    .all(|(_, _, enabled)| !enabled));
            }
        }
    }
}
