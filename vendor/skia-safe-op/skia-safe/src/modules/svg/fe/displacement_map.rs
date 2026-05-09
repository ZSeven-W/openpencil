use super::{DebugAttributes, Input, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

pub type ChannelSelector = sb::SkSVGFeDisplacementMap_ChannelSelector;
variant_name!(ChannelSelector::R);

pub type DisplacementMap = RCHandle<sb::SkSVGFeDisplacementMap>;

impl NodeSubtype for sb::SkSVGFeDisplacementMap {
    type Base = sb::SkSVGFe;
}

impl_default_make!(DisplacementMap, sb::C_SkSVGFeDisplacementMap_Make);

// TODO: Wrap resolveColorSpace

impl DebugAttributes for DisplacementMap {
    const NAME: &'static str = "FeDisplacementMap";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("input2", self.input2())
                .field("x_channel_selector", self.x_channel_selector())
                .field("y_channel_selector", self.y_channel_selector())
                .field("scale", self.scale()),
        );
    }
}

impl DisplacementMap {
    skia_svg_macros::attrs! {
        SkSVGFeDisplacementMap => {
            "in2" as input2: Input [get(value) => Input::from_native_ref(value), set(value) => value.into_native()],
            x_channel_selector: ChannelSelector [get(value) => value, set(value) => value],
            y_channel_selector: ChannelSelector [get(value) => value, set(value) => value],
            scale: scalar [get(value) => value, set(value) => value]
        }
    }
}
