use std::{
    collections::HashSet,
    fmt,
    ops::{self, Index},
};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Features(HashSet<&'static str>);

impl fmt::Display for Features {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut strs: Vec<_> = self.0.iter().cloned().collect();
        strs.sort();
        strs.join(",").fmt(f)
    }
}

impl Features {
    /// Returns the set of features compatible with this platform and warns about features that were
    /// configured, but not supported on the target.
    pub fn from_cargo_env() -> Self {
        let mut features = Self::default();

        if cfg!(feature = "pdf") {
            features += feature::PDF;
        }

        if cfg!(feature = "gl") {
            features += feature::GL;
        }
        if cfg!(feature = "egl") {
            features += feature::EGL;
        }
        if cfg!(feature = "wayland") {
            features += feature::WAYLAND;
        }
        if cfg!(feature = "x11") {
            features += feature::X11;
        }

        if cfg!(feature = "vulkan") {
            features += feature::VULKAN;
        }
        if cfg!(feature = "metal") {
            features += feature::METAL;
        }
        if cfg!(feature = "d3d") {
            features += feature::D3D;
        }

        if cfg!(feature = "textlayout") {
            features += feature::TEXTLAYOUT;
        }
        if cfg!(feature = "svg") {
            features += feature::SVG;
        }
        if cfg!(feature = "skottie") {
            features += feature::SKOTTIE;
        }
        if cfg!(feature = "webp-encode") {
            features += feature::WEBP_ENCODE;
        }
        if cfg!(feature = "webp-decode") {
            features += feature::WEBP_DECODE;
        }

        if cfg!(feature = "embed-freetype") {
            features += feature::EMBED_FREETYPE;
        }
        if cfg!(feature = "freetype-woff2") {
            features += feature::FREETYPE_WOFF2;
        }

        if cfg!(feature = "jpeg-encode") {
            features += feature::JPEG_ENCODE;
        }
        if cfg!(feature = "jpeg-decode") {
            features += feature::JPEG_DECODE;
        }

        features
    }

    pub fn gpu(&self) -> bool {
        self[feature::GL] || self[feature::VULKAN] || self[feature::METAL] || self[feature::D3D]
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn enable(&mut self, feature: &'static str) {
        self.0.insert(feature);
    }

    fn disable(&mut self, feature: &'static str) {
        self.0.remove(feature);
    }

    pub fn set(&mut self, feature: &'static str, enabled: bool) {
        if enabled {
            self.enable(feature);
        } else {
            self.disable(feature);
        }
    }

    /// A comparable set of feature ids (sorted and joined by `-`).
    #[allow(unused)]
    pub fn to_key(&self) -> String {
        let mut features: Vec<_> = self
            .0
            .iter()
            .map(|&f| {
                feature::KEY_REPLACEMENTS
                    .iter()
                    .find_map(|&(orig, repl)| if orig == f { Some(repl) } else { None })
                    .unwrap_or(f)
            })
            .collect();
        features.sort();
        features.join("-")
    }

    /// Returns missing dependent features.
    ///
    /// If the returned set of features is not empty, this is most likely a configuration error.
    pub fn missing_dependencies(&self) -> Features {
        let mut missing = Features::default();

        for (feature, dependencies) in feature::DEPENDENCIES {
            if !self[feature] {
                continue;
            }

            for dependency in *dependencies {
                if !self[dependency] {
                    missing += dependency
                }
            }
        }

        missing
    }
}

impl<const N: usize> From<[&'static str; N]> for Features {
    fn from(features_array: [&'static str; N]) -> Self {
        Features(HashSet::from_iter(features_array))
    }
}

impl Index<&str> for Features {
    type Output = bool;

    fn index(&self, index: &str) -> &Self::Output {
        if self.0.contains(index) {
            &true
        } else {
            &false
        }
    }
}

impl ops::AddAssign<&'static str> for Features {
    fn add_assign(&mut self, rhs: &'static str) {
        self.enable(rhs);
    }
}

impl ops::SubAssign<&'static str> for Features {
    fn sub_assign(&mut self, rhs: &'static str) {
        self.disable(rhs);
    }
}

impl ops::SubAssign for Features {
    fn sub_assign(&mut self, rhs: Self) {
        self.0.retain(|item| !rhs.0.contains(item));
    }
}

pub mod feature {
    /// Support PDF rendering.
    pub const PDF: &str = "pdf";

    /// Build with OpenGL support
    pub const GL: &str = "gl";
    /// Build with EGL support. If you set X11, setting this to false will use LibGL (GLX)
    pub const EGL: &str = "egl";
    /// Build with X11 support
    pub const X11: &str = "x11";
    /// Build with Wayland support. This requires EGL, as GLX does not work on Wayland
    pub const WAYLAND: &str = "wayland";

    /// Build with Vulkan support
    pub const VULKAN: &str = "vulkan";
    /// Build with Metal support
    pub const METAL: &str = "metal";
    /// Build with Direct3D support
    pub const D3D: &str = "d3d";

    /// Features related to text layout. Modules skshaper and skparagraph
    pub const TEXTLAYOUT: &str = "textlayout";
    /// Support for rendering SVG
    pub const SVG: &str = "svg";
    /// Support for Lottie animations via Skottie
    pub const SKOTTIE: &str = "skottie";
    /// Support the encoding of bitmap data to the WEBP image format
    pub const WEBP_ENCODE: &str = "webp-encode";
    /// Support the decoding of the WEBP image format to bitmap data
    pub const WEBP_DECODE: &str = "webp-decode";

    /// Build with FreeType embedded
    pub const EMBED_FREETYPE: &str = "embed-freetype";
    /// Build with FreeType WOFF2 support
    pub const FREETYPE_WOFF2: &str = "freetype-woff2";

    pub const JPEG_ENCODE: &str = "jpeg-encode";
    pub const JPEG_DECODE: &str = "jpeg-decode";

    pub const FREETYPE_SPECIFIC: &[&str] = &[EMBED_FREETYPE, FREETYPE_WOFF2];

    pub const DEPENDENCIES: &[(&str, &[&str])] = &[
        (EGL, &[GL]),
        (X11, &[GL]),
        (WAYLAND, &[EGL]),
        (SKOTTIE, &[TEXTLAYOUT]),
        (PDF, &[JPEG_ENCODE, JPEG_DECODE]),
    ];

    pub const KEY_REPLACEMENTS: &[(&str, &str)] = &[
        (WEBP_ENCODE, "webpe"),
        (WEBP_DECODE, "webpd"),
        (EMBED_FREETYPE, "ftembed"),
        (FREETYPE_WOFF2, "ftwoff2"),
        (JPEG_ENCODE, "jpege"),
        (JPEG_DECODE, "jpegd"),
    ];
}
