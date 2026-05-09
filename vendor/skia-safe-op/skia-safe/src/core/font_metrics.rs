use crate::scalar;
use skia_bindings::{self as sb, SkFontMetrics};

bitflags! {
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Flags: u32 {
        const UNDERLINE_THICKNESS_IS_VALID = sb::SkFontMetrics_FontMetricsFlags_kUnderlineThicknessIsValid_Flag as _;
        const UNDERLINE_POSITION_IS_VALID = sb::SkFontMetrics_FontMetricsFlags_kUnderlinePositionIsValid_Flag as _;
        const STRIKEOUT_THICKNESS_IS_VALID = sb::SkFontMetrics_FontMetricsFlags_kStrikeoutThicknessIsValid_Flag as _;
        const STRIKEOUT_POSITION_IS_VALID = sb::SkFontMetrics_FontMetricsFlags_kStrikeoutPositionIsValid_Flag as _;
        const BOUNDS_INVALID = sb::SkFontMetrics_FontMetricsFlags_kBoundsInvalid_Flag as _;
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct FontMetrics {
    flags: Flags,
    pub top: scalar,
    pub ascent: scalar,
    pub descent: scalar,
    pub bottom: scalar,
    pub leading: scalar,
    pub avg_char_width: scalar,
    pub max_char_width: scalar,
    pub x_min: scalar,
    pub x_max: scalar,
    pub x_height: scalar,
    pub cap_height: scalar,
    underline_thickness: scalar,
    underline_position: scalar,
    strikeout_thickness: scalar,
    strikeout_position: scalar,
}

native_transmutable!(SkFontMetrics, FontMetrics);

impl FontMetrics {
    pub fn underline_thickness(&self) -> Option<scalar> {
        self.if_valid(
            Flags::UNDERLINE_THICKNESS_IS_VALID,
            self.underline_thickness,
        )
    }

    pub fn underline_position(&self) -> Option<scalar> {
        self.if_valid(Flags::UNDERLINE_POSITION_IS_VALID, self.underline_position)
    }

    pub fn strikeout_thickness(&self) -> Option<scalar> {
        self.if_valid(
            Flags::STRIKEOUT_THICKNESS_IS_VALID,
            self.strikeout_thickness,
        )
    }

    pub fn strikeout_position(&self) -> Option<scalar> {
        self.if_valid(Flags::STRIKEOUT_POSITION_IS_VALID, self.strikeout_position)
    }

    fn if_valid(&self, flag: self::Flags, value: scalar) -> Option<scalar> {
        self.flags.contains(flag).then_some(value)
    }

    pub fn has_bounds(&self) -> bool {
        !self.flags.contains(Flags::BOUNDS_INVALID)
    }
}
