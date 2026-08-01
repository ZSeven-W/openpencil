//! Floating panel paint band for the native host.

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::{
    ComponentBrowserPanel, DesignMdPanel, IconPickerPanel, PaintCx, PromptCenterPanel,
    SceneTemplatePanel,
};

impl WidgetHostNative {
    pub(super) fn paint_floating_panels(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        if let (Some(panel), Some(rect)) = (
            ComponentBrowserPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.component_browser_panel_rect(viewport_width, viewport_height),
        ) {
            panel.paint(
                &mut PaintCx {
                    backend: &mut *frame,
                },
                rect,
            );
        }

        if let (Some(panel), Some(rect)) = (
            PromptCenterPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.prompt_center_panel_rect(viewport_width, viewport_height),
        ) {
            panel.paint(
                &mut PaintCx {
                    backend: &mut *frame,
                },
                rect,
            );
        }

        if let (Some(panel), Some(rect)) = (
            SceneTemplatePanel::for_editor_at(&self.editor_state, self.now_ms),
            self.scene_template_panel_rect(viewport_width, viewport_height),
        ) {
            panel.paint(
                &mut PaintCx {
                    backend: &mut *frame,
                },
                rect,
            );
        }

        if let (Some(panel), Some(rect)) = (
            IconPickerPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.icon_picker_panel_rect(viewport_width, viewport_height),
        ) {
            panel.paint(
                &mut PaintCx {
                    backend: &mut *frame,
                },
                rect,
            );
        }

        if let (Some(panel), Some(rect)) = (
            DesignMdPanel::for_editor(&self.editor_state),
            self.design_md_panel_rect(viewport_width, viewport_height),
        ) {
            panel.paint(
                &mut PaintCx {
                    backend: &mut *frame,
                },
                rect,
            );
        }
    }
}
