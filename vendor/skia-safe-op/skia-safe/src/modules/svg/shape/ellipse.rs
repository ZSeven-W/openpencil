use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length, NodeSubtype},
};
use skia_bindings as sb;

pub type Ellipse = RCHandle<sb::SkSVGEllipse>;

impl NodeSubtype for sb::SkSVGEllipse {
    type Base = sb::SkSVGShape;
}

impl_default_make!(Ellipse, sb::C_SkSVGEllipse_Make);

impl DebugAttributes for Ellipse {
    const NAME: &'static str = "Ellipse";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("cx", &self.cx())
                .field("cy", &self.cy())
                .field("rx", &self.rx())
                .field("ry", &self.ry()),
        );
    }
}

impl Ellipse {
    skia_svg_macros::attrs! {
        SkSVGEllipse => {
            cx: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            cy: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            rx?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            ry?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()]
        }
    }
}
