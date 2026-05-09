use skia_bindings as sb;

use crate::{scalar, Rect};

pub use sb::SkArc_Type as Type;
variant_name!(Type::Wedge);

/// Represents an arc along an oval boundary, or a closed wedge of the oval.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Arc {
    /// Bounds of oval containing the arc.
    pub oval: Rect,
    /// Angle in degrees where the arc begins. Zero means horizontally to the right.
    pub start_angle: scalar,
    /// Sweep angle in degrees; positive is clockwise.
    pub sweep_angle: scalar,
    pub ty: Type,
}
native_transmutable!(sb::SkArc, Arc);

impl Arc {
    pub fn new(
        oval: impl AsRef<Rect>,
        start_angle_degrees: scalar,
        sweep_angle_degrees: scalar,
        ty: Type,
    ) -> Self {
        Self {
            oval: *oval.as_ref(),
            start_angle: start_angle_degrees,
            sweep_angle: sweep_angle_degrees,
            ty,
        }
    }

    pub fn is_wedge(&self) -> bool {
        self.ty == Type::Wedge
    }
}
