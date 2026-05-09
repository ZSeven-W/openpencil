use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length, NodeSubtype},
};
use skia_bindings as sb;

pub type Linear = RCHandle<sb::SkSVGLinearGradient>;

impl NodeSubtype for sb::SkSVGLinearGradient {
    type Base = sb::SkSVGGradient;
}

impl_default_make!(Linear, sb::C_SkSVGLinearGradient_Make);

impl DebugAttributes for Linear {
    const NAME: &'static str = "LinearGradient";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x1", &self.x1())
                .field("y1", &self.y1())
                .field("x2", &self.x2())
                .field("y2", &self.y2()),
        );
    }
}

impl Linear {
    skia_svg_macros::attrs! {
        SkSVGLinearGradient => {
            x1: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y1: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            x2: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y2: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
