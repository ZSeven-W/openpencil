//! Compatibility surface over the mobile-safe render export core.
//!
//! Headless screenshot capture remains a host service because it adds MCP
//! base64 transport semantics. All file rendering lives in
//! [`op_render_export`] so desktop, server, and native mobile shells share
//! one renderer without pulling the rest of `op-host-services` into mobile.

pub use op_render_export::*;

// `capture_scene` / `CaptureSpec` / `ScreenshotPng` back both the optional
// debug MCP tool and the always-on vision-validation provider.
pub mod screenshot;

#[cfg(test)]
pub mod test_support {
    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
    use op_editor_ui::{Color, Rect};

    pub fn scene_with(children: Vec<SceneNode>) -> LayoutScene {
        LayoutScene {
            pages: vec![ScenePage {
                id: "p1".into(),
                name: "Page 1".into(),
                children,
            }],
            active_page_index: 0,
        }
    }

    pub fn filled_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> SceneNode {
        let mut node = SceneNode::leaf(id, NodeKind::Rect);
        node.bounds = Rect::xywh(x, y, w, h);
        node.fill = Some(fill);
        node
    }
}
