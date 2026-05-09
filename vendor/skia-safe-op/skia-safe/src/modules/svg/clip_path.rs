use super::{BoundingBoxUnits, DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type ClipPath = RCHandle<sb::SkSVGClipPath>;

impl NodeSubtype for sb::SkSVGClipPath {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(ClipPath, sb::C_SkSVGClipPath_Make);

impl DebugAttributes for ClipPath {
    const NAME: &'static str = "ClipPath";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()
            ._dbg(builder.field("clip_path_units", self.clip_path_units()));
    }
}

impl ClipPath {
    skia_svg_macros::attrs! {
        SkSVGClipPath => {
            clip_path_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }]
        }
    }
}
