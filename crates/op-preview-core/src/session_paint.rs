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

use jian_core::render::widget_style::{
    active_override, derived_overlay, resolve_authored_widget_visual, with_visual_opacity,
    InteractionState as WidgetInteractionState, WidgetTheme,
};
use jian_core::widget_state::WidgetState;
use jian_ops_schema::state_override::{StyleOverride, WidgetStates};
use jian_ops_schema::style::{
    PenEffect, PenFill, PenStroke, ShaderUniformValue, StrokeAlign, StrokeThickness,
};
use op_editor_ui::layout_scene::{
    BlurEffect, DropShadow, Effect, LayoutScene, SceneNode, SceneStroke, SceneStrokeAlign,
    SceneVideo,
};
use op_editor_ui::widgets::{paint_scene_page_without_video_badge, PaintCx};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};

use crate::scene_helpers::apply_widget_state;
use crate::session::PreviewSession;

/// A video poster that a web preview host should mount as a DOM media
/// element. The scene remains the source of truth for geometry; this DTO
/// keeps decoding out of the CanvasKit painter and lets native preview ignore
/// the media entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewVideoOverlay {
    pub node_id: String,
    pub scene_rect: Rect,
    pub poster: Option<std::sync::Arc<str>>,
    pub video: SceneVideo,
}

impl PreviewSession {
    /// Return visible video posters in the active preview scene. When
    /// `only_root` is supplied, only that page-root subtree is considered;
    /// device-frame preview uses this to avoid mounting videos from sibling
    /// artboards that are not being presented.
    pub fn video_overlays(&self, only_root: Option<&str>) -> Vec<PreviewVideoOverlay> {
        let overlaid;
        let scene = if self.runtime.widget_states.iter().next().is_none()
            && self.binding_sites.is_empty()
            && !self.ui_actions.has_visual_state()
            && !self.binding_overlay.has_visual_state()
            && !self.animation.has_visual_state()
        {
            &self.scene
        } else {
            overlaid = self.overlay_runtime_state(&self.scene);
            &overlaid
        };
        let Some(page) = scene.active_page() else {
            return Vec::new();
        };
        let roots: Vec<&SceneNode> = match only_root {
            Some(root_id) => page
                .children
                .iter()
                .filter(|node| node.id == root_id)
                .collect(),
            None => page.children.iter().collect(),
        };
        let mut result = Vec::new();
        for root in roots {
            collect_video_overlays(root, &mut result);
        }
        result
    }

    /// Whether `node_id` is in the visible scene subtree rooted at
    /// `ancestor_id`. Device-frame preview uses this to map a pinned video
    /// through the pinned strip origin instead of the scrolling content.
    pub fn video_overlay_is_in_subtree(&self, ancestor_id: &str, node_id: &str) -> bool {
        self.scene
            .active_page()
            .and_then(|page| page.find(ancestor_id))
            .is_some_and(|ancestor| subtree_contains(ancestor, node_id))
    }

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
        // Avoid cloning the (potentially large) design scene when it has no
        // live state and no preview widgets. A widget scene must use the
        // overlay from its first frame because a static disabled flag or a
        // later hover can change its paint without changing geometry.
        let overlaid;
        let scene: &LayoutScene = if self.runtime.widget_states.iter().next().is_none()
            && !self.has_preview_widget_visuals()
            && self.binding_sites.is_empty()
            && !self.ui_actions.has_visual_state()
            && !self.binding_overlay.has_visual_state()
            && !self.animation.has_visual_state()
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
            paint_scene_page_without_video_badge(&mut cx, page, viewport_origin, zoom, cull);
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
        let mut scene = self.overlay_runtime_state_without_animation(base);
        self.observe_motion(&scene);
        self.animation.apply_to_scene(&mut scene);
        scene
    }

    /// Preview widgets always need the cloned overlay: even before a runtime
    /// value is mounted, hover/pressed/focused/disabled visuals can change the
    /// paint-only scene.
    pub(crate) fn has_preview_widget_visuals(&self) -> bool {
        self.runtime.document.as_ref().is_some_and(|document| {
            document.tree.nodes.values().any(|node| {
                matches!(
                    node.schema,
                    jian_ops_schema::node::PenNode::TextInput(_)
                        | jian_ops_schema::node::PenNode::TextArea(_)
                        | jian_ops_schema::node::PenNode::Select(_)
                        | jian_ops_schema::node::PenNode::Switch(_)
                        | jian_ops_schema::node::PenNode::Checkbox(_)
                        | jian_ops_schema::node::PenNode::Slider(_)
                        | jian_ops_schema::node::PenNode::RadioGroup(_)
                        | jian_ops_schema::node::PenNode::NumberInput(_)
                        | jian_ops_schema::node::PenNode::Progress(_)
                        | jian_ops_schema::node::PenNode::Tabs(_)
                )
            })
        })
    }

    pub(crate) fn overlay_runtime_state_without_animation(
        &self,
        base: &LayoutScene,
    ) -> LayoutScene {
        #[cfg(test)]
        self.overlay_builds_for_test
            .set(self.overlay_builds_for_test.get() + 1);
        let mut scene = base.clone();
        let focused_id = self.focused_schema_id();
        let state_variables = self.state_variables();
        let idx = scene.active_page_index;
        if let Some(page) = scene.pages.get_mut(idx) {
            for node in page.children.iter_mut() {
                self.overlay_node(node, focused_id.as_deref(), state_variables.as_ref());
            }
        }
        let pointer = self.pointer_binding_value();
        self.binding_overlay.apply_to_scene(
            &mut scene,
            &self.binding_sites,
            &self.runtime.state,
            &pointer,
            &self.ui_actions,
        );
        scene
    }

    /// Recursively overlay runtime widget state + live binding values
    /// onto a scene subtree.
    fn overlay_node(
        &self,
        node: &mut SceneNode,
        focused_id: Option<&str>,
        state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
    ) {
        if let Some(widget) = node.widget.as_mut() {
            if let Some(state) = self.runtime.widget_states.get(&node.id) {
                apply_widget_state(widget, state);
            }
            self.apply_widget_visual_state(node, focused_id, state_variables);
        }
        for child in node.children.iter_mut() {
            self.overlay_node(child, focused_id, state_variables);
        }
    }

    /// Resolve and apply the preview-only state visual for one widget. The
    /// cloned `LayoutScene` is the only thing changed; the runtime schema and
    /// the editor document remain immutable.
    fn apply_widget_visual_state(
        &self,
        node: &mut SceneNode,
        focused_id: Option<&str>,
        state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
    ) {
        let Some((states, disabled)) = self.widget_state_inputs(&node.id) else {
            return;
        };
        let interaction = WidgetInteractionState {
            hovered: self.interaction.hovered_node() == Some(node.id.as_str()),
            pressed: self.interaction.pressed_nodes().contains(&node.id.as_str()),
            focused: focused_id == Some(node.id.as_str()),
            disabled,
        };
        let authored = active_override(states.as_ref(), interaction);
        if let Some(style) = authored {
            self.apply_style_override(node, style, state_variables);
        } else if let Some(overlay) = derived_overlay(&WidgetTheme::default(), interaction, false) {
            let overlay = color_from_jian(overlay);
            if let Some(fill) = node.fill {
                node.fill = Some(composite_over(fill, overlay));
            }
        }
    }

    /// Build the variable table used by state fills/strokes. State styles are
    /// resolved from the runtime's prepared schema, so their `$ref` colours
    /// follow the same active theme as the base scene.
    fn state_variables(&self) -> Option<op_editor_core::scene_vars::VariableTable> {
        let document = self.runtime.document.as_ref()?;
        let mut table = op_pen_loader::build_var_table(&document.schema);
        table.active_theme = self.reset_seed.active_theme.clone();
        Some(table)
    }

    /// Read authored widget states and the node's effective disabled flag from
    /// the runtime schema. The JSON view is intentional: all ten widget
    /// variants expose the same `states` contract while their Rust structs do
    /// not share a common style trait.
    fn widget_state_inputs(&self, id: &str) -> Option<(Option<WidgetStates>, bool)> {
        let document = self.runtime.document.as_ref()?;
        let key = document.tree.by_id.get(id).copied()?;
        let schema = &document.tree.nodes.get(key)?.schema;
        let json = serde_json::to_value(schema).ok()?;
        let states = json
            .get("states")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        let disabled = self.json_node_disabled(&json, id);
        Some((states, disabled))
    }

    fn json_node_disabled(&self, json: &serde_json::Value, id: &str) -> bool {
        if json.get("disabled").and_then(serde_json::Value::as_bool) == Some(true) {
            return true;
        }
        match json.get("enabled") {
            Some(value) if value.as_bool() == Some(false) => return true,
            Some(value) if self.json_bool_expression(value, id) == Some(false) => return true,
            _ => {}
        }
        for path in [
            "semantics.disabled",
            "gestures.disabled",
            "bindings.disabled",
        ] {
            let mut value = json;
            for segment in path.split('.') {
                let Some(next) = value.get(segment) else {
                    value = &serde_json::Value::Null;
                    break;
                };
                value = next;
            }
            if self.json_bool_expression(value, id) == Some(true) {
                return true;
            }
        }
        false
    }

    fn json_bool_expression(&self, value: &serde_json::Value, id: &str) -> Option<bool> {
        if let Some(value) = value.as_bool() {
            return Some(value);
        }
        let source = value.as_str()?;
        let expression = jian_core::expression::Expression::compile(source).ok()?;
        let page = self
            .runtime
            .document
            .as_ref()
            .and_then(|document| document.active_page.as_deref());
        expression
            .eval(&self.runtime.state, page, Some(id))
            .0
            .as_bool()
    }

    fn apply_style_override(
        &self,
        node: &mut SceneNode,
        style: &StyleOverride,
        state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
    ) {
        let current_opacity = node.opacity.clamp(0.0, 1.0);
        let target_opacity = style
            .opacity
            .map(|opacity| opacity as f32)
            .unwrap_or(current_opacity)
            .clamp(0.0, 1.0);
        let existing_factor = if style.opacity.is_some() {
            if current_opacity > f32::EPSILON {
                target_opacity / current_opacity
            } else {
                target_opacity
            }
        } else {
            1.0
        };

        if let Some(fills) = style.fill.as_deref() {
            if fills.is_empty() {
                node.fill = None;
                node.fill_layers.clear();
                node.gradient = None;
                node.shader = None;
                node.fill_type = op_editor_ui::layout_scene::SceneFillType::Solid;
            } else if let Some(color) = state_fill_color(fills, state_variables) {
                node.fill = Some(with_scene_opacity(color, target_opacity));
                node.fill_layers.clear();
                node.gradient = None;
                node.shader = None;
                node.fill_type = op_editor_ui::layout_scene::SceneFillType::Solid;
            }
        } else if style.opacity.is_some() {
            node.fill = node
                .fill
                .map(|color| multiply_color_alpha(color, existing_factor));
        }

        if let Some(stroke) = style.stroke.as_ref() {
            node.stroke = state_stroke(stroke, state_variables, target_opacity);
        } else if style.opacity.is_some() {
            node.stroke = node.stroke.map(|mut stroke| {
                stroke.color = multiply_color_alpha(stroke.color, existing_factor);
                stroke
            });
        }

        if let Some(effects) = style.effects.as_deref() {
            node.effects = state_effects(effects, state_variables, target_opacity);
        } else if style.opacity.is_some() {
            node.effects = node
                .effects
                .iter()
                .copied()
                .map(|effect| scale_effect_color(effect, existing_factor))
                .collect();
        }
        if style.opacity.is_some() {
            node.opacity = target_opacity;
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

fn state_fill_color(
    fills: &[PenFill],
    state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
) -> Option<Color> {
    fills.iter().find_map(|fill| {
        let (color, opacity) = match fill {
            PenFill::Solid(body) => (
                parse_state_color(&body.color, state_variables)?,
                body.opacity,
            ),
            PenFill::LinearGradient(body) => (
                parse_state_color(&body.stops.first()?.color, state_variables)?,
                body.opacity,
            ),
            PenFill::RadialGradient(body) => (
                parse_state_color(&body.stops.first()?.color, state_variables)?,
                body.opacity,
            ),
            PenFill::MeshGradient(body) => (
                parse_state_color(&body.stops.first()?.color, state_variables)?,
                body.opacity,
            ),
            PenFill::Shader(body) => {
                let color = body
                    .uniforms
                    .as_ref()?
                    .values()
                    .find_map(|value| match value {
                        ShaderUniformValue::Color(color) => {
                            parse_state_color(color, state_variables)
                        }
                        _ => None,
                    })?;
                (color, body.opacity)
            }
            PenFill::Image(_) => return None,
        };
        Some(with_scene_opacity(color, opacity.unwrap_or(1.0)))
    })
}

fn state_stroke(
    stroke: &PenStroke,
    state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
    opacity: f32,
) -> Option<SceneStroke> {
    let (width, sides) = match &stroke.thickness {
        StrokeThickness::Uniform(width) => (*width, None),
        StrokeThickness::PerSide(sides) => {
            (sides.iter().copied().fold(0.0, f32::max), Some(*sides))
        }
        StrokeThickness::Sided(sides) => {
            let sides = [
                sides.top.unwrap_or(0.0),
                sides.right.unwrap_or(0.0),
                sides.bottom.unwrap_or(0.0),
                sides.left.unwrap_or(0.0),
            ];
            (sides.iter().copied().fold(0.0, f32::max), Some(sides))
        }
    };
    if width <= 0.0 {
        return None;
    }
    let color = stroke
        .fill
        .as_deref()
        .and_then(|fills| state_fill_color(fills, state_variables))?;
    Some(SceneStroke {
        color: with_scene_opacity(color, opacity),
        width,
        sides,
        align: match stroke.align {
            Some(StrokeAlign::Inside) => SceneStrokeAlign::Inside,
            Some(StrokeAlign::Outside) => SceneStrokeAlign::Outside,
            Some(StrokeAlign::Center) | None => SceneStrokeAlign::Center,
        },
    })
}

fn state_effects(
    effects: &[PenEffect],
    state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
    opacity: f32,
) -> Vec<Effect> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            PenEffect::Blur(body) if body.visible != Some(false) && body.radius > 0.0 => {
                Some(Effect::Blur(BlurEffect {
                    radius: body.radius,
                }))
            }
            PenEffect::BackgroundBlur(body) if body.visible != Some(false) && body.radius > 0.0 => {
                Some(Effect::BackgroundBlur {
                    radius: body.radius,
                })
            }
            PenEffect::Shadow(body) if body.visible != Some(false) => {
                let color = parse_state_color(&body.color, state_variables)?;
                Some(Effect::DropShadow(DropShadow {
                    offset_x: body.offset_x,
                    offset_y: body.offset_y,
                    blur: body.blur,
                    color: with_scene_opacity(color, opacity),
                    inner: body.inner.unwrap_or(false),
                }))
            }
            _ => None,
        })
        .collect()
}

fn parse_state_color(
    source: &str,
    state_variables: Option<&op_editor_core::scene_vars::VariableTable>,
) -> Option<Color> {
    if let Some(name) = source.trim().strip_prefix('$') {
        return state_variables?.resolve_color(name);
    }
    let source = source.trim();
    if let Some(color) = op_editor_ui::util::parse_hex_color(source) {
        return Some(color);
    }
    let lower = source.to_ascii_lowercase();
    let inner = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let alpha = parts
        .get(3)
        .map(|value| value.parse::<f32>().ok())
        .unwrap_or(Some(1.0))?;
    Some(Color {
        r: (parts[0].parse::<f32>().ok()? / 255.0).clamp(0.0, 1.0),
        g: (parts[1].parse::<f32>().ok()? / 255.0).clamp(0.0, 1.0),
        b: (parts[2].parse::<f32>().ok()? / 255.0).clamp(0.0, 1.0),
        a: alpha.clamp(0.0, 1.0),
    })
}

fn color_from_jian(color: jian_core::scene::Color) -> Color {
    Color::rgba_u8(
        color.r(),
        color.g(),
        color.b(),
        f32::from(color.a()) / 255.0,
    )
}

fn multiply_color_alpha(color: Color, factor: f32) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: (color.a * factor).clamp(0.0, 1.0),
    }
}

fn with_scene_opacity(color: Color, opacity: f32) -> Color {
    multiply_color_alpha(color, opacity.clamp(0.0, 1.0))
}

fn scale_effect_color(effect: Effect, factor: f32) -> Effect {
    match effect {
        Effect::DropShadow(mut shadow) => {
            shadow.color = multiply_color_alpha(shadow.color, factor);
            Effect::DropShadow(shadow)
        }
        other => other,
    }
}

fn composite_over(base: Color, overlay: Color) -> Color {
    let source_a = overlay.a.clamp(0.0, 1.0);
    let destination_a = base.a.clamp(0.0, 1.0);
    let output_a = source_a + destination_a * (1.0 - source_a);
    if output_a <= f32::EPSILON {
        return Color::rgba_u8(0, 0, 0, 0.0);
    }
    Color {
        r: (overlay.r * source_a + base.r * destination_a * (1.0 - source_a)) / output_a,
        g: (overlay.g * source_a + base.g * destination_a * (1.0 - source_a)) / output_a,
        b: (overlay.b * source_a + base.b * destination_a * (1.0 - source_a)) / output_a,
        a: output_a,
    }
}

fn collect_video_overlays(node: &SceneNode, out: &mut Vec<PreviewVideoOverlay>) {
    if node.hidden {
        return;
    }
    if let Some(video) = node.video.clone() {
        out.push(PreviewVideoOverlay {
            node_id: node.id.clone(),
            scene_rect: node.bounds,
            poster: node.image_src.clone(),
            video,
        });
    }
    for child in node.visible_children() {
        collect_video_overlays(child, out);
    }
}

fn subtree_contains(node: &SceneNode, id: &str) -> bool {
    node.id == id
        || node
            .visible_children()
            .iter()
            .any(|child| subtree_contains(child, id))
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
