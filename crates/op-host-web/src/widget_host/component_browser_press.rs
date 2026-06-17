//! Component-Browser panel press dispatch — mirror of the native
//! host's `widget_host/component_browser_press.rs`.
//!
//! Insert requests raise the same `component_browser_pending_insert`
//! flag as native; the web host drains it synchronously right after
//! the press (it has no per-frame runner drain like the desktop loop).

use op_editor_ui::widgets::{ComponentBrowserHit, ComponentBrowserPanel};
use op_editor_ui::Point2D;

use super::{PanelDragState, WidgetHost};

impl WidgetHost {
    /// Dispatch a press inside the floating Component-Browser panel.
    ///
    /// Returns `true` when consumed — a close, a category-pill swap,
    /// a queued insert request, a drag start, or a swallowed click.
    pub(in crate::widget_host) fn dispatch_component_browser_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.component_browser_panel_rect(viewport_width, viewport_height)
        else {
            return false;
        };
        let point = Point2D::new(x, y);
        let Some((hit, pressed_button)) = ComponentBrowserPanel::for_editor(&self.editor_state)
            .and_then(|p| {
                Some((
                    p.hit_test(panel_rect, point)?,
                    p.hover_at(panel_rect, point),
                ))
            })
        else {
            return false;
        };
        if let Some(button) = pressed_button {
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::ComponentBrowser(button));
        }
        match hit {
            ComponentBrowserHit::Close => {
                let ui = &mut self.editor_state.editor_ui;
                ui.component_browser_open = false;
                ui.component_browser_hover = None;
                ui.component_browser_kit_picker_open = false;
                ui.component_browser_confirm_delete_kit = None;
            }
            ComponentBrowserHit::ExportKit => {
                self.editor_state.editor_ui.component_browser_kit_request =
                    Some(op_editor_core::KitIoRequest::Export);
            }
            ComponentBrowserHit::ImportKit => {
                self.editor_state.editor_ui.component_browser_kit_request =
                    Some(op_editor_core::KitIoRequest::Import);
            }
            ComponentBrowserHit::ToggleKitPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.component_browser_kit_picker_open = !ui.component_browser_kit_picker_open;
            }
            ComponentBrowserHit::SelectKitFilter(kit_id) => {
                let ui = &mut self.editor_state.editor_ui;
                ui.component_browser_kit_id = kit_id;
                ui.component_browser_kit_picker_open = false;
            }
            ComponentBrowserHit::RequestDeleteKit(kit_id) => {
                self.editor_state
                    .editor_ui
                    .component_browser_confirm_delete_kit = Some(kit_id);
            }
            ComponentBrowserHit::ConfirmDeleteKit(kit_id) => {
                // State-only on web (no persistence file): the kit
                // disappears for this session.
                let _ = self.editor_state.remove_kit(&kit_id);
            }
            ComponentBrowserHit::CancelDeleteKit => {
                self.editor_state
                    .editor_ui
                    .component_browser_confirm_delete_kit = None;
            }
            ComponentBrowserHit::DragHeader => {
                self.component_browser_drag = Some(PanelDragState {
                    grab_dx: x - panel_rect.origin.x,
                    grab_dy: y - panel_rect.origin.y,
                });
            }
            ComponentBrowserHit::SelectCategory(cat) => {
                self.editor_state.editor_ui.component_browser_category = cat;
            }
            ComponentBrowserHit::InsertComponent(kit_id, comp_id) => {
                // Same flag contract as native; `apply_press` drains it
                // against this viewport immediately after dispatch.
                self.editor_state.editor_ui.component_browser_pending_insert =
                    Some((kit_id, comp_id));
            }
            ComponentBrowserHit::Inside => {
                // Blank press on panel chrome — blur chrome inputs
                // (the browser's search box stays live while open).
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.mark_dirty();
        true
    }

    /// Drain a queued Component-Browser insert: place the chosen UIKit
    /// component at the viewport's centre (top-left = centre − half the
    /// component's size) and call
    /// [`op_editor_core::EditorState::instantiate_kit_component`].
    /// Mirrors the native host's `drain_component_browser_insert`.
    pub(in crate::widget_host) fn drain_component_browser_insert(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some((kit_id, comp_id)) = self
            .editor_state
            .editor_ui
            .component_browser_pending_insert
            .take()
        else {
            return false;
        };
        let dims = self
            .editor_state
            .ui_kits
            .iter()
            .find(|k| k.id == kit_id)
            .and_then(|k| k.components.iter().find(|c| c.id == comp_id))
            .map(|c| (c.width as f64, c.height as f64));
        let Some((cw_comp, ch_comp)) = dims else {
            return false;
        };
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(cw / 2.0, ch / 2.0);
        let doc = self.editor_state.viewport.to_document(canvas_local);
        let dx = doc.x as f64 - cw_comp / 2.0;
        let dy = doc.y as f64 - ch_comp / 2.0;
        if self
            .editor_state
            .instantiate_kit_component(&kit_id, &comp_id, dx, dy)
            .is_some()
        {
            self.mark_dirty();
            true
        } else {
            false
        }
    }
}
