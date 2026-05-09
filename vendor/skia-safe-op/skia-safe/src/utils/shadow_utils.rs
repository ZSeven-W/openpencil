use crate::{prelude::*, scalar, Canvas, Color, Matrix, Path, Point3, Rect};
use skia_bindings::{self as sb, SkShadowUtils};

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ShadowFlags: u32 {
        #[allow(clippy::unnecessary_cast)]
        const TRANSPARENT_OCCLUDER = sb::SkShadowFlags_kTransparentOccluder_ShadowFlag as u32;
        #[allow(clippy::unnecessary_cast)]
        const GEOMETRIC_ONLY = sb::SkShadowFlags_kGeometricOnly_ShadowFlag as u32;
        #[allow(clippy::unnecessary_cast)]
        const DIRECTIONAL_LIGHT = sb::SkShadowFlags_kDirectionalLight_ShadowFlag as u32;
        #[allow(clippy::unnecessary_cast)]
        const CONCAVE_BLUR_ONLY = sb::SkShadowFlags_kConcaveBlurOnly_ShadowFlag as u32;
        const ALL = Self::TRANSPARENT_OCCLUDER.bits() | Self::GEOMETRIC_ONLY.bits()
            | Self::DIRECTIONAL_LIGHT.bits() | Self::CONCAVE_BLUR_ONLY.bits();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_shadow(
    canvas: &Canvas,
    path: &Path,
    z_plane_params: impl Into<Point3>,
    light_pos: impl Into<Point3>,
    light_radius: scalar,
    ambient_color: impl Into<Color>,
    spot_color: impl Into<Color>,
    flags: impl Into<Option<ShadowFlags>>,
) {
    unsafe {
        SkShadowUtils::DrawShadow(
            canvas.native_mut(),
            path.native(),
            z_plane_params.into().native(),
            light_pos.into().native(),
            light_radius,
            ambient_color.into().into_native(),
            spot_color.into().into_native(),
            flags.into().unwrap_or_else(ShadowFlags::empty).bits(),
        )
    }
}

pub fn local_bounds(
    ctm: &Matrix,
    path: &Path,
    z_plane_params: impl Into<Point3>,
    light_pos: impl Into<Point3>,
    light_radius: scalar,
    flags: u32,
) -> Option<Rect> {
    let mut r = crate::Rect::default();
    unsafe {
        SkShadowUtils::GetLocalBounds(
            ctm.native(),
            path.native(),
            z_plane_params.into().native(),
            light_pos.into().native(),
            light_radius,
            flags,
            r.native_mut(),
        )
    }
    .then_some(r)
}

impl Canvas {
    #[allow(clippy::too_many_arguments)]
    pub fn draw_shadow(
        &self,
        path: &Path,
        z_plane_params: impl Into<Point3>,
        light_pos: impl Into<Point3>,
        light_radius: scalar,
        ambient_color: impl Into<Color>,
        spot_color: impl Into<Color>,
        flags: impl Into<Option<ShadowFlags>>,
    ) -> &Self {
        draw_shadow(
            self,
            path,
            z_plane_params,
            light_pos,
            light_radius,
            ambient_color,
            spot_color,
            flags,
        );
        self
    }
}

pub fn compute_tonal_colors(
    ambient_color: impl Into<Color>,
    spot_color: impl Into<Color>,
) -> (Color, Color) {
    let mut out_ambient_color = Color::default();
    let mut out_spot_color = Color::default();
    unsafe {
        SkShadowUtils::ComputeTonalColors(
            ambient_color.into().into_native(),
            spot_color.into().into_native(),
            out_ambient_color.native_mut(),
            out_spot_color.native_mut(),
        )
    }
    (out_ambient_color, out_spot_color)
}
