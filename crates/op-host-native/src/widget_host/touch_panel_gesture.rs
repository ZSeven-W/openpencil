//! One-finger touch arbitration for Asset Center, Property, Layers, Slides,
//! and the Property font picker.
//!
//! Native touch shells deliver mouse-shaped down/move/up events. A body down
//! therefore remains pending until release: crossing touch slop captures the
//! gesture for scrolling, while a stationary release replays the down point
//! through the ordinary press ladder exactly once.

use super::press_ctx::PressCtx;
use super::WidgetHostNative;
use op_editor_core::size_class::MobileSheetKind;
use op_editor_ui::Point2D;

const TOUCH_SCROLL_SLOP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::widget_host) enum TouchPanelTarget {
    AssetCenter,
    Property,
    FontPicker,
    Layers,
    Slides,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TouchScrollAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::widget_host) struct TouchPanelGesture {
    start: Point2D,
    last: Point2D,
    viewport_w: f32,
    viewport_h: f32,
    target: TouchPanelTarget,
    axis: Option<TouchScrollAxis>,
    scrolling: bool,
}

impl WidgetHostNative {
    /// Defer Asset Center body controls and cards until release so the same
    /// finger can promote to gallery scrolling without selecting an asset.
    /// Close, tabs, and the outside scrim keep their immediate press path.
    pub(in crate::widget_host) fn begin_asset_center_touch_gesture(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.touch_chrome() {
            return false;
        }
        // A second down delivered by a shell is multi-pointer intent, never a
        // replacement tap. Drop the first delayed action instead of moving it
        // to the new finger and replaying either card on release.
        if self.touch_panel_gesture.is_some() {
            self.cancel_touch_panel_gesture();
            return true;
        }
        let Some(rect) = self.scene_template_panel_rect(viewport_w, viewport_h) else {
            return false;
        };
        let point = Point2D::new(x, y);
        if !rect.contains(point) {
            return false;
        }
        let Some(panel) = op_editor_ui::widgets::SceneTemplatePanel::for_editor_at(
            &self.editor_state,
            self.now_ms,
        ) else {
            return false;
        };
        if matches!(
            panel.hit_test(rect, point),
            Some(
                op_editor_ui::widgets::SceneTemplateHit::Close
                    | op_editor_ui::widgets::SceneTemplateHit::SelectTab(_)
            )
        ) {
            return false;
        }
        self.arm_touch_panel_gesture_at(
            point,
            viewport_w,
            viewport_h,
            TouchPanelTarget::AssetCenter,
        );
        true
    }

    /// Defer a press inside the Property font picker so a row tap and a
    /// one-finger list scroll do not both fire. The popup can extend outside
    /// the inspector rect, so it must be armed from the overlay tier rather
    /// than the Property body tier below it.
    pub(in crate::widget_host) fn begin_font_picker_touch_gesture(
        &mut self,
        ctx: &PressCtx,
    ) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome() || !ui.font_picker.open || ctx.in_git_panel {
            return false;
        }
        let Some(panel) =
            op_editor_ui::widgets::PropertyPanel::for_selection_at(&self.editor_state, self.now_ms)
        else {
            return false;
        };
        let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
            &self.editor_state,
            ctx.viewport_width,
            ctx.viewport_height,
        );
        if !panel.font_picker_contains(rect, Point2D::new(ctx.x, ctx.y)) {
            return false;
        }
        self.arm_touch_panel_gesture(ctx, TouchPanelTarget::FontPicker);
        true
    }

    /// Arm after property overlays and sheet chrome have declined the press.
    pub(in crate::widget_host) fn begin_property_touch_gesture(&mut self, ctx: &PressCtx) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome()
            || !(ui.mobile_sheet == Some(MobileSheetKind::Properties)
                || (ui.expanded_touch_layout() && ui.mobile_sheet.is_none()))
            || ctx.in_git_panel
        {
            return false;
        }
        let Some(panel) =
            op_editor_ui::widgets::PropertyPanel::for_selection_at(&self.editor_state, self.now_ms)
        else {
            return false;
        };
        let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
            &self.editor_state,
            ctx.viewport_width,
            ctx.viewport_height,
        );
        let point = Point2D::new(ctx.x, ctx.y);
        if !rect.contains(point) || panel.tab_hover_at(rect, point).is_some() {
            return false;
        }
        self.arm_touch_panel_gesture(ctx, TouchPanelTarget::Property);
        true
    }

    /// Arm after the Slides sub-surface has declined and before ordinary
    /// LayerPanel code can seed its mouse reorder candidate.
    pub(in crate::widget_host) fn begin_layers_touch_gesture(&mut self, ctx: &PressCtx) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome()
            || !(ui.mobile_sheet == Some(MobileSheetKind::Layers)
                || (ui.expanded_touch_layout() && ui.sidebar_open && ui.mobile_sheet.is_none()))
        {
            return false;
        }
        let point = Point2D::new(ctx.x, ctx.y);
        if !self
            .layers_content_rect(ctx.viewport_width, ctx.viewport_height)
            .contains(point)
        {
            return false;
        }
        self.arm_touch_panel_gesture(ctx, TouchPanelTarget::Layers);
        true
    }

    /// Arm before `slides_panel_press` can seed its mouse reorder gesture.
    ///
    /// Only the scrolling list defers. Tabs, footer actions, and the export
    /// menu keep their ordinary press/release path because none of them can
    /// turn into list scrolling.
    pub(in crate::widget_host) fn begin_slides_touch_gesture(&mut self, ctx: &PressCtx) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome()
            || ui.slides_panel.export_menu_open
            || !(ui.mobile_sheet == Some(MobileSheetKind::Layers)
                || (ui.expanded_touch_layout() && ui.sidebar_open && ui.mobile_sheet.is_none()))
        {
            return false;
        }
        let point = Point2D::new(ctx.x, ctx.y);
        let Some(slides) = self.slides_panel_frame(ctx.viewport_width, ctx.viewport_height) else {
            return false;
        };
        if !slides.layout.list.contains(point) {
            return false;
        }
        self.arm_touch_panel_gesture(ctx, TouchPanelTarget::Slides);
        true
    }

    fn arm_touch_panel_gesture(&mut self, ctx: &PressCtx, target: TouchPanelTarget) {
        self.arm_touch_panel_gesture_at(
            Point2D::new(ctx.x, ctx.y),
            ctx.viewport_width,
            ctx.viewport_height,
            target,
        );
    }

    fn arm_touch_panel_gesture_at(
        &mut self,
        point: Point2D,
        viewport_w: f32,
        viewport_h: f32,
        target: TouchPanelTarget,
    ) {
        self.layer_drag = None;
        if target == TouchPanelTarget::Slides {
            // A touch down never seeds the desktop SlidesDrag. Drop any stale
            // pointer bookkeeping before holding the tap for release.
            self.editor_state.editor_ui.slides_panel.clear_pointer();
        }
        self.touch_panel_gesture = Some(TouchPanelGesture {
            start: point,
            last: point,
            viewport_w,
            viewport_h,
            target,
            axis: None,
            scrolling: false,
        });
    }

    /// Crossing slop permanently promotes the gesture to panel scrolling.
    pub(in crate::widget_host) fn update_touch_panel_gesture(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let mut gesture = self.touch_panel_gesture?;
        if !self.touch_panel_target_is_visible(gesture.target) {
            self.cancel_touch_panel_gesture();
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
            gesture.axis = Some(self.touch_panel_scroll_axis(gesture));
            self.layer_drag = None;
        }

        let dx = point.x - gesture.last.x;
        let dy = point.y - gesture.last.y;
        gesture.last = point;
        self.touch_panel_gesture = Some(gesture);
        // The shared scroll flows negate their input before advancing the
        // stored offset, so passing the finger delta directly makes content
        // track the finger (an upward move advances the panel offset).
        let scroll_dx = dx;
        let scroll_dy = dy;
        let consumed = match gesture.target {
            TouchPanelTarget::AssetCenter => self.try_scroll_scene_template_center(
                gesture.start.x,
                gesture.start.y,
                scroll_dy,
                gesture.viewport_w,
                gesture.viewport_h,
            ),
            TouchPanelTarget::Property => self.try_scroll_property_panel_2d(
                gesture.start.x,
                gesture.start.y,
                if gesture.axis == Some(TouchScrollAxis::Horizontal) {
                    scroll_dx
                } else {
                    0.0
                },
                if gesture.axis == Some(TouchScrollAxis::Vertical) {
                    scroll_dy
                } else {
                    0.0
                },
                gesture.viewport_w,
                gesture.viewport_h,
            ),
            TouchPanelTarget::FontPicker => self.try_scroll_font_picker(
                gesture.start.x,
                gesture.start.y,
                scroll_dy,
                gesture.viewport_w,
                gesture.viewport_h,
            ),
            TouchPanelTarget::Layers => {
                // Touch scrolling and mouse-style layer reorder are mutually
                // exclusive for the entire gesture.
                self.layer_drag = None;
                self.try_scroll_layer_panel(
                    gesture.start.x,
                    gesture.start.y,
                    scroll_dx,
                    scroll_dy,
                    gesture.viewport_w,
                    gesture.viewport_h,
                )
            }
            TouchPanelTarget::Slides => {
                self.editor_state.editor_ui.slides_panel.clear_pointer();
                let dirty = self.slides_panel_scroll(
                    gesture.start,
                    scroll_dy,
                    gesture.viewport_w,
                    gesture.viewport_h,
                );
                if dirty == Some(true) {
                    self.mark_dirty();
                }
                dirty.is_some()
            }
        };
        // Crossing slop owns the event even when the panel has no remaining
        // overflow in this direction; never leak the move into canvas drags.
        Some(consumed || gesture.scrolling)
    }

    /// A stationary release dispatches the original down point exactly once.
    /// A scroll release only ends capture and never commits a reorder.
    pub(in crate::widget_host) fn release_touch_panel_gesture(&mut self) -> Option<bool> {
        let gesture = self.touch_panel_gesture.take()?;
        self.layer_drag = None;
        if gesture.scrolling || !self.touch_panel_target_is_visible(gesture.target) {
            return Some(true);
        }
        let consumed = self.replay_touch_panel_press(
            gesture.start.x,
            gesture.start.y,
            gesture.viewport_w,
            gesture.viewport_h,
        );
        let consumed = if gesture.target == TouchPanelTarget::Slides {
            // Slides actions resolve on release. Replaying just the down would
            // leave `pressed` and `drag` armed until an unrelated pointer-up;
            // close the replay immediately so a tap activates exactly once.
            self.slides_panel_release(gesture.viewport_w, gesture.viewport_h) || consumed
        } else {
            consumed
        };
        // A replayed Layer row follows the normal click path, which seeds a
        // desktop reorder candidate. Touch has already ended, so discard it.
        self.layer_drag = None;
        Some(consumed)
    }

    pub(in crate::widget_host) fn cancel_touch_panel_gesture(&mut self) -> bool {
        // Take both independently: short-circuiting `||` here would leave a
        // reorder candidate alive whenever a deferred panel tap also existed.
        let touch = self.touch_panel_gesture.take();
        if touch.is_some_and(|gesture| gesture.target == TouchPanelTarget::Slides) {
            self.editor_state.editor_ui.slides_panel.clear_pointer();
        }
        let layer = self.layer_drag.take().is_some();
        touch.is_some() || layer
    }

    /// Cancel every in-flight native pointer gesture without routing it
    /// through the release ladder. This is the platform `touchesCancelled` /
    /// `ACTION_CANCEL` seam: release-delayed taps are discarded, preview gets
    /// a real Cancel phase, and drag-only history/drop commits never run.
    pub fn cancel_native_touch_gestures(&mut self) -> bool {
        let mut cancelled = self.agent_settings_touch_gesture.take().is_some();
        cancelled |= self.cancel_touch_panel_gesture();

        cancelled |= self.editor_state.editor_ui.pressed_button.take().is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .git_panel
            .tracked_picker
            .pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .icon_picker
            .pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .chat_model_picker
            .pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .preview
            .switcher_pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .preview
            .screen_switcher_pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .preview
            .toolbar_pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .slides_panel
            .pressed
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .slides_panel
            .drag
            .take()
            .is_some();
        cancelled |= self
            .editor_state
            .editor_ui
            .agent_settings_drag
            .take()
            .is_some();
        if self
            .editor_state
            .ui
            .color_picker
            .as_ref()
            .is_some_and(|picker| picker.drag.is_some())
        {
            self.editor_state.color_picker_set_drag(None);
            cancelled = true;
        }
        if self.editor_state.ui.pen_dragging_handle {
            self.editor_state.pen_release();
            cancelled = true;
        }

        if std::mem::take(&mut self.preview_press_active) {
            if let Some((x, y)) = self.preview_last_doc {
                if let Some(preview) = self.preview.as_mut() {
                    preview.dispatch_pointer_phase(
                        x,
                        y,
                        jian_core::gesture::pointer::PointerPhase::Cancel,
                    );
                }
            }
            self.preview_last_doc = None;
            self.preview_surface_capture = None;
            cancelled = true;
        }
        cancelled |= self.preview_edge_swipe_start_x.take().is_some();
        cancelled |= self.slideshow_press_screen.take().is_some();

        cancelled |= self.drag.take().is_some();
        cancelled |= self.chat_drag.take().is_some();
        cancelled |= self.chat_resize.take().is_some();
        cancelled |= self.design_md_drag.take().is_some();
        cancelled |= self.component_browser_drag.take().is_some();
        cancelled |= self.icon_picker_drag.take().is_some();
        cancelled |= self.image_adjustment_drag.take().is_some();
        cancelled |= self.image_crop_drag.take().is_some();
        cancelled |= self.effect_radius_drag.take().is_some();
        cancelled |= self.code_selection_drag.take().is_some();
        cancelled |= self.chat_input_selection_drag.take().is_some();
        cancelled |= self.image_input_selection_drag.take().is_some();
        cancelled |= self.chat_text_selection_drag.take().is_some();
        cancelled |= self.text_edit_selection_drag.take().is_some();
        cancelled |= self.panel_resize.take().is_some();
        cancelled |= self.variables_resize.take().is_some();
        cancelled |= self.node_drag.take().is_some();
        cancelled |= self.handle_drag.take().is_some();
        cancelled |= self.rotate_drag.take().is_some();
        cancelled |= self.create_drag.take().is_some();
        cancelled |= self.marquee_drag.take().is_some();
        cancelled |= self.layer_drag.take().is_some();
        cancelled |= self.path_anchor_drag.take().is_some();
        cancelled |= self.arc_handle_drag.take().is_some();

        if !self.option_drag_source_ids.is_empty() {
            self.option_drag_source_ids.clear();
            cancelled = true;
        }
        if !self.editor_state.editor_ui.active_guides.is_empty() {
            self.editor_state.editor_ui.active_guides.clear();
            cancelled = true;
        }
        cancelled |= self
            .editor_state
            .editor_ui
            .canvas_drop_indicator
            .take()
            .is_some();
        if cancelled {
            self.mark_dirty();
        }
        cancelled
    }

    fn touch_panel_target_is_visible(&self, target: TouchPanelTarget) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome() {
            return false;
        }
        match target {
            TouchPanelTarget::AssetCenter => ui.scene_template_center.open,
            TouchPanelTarget::Property => {
                (ui.mobile_sheet == Some(MobileSheetKind::Properties)
                    || (ui.expanded_touch_layout() && ui.mobile_sheet.is_none()))
                    && op_editor_ui::widgets::PropertyPanel::for_selection_at(
                        &self.editor_state,
                        self.now_ms,
                    )
                    .is_some()
            }
            TouchPanelTarget::FontPicker => {
                ui.font_picker.open
                    && (ui.mobile_sheet == Some(MobileSheetKind::Properties)
                        || (ui.expanded_touch_layout() && ui.mobile_sheet.is_none()))
                    && op_editor_ui::widgets::PropertyPanel::for_selection_at(
                        &self.editor_state,
                        self.now_ms,
                    )
                    .is_some()
            }
            TouchPanelTarget::Layers => {
                ui.mobile_sheet == Some(MobileSheetKind::Layers)
                    || (ui.expanded_touch_layout() && ui.sidebar_open && ui.mobile_sheet.is_none())
            }
            TouchPanelTarget::Slides => {
                (ui.mobile_sheet == Some(MobileSheetKind::Layers)
                    || (ui.expanded_touch_layout() && ui.sidebar_open && ui.mobile_sheet.is_none()))
                    && op_editor_ui::widgets::slides_panel_flow::slides_tab_active(
                        &self.editor_state,
                    )
            }
        }
    }

    fn touch_panel_scroll_axis(&self, gesture: TouchPanelGesture) -> TouchScrollAxis {
        if gesture.target != TouchPanelTarget::Property
            || !matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            )
        {
            return TouchScrollAxis::Vertical;
        }
        let Some(panel) =
            op_editor_ui::widgets::PropertyPanel::for_selection_at(&self.editor_state, self.now_ms)
        else {
            return TouchScrollAxis::Vertical;
        };
        let rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
            &self.editor_state,
            gesture.viewport_w,
            gesture.viewport_h,
        );
        if panel.code_framework_strip_contains(rect, gesture.start) {
            TouchScrollAxis::Horizontal
        } else {
            TouchScrollAxis::Vertical
        }
    }
}
