use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct KernelUnitLength {
    pub dx: scalar,
    pub dy: scalar,
}

impl KernelUnitLength {
    pub fn new(dx: scalar, dy: scalar) -> Self {
        Self { dx, dy }
    }

    pub fn new_all(value: scalar) -> Self {
        Self {
            dx: value,
            dy: value,
        }
    }
}

native_transmutable!(sb::SkSVGFeLighting_KernelUnitLength, KernelUnitLength);

pub type Lighting = RCHandle<sb::SkSVGFeLighting>;

impl NodeSubtype for sb::SkSVGFeLighting {
    type Base = sb::SkSVGFe;
}

impl DebugAttributes for Lighting {
    const NAME: &'static str = "FeLighting";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("surface_scale", &self.surface_scale())
                .field("kernel_unit_length", &self.kernel_unit_length()),
        );
    }
}

impl Lighting {
    skia_svg_macros::attrs! {
        SkSVGFeLighting => {
            *surface_scale: scalar [get(value) => value, set(value) => value],
            *kernel_unit_length?: KernelUnitLength [get(value) => value.map(KernelUnitLength::from_native_c), set(value) => value.into_native()]
        }
    }
}

pub type Specular = RCHandle<sb::SkSVGFeSpecularLighting>;

impl NodeSubtype for sb::SkSVGFeSpecularLighting {
    type Base = sb::SkSVGFeLighting;
}

impl_default_make!(Specular, sb::C_SkSVGFeSpecularLighting_Make);

impl DebugAttributes for Specular {
    const NAME: &'static str = "FeSpecularLighting";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("specular_constant", &self.specular_constant())
                .field("specular_exponent", &self.specular_exponent()),
        );
    }
}

impl Specular {
    skia_svg_macros::attrs! {
        SkSVGFeSpecularLighting => {
            *specular_constant: scalar [get(value) => value, set(value) => value],
            *specular_exponent: scalar [get(value) => value, set(value) => value]
        }
    }
}

pub type Diffuse = RCHandle<sb::SkSVGFeDiffuseLighting>;

impl NodeSubtype for sb::SkSVGFeDiffuseLighting {
    type Base = sb::SkSVGFeLighting;
}

impl_default_make!(Diffuse, sb::C_SkSVGFeDiffuseLighting_Make);

impl DebugAttributes for Diffuse {
    const NAME: &'static str = "FeDiffuseLighting";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()
            ._dbg(builder.field("diffuse_constant", &self.diffuse_constant()));
    }
}

impl Diffuse {
    skia_svg_macros::attrs! {
        SkSVGFeDiffuseLighting => {
            *diffuse_constant: scalar [get(value) => value, set(value) => value]
        }
    }
}
