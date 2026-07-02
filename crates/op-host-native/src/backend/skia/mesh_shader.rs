//! Mesh-gradient + SkSL-shader fill methods on [`NativeBackend`].
//!
//! Ports jian-skia's `draw_mesh_gradient_rect` / `draw_shader_rect`
//! (vendor/jian/crates/jian-skia/src/backend.rs) onto the editor's
//! direct-canvas painter so canvas nodes with `PenFill::MeshGradient`
//! / `PenFill::Shader` render for real on the native host instead of
//! falling back to the jian `Painter` trait's first-vertex / fallback
//! solid defaults (which remain the web/CanvasKit behaviour — the
//! documented parity gap). Carved into a sibling file per the 800-line
//! cap, same as `gradient.rs`.

use super::{jian_color_to_color4f, to_sk_rect, NativeBackend};
use op_editor_ui::{Color, Rect};

impl NativeBackend {
    /// Gouraud-fill a (round-)rect from a row-major `rows`×`cols`
    /// vertex-colour lattice. Grid cells triangulate into two CCW
    /// triangles each; `opacity` rides in the paint alpha so the
    /// vertex colours stay at their authored values.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_round_rect_mesh_gradient(
        &self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        rows: u32,
        cols: u32,
        colors: &[Color],
        opacity: f32,
    ) {
        use skia_safe::{vertices::VertexMode, BlendMode, Color4f, Paint, Vertices};

        let rows = rows.max(2);
        let cols = cols.max(2);
        let vcount = (rows * cols) as usize;
        if colors.len() != vcount {
            // Malformed lattice — paint the first-vertex fallback so the
            // node stays visible (mirrors the trait default).
            if let Some(first) = colors.first() {
                let mut c = *first;
                c.a = (c.a * opacity).clamp(0.0, 1.0);
                self.fill_round_rect(canvas, rect, radius, c);
            }
            return;
        }

        let mut positions: Vec<skia_safe::Point> = Vec::with_capacity(vcount);
        let mut sk_colors: Vec<skia_safe::Color> = Vec::with_capacity(vcount);
        let denom_c = (cols - 1) as f32;
        let denom_r = (rows - 1) as f32;
        for r in 0..rows {
            for c in 0..cols {
                let fx = rect.origin.x + (c as f32 / denom_c) * rect.size.x;
                let fy = rect.origin.y + (r as f32 / denom_r) * rect.size.y;
                positions.push(skia_safe::Point::new(fx, fy));
                let jc = colors[(r * cols + c) as usize];
                sk_colors.push(jian_color_to_color4f(jc).to_color());
            }
        }

        // Index of vertex (r, c) is `r * cols + c`.
        let mut indices: Vec<u16> = Vec::with_capacity(((rows - 1) * (cols - 1) * 6) as usize);
        for r in 0..(rows - 1) {
            for c in 0..(cols - 1) {
                let tl = (r * cols + c) as u16;
                let tr = (r * cols + c + 1) as u16;
                let bl = ((r + 1) * cols + c) as u16;
                let br = ((r + 1) * cols + c + 1) as u16;
                indices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
            }
        }

        let vertices = Vertices::new_copy(
            VertexMode::Triangles,
            &positions,
            &positions,
            &sk_colors,
            Some(&indices),
        );

        // Clip to the round-rect so the Gouraud fill respects corner
        // radius, then draw. A default Paint is opaque black — seed it
        // white and Modulate (white × vertex == vertex) so the vertex
        // colours pass through unchanged; `opacity` rides in the alpha.
        let restore_count = canvas.save();
        if radius > 0.0 {
            let rrect = skia_safe::RRect::new_rect_xy(to_sk_rect(rect), radius, radius);
            canvas.clip_rrect(rrect, None, Some(true));
        } else {
            canvas.clip_rect(to_sk_rect(rect), None, Some(true));
        }
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_color4f(Color4f::new(1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)), None);
        canvas.draw_vertices(&vertices, BlendMode::Modulate, &paint);
        canvas.restore_to_count(restore_count);
    }

    /// Fill a (round-)rect with a compiled SkSL program. The compile is
    /// cached per distinct source (failures too) in `shader_cache`;
    /// uniform bind errors are ignored per-uniform so one bad binding
    /// doesn't sink the fill. On compile failure the visible `fallback`
    /// solid paints instead.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_round_rect_shader(
        &mut self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        radius: f32,
        sksl: &str,
        uniforms: &[(&str, &[f32])],
        opacity: f32,
        fallback: Color,
    ) {
        use skia_safe::runtime_effect::RuntimeShaderBuilder;
        use skia_safe::{Paint, PaintStyle};

        let shader = self.shader_cache.get_or_compile(sksl).and_then(|effect| {
            // `RuntimeEffect` is an RCHandle — the clone handed to the
            // builder is a refcount bump, not a recompile.
            let mut builder = RuntimeShaderBuilder::new(effect);
            for (name, values) in uniforms {
                let _ = builder.set_uniform_float(name, values);
            }
            builder.make_shader(&skia_safe::Matrix::default())
        });

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        match shader {
            Some(s) => {
                paint.set_shader(s);
                paint.set_alpha_f(opacity.clamp(0.0, 1.0));
            }
            None => {
                let mut c = jian_color_to_color4f(fallback);
                c.a = (c.a * opacity).clamp(0.0, 1.0);
                paint.set_color4f(c, None);
            }
        }
        if radius > 0.0 {
            let rrect = skia_safe::RRect::new_rect_xy(to_sk_rect(rect), radius, radius);
            canvas.draw_rrect(rrect, &paint);
        } else {
            canvas.draw_rect(to_sk_rect(rect), &paint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::Point2D;

    fn raster_canvas_run(
        w: i32,
        h: i32,
        f: impl FnOnce(&mut NativeBackend, &skia_safe::Canvas),
    ) -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).expect("raster surface");
        let mut backend = NativeBackend::with_dpi(1.0);
        f(&mut backend, surface.canvas());
        let image = surface.image_snapshot();
        let pixmap = image.peek_pixels().expect("cpu raster pixels");
        pixmap.bytes().expect("pixel bytes").to_vec()
    }

    fn rect_full(w: f32, h: f32) -> Rect {
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(w, h),
        }
    }

    #[test]
    fn mesh_gradient_interpolates_between_vertex_colors() {
        // 2×2 lattice: red / red on top, blue / blue on bottom — the
        // vertical midline must be neither pure red nor pure blue.
        let red = Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let blue = Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        };
        let px = raster_canvas_run(64, 64, |b, canvas| {
            b.fill_round_rect_mesh_gradient(
                canvas,
                rect_full(64.0, 64.0),
                0.0,
                2,
                2,
                &[red, red, blue, blue],
                1.0,
            );
        });
        // BGRA/RGBA layout differs per platform; compare channel maxes
        // instead of fixed offsets. Top row ≈ red-ish, bottom ≈ blue-ish,
        // middle has BOTH components clearly present (interpolated).
        let at = |x: usize, y: usize| -> (u8, u8, u8) {
            let i = (y * 64 + x) * 4;
            (px[i], px[i + 1], px[i + 2])
        };
        let (m0, m1, m2) = at(32, 32);
        let mid_channels = [m0, m1, m2];
        let strong = mid_channels.iter().filter(|v| **v > 60).count();
        assert!(
            strong >= 2,
            "midline should be an interpolated mix, got {mid_channels:?}"
        );
        let top = at(32, 2);
        let bottom = at(32, 61);
        assert_ne!(top, bottom, "gradient must vary across the lattice");
    }

    #[test]
    fn shader_fill_compiles_and_beats_the_fallback() {
        let green_fallback = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        // Constant-red program: any pixel being red proves the shader
        // ran; green would mean the fallback painted instead.
        let px = raster_canvas_run(16, 16, |b, canvas| {
            b.fill_round_rect_shader(
                canvas,
                rect_full(16.0, 16.0),
                0.0,
                "half4 main(float2 p){ return half4(1.0, 0.0, 0.0, 1.0); }",
                &[],
                1.0,
                green_fallback,
            );
        });
        let i = (8 * 16 + 8) * 4;
        let channels = [px[i], px[i + 1], px[i + 2]];
        assert!(
            channels.contains(&255) && channels.contains(&0),
            "expected pure shader red, got {channels:?}"
        );
        // Green channel must NOT be the strong one (that's the fallback).
        assert!(px[i + 1] < 60, "fallback green leaked: {channels:?}");
    }

    #[test]
    fn shader_compile_failure_paints_fallback() {
        let fallback = Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        };
        let px = raster_canvas_run(8, 8, |b, canvas| {
            b.fill_round_rect_shader(
                canvas,
                rect_full(8.0, 8.0),
                0.0,
                "this is not sksl",
                &[],
                1.0,
                fallback,
            );
        });
        let i = (4 * 8 + 4) * 4;
        assert_eq!(px[i + 1], 255, "fallback green expected");
    }
}
