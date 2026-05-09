use super::{BoundingBoxUnits, DebugAttributes, Length, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type Mask = RCHandle<sb::SkSVGMask>;

impl NodeSubtype for sb::SkSVGMask {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Mask, sb::C_SkSVGMask_Make);

impl DebugAttributes for Mask {
    const NAME: &'static str = "Mask";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x", &self.x())
                .field("y", &self.y())
                .field("width", &self.width())
                .field("height", &self.height())
                .field("mask_units", self.mask_units())
                .field("mask_content_units", self.mask_content_units()),
        );
    }
}

impl Mask {
    skia_svg_macros::attrs! {
        SkSVGMask => {
            x: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            width: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            height: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            mask_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }],
            mask_content_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }]
        }
    }
}
