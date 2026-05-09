use crate::{
    impl_default_make,
    prelude::*,
    svg::{DebugAttributes, Iri, NodeSubtype, PreserveAspectRatio},
};
use skia_bindings as sb;

pub type Image = RCHandle<sb::SkSVGFeImage>;

impl NodeSubtype for sb::SkSVGFeImage {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Image, sb::C_SkSVGFeImage_Make);

impl DebugAttributes for Image {
    const NAME: &'static str = "FeImage";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("href", &self.href())
                .field("preserve_aspect_ratio", self.preserve_aspect_ratio()),
        );
    }
}

impl Image {
    skia_svg_macros::attrs! {
        SkSVGFeImage => {
            href: Iri [get(value) => Iri::from_native_ref(value), set(value) => value.into_native()],
            preserve_aspect_ratio: PreserveAspectRatio [get(value) => PreserveAspectRatio::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
