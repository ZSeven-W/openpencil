use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length, NodeSubtype},
};
use skia_bindings as sb;

pub type Rect = RCHandle<sb::SkSVGRect>;

impl NodeSubtype for sb::SkSVGRect {
    type Base = sb::SkSVGShape;
}

impl_default_make!(Rect, sb::C_SkSVGRect_Make);

impl DebugAttributes for Rect {
    const NAME: &'static str = "Rect";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x", &self.x())
                .field("y", &self.y())
                .field("width", &self.width())
                .field("height", &self.height())
                .field("rx", &self.rx())
                .field("ry", &self.ry()),
        );
    }
}

impl Rect {
    skia_svg_macros::attrs! {
        SkSVGRect => {
            x: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            width: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            height: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            rx?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            ry?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()]
        }
    }
}
