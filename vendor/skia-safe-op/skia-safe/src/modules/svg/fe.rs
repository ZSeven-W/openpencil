mod blend;
mod color_matrix;
mod component_transfer;
mod composite;
mod displacement_map;
mod flood;
mod gaussian_blur;
mod image;
mod light_source;
pub mod lighting;
mod merge;
pub mod morphology;
mod offset;
mod turbulence;
mod types;

pub use self::{
    blend::*, color_matrix::*, component_transfer::*, composite::*, displacement_map::*, flood::*,
    gaussian_blur::*, image::*, light_source::*, lighting::Diffuse as DiffuseLighting,
    lighting::Lighting, lighting::Specular as SpecularLighting, merge::*, morphology::Morphology,
    offset::*, turbulence::*, types::*,
};

use super::{DebugAttributes, Length, NodeSubtype};
use crate::prelude::*;
use skia_bindings as sb;

pub type Fe = RCHandle<sb::SkSVGFe>;

impl NodeSubtype for sb::SkSVGFe {
    type Base = sb::SkSVGContainer;
}

impl DebugAttributes for Fe {
    const NAME: &'static str = "Fe";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("input", self.input())
                .field("result", self.result())
                .field("x", &self.x())
                .field("y", &self.y())
                .field("width", &self.width())
                .field("height", &self.height()),
        );
    }
}

impl Fe {
    // TODO: Wrap IsFilterEffect (via typed)
    // TODO: Wrap makeImageFilter()
    // TODO: Wrap resolveFilterSubregion()
    // TODO: Wrap resolveColorSpace()
    // TODO: Wrap applyProperties()

    skia_svg_macros::attrs! {
        SkSVGFe => {
            "in" as input: Input [get(value) => Input::from_native_ref(value), set(value) => value.into_native()],
            result: crate::interop::String [get(value) => crate::interop::String::from_native_ref(value), set(value) => value.into_native()],
            x?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            y?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            width?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            height?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()]
        }
    }
}
