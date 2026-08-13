//! One-finger touch arbitration for the Agent Settings content body.
//!
//! Touch shells report the same press/move/release stream as a mouse. A body
//! press therefore stays pending until release: crossing the touch slop turns
//! it into scrolling, while a stationary release dispatches the original hit
//! exactly once. Header controls keep their ordinary immediate press path.

use super::WidgetHostNative;
use op_editor_ui::Point2D;

const TOUCH_SCROLL_SLOP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::widget_host) struct AgentSettingsTouchGesture {
    start: Point2D,
    last: Point2D,
    viewport_w: f32,
    viewport_h: f32,
    target: AgentSettingsTouchTarget,
    scrolling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentSettingsTouchTarget {
    Body,
    FontPicker,
}

impl WidgetHostNative {
    /// Arm a release-delayed press only for the responsive content viewport.
    /// Close and navigation controls live outside this rect and continue
    /// through the normal immediate dispatcher.
    pub(in crate::widget_host) fn begin_agent_settings_touch_gesture(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.touch_chrome()
            || !self.editor_state.editor_ui.agent_settings_open
        {
            self.agent_settings_touch_gesture = None;
            return false;
        }

        let point = Point2D::new(x, y);
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_w, viewport_h);
        let target = if panel
            .font_picker_layout(panel_rect)
            .is_some_and(|layout| layout.popup.contains(point))
        {
            AgentSettingsTouchTarget::FontPicker
        } else if panel.resolved_content_viewport(panel_rect).contains(point) {
            AgentSettingsTouchTarget::Body
        } else {
            return false;
        };

        self.agent_settings_touch_gesture = Some(AgentSettingsTouchGesture {
            start: point,
            last: point,
            viewport_w,
            viewport_h,
            target,
            scrolling: false,
        });
        true
    }

    /// Consume a pending touch move before the ordinary modal-hover ladder.
    /// Returning `Some` means the gesture owns the event even when no repaint
    /// was needed (for example, sub-slop jitter or a non-overflowing page).
    pub(in crate::widget_host) fn update_agent_settings_touch_gesture(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let mut gesture = self.agent_settings_touch_gesture?;
        if !self.editor_state.editor_ui.touch_chrome()
            || !self.editor_state.editor_ui.agent_settings_open
        {
            self.agent_settings_touch_gesture = None;
            return Some(false);
        }

        let point = Point2D::new(x, y);
        if !gesture.scrolling {
            let dx = point.x - gesture.start.x;
            let dy = point.y - gesture.start.y;
            if dx * dx + dy * dy <= TOUCH_SCROLL_SLOP * TOUCH_SCROLL_SLOP {
                return Some(false);
            }
            gesture.scrolling = true;
        }

        let delta_y = point.y - gesture.last.y;
        gesture.last = point;
        self.agent_settings_touch_gesture = Some(gesture);
        Some(match gesture.target {
            AgentSettingsTouchTarget::Body => self.scroll_agent_settings_at(
                gesture.start.x,
                gesture.start.y,
                delta_y,
                gesture.viewport_w,
                gesture.viewport_h,
            ),
            AgentSettingsTouchTarget::FontPicker => self.try_scroll_settings_font_picker(
                gesture.start.x,
                gesture.start.y,
                delta_y,
                gesture.viewport_w,
                gesture.viewport_h,
            ),
        })
    }

    /// Finish the owned gesture before generic release feedback. A tap uses
    /// the down point, so minor finger jitter cannot redirect the action.
    pub(in crate::widget_host) fn release_agent_settings_touch_gesture(&mut self) -> Option<bool> {
        let gesture = self.agent_settings_touch_gesture.take()?;
        if gesture.scrolling
            || !self.editor_state.editor_ui.touch_chrome()
            || !self.editor_state.editor_ui.agent_settings_open
        {
            return Some(true);
        }
        Some(self.dispatch_agent_settings_press(
            gesture.start.x,
            gesture.start.y,
            gesture.viewport_w,
            gesture.viewport_h,
        ))
    }

    pub(in crate::widget_host) fn cancel_agent_settings_touch_gesture(&mut self) {
        self.agent_settings_touch_gesture = None;
    }
}
