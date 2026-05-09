use crate::{scalar, PathEffect};
use skia_bindings as sb;

impl PathEffect {
    pub fn discrete(
        seg_length: scalar,
        dev: scalar,
        seed_assist: impl Into<Option<u32>>,
    ) -> Option<Self> {
        new(seg_length, dev, seed_assist)
    }
}

pub fn new(
    seg_length: scalar,
    dev: scalar,
    seed_assist: impl Into<Option<u32>>,
) -> Option<PathEffect> {
    PathEffect::from_ptr(unsafe {
        sb::C_SkDiscretePathEffect_Make(seg_length, dev, seed_assist.into().unwrap_or(0))
    })
}
