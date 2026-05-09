use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, NodeSubtype},
};
use skia_bindings as sb;

pub type Path = RCHandle<sb::SkSVGPath>;

impl NodeSubtype for sb::SkSVGPath {
    type Base = sb::SkSVGShape;
}

impl_default_make!(Path, sb::C_SkSVGPath_Make);

impl DebugAttributes for Path {
    const NAME: &'static str = "Path";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder.field("path", &self.path()));
    }
}

impl Path {
    skia_svg_macros::attrs! {
        SkSVGPath => {
            path: crate::Path [get(value) => crate::Path::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
