mod linear;
mod radial;

pub use self::{linear::Linear as LinearGradient, radial::Radial as RadialGradient};

use super::{BoundingBoxUnits, DebugAttributes, Iri, NodeSubtype, SpreadMethod};
use crate::{prelude::*, Matrix};
use skia_bindings as sb;

pub type Gradient = RCHandle<sb::SkSVGGradient>;

impl NodeSubtype for sb::SkSVGGradient {
    type Base = sb::SkSVGContainer;
}

impl DebugAttributes for Gradient {
    const NAME: &'static str = "Gradient";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("href", self.href())
                .field("gradient_transform", self.gradient_transform())
                .field("spread_method", self.spread_method())
                .field("gradient_units", self.gradient_units()),
        );
    }
}

impl Gradient {
    skia_svg_macros::attrs! {
        SkSVGGradient => {
            href: Iri [get(value) => Iri::from_native_ref(value), set(value) => value.into_native()],
            gradient_transform: Matrix [get(value) => Matrix::from_native_ref(value), set(value) => value.into_native()],
            spread_method: SpreadMethod [get(value) => &value.fType, set(value) => sb::SkSVGSpreadMethod { fType: value }],
            gradient_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }]
        }
    }
}
