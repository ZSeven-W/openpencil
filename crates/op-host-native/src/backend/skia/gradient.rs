//! Gradient shader paint methods on [`NativeBackend`] plus their
//! pure helpers (`linear_gradient_endpoints`, `gradient_color_arrays`,
//! `premul_interpolation`, `fold_opacity`).
//!
//! Carved out of `skia.rs` so the parent file stays under the
//! workspace's 800-line cap. The methods continue to live on
//! `NativeBackend` via a sibling `impl` block; the helpers stay
//! `pub(super)` so the parent's tests module (`skia/tests.rs`) can
//! exercise them without re-exporting.

use super::{to_sk_rect, NativeBackend};
use op_editor_ui::{Color, Rect};

impl NativeBackend {
    /// Fill a (round-)rectangle with a linear gradient. `stops`
    /// carries `(offset, color)` pairs ordered by ascending offset;
    /// `angle_deg` is the canonical `.op` angle (0° = bottom→top,
    /// 90° = left→right, matches CSS `to-top`); `opacity` is folded
    /// into each stop's alpha. Endpoints sit on the bounding ellipse,
    /// not the AABB — matches `pen-renderer/src/node-renderer.ts:155`
    /// so native, web, and export agree on gradient direction +
    /// extent.
    pub fn fill_round_rect_linear_gradient(
        &self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        if stops.is_empty() {
            return;
        }
        let (start, end) = linear_gradient_endpoints(rect, angle_deg);
        let (colors, offsets) = gradient_color_arrays(stops, opacity);
        let grad_colors = skia_safe::gradient::Colors::new(
            &colors[..],
            Some(offsets.as_slice()),
            skia_safe::TileMode::Clamp,
            None,
        );
        let gradient = skia_safe::gradient::Gradient::new(grad_colors, premul_interpolation());
        let Some(shader) =
            skia_safe::gradient::shaders::linear_gradient((start, end), &gradient, None)
        else {
            // Skia refused to build the shader (degenerate input);
            // degrade to first-stop solid so the node still paints.
            self.fill_round_rect(canvas, rect, radius, fold_opacity(stops[0].1, opacity));
            return;
        };
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(shader);
        canvas.draw_round_rect(to_sk_rect(rect), radius, radius, &paint);
    }

    /// Fill a (round-)rectangle with a radial gradient. `cx_frac` /
    /// `cy_frac` are 0.0..=1.0 fractions of `rect`'s width / height
    /// for the centre; `radius_frac` is a 0.0..=1.0 fraction of
    /// `max(w, h)` for the outer radius — matches
    /// `pen-renderer/src/node-renderer.ts:187` so existing `.op`
    /// files render at the same radial size on native + web +
    /// export. Stops + opacity follow the linear convention.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_round_rect_radial_gradient(
        &self,
        canvas: &skia_safe::Canvas,
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
        let center = skia_safe::Point::new(
            rect.origin.x + rect.size.x * cx_frac.clamp(0.0, 1.0),
            rect.origin.y + rect.size.y * cy_frac.clamp(0.0, 1.0),
        );
        // Match `pen-renderer/src/node-renderer.ts:187` — outer
        // radius is `radius_frac × max(w, h)`. Earlier this used
        // `min(w, h) / 2`, which made existing `.op` radial
        // gradients render at roughly 1/4 the size on native vs.
        // web/export.
        let outer = (rect.size.x.max(rect.size.y)) * radius_frac.clamp(0.0, 1.0);
        let outer = outer.max(0.01); // Sub-pixel radii confuse skia's shader.
        let (colors, offsets) = gradient_color_arrays(stops, opacity);
        let grad_colors = skia_safe::gradient::Colors::new(
            &colors[..],
            Some(offsets.as_slice()),
            skia_safe::TileMode::Clamp,
            None,
        );
        let gradient = skia_safe::gradient::Gradient::new(grad_colors, premul_interpolation());
        let Some(shader) =
            skia_safe::gradient::shaders::radial_gradient((center, outer), &gradient, None)
        else {
            self.fill_round_rect(canvas, rect, radius, fold_opacity(stops[0].1, opacity));
            return;
        };
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_shader(shader);
        canvas.draw_round_rect(to_sk_rect(rect), radius, radius, &paint);
    }
}

/// Multiply a colour's alpha by a per-fill opacity factor. Used as
/// the fallback when skia refuses to build a gradient shader (e.g.
/// degenerate single-stop input).
pub(super) fn fold_opacity(c: Color, opacity: f32) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a * opacity.clamp(0.0, 1.0),
    }
}

/// Project the canonical `.op` `angle` onto the two gradient
/// endpoints, matching `pen-renderer/src/node-renderer.ts:155`
/// verbatim:
///
/// ```text
/// rad  = (angle - 90) · π/180     // 0° = bottom→top, 90° = left→right
/// x1   = cx - cos · w/2,   y1 = cy - sin · h/2
/// x2   = cx + cos · w/2,   y2 = cy + sin · h/2
/// ```
///
/// The endpoint scale uses `w/2` on the x-axis and `h/2` on the
/// y-axis — endpoints sit on the bounding ellipse, NOT the AABB.
/// Re-using the AABB-projection trick here would stretch the
/// gradient band at off-axis angles and diverge from the TS
/// renderer + the export pipeline.
pub(super) fn linear_gradient_endpoints(
    rect: Rect,
    angle_deg: f32,
) -> (skia_safe::Point, skia_safe::Point) {
    let cx = rect.origin.x + rect.size.x / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;
    let rad = (angle_deg - 90.0).to_radians();
    let (sin, cos) = (rad.sin(), rad.cos());
    let dx = cos * rect.size.x * 0.5;
    let dy = sin * rect.size.y * 0.5;
    (
        skia_safe::Point::new(cx - dx, cy - dy),
        skia_safe::Point::new(cx + dx, cy + dy),
    )
}

/// Premultiplied-colour interpolation in the destination colour
/// space — matches what skia's deprecated `Flags::INTERPOLATE_COLORS_IN_PREMUL`
/// configured, with the new gradient API.
pub(super) fn premul_interpolation() -> skia_safe::gradient::Interpolation {
    use skia_safe::gradient::interpolation::{ColorSpace, HueMethod, InPremul};
    skia_safe::gradient::Interpolation {
        in_premul: InPremul::Yes,
        color_space: ColorSpace::Destination,
        hue_method: HueMethod::Shorter,
    }
}

/// Build the parallel `Color4f` + offset arrays skia's gradient
/// shaders consume from our `(offset, Color)` stops; `opacity` is
/// folded into each stop's alpha so the shader stays single-pass.
pub(super) fn gradient_color_arrays(
    stops: &[(f32, Color)],
    opacity: f32,
) -> (Vec<skia_safe::Color4f>, Vec<f32>) {
    let op = opacity.clamp(0.0, 1.0);
    let colors: Vec<skia_safe::Color4f> = stops
        .iter()
        .map(|(_, c)| {
            skia_safe::Color4f::new(
                c.r.clamp(0.0, 1.0),
                c.g.clamp(0.0, 1.0),
                c.b.clamp(0.0, 1.0),
                (c.a * op).clamp(0.0, 1.0),
            )
        })
        .collect();
    let offsets: Vec<f32> = stops.iter().map(|(t, _)| t.clamp(0.0, 1.0)).collect();
    (colors, offsets)
}
