use std::fmt;

use skia_bindings::{self as sb, SkPixelGeometry, SkSurfaceProps};

use crate::{prelude::*, scalar};

// TODO: use the enum rewriter and strip underscores?
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(i32)]
pub enum PixelGeometry {
    #[default]
    Unknown = SkPixelGeometry::kUnknown_SkPixelGeometry as _,
    RGBH = SkPixelGeometry::kRGB_H_SkPixelGeometry as _,
    BGRH = SkPixelGeometry::kBGR_H_SkPixelGeometry as _,
    RGBV = SkPixelGeometry::kRGB_V_SkPixelGeometry as _,
    BGRV = SkPixelGeometry::kBGR_V_SkPixelGeometry as _,
}

native_transmutable!(SkPixelGeometry, PixelGeometry);

impl PixelGeometry {
    pub fn is_rgb(self) -> bool {
        self == PixelGeometry::RGBH || self == PixelGeometry::RGBV
    }

    pub fn is_bgr(self) -> bool {
        self == PixelGeometry::BGRH || self == PixelGeometry::BGRV
    }

    pub fn is_h(self) -> bool {
        self == PixelGeometry::RGBH || self == PixelGeometry::BGRH
    }

    pub fn is_v(self) -> bool {
        self == PixelGeometry::RGBV || self == PixelGeometry::BGRV
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SurfacePropsFlags: u32 {
        #[allow(clippy::unnecessary_cast)]
        const DEFAULT = sb::SkSurfaceProps_Flags_kDefault_Flag as u32;
        #[allow(clippy::unnecessary_cast)]
        const USE_DEVICE_INDEPENDENT_FONTS =
            sb::SkSurfaceProps_Flags_kUseDeviceIndependentFonts_Flag as u32;
        #[allow(clippy::unnecessary_cast)]
        const DYNAMIC_MSAA =
            sb::SkSurfaceProps_Flags_kDynamicMSAA_Flag as u32;
        #[allow(clippy::unnecessary_cast)]
        const ALWAYS_DITHER =
            sb::SkSurfaceProps_Flags_kAlwaysDither_Flag as u32;
        #[allow(clippy::unnecessary_cast)]
        const PRESERVES_TRANSPARENT_DRAWS =
            sb::SkSurfaceProps_Flags_kPreservesTransparentDraws_Flag as u32;
    }
}

impl Default for SurfacePropsFlags {
    fn default() -> Self {
        SurfacePropsFlags::empty()
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct SurfaceProps(SkSurfaceProps);

native_transmutable!(SkSurfaceProps, SurfaceProps);

impl PartialEq for SurfaceProps {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_SkSurfaceProps_Equals(self.native(), other.native()) }
    }
}

impl Eq for SurfaceProps {}

impl Default for SurfaceProps {
    fn default() -> Self {
        SurfaceProps::new(Default::default(), Default::default())
    }
}

impl fmt::Debug for SurfaceProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurfaceProps")
            .field("flags", &self.flags())
            .field("pixel_geometry", &self.pixel_geometry())
            .field("text_contrast", &self.text_contrast())
            .field("text_gamma", &self.text_gamma())
            .finish()
    }
}

impl SurfaceProps {
    pub fn new(flags: SurfacePropsFlags, pixel_geometry: PixelGeometry) -> SurfaceProps {
        Self::from_native_c(unsafe {
            SkSurfaceProps::new1(flags.bits(), pixel_geometry.into_native())
        })
    }

    pub fn new_with_text_properties(
        flags: SurfacePropsFlags,
        pixel_geometry: PixelGeometry,
        text_contrast: scalar,
        text_gamma: scalar,
    ) -> SurfaceProps {
        Self::from_native_c(unsafe {
            SkSurfaceProps::new2(
                flags.bits(),
                pixel_geometry.into_native(),
                text_contrast,
                text_gamma,
            )
        })
    }

    pub fn flags(self) -> SurfacePropsFlags {
        SurfacePropsFlags::from_bits_truncate(self.native().fFlags)
    }

    #[must_use]
    pub fn clone_with_pixel_geometry(&self, new_pixel_geometry: PixelGeometry) -> Self {
        Self::new_with_text_properties(
            self.flags(),
            new_pixel_geometry,
            self.text_contrast(),
            self.text_gamma(),
        )
    }

    pub const MAX_CONTRAST_INCLUSIVE: scalar = 1.;
    pub const MIN_CONTRAST_INCLUSIVE: scalar = 0.;
    pub const MAX_GAMMA_EXCLUSIVE: scalar = 4.;
    pub const MIN_GAMMA_INCLUSIVE: scalar = 0.;

    pub fn pixel_geometry(self) -> PixelGeometry {
        PixelGeometry::from_native_c(self.native().fPixelGeometry)
    }

    pub fn text_contrast(self) -> scalar {
        self.native().fTextContrast
    }

    pub fn text_gamma(self) -> scalar {
        self.native().fTextGamma
    }

    pub fn is_use_device_independent_fonts(self) -> bool {
        self.flags()
            .contains(SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS)
    }

    pub fn is_always_dither(self) -> bool {
        self.flags().contains(SurfacePropsFlags::ALWAYS_DITHER)
    }

    pub fn preserves_transparent_draws(self) -> bool {
        self.flags()
            .contains(SurfacePropsFlags::PRESERVES_TRANSPARENT_DRAWS)
    }
}

#[test]
fn create() {
    let props = SurfaceProps::new(
        SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS,
        PixelGeometry::RGBH,
    );
    assert_eq!(
        SurfacePropsFlags::USE_DEVICE_INDEPENDENT_FONTS,
        props.flags()
    );
    assert_eq!(PixelGeometry::RGBH, props.pixel_geometry());
    assert!(props.is_use_device_independent_fonts());
}
