use skia_bindings::{SkCubicResampler, SkSamplingOptions};

pub use skia_bindings::SkFilterMode as FilterMode;
variant_name!(FilterMode::Linear);

#[deprecated(since = "0.38.0", note = "Use FilterMode")]
pub type SamplingMode = FilterMode;

pub use skia_bindings::SkMipmapMode as MipmapMode;
variant_name!(MipmapMode::Nearest);

/// Specify `b` and `c` (each between 0...1) to create a shader that applies the corresponding
/// cubic reconstruction filter to the image.
///
/// Example values:
///     b = 1/3, c = 1/3        "Mitchell" filter
///     b = 0,   c = 1/2        "Catmull-Rom" filter
///
/// See "Reconstruction Filters in Computer Graphics"
///         Don P. Mitchell
///         Arun N. Netravali
///         1988
/// <https://www.cs.utexas.edu/~fussell/courses/cs384g-fall2013/lectures/mitchell/Mitchell.pdf>
/// Desmos worksheet <https://www.desmos.com/calculator/aghdpicrvr>
/// Nice overview <https://entropymine.com/imageworsener/bicubic/>
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CubicResampler {
    pub b: f32,
    pub c: f32,
}

impl CubicResampler {
    pub fn mitchell() -> Self {
        Self {
            b: 1.0 / 3.0,
            c: 1.0 / 3.0,
        }
    }

    pub fn catmull_rom() -> Self {
        Self {
            b: 0.0,
            c: 1.0 / 2.0,
        }
    }
}

native_transmutable!(SkCubicResampler, CubicResampler);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[deprecated(since = "0.38.0", note = "Use SamplingOptions")]
pub struct FilterOptions {
    pub sampling: FilterMode,
    pub mipmap: MipmapMode,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SamplingOptions {
    pub max_aniso: i32,
    pub use_cubic: bool,
    pub cubic: CubicResampler,
    pub filter: FilterMode,
    pub mipmap: MipmapMode,
}

native_transmutable!(SkSamplingOptions, SamplingOptions);

impl Default for SamplingOptions {
    fn default() -> Self {
        Self {
            max_aniso: 0,
            use_cubic: false,
            // ignored
            cubic: CubicResampler { b: 0.0, c: 0.0 },
            filter: FilterMode::Nearest,
            mipmap: MipmapMode::None,
        }
    }
}

impl SamplingOptions {
    pub fn new(filter_mode: FilterMode, mm: MipmapMode) -> Self {
        Self {
            filter: filter_mode,
            mipmap: mm,
            ..Default::default()
        }
    }
}

impl From<FilterMode> for SamplingOptions {
    fn from(fm: FilterMode) -> Self {
        Self {
            filter: fm,
            ..Default::default()
        }
    }
}

#[allow(deprecated)]
impl From<FilterOptions> for SamplingOptions {
    fn from(filter: FilterOptions) -> Self {
        Self {
            filter: filter.sampling,
            mipmap: filter.mipmap,
            ..Default::default()
        }
    }
}

impl From<CubicResampler> for SamplingOptions {
    #[allow(deprecated)]
    fn from(cubic: CubicResampler) -> Self {
        Self {
            use_cubic: true,
            cubic,
            ..Default::default()
        }
    }
}

impl SamplingOptions {
    pub fn from_aniso(max_aniso: i32) -> Self {
        Self {
            max_aniso: max_aniso.max(1),
            ..Default::default()
        }
    }

    pub fn is_aniso(&self) -> bool {
        self.max_aniso != 0
    }
}
