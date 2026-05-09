use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type Defs = RCHandle<sb::SkSVGDefs>;

impl NodeSubtype for sb::SkSVGDefs {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Defs, sb::C_SkSVGDefs_Make);

impl DebugAttributes for Defs {
    const NAME: &'static str = "Defs";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder);
    }
}
