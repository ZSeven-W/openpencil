use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TurbulenceBaseFrequency {
    pub x: scalar,
    pub y: scalar,
}

impl TurbulenceBaseFrequency {
    pub fn new(x: scalar, y: scalar) -> Self {
        Self { x, y }
    }

    pub fn new_all(value: scalar) -> Self {
        Self { x: value, y: value }
    }
}

native_transmutable!(sb::SkSVGFeTurbulenceBaseFrequency, TurbulenceBaseFrequency);

pub type TurbulenceType = sb::SkSVGFeTurbulenceType_Type;
variant_name!(TurbulenceType::FractalNoise);

pub type Turbulence = RCHandle<sb::SkSVGFeTurbulence>;

impl NodeSubtype for sb::SkSVGFeTurbulence {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Turbulence, sb::C_SkSVGFeTurbulence_Make);

impl DebugAttributes for Turbulence {
    const NAME: &'static str = "FeTurbulence";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("base_frequency", self.base_frequency())
                .field("num_octaves", &self.num_octaves())
                .field("seed", &self.seed())
                .field("turbulence_type", self.turbulence_type()),
        );
    }
}

impl Turbulence {
    skia_svg_macros::attrs! {
        SkSVGFeTurbulence => {
            base_frequency: TurbulenceBaseFrequency [get(value) => TurbulenceBaseFrequency::from_native_ref(value), set(value) => value.into_native()],
            *num_octaves: i32 [get(value) => value, set(value) => value],
            *seed: scalar [get(value) => value, set(value) => value],
            turbulence_type: TurbulenceType [get(value) => &value.fType, set(value) => sb::SkSVGFeTurbulenceType { fType: value }]
        }
    }
}
