//! Flat `OpCk` draw-call bodies for the `RenderBackend` impl.
//!
//! Split out of `canvaskit.rs`: the `impl RenderBackend for CanvasKitBackend`
//! block must stay a single block, so the shape / gradient / SVG-path call
//! bodies live here as free functions over the bridge object and the trait
//! methods in `backend.rs` forward to them verbatim.

use js_sys::Array;
use op_editor_ui::{Color, Point2D, Rect};
use wasm_bindgen::JsValue;

use super::bindings::OpCk;
use super::convert::{flatten_gradient_colors, flatten_gradient_stops};

fn flatten_shader_uniforms(uniforms: &[(&str, &[f32])]) -> (Array, Vec<f32>, Vec<u32>) {
    let names = Array::new();
    let mut values = Vec::new();
    let mut arities = Vec::with_capacity(uniforms.len());
    for (name, uniform_values) in uniforms {
        names.push(&JsValue::from_str(name));
        arities.push(uniform_values.len() as u32);
        values.extend_from_slice(uniform_values);
    }
    (names, values, arities)
}

pub(super) fn fill_rect(ck: &OpCk, rect: Rect, color: Color) {
    ck.fill_rect(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn stroke_rect(ck: &OpCk, rect: Rect, color: Color, width: f32) {
    ck.stroke_rect(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

pub(super) fn fill_round_rect(ck: &OpCk, rect: Rect, radius: f32, color: Color) {
    ck.fill_round_rect(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn fill_round_rect_per_corner(ck: &OpCk, rect: Rect, radii: [f32; 4], color: Color) {
    ck.fill_round_rect_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn fill_round_rect_linear_gradient(
    ck: &OpCk,
    rect: Rect,
    radius: f32,
    stops: &[(f32, Color)],
    angle_deg: f32,
    opacity: f32,
) {
    if stops.is_empty() {
        return;
    }
    let stops = flatten_gradient_stops(stops);
    ck.fill_round_rect_linear_gradient(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        &stops,
        angle_deg,
        opacity,
    );
}

pub(super) fn fill_round_rect_linear_gradient_per_corner(
    ck: &OpCk,
    rect: Rect,
    radii: [f32; 4],
    stops: &[(f32, Color)],
    angle_deg: f32,
    opacity: f32,
) {
    if stops.is_empty() {
        return;
    }
    let stops = flatten_gradient_stops(stops);
    ck.fill_round_rect_linear_gradient_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        &stops,
        angle_deg,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_round_rect_radial_gradient(
    ck: &OpCk,
    rect: Rect,
    radius: f32,
    stops: &[(f32, Color)],
    cx_frac: f32,
    cy_frac: f32,
    radius_frac: f32,
    opacity: f32,
) {
    if stops.is_empty() {
        return;
    }
    let stops = flatten_gradient_stops(stops);
    ck.fill_round_rect_radial_gradient(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        &stops,
        cx_frac,
        cy_frac,
        radius_frac,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_round_rect_radial_gradient_per_corner(
    ck: &OpCk,
    rect: Rect,
    radii: [f32; 4],
    stops: &[(f32, Color)],
    cx_frac: f32,
    cy_frac: f32,
    radius_frac: f32,
    opacity: f32,
) {
    if stops.is_empty() {
        return;
    }
    let stops = flatten_gradient_stops(stops);
    ck.fill_round_rect_radial_gradient_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        &stops,
        cx_frac,
        cy_frac,
        radius_frac,
        opacity,
    );
}

pub(super) fn fill_round_rect_mesh_gradient(
    ck: &OpCk,
    rect: Rect,
    radius: f32,
    rows: u32,
    cols: u32,
    colors: &[Color],
    opacity: f32,
) {
    if colors.is_empty() {
        return;
    }
    let colors = flatten_gradient_colors(colors);
    ck.fill_round_rect_mesh_gradient(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        rows,
        cols,
        &colors,
        opacity,
    );
}

pub(super) fn fill_round_rect_mesh_gradient_per_corner(
    ck: &OpCk,
    rect: Rect,
    radii: [f32; 4],
    rows: u32,
    cols: u32,
    colors: &[Color],
    opacity: f32,
) {
    if colors.is_empty() {
        return;
    }
    let colors = flatten_gradient_colors(colors);
    ck.fill_round_rect_mesh_gradient_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        rows,
        cols,
        &colors,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_round_rect_shader(
    ck: &OpCk,
    rect: Rect,
    radius: f32,
    sksl: &str,
    uniforms: &[(&str, &[f32])],
    opacity: f32,
    fallback: Color,
) {
    let (names, values, arities) = flatten_shader_uniforms(uniforms);
    ck.fill_round_rect_shader(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        sksl,
        &names,
        &values,
        &arities,
        opacity,
        fallback.r,
        fallback.g,
        fallback.b,
        fallback.a,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_round_rect_shader_per_corner(
    ck: &OpCk,
    rect: Rect,
    radii: [f32; 4],
    sksl: &str,
    uniforms: &[(&str, &[f32])],
    opacity: f32,
    fallback: Color,
) {
    let (names, values, arities) = flatten_shader_uniforms(uniforms);
    ck.fill_round_rect_shader_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        sksl,
        &names,
        &values,
        &arities,
        opacity,
        fallback.r,
        fallback.g,
        fallback.b,
        fallback.a,
    );
}

pub(super) fn stroke_round_rect(ck: &OpCk, rect: Rect, radius: f32, color: Color, width: f32) {
    ck.stroke_round_rect(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radius,
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

pub(super) fn stroke_round_rect_per_corner(
    ck: &OpCk,
    rect: Rect,
    radii: [f32; 4],
    color: Color,
    width: f32,
) {
    ck.stroke_round_rect_per_corner(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        radii[0],
        radii[1],
        radii[2],
        radii[3],
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

pub(super) fn fill_oval(ck: &OpCk, bounds: Rect, color: Color) {
    ck.fill_oval(
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.x,
        bounds.size.y,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn stroke_oval(ck: &OpCk, bounds: Rect, color: Color, width: f32) {
    ck.stroke_oval(
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.x,
        bounds.size.y,
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

pub(super) fn fill_polygon(ck: &OpCk, points: &[Point2D], color: Color) {
    if points.len() < 3 {
        return;
    }
    let mut flat: Vec<f32> = Vec::with_capacity(points.len() * 2);
    for p in points {
        flat.push(p.x);
        flat.push(p.y);
    }
    ck.fill_polygon(&flat, color.r, color.g, color.b, color.a);
}

pub(super) fn stroke_svg_path(
    ck: &OpCk,
    d: &str,
    top_left: Point2D,
    size: f32,
    color: Color,
    width: f32,
) {
    // lucide d-strings use a 24x24 viewBox.
    ck.stroke_svg_path(
        d,
        top_left.x,
        top_left.y,
        size / 24.0,
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_svg_path_with_fill_rule(
    ck: &OpCk,
    d: &str,
    top_left: Point2D,
    size: f32,
    viewbox: f32,
    color: Color,
    even_odd: bool,
) {
    ck.fill_svg_path(
        d,
        top_left.x,
        top_left.y,
        size / viewbox.max(1.0),
        even_odd,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn fill_svg_path_in_rect_with_fill_rule(
    ck: &OpCk,
    d: &str,
    rect: Rect,
    color: Color,
    even_odd: bool,
) {
    ck.fill_svg_path_in_rect(
        d,
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        even_odd,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}

pub(super) fn stroke_svg_path_in_rect(ck: &OpCk, d: &str, rect: Rect, color: Color, width: f32) {
    ck.stroke_svg_path_in_rect(
        d,
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        color.r,
        color.g,
        color.b,
        color.a,
        width,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_svg_path_in_rect_linear_gradient_with_fill_rule(
    ck: &OpCk,
    d: &str,
    rect: Rect,
    stops: &[(f32, Color)],
    angle_deg: f32,
    opacity: f32,
    even_odd: bool,
) {
    if stops.is_empty() {
        return;
    }
    let flat = flatten_gradient_stops(stops);
    ck.fill_svg_path_in_rect_linear_gradient(
        d,
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        even_odd,
        &flat,
        angle_deg,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_svg_path_in_rect_radial_gradient_with_fill_rule(
    ck: &OpCk,
    d: &str,
    rect: Rect,
    stops: &[(f32, Color)],
    cx_frac: f32,
    cy_frac: f32,
    radius_frac: f32,
    opacity: f32,
    even_odd: bool,
) {
    if stops.is_empty() {
        return;
    }
    let flat = flatten_gradient_stops(stops);
    ck.fill_svg_path_in_rect_radial_gradient(
        d,
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        even_odd,
        &flat,
        cx_frac,
        cy_frac,
        radius_frac,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_inner_shadow_svg_path_with_fill_rule(
    ck: &OpCk,
    d: &str,
    rect: Rect,
    offset_x: f32,
    offset_y: f32,
    blur: f32,
    color: Color,
    even_odd: bool,
) {
    ck.fill_inner_shadow_svg_path(
        d,
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        rect.size.y,
        even_odd,
        offset_x,
        offset_y,
        blur,
        color.r,
        color.g,
        color.b,
        color.a,
    );
}
