//! Per-kind node painter for [`super::canvas_viewport::CanvasViewport`].
//!
//! Walks a [`LayoutScene`](crate::layout_scene::LayoutScene)
//! [`SceneNode`] tree and reproduces the canvas pixel-for-pixel:
//! per-kind paint (Frame / Group / Rect / Ellipse / Polygon / Line /
//! Path / Text / `icon_font`), per-node rotation, corner radius,
//! pre-resolved fills / strokes, drop-shadow effects, viewport culling
//! and CJK-aware text wrap.
//!
//! Split out of `canvas_viewport.rs` to keep that file under the
//! 800-line ceiling. The scene's geometry is already layout-resolved
//! and its fills are already `$ref`-resolved, so this painter applies
//! only the viewport transform — no second layout pass, no variable
//! lookup.

use crate::layout_scene::SceneNode;
use crate::layout_scene::{Effect, NodeKind};
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::canvas_viewport_overlay::{paint_fill_then_stroke, wrap_text};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// Paint every `Effect::DropShadow` on `node` as a blurred shape
/// behind its fill. The shadow corner radius matches the node
/// kind — `corner_radius` for Frame / Rect, min-half for an
/// ellipse silhouette. Offset + blur scale by `zoom` so the
/// shadow tracks the node across viewport zoom.
fn paint_drop_shadows(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) {
    let radius = if node.kind == NodeKind::Ellipse {
        world_rect.size.x.min(world_rect.size.y) / 2.0
    } else {
        node.corner_radius * zoom
    };
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect;
        let shadow_rect = Rect {
            origin: Point2D::new(
                world_rect.origin.x + s.offset_x * zoom,
                world_rect.origin.y + s.offset_y * zoom,
            ),
            size: world_rect.size,
        };
        cx.backend
            .fill_drop_shadow(shadow_rect, radius, s.blur * zoom, s.color);
    }
}

/// Recursively paint one resolved [`SceneNode`] and its subtree.
///
/// `viewport_origin` is the canvas-rect origin shifted by the
/// viewport pan; `zoom` is the viewport zoom. The scene already
/// carries layout-resolved absolute doc-space bounds, so paint is a
/// straight `doc → world` transform.
pub fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
) {
    // Hidden nodes (and their subtree) skip canvas paint entirely.
    // Layer panel still shows them, dimmed, so the user can unhide.
    if node.hidden {
        return;
    }
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x * zoom,
            viewport_origin.y + node.bounds.origin.y * zoom,
        ),
        size: Point2D::new(node.bounds.size.x * zoom, node.bounds.size.y * zoom),
    };
    // Viewport culling — bounded leaves skip paint entirely when
    // off-screen. Containers (bounds = ZERO) always recurse.
    if world_rect.size.x > 0.0 && world_rect.size.y > 0.0 && node.children.is_empty() {
        let off = world_rect.origin.x + world_rect.size.x < cull.origin.x
            || world_rect.origin.x > cull.origin.x + cull.size.x
            || world_rect.origin.y + world_rect.size.y < cull.origin.y
            || world_rect.origin.y > cull.origin.y + cull.size.y;
        if off {
            return;
        }
    }

    // Wrap the paint in save/rotate/restore if the node carries a
    // non-zero rotation. Rotation pivots around the node's own
    // bounds centre — for containers, this is the aggregate centre.
    let rotated = node.rotation.abs() > f32::EPSILON;
    if rotated {
        let pivot_doc = node.aggregate_bounds();
        let pivot = Point2D::new(
            viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
            viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }

    // Drop shadows paint behind the node's own fill. Only kinds
    // whose silhouette a rounded rect / ellipse can represent
    // faithfully (Frame / Rect / Ellipse) cast one; Polygon / Line
    // / Path shadows are deferred until a shape-mask path exists.
    if !node.effects.is_empty()
        && world_rect.size.x > 0.0
        && world_rect.size.y > 0.0
        && matches!(
            node.kind,
            NodeKind::Frame | NodeKind::Rect | NodeKind::Ellipse
        )
    {
        paint_drop_shadows(cx, node, world_rect, zoom);
    }

    match &node.kind {
        NodeKind::Frame => {
            paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Other(tag) if tag == "icon_font" => crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            node.text.as_deref().unwrap_or(""),
            world_rect,
            node.fill,
        ),
        NodeKind::Group | NodeKind::Other(_) => {
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Rect => {
            paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
        }
        NodeKind::Ellipse => {
            if let Some(fill) = node.fill {
                cx.backend.fill_oval(world_rect, fill);
            }
            if let Some(stroke) = node.stroke {
                cx.backend
                    .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
            }
        }
        NodeKind::Polygon => {
            // Default triangle: top-centre, bottom-left, bottom-right.
            let cx_pt = world_rect.origin.x + world_rect.size.x / 2.0;
            let top_y = world_rect.origin.y;
            let bottom_y = world_rect.origin.y + world_rect.size.y;
            let left_x = world_rect.origin.x;
            let right_x = world_rect.origin.x + world_rect.size.x;
            let pts = [
                Point2D::new(cx_pt, top_y),
                Point2D::new(left_x, bottom_y),
                Point2D::new(right_x, bottom_y),
            ];
            if let Some(fill) = node.fill {
                cx.backend.fill_polygon(&pts, fill);
            }
            if let Some(stroke) = node.stroke {
                cx.backend
                    .stroke_polygon(&pts, stroke.color, stroke.width * zoom);
            }
        }
        NodeKind::Line => {
            // Top-left → bottom-right diagonal across the bounds,
            // stroked at the stroke width (or 1.5 if no stroke).
            let from = Point2D::new(world_rect.origin.x, world_rect.origin.y);
            let to = Point2D::new(
                world_rect.origin.x + world_rect.size.x,
                world_rect.origin.y + world_rect.size.y,
            );
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            cx.backend.stroke_line(from, to, color, width);
        }
        NodeKind::Path => {
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            let to_world = |p: Point2D| -> Point2D {
                Point2D::new(
                    viewport_origin.x + p.x * zoom,
                    viewport_origin.y + p.y * zoom,
                )
            };
            for pair in node.points.windows(2) {
                cx.backend
                    .stroke_line(to_world(pair[0]), to_world(pair[1]), color, width);
            }
        }
        NodeKind::Text => {
            paint_text_node(cx, node, world_rect, zoom, &edit_caret);
        }
    }

    if rotated {
        cx.backend.restore();
    }
}

/// Paint a Text `SceneNode` — wrapped or single-line text plus the
/// edit caret when the node is the one being edited.
fn paint_text_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    edit_caret: &Option<EditCaret>,
) {
    let text = node.text.as_deref().unwrap_or("");
    // Ink colour follows the resolved fill (defaults to near black).
    let ink = node.fill.unwrap_or(crate::Color {
        r: 0.08,
        g: 0.08,
        b: 0.08,
        a: 1.0,
    });
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    // Honour authored font size from the canonical schema; default to
    // 13 px so editor-created text stays uniform. Baseline ≈ 1.08 × size.
    let base_size = if node.font_size > 0.0 {
        node.font_size
    } else {
        13.0
    };
    let font_size = base_size * zoom;
    let baseline_y = world_rect.origin.y + (base_size + 1.0) * zoom;
    if !text.is_empty() {
        let weight = if node.font_weight > 0 {
            node.font_weight
        } else {
            400
        };
        let jc = jian_core::scene::Color::rgba(ch(ink.r), ch(ink.g), ch(ink.b), ch(ink.a));
        let line_h = base_size * 1.35 * zoom;
        let mut ly = baseline_y;
        let lines: Vec<String> = if node.text_wrap {
            wrap_text(cx.backend, text, font_size, world_rect.size.x, weight)
        } else {
            text.split('\n').map(str::to_string).collect()
        };
        for line in lines {
            cx.backend.draw_text(
                &TextLayout::single_run(&line, "system-ui", font_size, jc, Point2D::new(0.0, 0.0))
                    .with_font_weight(weight),
                Point2D::new(world_rect.origin.x, ly),
            );
            ly += line_h;
        }
    }
    // Caret while editing — sits at the end of the text.
    if let Some(c) = edit_caret {
        if c.editing == node.id && jian_core::anim::blink_visible(c.now_ms, c.anchor_ms, 500) {
            let text_w = cx.backend.measure_text(text, font_size);
            let caret = Rect {
                origin: Point2D::new(
                    world_rect.origin.x + text_w,
                    world_rect.origin.y + 2.0 * zoom,
                ),
                size: Point2D::new(1.0_f32.max(zoom), font_size * 1.15),
            };
            cx.backend.fill_rect(caret, ink);
        }
    }
}
