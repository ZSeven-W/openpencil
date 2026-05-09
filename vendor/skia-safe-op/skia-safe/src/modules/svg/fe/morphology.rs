use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Radius {
    pub x: scalar,
    pub y: scalar,
}

impl Radius {
    pub fn new(x: scalar, y: scalar) -> Self {
        Self { x, y }
    }

    pub fn new_all(value: scalar) -> Self {
        Self { x: value, y: value }
    }
}

native_transmutable!(sb::SkSVGFeMorphology_Radius, Radius);

pub type Operator = sb::SkSVGFeMorphology_Operator;
variant_name!(Operator::Dilate);

pub type Morphology = RCHandle<sb::SkSVGFeMorphology>;

impl NodeSubtype for sb::SkSVGFeMorphology {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Morphology, sb::C_SkSVGFeMorphology_Make);

impl DebugAttributes for Morphology {
    const NAME: &'static str = "FeMorphology";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("operator", self.operator())
                .field("radius", &self.radius()),
        );
    }
}

impl Morphology {
    skia_svg_macros::attrs! {
        SkSVGFeMorphology => {
            operator: Operator [get(value) => value, set(value) => value],
            radius: Radius [get(value) => Radius::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
