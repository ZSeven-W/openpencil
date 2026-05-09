use crate::prelude::*;
use skia_bindings::{self as sb, SkFontStyle, SkFontStyle_Weight, SkFontStyle_Width};
use std::{fmt, ops::Deref};

/// Wrapper type of a font weight.
///
/// Use Weight::from() to create a weight from an i32.
/// Use *weight to pull out the wrapped value of the Weight.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Weight(i32);

native_transmutable!(i32, Weight);

impl From<i32> for Weight {
    fn from(weight: i32) -> Self {
        Weight(weight)
    }
}

impl Deref for Weight {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(non_upper_case_globals)]
impl Weight {
    pub const INVISIBLE: Self = Self(SkFontStyle_Weight::kInvisible_Weight as _);
    pub const THIN: Self = Self(SkFontStyle_Weight::kThin_Weight as _);
    pub const EXTRA_LIGHT: Self = Self(SkFontStyle_Weight::kExtraLight_Weight as _);
    pub const LIGHT: Self = Self(SkFontStyle_Weight::kLight_Weight as _);
    pub const NORMAL: Self = Self(SkFontStyle_Weight::kNormal_Weight as _);
    pub const MEDIUM: Self = Self(SkFontStyle_Weight::kMedium_Weight as _);
    pub const SEMI_BOLD: Self = Self(SkFontStyle_Weight::kSemiBold_Weight as _);
    pub const BOLD: Self = Self(SkFontStyle_Weight::kBold_Weight as _);
    pub const EXTRA_BOLD: Self = Self(SkFontStyle_Weight::kExtraBold_Weight as _);
    pub const BLACK: Self = Self(SkFontStyle_Weight::kBlack_Weight as _);
    pub const EXTRA_BLACK: Self = Self(SkFontStyle_Weight::kExtraBlack_Weight as _);
}

/// Wrapper type for the width of a font.
///
/// To create a width of a font from an i32, use Width::from().
/// To access the underlying value of the font weight, dereference *weight.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Width(i32);

native_transmutable!(i32, Width);

impl From<i32> for Width {
    fn from(width: i32) -> Self {
        Width(width)
    }
}

impl Deref for Width {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(non_upper_case_globals)]
impl Width {
    pub const ULTRA_CONDENSED: Self = Self(SkFontStyle_Width::kUltraCondensed_Width as _);
    pub const EXTRA_CONDENSED: Self = Self(SkFontStyle_Width::kExtraCondensed_Width as _);
    pub const CONDENSED: Self = Self(SkFontStyle_Width::kCondensed_Width as _);
    pub const SEMI_CONDENSED: Self = Self(SkFontStyle_Width::kSemiCondensed_Width as _);
    pub const NORMAL: Self = Self(SkFontStyle_Width::kNormal_Width as _);
    pub const SEMI_EXPANDED: Self = Self(SkFontStyle_Width::kSemiExpanded_Width as _);
    pub const EXPANDED: Self = Self(SkFontStyle_Width::kExpanded_Width as _);
    pub const EXTRA_EXPANDED: Self = Self(SkFontStyle_Width::kExtraExpanded_Width as _);
    pub const ULTRA_EXPANDED: Self = Self(SkFontStyle_Width::kUltraExpanded_Width as _);
}

pub use skia_bindings::SkFontStyle_Slant as Slant;
variant_name!(Slant::Upright);

// TODO: implement Display
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct FontStyle(SkFontStyle);

native_transmutable!(SkFontStyle, FontStyle);

impl PartialEq for FontStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkFontStyle_Equals(self.native(), rhs.native()) }
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::construct(|fs| unsafe { sb::C_SkFontStyle_Construct(fs) })
    }
}

impl fmt::Debug for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontStyle")
            .field("weight", &self.weight())
            .field("width", &self.width())
            .field("slant", &self.slant())
            .finish()
    }
}

impl FontStyle {
    pub fn new(weight: Weight, width: Width, slant: Slant) -> Self {
        Self::construct(|fs| unsafe {
            sb::C_SkFontStyle_Construct2(fs, weight.into_native(), width.into_native(), slant)
        })
    }

    pub fn weight(self) -> Weight {
        Weight::from_native_c(unsafe { sb::C_SkFontStyle_weight(self.native()) })
    }

    pub fn width(self) -> Width {
        Width::from_native_c(unsafe { sb::C_SkFontStyle_width(self.native()) })
    }

    pub fn slant(self) -> Slant {
        unsafe { sb::C_SkFontStyle_slant(self.native()) }
    }

    pub fn normal() -> FontStyle {
        *font_style_static::NORMAL
    }

    pub fn bold() -> FontStyle {
        *font_style_static::BOLD
    }

    pub fn italic() -> FontStyle {
        *font_style_static::ITALIC
    }

    pub fn bold_italic() -> FontStyle {
        *font_style_static::BOLD_ITALIC
    }
}

mod font_style_static {
    use std::sync::LazyLock;

    use super::{FontStyle, Slant, Weight, Width};

    pub static NORMAL: LazyLock<FontStyle> =
        LazyLock::new(|| FontStyle::new(Weight::NORMAL, Width::NORMAL, Slant::Upright));
    pub static BOLD: LazyLock<FontStyle> =
        LazyLock::new(|| FontStyle::new(Weight::BOLD, Width::NORMAL, Slant::Upright));
    pub static ITALIC: LazyLock<FontStyle> =
        LazyLock::new(|| FontStyle::new(Weight::NORMAL, Width::NORMAL, Slant::Italic));
    pub static BOLD_ITALIC: LazyLock<FontStyle> =
        LazyLock::new(|| FontStyle::new(Weight::BOLD, Width::NORMAL, Slant::Italic));
}

#[test]
fn test_equality() {
    let style: FontStyle = Default::default();
    let style2: FontStyle = Default::default();
    assert!(style == style2);
}
