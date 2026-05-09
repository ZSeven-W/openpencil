use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length, NodeSubtype},
};
use skia_bindings as sb;

pub type Line = RCHandle<sb::SkSVGLine>;

impl NodeSubtype for sb::SkSVGLine {
    type Base = sb::SkSVGShape;
}

impl_default_make!(Line, sb::C_SkSVGLine_Make);

impl DebugAttributes for Line {
    const NAME: &'static str = "Line";

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

impl Line {
    skia_svg_macros::attrs! {
        SkSVGLine => {
            x1: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y1: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            x2: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y2: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
