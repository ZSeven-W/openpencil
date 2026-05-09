#![allow(deprecated)]

use crate::{
    gradient, scalar, shaders, Color, Color4f, ColorSpace, Matrix, Point, Shader, TileMode,
};

pub use gradient::{interpolation, Colors as GradientColors, Gradient, Interpolation};

impl From<Flags> for Interpolation {
    fn from(flags: Flags) -> Self {
        let in_premul = if flags.contains(Flags::INTERPOLATE_COLORS_IN_PREMUL) {
            interpolation::InPremul::Yes
        } else {
            interpolation::InPremul::No
        };
        Self {
            in_premul,
            color_space: interpolation::ColorSpace::Destination,
            hue_method: interpolation::HueMethod::Shorter,
        }
    }
}

impl Shader {
    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::linear_gradient() instead"
    )]
    pub fn linear_gradient<'a>(
        points: (impl Into<Point>, impl Into<Point>),
        colors: impl Into<GradientShaderColors<'a>>,
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        flags: impl Into<Option<Flags>>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Self> {
        linear(points, colors, pos, mode, flags, local_matrix)
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::linear_gradient() instead"
    )]
    pub fn linear_gradient_with_interpolation<'a>(
        points: (impl Into<Point>, impl Into<Point>),
        colors: (&'a [Color4f], impl Into<Option<ColorSpace>>),
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        interpolation: impl Into<Interpolation>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Self> {
        linear_with_interpolation(points, colors, pos, mode, interpolation, local_matrix)
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::radial_gradient() instead"
    )]
    pub fn radial_gradient<'a>(
        center: impl Into<Point>,
        radius: scalar,
        colors: impl Into<GradientShaderColors<'a>>,
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        flags: impl Into<Option<self::Flags>>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Self> {
        radial(center, radius, colors, pos, mode, flags, local_matrix)
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::radial_gradient() instead"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn radial_gradient_with_interpolation<'a>(
        center_and_radius: (impl Into<Point>, scalar),
        colors: (&'a [Color4f], impl Into<Option<ColorSpace>>),
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        interpolation: impl Into<Interpolation>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        radial_with_interpolation(
            center_and_radius,
            colors,
            pos,
            mode,
            interpolation,
            local_matrix,
        )
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::two_point_conical_gradient() instead"
    )]
    #[allow(clippy::too_many_arguments)]
    pub fn two_point_conical_gradient<'a>(
        start: impl Into<Point>,
        start_radius: scalar,
        end: impl Into<Point>,
        end_radius: scalar,
        colors: impl Into<GradientShaderColors<'a>>,
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        flags: impl Into<Option<self::Flags>>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Self> {
        two_point_conical(
            start,
            start_radius,
            end,
            end_radius,
            colors,
            pos,
            mode,
            flags,
            local_matrix,
        )
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::two_point_conical_gradient() instead"
    )]
    pub fn two_point_conical_gradient_with_interpolation<'a>(
        start_and_radius: (impl Into<Point>, scalar),
        end_and_radius: (impl Into<Point>, scalar),
        colors: (&'a [Color4f], impl Into<Option<ColorSpace>>),
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        interpolation: impl Into<Interpolation>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        two_point_conical_with_interpolation(
            start_and_radius,
            end_and_radius,
            colors,
            pos,
            mode,
            interpolation,
            local_matrix,
        )
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::sweep_gradient() instead"
    )]
    pub fn sweep_gradient<'a>(
        center: impl Into<Point>,
        colors: impl Into<GradientShaderColors<'a>>,
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        angles: impl Into<Option<(scalar, scalar)>>,
        flags: impl Into<Option<self::Flags>>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Self> {
        sweep(center, colors, pos, mode, angles, flags, local_matrix)
    }

    #[deprecated(
        since = "0.93.0",
        note = "Use gradient::shaders::sweep_gradient() instead"
    )]
    pub fn sweep_gradient_with_interpolation<'a>(
        center: impl Into<Point>,
        colors: (&'a [Color4f], impl Into<Option<ColorSpace>>),
        pos: impl Into<Option<&'a [scalar]>>,
        mode: TileMode,
        angles: impl Into<Option<(scalar, scalar)>>,
        interpolation: impl Into<Interpolation>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        sweep_with_interpolation(
            center,
            colors,
            pos,
            mode,
            angles,
            interpolation,
            local_matrix,
        )
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Flags: u32 {
        const INTERPOLATE_COLORS_IN_PREMUL = 1 << 0;
    }
}

impl Default for self::Flags {
    fn default() -> Self {
        Self::empty()
    }
}

pub fn linear<'a>(
    points: (impl Into<Point>, impl Into<Point>),
    colors: impl Into<GradientShaderColors<'a>>,
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    flags: impl Into<Option<Flags>>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let colors = colors.into();
    let pos = pos.into();
    let flags = flags.into().unwrap_or_default();

    match colors {
        GradientShaderColors::Colors(colors) => {
            // Convert Color to Color4f
            let colors4f: Vec<Color4f> = colors.iter().map(|c| Color4f::from(*c)).collect();
            let grad_colors = GradientColors::new(&colors4f, pos, mode, None);
            let grad = Gradient::new(grad_colors, flags);
            shaders::linear_gradient(points, &grad, local_matrix)
        }
        GradientShaderColors::ColorsInSpace(colors, color_space) => linear_with_interpolation(
            points,
            (colors, color_space),
            pos,
            mode,
            flags,
            local_matrix,
        ),
    }
}

pub fn linear_with_interpolation<'a>(
    points: (impl Into<Point>, impl Into<Point>),
    (colors, color_space): (&'a [Color4f], impl Into<Option<ColorSpace>>),
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    interpolation: impl Into<Interpolation>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let pos = pos.into();
    let color_space = color_space.into();
    let grad_colors = GradientColors::new(colors, pos, mode, color_space);
    let grad = Gradient::new(grad_colors, interpolation);
    shaders::linear_gradient(points, &grad, local_matrix)
}

pub fn radial<'a>(
    center: impl Into<Point>,
    radius: scalar,
    colors: impl Into<GradientShaderColors<'a>>,
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    flags: impl Into<Option<self::Flags>>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let colors = colors.into();
    let pos = pos.into();
    let flags = flags.into().unwrap_or_default();

    match colors {
        GradientShaderColors::Colors(colors) => {
            let colors4f: Vec<Color4f> = colors.iter().map(|c| Color4f::from(*c)).collect();
            let grad_colors = GradientColors::new(&colors4f, pos, mode, None);
            let grad = Gradient::new(grad_colors, flags);
            shaders::radial_gradient((center, radius), &grad, local_matrix)
        }
        GradientShaderColors::ColorsInSpace(colors, color_space) => radial_with_interpolation(
            (center, radius),
            (colors, color_space),
            pos,
            mode,
            flags,
            local_matrix,
        ),
    }
}

pub fn radial_with_interpolation<'a>(
    (center, radius): (impl Into<Point>, scalar),
    (colors, color_space): (&'a [Color4f], impl Into<Option<ColorSpace>>),
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    interpolation: impl Into<Interpolation>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let pos = pos.into();
    let color_space = color_space.into();
    let grad_colors = GradientColors::new(colors, pos, mode, color_space);
    let grad = Gradient::new(grad_colors, interpolation);
    shaders::radial_gradient((center, radius), &grad, local_matrix)
}

#[allow(clippy::too_many_arguments)]
pub fn two_point_conical<'a>(
    start: impl Into<Point>,
    start_radius: scalar,
    end: impl Into<Point>,
    end_radius: scalar,
    colors: impl Into<GradientShaderColors<'a>>,
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    flags: impl Into<Option<self::Flags>>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let colors = colors.into();
    let pos = pos.into();
    let flags = flags.into().unwrap_or_default();

    match colors {
        GradientShaderColors::Colors(colors) => {
            let colors4f: Vec<Color4f> = colors.iter().map(|c| Color4f::from(*c)).collect();
            let grad_colors = GradientColors::new(&colors4f, pos, mode, None);
            let grad = Gradient::new(grad_colors, flags);
            shaders::two_point_conical_gradient(
                (start, start_radius),
                (end, end_radius),
                &grad,
                local_matrix,
            )
        }
        GradientShaderColors::ColorsInSpace(colors, color_space) => {
            two_point_conical_with_interpolation(
                (start, start_radius),
                (end, end_radius),
                (colors, color_space),
                pos,
                mode,
                flags,
                local_matrix,
            )
        }
    }
}

pub fn two_point_conical_with_interpolation<'a>(
    (start, start_radius): (impl Into<Point>, scalar),
    (end, end_radius): (impl Into<Point>, scalar),
    (colors, color_space): (&'a [Color4f], impl Into<Option<ColorSpace>>),
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    interpolation: impl Into<Interpolation>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let pos = pos.into();
    let color_space = color_space.into();
    let grad_colors = GradientColors::new(colors, pos, mode, color_space);
    let grad = Gradient::new(grad_colors, interpolation);
    shaders::two_point_conical_gradient(
        (start, start_radius),
        (end, end_radius),
        &grad,
        local_matrix,
    )
}

pub fn sweep<'a>(
    center: impl Into<Point>,
    colors: impl Into<GradientShaderColors<'a>>,
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    angles: impl Into<Option<(scalar, scalar)>>,
    flags: impl Into<Option<self::Flags>>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let colors = colors.into();
    let pos = pos.into();
    let angles = angles.into();
    let flags = flags.into().unwrap_or_default();

    let (start_angle, end_angle) = (
        angles.map(|a| a.0).unwrap_or(0.0),
        angles.map(|a| a.1).unwrap_or(360.0),
    );

    match colors {
        GradientShaderColors::Colors(colors) => {
            let colors4f: Vec<Color4f> = colors.iter().map(|c| Color4f::from(*c)).collect();
            let grad_colors = GradientColors::new(&colors4f, pos, mode, None);
            let grad = Gradient::new(grad_colors, flags);
            shaders::sweep_gradient(center, (start_angle, end_angle), &grad, local_matrix)
        }
        GradientShaderColors::ColorsInSpace(colors, color_space) => sweep_with_interpolation(
            center,
            (colors, color_space),
            pos,
            mode,
            angles,
            flags,
            local_matrix,
        ),
    }
}

pub fn sweep_with_interpolation<'a>(
    center: impl Into<Point>,
    (colors, color_space): (&'a [Color4f], impl Into<Option<ColorSpace>>),
    pos: impl Into<Option<&'a [scalar]>>,
    mode: TileMode,
    angles: impl Into<Option<(scalar, scalar)>>,
    interpolation: impl Into<Interpolation>,
    local_matrix: impl Into<Option<&'a Matrix>>,
) -> Option<Shader> {
    let pos = pos.into();
    let angles = angles.into();
    let color_space = color_space.into();

    let (start_angle, end_angle) = (
        angles.map(|a| a.0).unwrap_or(0.0),
        angles.map(|a| a.1).unwrap_or(360.0),
    );

    let grad_colors = GradientColors::new(colors, pos, mode, color_space);
    let grad = Gradient::new(grad_colors, interpolation);
    shaders::sweep_gradient(center, (start_angle, end_angle), &grad, local_matrix)
}

/// Type that represents either a slice of [`Color`], or a slice of [`Color4f`] and a color space.
/// Whenever this type is expected, it's either possible to directly pass a `&[Color]` , or
/// a tuple of type `(&[Color4f], &ColorSpace)`.
#[derive(Debug)]
pub enum GradientShaderColors<'a> {
    Colors(&'a [Color]),
    ColorsInSpace(&'a [Color4f], Option<ColorSpace>),
}

impl GradientShaderColors<'_> {
    pub fn len(&self) -> usize {
        match self {
            GradientShaderColors::Colors(colors) => colors.len(),
            GradientShaderColors::ColorsInSpace(colors, _) => colors.len(),
        }
    }

    // to keep clippy happy.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> From<&'a [Color]> for GradientShaderColors<'a> {
    fn from(colors: &'a [Color]) -> Self {
        GradientShaderColors::<'a>::Colors(colors)
    }
}

impl<'a> From<(&'a [Color4f], ColorSpace)> for GradientShaderColors<'a> {
    fn from(c: (&'a [Color4f], ColorSpace)) -> Self {
        GradientShaderColors::<'a>::ColorsInSpace(c.0, Some(c.1))
    }
}

impl<'a> From<(&'a [Color4f], Option<ColorSpace>)> for GradientShaderColors<'a> {
    fn from(c: (&'a [Color4f], Option<ColorSpace>)) -> Self {
        GradientShaderColors::<'a>::ColorsInSpace(c.0, c.1)
    }
}

impl<'a> From<&'a [Color4f]> for GradientShaderColors<'a> {
    fn from(c: &'a [Color4f]) -> Self {
        GradientShaderColors::<'a>::ColorsInSpace(c, None)
    }
}
