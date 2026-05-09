use skia_bindings as sb;

pub type PathFillType = sb::SkPathFillType;
variant_name!(PathFillType::InverseEvenOdd);

pub type PathDirection = sb::SkPathDirection;
variant_name!(PathDirection::CW);

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PathSegmentMask: u32 {
        const LINE = sb::SkPathSegmentMask_kLine_SkPathSegmentMask as _;
        const QUAD = sb::SkPathSegmentMask_kQuad_SkPathSegmentMask as _;
        const CONIC = sb::SkPathSegmentMask_kConic_SkPathSegmentMask as _;
        const CUBIC = sb::SkPathSegmentMask_kCubic_SkPathSegmentMask as _;
    }
}

pub type PathVerb = sb::SkPathVerb;
variant_name!(PathVerb::Conic);
