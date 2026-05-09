use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

pub type Offset = RCHandle<sb::SkSVGFeOffset>;

impl NodeSubtype for sb::SkSVGFeOffset {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Offset, sb::C_SkSVGFeOffset_Make);

impl DebugAttributes for Offset {
    const NAME: &'static str = "FeOffset";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()
            ._dbg(builder.field("dx", &self.dx()).field("dy", &self.dy()));
    }
}

impl Offset {
    skia_svg_macros::attrs! {
        SkSVGFeOffset => {
            *dx: scalar [get(value) => value, set(value) => value],
            *dy: scalar [get(value) => value, set(value) => value]
        }
    }
}
