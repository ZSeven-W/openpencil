use std::fmt;

use super::Length;
use crate::{interop, prelude::*};
use skia_bindings as sb;

pub type FontStyle = sb::SkSVGFontStyle_Type;
variant_name!(FontStyle::Normal);
pub type FontWeight = sb::SkSVGFontWeight_Type;
variant_name!(FontWeight::Lighter);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FontSize {
    ty: sb::SkSVGFontSize_Type,
    size: sb::SkSVGLength,
}

native_transmutable!(sb::SkSVGFontSize, FontSize);

impl fmt::Debug for FontSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgFontSize")
            .field("size", &self.size())
            .finish()
    }
}

impl FontSize {
    pub fn size(&self) -> Option<&Length> {
        if self.ty == sb::SkSVGFontSize_Type::Length {
            Some(Length::from_native_ref(&self.size))
        } else {
            None
        }
    }

    pub fn inherit() -> Self {
        Self {
            ty: sb::SkSVGFontSize_Type::Inherit,
            size: sb::SkSVGLength {
                fValue: 0.0,
                fUnit: sb::SkSVGLength_Unit::Unknown,
            },
        }
    }

    pub fn new(size: Length) -> Self {
        Self {
            ty: sb::SkSVGFontSize_Type::Length,
            size: size.into_native(),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct FontFamily {
    ty: sb::SkSVGFontFamily_Type,
    family: interop::String,
}

impl fmt::Debug for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgFontFamily")
            .field("family", &self.family())
            .finish()
    }
}

impl FontFamily {
    pub fn family(&self) -> Option<&str> {
        if self.ty == sb::SkSVGFontFamily_Type::Family {
            Some(self.family.as_str())
        } else {
            None
        }
    }

    pub fn inherit() -> Self {
        Self {
            ty: sb::SkSVGFontFamily_Type::Inherit,
            family: interop::String::default(),
        }
    }

    pub fn new<T: AsRef<str>>(family: T) -> Self {
        Self {
            ty: sb::SkSVGFontFamily_Type::Family,
            family: interop::String::from_str(family.as_ref()),
        }
    }
}

native_transmutable!(sb::SkSVGFontFamily, FontFamily);
