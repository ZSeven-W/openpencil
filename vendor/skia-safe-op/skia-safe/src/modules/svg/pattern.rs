use super::{DebugAttributes, Iri, Length, NodeSubtype};
use crate::{impl_default_make, prelude::*, Matrix};
use skia_bindings as sb;

pub type Pattern = RCHandle<sb::SkSVGPattern>;

impl NodeSubtype for sb::SkSVGPattern {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Pattern, sb::C_SkSVGPattern_Make);

impl DebugAttributes for Pattern {
    const NAME: &'static str = "Pattern";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder);

        builder
            .field("href", &self.href())
            .field("x", &self.x())
            .field("y", &self.y())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("pattern_transform", &self.pattern_transform());
    }
}

impl Pattern {
    skia_svg_macros::attrs! {
        SkSVGPattern => {
            href: Iri [get(value) => Iri::from_native_ref(value), set(value) => value.into_native()],
            x?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            y?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            width?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            height?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            pattern_transform?: Matrix [get(value) => value.map(Matrix::from_native_ref), set(value) => value.into_native()]
        }
    }
}
