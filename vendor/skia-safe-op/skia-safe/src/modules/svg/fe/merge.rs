use super::{DebugAttributes, Input, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type MergeNode = RCHandle<sb::SkSVGFeMergeNode>;

impl NodeSubtype for sb::SkSVGFeMergeNode {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(MergeNode, sb::C_SkSVGFeMergeNode_Make);

impl DebugAttributes for MergeNode {
    const NAME: &'static str = "FeMergeNode";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder.field("input", self.input()));
    }
}

impl MergeNode {
    skia_svg_macros::attrs! {
        SkSVGFeMergeNode => {
            "in" as input: Input [get(value) => Input::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}

pub type Merge = RCHandle<sb::SkSVGFeMerge>;

impl NodeSubtype for sb::SkSVGFeMerge {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Merge, sb::C_SkSVGFeMerge_Make);

impl DebugAttributes for Merge {
    const NAME: &'static str = "FeMerge";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder);
    }
}
