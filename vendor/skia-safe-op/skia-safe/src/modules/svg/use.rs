use super::{DebugAttributes, Iri, Length, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type Use = RCHandle<sb::SkSVGUse>;

impl NodeSubtype for sb::SkSVGUse {
    type Base = sb::SkSVGTransformableNode;
}

impl_default_make!(Use, sb::C_SkSVGUse_Make);

impl DebugAttributes for Use {
    const NAME: &'static str = "Use";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x", &self.x())
                .field("y", &self.y())
                .field("href", &self.href()),
        );
    }
}

impl Use {
    skia_svg_macros::attrs! {
        SkSVGUse => {
            x: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            href: Iri [get(value) => Iri::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
