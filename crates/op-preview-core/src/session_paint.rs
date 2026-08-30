//! Preview scene overlay + paint impls for [`PreviewSession`].
//!
//! Split out of `preview/mod.rs` (the crate spine) so no file exceeds the
//! repo's 800-line-per-file cap. Owns `paint_scene` — the design-scene
//! paint pass with live runtime widget values overlaid and a focus caret on
//! top — plus the overlay walkers (`overlay_runtime_state` /
//! `overlay_node`), the caret paint, and the
//! contrast-derived caret color helper. Typed binding/scroll application
//! lives in `binding_overlay.rs`.
//!
//! `paint_scene` is the public entry; `overlay_runtime_state` and
//! `paint_focus_caret` are `pub(crate)` so the sibling `present` /
//! `transition` / `app_mode` modules reuse the exact same overlay for
//! framed / animated rendering, and the render tests can walk it.

use jian_core::render::widget_style::{resolve_authored_widget_visual, with_visual_opacity};
use jian_core::widget_state::WidgetState;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use op_editor_ui::widgets::{paint_scene_page, PaintCx};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

use crate::scene_helpers::apply_widget_state;
use crate::session::PreviewSession;

impl PreviewSession {
    /// Paint the live preview by rendering the session's own design
    /// `LayoutScene` (built from the promoted document) with the current
    /// widget runtime state overlaid, then a focus caret on top.
    /// `canvas_region` is the screen-space canvas rect (clip + transform
    /// origin); `pan` / `zoom` come from the editor viewport. The scene
    /// paints through the SAME painter the design canvas uses, so preview
    /// is pixel-identical plus live.
    pub fn paint_scene(
        &self,
        backend: &mut dyn RenderBackend,
        canvas_region: Rect,
        pan: (f32, f32),
        zoom: f32,
        now_ms: u64,
    ) {
        // Avoid cloning the (potentially large) design scene until the
        // user has actually interacted: with no live widget state the
        // overlay is a no-op, so paint the scene directly. Once any
        // widget carries runtime state we clone + overlay it.
        let overlaid;
        let scene: &LayoutScene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
            && !self.ui_actions.has_visual_state()
            && !self.binding_overlay.has_visual_state()
        {
            &self.scene
        } else {
            overlaid = self.overlay_runtime_state(&self.scene);
            &overlaid
        };
        let Some(page) = scene.active_page() else {
            return;
        };
        let viewport_origin = Point2D::new(
            canvas_region.origin.x + pan.0,
            canvas_region.origin.y + pan.1,
        );
        const CULL_MARGIN: f32 = 64.0;
        let cull = Rect {
            origin: Point2D::new(
                canvas_region.origin.x - CULL_MARGIN,
                canvas_region.origin.y - CULL_MARGIN,
            ),
            size: Point2D::new(
                canvas_region.size.x + CULL_MARGIN * 2.0,
                canvas_region.size.y + CULL_MARGIN * 2.0,
            ),
        };
        backend.save();
        backend.clip_rect(canvas_region);
        {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            paint_scene_page(&mut cx, page, viewport_origin, zoom, cull);
        }
        self.paint_focus_caret(backend, scene, viewport_origin, zoom, now_ms);
        backend.restore();
    }

    /// Clone the design scene and overlay each interactive widget's LIVE
    /// runtime value so the preview reflects typed text / toggles /
    /// slider drags / selection, then apply R6's typed binding and scroll
    /// overlay. Pure (no paint), so visual and hit geometry share the same
    /// deterministic scene snapshot.
    pub(crate) fn overlay_runtime_state(&self, base: &LayoutScene) -> LayoutScene {
        let mut scene = base.clone();
        let idx = scene.active_page_index;
        if let Some(page) = scene.pages.get_mut(idx) {
            for node in page.children.iter_mut() {
                self.overlay_node(node);
            }
        }
        self.binding_overlay.apply_to_scene(
            &mut scene,
            &self.binding_sites,
            &self.runtime.state,
            &self.ui_actions,
        );
        scene
    }

    /// Recursively overlay runtime widget state + live binding values
    /// onto a scene subtree.
    fn overlay_node(&self, node: &mut SceneNode) {
        if let Some(widget) = node.widget.as_mut() {
            if let Some(state) = self.runtime.widget_states.get(&node.id) {
                apply_widget_state(widget, state);
            }
        }
        for child in node.children.iter_mut() {
            self.overlay_node(child);
        }
    }

    /// Draw a 1px caret for the focused text widget, aligned to the
    /// glyph advance of the value up to the caret. Metrics mirror
    /// `op_editor_ui`'s `paint_text_field` (14px font; left inset via the
    /// shared `widget_text_inset_left`, so the caret clears a leading
    /// icon). No-op unless a text widget is focused with a blink-visible
    /// caret.
    pub(crate) fn paint_focus_caret(
        &self,
        backend: &mut dyn RenderBackend,
        scene: &LayoutScene,
        viewport_origin: Point2D,
        zoom: f32,
        now_ms: u64,
    ) {
        let Some(id) = self.focused_schema_id() else {
            return;
        };
        let Some(WidgetState::TextInput(st)) = self.runtime.widget_states.get(&id) else {
            return;
        };
        if !st.caret_visible(now_ms) {
            return;
        }
        let Some(node) = scene.active_page().and_then(|p| p.find(&id)) else {
            return;
        };
        let Some(widget) = node.widget.as_ref() else {
            return;
        };
        if !matches!(
            widget.kind.as_str(),
            "text_input" | "text_area" | "number_input"
        ) {
            return;
        }

        // Mirror `paint_text_field`: fixed 14px label, left inset via the
        // shared helper (8px pad, +icon box when a leading icon is set),
        // single-line vertically centred (text_area top-aligned).
        const FONT: f32 = 14.0;
        let inset = op_editor_ui::widgets::widget_text_inset_left(widget);
        let world_x = viewport_origin.x + node.bounds.origin.x * zoom;
        let world_y = viewport_origin.y + node.bounds.origin.y * zoom;
        let world_h = node.bounds.size.y * zoom;
        let fs_world = FONT * zoom;
        let text_x = world_x + inset * zoom;
        let top_y = if widget.kind == "text_area" {
            world_y + 8.0 * zoom
        } else {
            world_y + (world_h - fs_world) / 2.0
        };

        // Caret byte offset, clamped to a UTF-8 boundary so slicing a
        // multi-byte value never panics.
        let text = st.text();
        let mut caret_byte = st.caret().min(text.len());
        while caret_byte > 0 && !text.is_char_boundary(caret_byte) {
            caret_byte -= 1;
        }
        let advance = backend.measure_text_weighted(&text[..caret_byte], fs_world, 400);
        let caret_x = text_x + advance;

        // Match the field's value foreground. The shared resolver derives a
        // readable caret from the authored surface/stroke, so a dark input no
        // longer receives the old hard-coded near-black caret.
        let color = widget_field_foreground(node);
        backend.stroke_line(
            Point2D::new(caret_x, top_y),
            Point2D::new(caret_x, top_y + fs_world),
            color,
            zoom.max(1.0),
        );
    }

    /// Schema id of the currently focused node, mapped from the runtime
    /// focus chain.
    pub(crate) fn focused_schema_id(&self) -> Option<String> {
        let key = self.runtime.focus.current()?;
        let doc = self.runtime.document.as_ref()?;
        let node = doc.tree.nodes.get(key)?;
        Some(jian_core::document::tree::node_schema_id(&node.schema).to_owned())
    }
}

/// Contrast-derived foreground for the focus caret, resolved from the
/// authored widget surface (fill) + stroke via the shared widget policy.
fn widget_field_foreground(node: &SceneNode) -> Color {
    let visual = resolve_authored_widget_visual(
        node.fill.map(Color::to_jian),
        node.stroke.map(|stroke| stroke.color.to_jian()),
    );
    // Scene fill/stroke already carry direct-paint opacity; the contrast-derived
    // caret does not, so fold it exactly once through the shared widget policy.
    let color = with_visual_opacity(visual.foreground, node.opacity);
    Color::rgba_u8(
        color.r(),
        color.g(),
        color.b(),
        f32::from(color.a()) / 255.0,
    )
}
