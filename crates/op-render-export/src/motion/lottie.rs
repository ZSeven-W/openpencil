//! Lottie JSON with outlined vector paint and sampled keyframes.
//! Effects outside Lottie's shape model remain self-contained image assets.
use super::MotionExportOptions;
use crate::{paint_node, ExportError, RasterFormat};
use base64::{engine::general_purpose::STANDARD, Engine};
use jian_core::motion::Clip;
use op_editor_core::motion_export::MotionExportBackground;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use op_motion_export::{FrameClock, MotionScene};
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, HashMap},
    io::Write,
};

mod path;
mod properties;
mod svg;
#[cfg(test)]
mod tests;
mod text;
use properties::{animated, property, timed_layers, transform};

pub(super) fn encode(
    writer: impl Write,
    layout: &LayoutScene,
    root: &jian_ops_schema::node::PenNode,
    clip: Clip,
    clock: FrameClock,
    options: &MotionExportOptions,
) -> Result<(), ExportError> {
    let root_id = jian_core::document::tree::node_schema_id(root);
    let scene = MotionScene::new(layout, root_id, options.scale, clip)?;
    let frames: Vec<_> = (0..clock.frame_count())
        .map(|index| scene.sample(clock.time_at(index), options.reduced_motion))
        .collect();
    let mut builder = Builder {
        scene: &scene,
        end: clock.frame_count(),
        assets: Vec::new(),
        images: HashMap::new(),
        warnings: BTreeSet::new(),
        native: op_host_native::NativeBackend::with_dpi(1.0),
        clock,
        reduced: options.reduced_motion,
        time_ms: 0,
    };
    if op_editor_core::audio_tracks::audio_duration_ms(root, &options.trigger) > 0 {
        builder
            .warnings
            .insert("Lottie exports visual animation; soundtrack audio is omitted".into());
    }
    let root_layer = builder.node(&frames.iter().collect::<Vec<_>>())?;
    let mut layers = vec![root_layer];
    if options.background == MotionExportBackground::White {
        let mut background = layer("White background", 1);
        background["sw"] = json!(scene.size.0);
        background["sh"] = json!(scene.size.1);
        background["sc"] = json!("#ffffff");
        background["op"] = json!(clock.frame_count());
        layers.push(background);
    }
    assign_indices(&mut layers);
    let value = json!({"v":"5.12.2", "nm":root_id, "ddd":0,
        "w":scene.size.0,"h":scene.size.1,"fr":options.fps,"ip":0,"op":clock.frame_count(),
        "assets":builder.assets,"layers":layers,
        "meta":{"g":"OpenPencil", "warnings":builder.warnings}});
    serde_json::to_writer(writer, &value).map_err(|error| ExportError::MotionEncoding {
        format: "Lottie",
        reason: error.to_string(),
    })
}

fn layer(name: &str, kind: u8) -> Value {
    json!({"ty":kind,"nm":name,"ddd":0,"ks":transform(),"ip":0,"op":1,"st":0,"sr":1,"ao":0})
}

fn assign_indices(layers: &mut [Value]) {
    for (index, layer) in layers.iter_mut().enumerate() {
        layer["ind"] = json!(index + 1);
    }
}

struct Builder<'a> {
    scene: &'a MotionScene,
    end: u64,
    assets: Vec<Value>,
    images: HashMap<Vec<u8>, String>,
    native: op_host_native::NativeBackend,
    clock: FrameClock,
    reduced: bool,
    time_ms: u64,
    warnings: BTreeSet<String>,
}

impl Builder<'_> {
    fn composition(&mut self, name: &str, mut layers: Vec<Value>) -> Value {
        assign_indices(&mut layers);
        let id = format!("composition-{}", self.assets.len());
        self.assets.push(json!({"id":id,"layers":layers}));
        let mut layer = layer(name, 0);
        layer["refId"] = json!(id);
        layer["w"] = json!(self.scene.size.0);
        layer["h"] = json!(self.scene.size.1);
        layer["op"] = json!(self.end);
        layer
    }

    fn node(&mut self, frames: &[&SceneNode]) -> Result<Value, ExportError> {
        let first = frames[0];
        // Native effects, sibling masks and interactive widgets paint as one
        // subtree, preserving visuals without a Lottie equivalent.
        let flatten = frames.iter().any(|node| {
            !node.effects.is_empty()
                || node.shader.is_some()
                || node.widget.is_some()
                || node.children.iter().any(|n| {
                    n.is_mask
                        || n.mask_type.is_some()
                        || n.blend_mode != op_editor_ui::ImageBlendMode::Normal
                })
        });
        let mut own = Vec::new();
        let mut previous: Option<(SceneNode, Value)> = None;
        for (index, node) in frames.iter().enumerate() {
            self.time_ms = if self.reduced {
                0
            } else {
                self.clock.time_at(index as u64)
            };
            let mut art = (*node).clone();
            art.rotation = 0.0;
            art.flip_x = false;
            art.flip_y = false;
            // A leaf faded on its composite layer (see jian's leaf opacity)
            // becomes paint alpha here, since this layer keeps alpha 1.
            if art.children.is_empty() && art.composite_opacity < 1.0 {
                art.scale_paint_alpha(art.composite_opacity.max(0.0));
            }
            art.composite_opacity = 1.0;
            if !flatten {
                art.children.clear();
                if !node.children.is_empty() {
                    art.stroke = None;
                }
            }
            let painted = match previous.as_ref() {
                Some((cached_node, paint)) if cached_node == &art && !has_shader(&art) => {
                    paint.clone()
                }
                _ => {
                    let paint = if flatten {
                        self.raster(&art)?
                    } else {
                        self.vector(&art)?
                    };
                    previous = Some((art, paint.clone()));
                    paint
                }
            };
            own.push(painted);
        }
        let own = timed_layers(own, self.end);
        let mut layers = Vec::new();
        if !flatten {
            for index in 0..first.visible_children().len() {
                let children: Vec<_> = frames
                    .iter()
                    .map(|node| &node.visible_children()[index])
                    .collect();
                layers.push(self.node(&children)?);
            }
            if first.clip_content
                && !layers.is_empty()
                && first.bounds.size.x > 0.0
                && first.bounds.size.y > 0.0
            {
                let mut clipped = self.composition("Clipped children", layers);
                clipped["hasMask"] = json!(true);
                let shapes: Vec<_> = frames.iter().map(|node| self.clip_shape(node)).collect();
                clipped["masksProperties"] = json!([{"inv":false,"mode":"a","pt":animated(&shapes),
                    "o":property(100),"x":property(0)}]);
                layers = vec![clipped];
            }
        }
        layers.extend(own);
        if !flatten && first.stroke.is_some() && !first.children.is_empty() {
            let mut borders = Vec::new();
            for node in frames {
                let mut border = (*node).clone();
                border.children.clear();
                border.fill = None;
                border.gradient = None;
                border.fill_layers.clear();
                border.rotation = 0.0;
                border.flip_x = false;
                border.flip_y = false;
                border.composite_opacity = 1.0;
                let painted = self.vector(&border)?;
                borders.push(painted);
            }
            let mut border_layers = timed_layers(borders, self.end);
            border_layers.extend(layers);
            layers = border_layers;
        }
        // Canvas players multiply precomp alpha per child. During a partial
        // group fade, paint the assembled subtree once; keep vector content
        // for the opaque portion of the animation.
        let partial_opacity =
            |node: &SceneNode| node.composite_opacity > 0.0 && node.composite_opacity < 1.0;
        if !flatten && !first.children.is_empty() && frames.iter().any(|node| partial_opacity(node))
        {
            let mut vectors = self.composition("Vector content", layers);
            vectors["ks"]["o"] = animated(
                &frames
                    .iter()
                    .map(|node| json!(if partial_opacity(node) { 0 } else { 100 }))
                    .collect::<Vec<_>>(),
            );
            let mut raster_frames = Vec::new();
            for (index, node) in frames.iter().enumerate() {
                if partial_opacity(node) {
                    self.time_ms = if self.reduced {
                        0
                    } else {
                        self.clock.time_at(index as u64)
                    };
                    let mut art = (*node).clone();
                    art.rotation = 0.0;
                    art.flip_x = false;
                    art.flip_y = false;
                    art.composite_opacity = 1.0;
                    raster_frames.push(self.raster(&art)?);
                } else {
                    let mut empty = layer("Transparent frame", 4);
                    empty["shapes"] = json!([]);
                    raster_frames.push(empty);
                }
            }
            layers = timed_layers(raster_frames, self.end);
            layers.push(vectors);
        }
        let mut result = self.composition(&first.id, layers);
        let pivots: Vec<_> = frames
            .iter()
            .map(|node| {
                let r = node.aggregate_bounds();
                json!([
                    (r.origin.x + r.size.x * 0.5 - self.scene.bounds.origin.x) * self.scene.scale,
                    (r.origin.y + r.size.y * 0.5 - self.scene.bounds.origin.y) * self.scene.scale
                ])
            })
            .collect();
        result["ks"]["s"] = animated(
            &frames
                .iter()
                .map(|n| {
                    json!([
                        if n.flip_x { -100 } else { 100 },
                        if n.flip_y { -100 } else { 100 }
                    ])
                })
                .collect::<Vec<_>>(),
        );
        result["ks"]["p"] = animated(&pivots);
        result["ks"]["a"] = animated(&pivots);
        result["ks"]["r"] = animated(
            &frames
                .iter()
                .map(|node| json!(node.rotation.to_degrees()))
                .collect::<Vec<_>>(),
        );
        result["ks"]["o"] = animated(
            &frames
                .iter()
                .map(|node| {
                    json!(if node.hidden {
                        0.0
                    } else {
                        node.composite_opacity * 100.0
                    })
                })
                .collect::<Vec<_>>(),
        );
        Ok(result)
    }

    fn clip_shape(&self, node: &SceneNode) -> Value {
        let scale = self.scene.scale;
        let r = node.bounds;
        path::rounded_rect(
            [
                (r.origin.x - self.scene.bounds.origin.x) * scale,
                (r.origin.y - self.scene.bounds.origin.y) * scale,
                r.size.x * scale,
                r.size.y * scale,
            ],
            node.corner_radii
                .unwrap_or([node.corner_radius; 4])
                .map(|r| r * scale),
        )
    }

    fn vector(&mut self, node: &SceneNode) -> Result<Value, ExportError> {
        use op_editor_ui::layout_scene::{NodeKind, SceneFillLayer, SceneGradient, ScenePage};
        if node
            .fill_layers
            .iter()
            .any(|layer| layer.blend_mode() != op_editor_ui::ImageBlendMode::Normal)
        {
            return self.raster(node);
        }
        // Canonical stacks apply opacity to their fill, while legacy text and
        // stroke colors already carry direct-paint alpha. Preserve that split.
        let mut paint = node.clone();
        if !matches!(node.kind, NodeKind::Text | NodeKind::Line) {
            match node.fill_layers.first() {
                Some(SceneFillLayer::Solid { color, .. }) => {
                    paint.fill = Some(color.with_alpha(color.a * node.opacity));
                }
                Some(SceneFillLayer::Gradient { gradient, .. }) => {
                    let mut gradient = gradient.clone();
                    match &mut gradient {
                        SceneGradient::Linear { opacity, .. }
                        | SceneGradient::Radial { opacity, .. }
                        | SceneGradient::Mesh { opacity, .. } => *opacity *= node.opacity,
                    }
                    paint.gradient = Some(gradient);
                }
                _ => {}
            }
        }
        let node = &paint;
        let inner = if node.kind == NodeKind::Text {
            text::markup(node, &mut self.native)
        } else if node.image_src.is_some()
            || node.fill_layers.len() > 1
            || matches!(
                node.gradient,
                Some(op_editor_ui::layout_scene::SceneGradient::Mesh { .. })
            )
            || node
                .path_anchors
                .iter()
                .any(|p| p.handle_in.is_some() || p.handle_out.is_some())
        {
            None
        } else {
            let scene = LayoutScene {
                pages: vec![ScenePage {
                    id: "export".into(),
                    name: String::new(),
                    children: vec![node.clone()],
                }],
                active_page_index: 0,
            };
            op_editor_ui::svg_export::serialize_node_svg(&scene, &node.id)
                .map(|svg| {
                    let start = svg.find('>').unwrap() + 1;
                    svg[start..svg.len() - 6].to_owned()
                })
                .or_else(|error| match error {
                    op_editor_ui::svg_export::SvgExportError::NodePaintsNothing(_) => {
                        Ok(String::new())
                    }
                    other => Err(other),
                })
                .ok()
        };
        let Some(inner) = inner else {
            return self.raster(node);
        };
        let b = self.scene.bounds;
        let data = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">{inner}</svg>"#,
            self.scene.size.0,
            self.scene.size.1,
            node.bounds.origin.x,
            node.bounds.origin.y,
            self.scene.size.0 as f32 / self.scene.scale,
            self.scene.size.1 as f32 / self.scene.scale
        );
        if let Some(shapes) = svg::shapes(data.as_bytes()) {
            let mut result = layer(&node.id, 4);
            result["shapes"] = shapes;
            result["ks"]["p"] = property(json!([
                (node.bounds.origin.x - b.origin.x) * self.scene.scale,
                (node.bounds.origin.y - b.origin.y) * self.scene.scale
            ]));
            Ok(result)
        } else {
            self.raster(node)
        }
    }

    fn raster(&mut self, node: &SceneNode) -> Result<Value, ExportError> {
        self.warnings
            .insert(format!("Rasterized unsupported paint on {}", node.id));
        let mut surface = crate::raster_surface(
            self.scene.size,
            self.scene.bounds,
            self.scene.scale,
            0.0,
            true,
            |canvas| {
                op_editor_ui::shader_clock::with_sample_time(self.time_ms, || {
                    paint_node(canvas, node)
                })
            },
        )?;
        let bytes = crate::encode_surface(&mut surface, RasterFormat::Png)?;
        let id = if let Some(id) = self.images.get(&bytes) {
            id.clone()
        } else {
            let id = format!("image-{}", self.images.len());
            self.assets
                .push(json!({"id":id,"w":self.scene.size.0,"h":self.scene.size.1,
                "u":"","p":format!("data:image/png;base64,{}",STANDARD.encode(&bytes)),"e":1}));
            self.images.insert(bytes, id.clone());
            id
        };
        let mut result = layer(&node.id, 2);
        result["refId"] = json!(id);
        Ok(result)
    }
}

fn has_shader(node: &SceneNode) -> bool {
    node.shader.is_some() || node.children.iter().any(has_shader)
}
