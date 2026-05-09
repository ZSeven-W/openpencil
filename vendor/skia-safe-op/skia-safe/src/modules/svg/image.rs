use super::{DebugAttributes, Iri, Length, NodeSubtype, PreserveAspectRatio};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type Image = RCHandle<sb::SkSVGImage>;

impl NodeSubtype for sb::SkSVGImage {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Image, sb::C_SkSVGImage_Make);

impl DebugAttributes for Image {
    const NAME: &'static str = "Image";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x", &self.x())
                .field("y", &self.y())
                .field("width", &self.width())
                .field("height", &self.height())
                .field("href", &self.href())
                .field("preserve_aspect_ratio", self.preserve_aspect_ratio()),
        );
    }
}

impl Image {
    // TODO: wrap LoadImage

    skia_svg_macros::attrs! {
        SkSVGImage => {
            x: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            width: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            height: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            href: Iri [get(value) => Iri::from_native_ref(value), set(value) => value.into_native()],
            preserve_aspect_ratio: PreserveAspectRatio [get(value) => PreserveAspectRatio::from_native_ref(value), set(value) => value.into_native()]
        }
    }
}
