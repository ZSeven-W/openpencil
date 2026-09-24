//! Keyboard navigation for the desktop Studio surfaces (Home and the
//! generation workspace): Tab / Shift+Tab move a focus ring through the
//! surface's validated focus order, Enter / Space activate the focused
//! target.
//!
//! Activation is a press at the target's centre through the surface's
//! own press tier (`press_home` / `press_workspace`) — the focus order
//! only contains targets whose centre hit-tests to themselves — so a key
//! can never do something a click on the same control would not.

use super::WidgetHostNative;
use op_editor_core::{HomeHit, WorkspaceHit};
use op_editor_ui::widgets::{HomeSurface, WorkspaceSurface, HOME_TOPBAR_H};
use op_editor_ui::Rect;

fn centre(rect: Rect) -> (f32, f32) {
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// The target after (or before, `backward`) `current` in `order`,
/// wrapping; the first (or last) target when nothing is focused yet or
/// the focused one left the order.
fn step<T: PartialEq + Copy>(order: &[T], current: Option<T>, backward: bool) -> Option<T> {
    if order.is_empty() {
        return None;
    }
    let len = order.len();
    let index = current.and_then(|focus| order.iter().position(|each| *each == focus));
    let next = match (index, backward) {
        (None, false) => 0,
        (None, true) => len - 1,
        (Some(index), false) => (index + 1) % len,
        (Some(index), true) => (index + len - 1) % len,
    };
    Some(order[next])
}

impl WidgetHostNative {
    /// Whether a Studio surface owns the keyboard's Tab navigation right
    /// now: desktop Home or the desktop workspace, with no modal layer
    /// (settings, sign-in, export, template / prompt centres, the model
    /// picker, a presentation) above it.
    pub fn studio_focus_available(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        if ui.touch_chrome()
            || self.preview.is_some()
            || ui.agent_settings_open
            || ui.login_modal_open
            || ui.account_menu_open
            || ui.export_dialog_open
            || ui.save_name_dialog.open
            || ui.scene_template_center.open
            || ui.prompt_center.open
            || ui.chat_model_picker.open
        {
            return false;
        }
        ui.home.visible || self.workspace_visible()
    }

    /// Tab / Shift+Tab: move the Studio focus ring. Returns whether the
    /// key was consumed.
    pub fn apply_studio_focus_step(&mut self, backward: bool) -> bool {
        if !self.studio_focus_available() {
            return false;
        }
        let (w, h) = (self.last_viewport_w, self.last_viewport_h);
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        if self.editor_state.editor_ui.home.visible {
            let (next, scroll) = {
                let Some(home) = HomeSurface::for_editor_at(&self.editor_state, self.now_ms) else {
                    return false;
                };
                let order = home.focus_order(w, h);
                let next = step(&order, home.state.key_focus, backward);
                let scroll = next.and_then(|hit| home.focus_scroll_for(w, h, hit, HOME_TOPBAR_H));
                (next, scroll)
            };
            let home = &mut self.editor_state.editor_ui.home;
            home.key_focus = next;
            if let Some(scroll) = scroll {
                let chip_w = op_editor_ui::widgets::home_surface::model_chip_width(
                    &op_editor_ui::widgets::home_surface::model_chip_label(&self.editor_state),
                );
                let compact = self.editor_state.editor_ui.compact_layout();
                let max = self.home_max_scroll(w, h, chip_w, compact);
                self.editor_state.editor_ui.home.scroll_y = scroll.clamp(0.0, max);
            }
        } else {
            let next = {
                let Some(surface) =
                    WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)
                else {
                    return false;
                };
                let layout = surface.layout(w, h);
                let order: Vec<WorkspaceHit> = surface
                    .focus_order(&layout)
                    .into_iter()
                    .map(|(hit, _)| hit)
                    .collect();
                step(&order, surface.state.key_focus, backward)
            };
            self.editor_state.editor_ui.workspace.key_focus = next;
            // The chat input would otherwise keep swallowing the keys the
            // focused control is meant to receive.
            self.editor_state.chat.blur_input(self.now_ms);
        }
        self.mark_dirty();
        true
    }

    /// Whether Enter / Space should activate a focused Studio target
    /// instead of their ordinary meaning. The composer box is a text
    /// field: with it focused, Enter still sends and Space still types.
    pub fn studio_key_focus_activatable(&self) -> bool {
        if !self.studio_focus_available() {
            return false;
        }
        let ui = &self.editor_state.editor_ui;
        if ui.home.visible {
            return ui.home.key_focus.is_some_and(|hit| hit != HomeHit::Sheet);
        }
        ui.workspace.key_focus.is_some()
    }

    /// Enter / Space on the focused Studio target: a press at its centre
    /// through the surface's own press tier. The focus stays where it was
    /// (the press would otherwise drop it as a pointer interaction).
    pub fn activate_studio_key_focus(&mut self) -> bool {
        if !self.studio_key_focus_activatable() {
            return false;
        }
        let (w, h) = (self.last_viewport_w, self.last_viewport_h);
        if self.editor_state.editor_ui.home.visible {
            let Some(focus) = self.editor_state.editor_ui.home.key_focus else {
                return false;
            };
            let target = HomeSurface::for_editor_at(&self.editor_state, self.now_ms)
                .and_then(|home| home.focus_rect(&home.layout(w, h), focus));
            let Some(rect) = target else {
                return false;
            };
            let (x, y) = centre(rect);
            self.press_home(x, y, w, h);
            let home = &mut self.editor_state.editor_ui.home;
            home.pressed = None;
            // The press may have closed the surface (专业画布, a recent
            // file) — keep the focus only while Home is still up.
            if home.visible {
                home.key_focus = Some(focus);
            }
        } else {
            let Some(focus) = self.editor_state.editor_ui.workspace.key_focus else {
                return false;
            };
            let target = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms).and_then(
                |surface| {
                    let layout = surface.layout(w, h);
                    surface
                        .focus_order(&layout)
                        .into_iter()
                        .find(|(hit, _)| *hit == focus)
                        .map(|(_, rect)| rect)
                },
            );
            let Some(rect) = target else {
                return false;
            };
            let (x, y) = centre(rect);
            self.press_workspace(x, y, w, h);
            let workspace = &mut self.editor_state.editor_ui.workspace;
            workspace.pressed = None;
            // A dock-resize press starts a drag; a key has no release.
            self.workspace_dock_drag = None;
            if workspace.visible {
                workspace.key_focus = Some(focus);
            }
        }
        self.mark_dirty();
        true
    }

    /// Escape rung: drop the Studio focus ring. Returns whether one was up.
    pub(in crate::widget_host) fn escape_studio_key_focus(&mut self) -> bool {
        let ui = &mut self.editor_state.editor_ui;
        let had = ui.home.key_focus.take().is_some() | ui.workspace.key_focus.take().is_some();
        if had {
            self.mark_dirty();
        }
        had
    }
}

#[cfg(test)]
#[path = "studio_focus_tests.rs"]
mod tests;
