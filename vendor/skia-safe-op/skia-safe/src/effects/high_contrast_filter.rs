use crate::{high_contrast_config::InvertStyle, prelude::*, scalar, ColorFilter};
use skia_bindings::{self as sb, SkHighContrastConfig};

pub mod high_contrast_config {
    pub use skia_bindings::SkHighContrastConfig_InvertStyle as InvertStyle;
    #[test]
    fn invert_style_naming() {
        let _ = InvertStyle::InvertLightness;
    }
}

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct HighContrastConfig {
    pub grayscale: bool,
    pub invert_style: InvertStyle,
    pub contrast: scalar,
}

native_transmutable!(SkHighContrastConfig, HighContrastConfig);

impl Default for HighContrastConfig {
    fn default() -> Self {
        Self {
            grayscale: false,
            invert_style: InvertStyle::NoInvert,
            contrast: 0.0,
        }
    }
}

impl HighContrastConfig {
    pub fn new(grayscale: bool, invert_style: InvertStyle, contrast: scalar) -> Self {
        Self {
            grayscale,
            invert_style,
            contrast,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.contrast >= -1.0 && self.contrast <= 1.0
    }
}

impl ColorFilter {
    pub fn high_contrast(config: &HighContrastConfig) -> Option<Self> {
        new(config)
    }
}

pub fn new(config: &HighContrastConfig) -> Option<ColorFilter> {
    ColorFilter::from_ptr(unsafe { sb::C_SkHighContrastFilter_Make(config.native()) })
}
