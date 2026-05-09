use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

pub type ColorMatrixKind = sb::SkSVGFeColorMatrixType;
variant_name!(ColorMatrixKind::Matrix);

pub type ColorMatrix = RCHandle<sb::SkSVGFeColorMatrix>;

impl NodeSubtype for sb::SkSVGFeColorMatrix {
    type Base = sb::SkSVGFe;
}

impl_default_make!(ColorMatrix, sb::C_SkSVGFeColorMatrix_Make);

impl DebugAttributes for ColorMatrix {
    const NAME: &'static str = "FeColorMatrix";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("values", &self.values())
                .field("kind", self.kind()),
        );
    }
}

impl ColorMatrix {
    skia_svg_macros::attrs! {
        SkSVGFeColorMatrix => {
            "type" as kind: ColorMatrixKind [get(value) => value, set(value) => value]
        }
    }

    pub fn values(&self) -> &[scalar] {
        unsafe {
            safer::from_raw_parts(
                sb::C_SkSVGFeColorMatrix_getValues(self.native()),
                sb::C_SkSVGFeColorMatrix_getValuesCount(self.native()),
            )
        }
    }
}
