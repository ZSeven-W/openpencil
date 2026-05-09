use crate::{scalar, PathEffect};
use skia_bindings as sb;

impl PathEffect {
    pub fn corner_path(radius: scalar) -> Option<Self> {
        new(radius)
    }
}

pub fn new(radius: scalar) -> Option<PathEffect> {
    PathEffect::from_ptr(unsafe { sb::C_SkCornerPathEffect_Make(radius) })
}
