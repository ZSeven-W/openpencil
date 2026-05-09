use super::NodeSubtype;
use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Length},
};
use skia_bindings as sb;

pub type Stop = RCHandle<sb::SkSVGStop>;

impl NodeSubtype for sb::SkSVGStop {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Stop, sb::C_SkSVGStop_Make);

impl DebugAttributes for Stop {
    const NAME: &'static str = "Stop";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder.field("offset", &self.offset()));
    }
}

impl Stop {
    skia_svg_macros::attrs! {
        SkSVGStop => {
            offset: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
