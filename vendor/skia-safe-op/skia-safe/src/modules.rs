#[cfg(feature = "textlayout")]
pub(crate) mod paragraph;
#[cfg(any(feature = "svg", feature = "skottie"))]
pub mod resources;
#[cfg(feature = "textlayout")]
pub mod shaper;
#[cfg(feature = "skottie")]
pub mod skottie;
#[cfg(feature = "svg")]
pub mod svg;
#[cfg(feature = "textlayout")]
pub use shaper::{icu, Shaper};

// Export everything below paragraph under textlayout
#[cfg(feature = "textlayout")]
pub mod textlayout {
    pub use super::paragraph::*;
}

#[cfg(feature = "textlayout")]
pub mod shapers {
    // Re-exports `shapers::primitive`.
    pub use crate::shaper::shapers::*;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub mod ct {
        pub use crate::shaper::core_text::*;
    }

    pub mod hb {
        pub use crate::shaper::harfbuzz::*;
    }

    pub mod unicode {
        pub use crate::shaper::unicode::*;
    }
}
