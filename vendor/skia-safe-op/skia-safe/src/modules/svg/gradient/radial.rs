use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length, NodeSubtype},
};
use skia_bindings as sb;

pub type Radial = RCHandle<sb::SkSVGRadialGradient>;

impl NodeSubtype for sb::SkSVGRadialGradient {
    type Base = sb::SkSVGGradient;
}

impl_default_make!(Radial, sb::C_SkSVGRadialGradient_Make);

impl DebugAttributes for Radial {
    const NAME: &'static str = "RadialGradient";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("cx", &self.cx())
                .field("cy", &self.cy())
                .field("r", &self.r())
                .field("fx", &self.fx())
                .field("fy", &self.fy()),
        );
    }
}

impl Radial {
    skia_svg_macros::attrs! {
        SkSVGRadialGradient => {
            cx: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            cy: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            r: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            fx?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()],
            fy?: Length [get(value) => value.map(Length::from_native_ref), set(value) => value.into_native()]
        }
    }
}
