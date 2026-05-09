use std::ffi::CString;

use crate::{interop, prelude::*, Path};

use skia_bindings as sb;

pub fn from_svg(svg: impl AsRef<str>) -> Option<Path> {
    let str = CString::new(svg.as_ref()).unwrap();
    Path::try_construct(|p| unsafe { sb::C_SkParsePath_FromSVGString(str.as_ptr(), p) })
}

pub use skia_bindings::SkParsePath_PathEncoding as PathEncoding;
variant_name!(PathEncoding::Absolute);

impl Path {
    pub fn from_svg(svg: impl AsRef<str>) -> Option<Path> {
        from_svg(svg)
    }

    pub fn to_svg(&self) -> String {
        to_svg(self)
    }

    pub fn to_svg_with_encoding(&self, encoding: PathEncoding) -> String {
        to_svg_with_encoding(self, encoding)
    }
}

pub fn to_svg(path: &Path) -> String {
    to_svg_with_encoding(path, PathEncoding::Absolute)
}

pub fn to_svg_with_encoding(path: &Path, encoding: PathEncoding) -> String {
    interop::String::construct(|svg| {
        unsafe { sb::C_SkParsePath_ToSVGString(path.native(), svg, encoding) };
    })
    .as_str()
    .into()
}

#[cfg(test)]
mod tests {
    use crate::PathBuilder;

    use super::Path;

    #[test]
    fn simple_path_to_svg_and_back() {
        let mut path = PathBuilder::default();
        path.move_to((0, 0));
        path.line_to((100, 100));
        path.line_to((0, 100));
        path.close();

        let svg = Path::to_svg(&path.detach());
        assert_eq!(svg, "M0 0L100 100L0 100L0 0Z");
        // And back. Someone should find out why they are not equal.
        let _recreated = Path::from_svg(svg).expect("Failed to parse SVG path");
    }
}
