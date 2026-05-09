use crate::{gpu, prelude::*};
use skia_bindings::{self as sb, GrGLFramebufferInfo, GrGLSurfaceInfo, GrGLTextureInfo};

pub use skia_bindings::GrGLFormat as Format;
variant_name!(Format::ALPHA8);
pub use skia_bindings::GrGLStandard as Standard;
variant_name!(Standard::GLES);
pub use skia_bindings::GrGLenum as Enum;
pub use skia_bindings::GrGLuint as UInt;

#[derive(Copy, Clone, Eq, Debug)]
#[repr(C)]
pub struct TextureInfo {
    pub target: Enum,
    pub id: Enum,
    pub format: Enum,
    pub protected: gpu::Protected,
}

native_transmutable!(GrGLTextureInfo, TextureInfo);

impl Default for TextureInfo {
    fn default() -> Self {
        Self {
            target: 0,
            id: 0,
            format: 0,
            protected: gpu::Protected::No,
        }
    }
}

impl PartialEq for TextureInfo {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_GrGLTextureInfo_Equals(self.native(), other.native()) }
    }
}

impl TextureInfo {
    pub fn from_target_and_id(target: Enum, id: Enum) -> Self {
        Self {
            target,
            id,
            ..Default::default()
        }
    }

    pub fn is_protected(&self) -> bool {
        self.protected == gpu::Protected::Yes
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct FramebufferInfo {
    pub fboid: UInt,
    pub format: Enum,
    pub protected: gpu::Protected,
}

native_transmutable!(GrGLFramebufferInfo, FramebufferInfo);

impl Default for FramebufferInfo {
    fn default() -> Self {
        Self {
            fboid: 0,
            format: 0,
            protected: gpu::Protected::No,
        }
    }
}

impl FramebufferInfo {
    pub fn from_fboid(fboid: UInt) -> Self {
        Self {
            fboid,
            ..Default::default()
        }
    }

    pub fn is_protected(&self) -> bool {
        self.protected == gpu::Protected::Yes
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct SurfaceInfo {
    pub sample_count: u32,
    pub level_count: u32,
    pub protected: gpu::Protected,

    pub target: Enum,
    pub format: Enum,
}

native_transmutable!(GrGLSurfaceInfo, SurfaceInfo);

impl Default for SurfaceInfo {
    fn default() -> Self {
        Self {
            sample_count: 1,
            level_count: 0,
            protected: gpu::Protected::No,
            target: 0,
            format: 0,
        }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BackendState: u32 {
        const RENDER_TARGET = sb::GrGLBackendState_kRenderTarget_GrGLBackendState as _;
        const TEXTURE_BINDING = sb::GrGLBackendState_kTextureBinding_GrGLBackendState as _;
        const VIEW = sb::GrGLBackendState_kView_GrGLBackendState as _;
        const BLEND = sb::GrGLBackendState_kBlend_GrGLBackendState as _;
        const MSAA_ENABLE = sb::GrGLBackendState_kMSAAEnable_GrGLBackendState as _;
        const VERTEX = sb::GrGLBackendState_kVertex_GrGLBackendState as _;
        const STENCIL = sb::GrGLBackendState_kStencil_GrGLBackendState as _;
        const PIXEL_STORE = sb::GrGLBackendState_kPixelStore_GrGLBackendState as _;
        const PROGRAM = sb::GrGLBackendState_kProgram_GrGLBackendState as _;
        const FIXED_FUNCTION = sb::GrGLBackendState_kFixedFunction_GrGLBackendState as _;
        const MISC = sb::GrGLBackendState_kMisc_GrGLBackendState as _;
    }
}

// TODO: BackendState::ALL

#[cfg(test)]
mod tests {
    use super::{Enum, Format};

    #[test]
    fn test_support_from_format_to_enum_and_back() {
        let e: Enum = Format::ALPHA8.into();
        let f: Format = e.into();
        assert_eq!(f, Format::ALPHA8);
    }

    #[test]
    fn test_all_formats_exhaustive() {
        use Format::*;
        let x = ALPHA8;
        // !!!!!
        // IF this match is not exhaustive anymore, the implementations of the format conversions
        // need to be updated in `skia-bindings/src/gl.cpp`, too.
        match x {
            Unknown => {}
            RGBA8 => {}
            R8 => {}
            ALPHA8 => {}
            LUMINANCE8 => {}
            LUMINANCE8_ALPHA8 => {}
            BGRA8 => {}
            RGB565 => {}
            RGBA16F => {}
            R16F => {}
            RGB8 => {}
            RGBX8 => {}
            RG8 => {}
            RGB10_A2 => {}
            RGBA4 => {}
            SRGB8_ALPHA8 => {}
            COMPRESSED_ETC1_RGB8 => {}
            COMPRESSED_RGB8_ETC2 => {}
            COMPRESSED_RGB8_BC1 => {}
            COMPRESSED_RGBA8_BC1 => {}
            R16 => {}
            RG16 => {}
            RGBA16 => {}
            RG16F => {}
            LUMINANCE16F => {}
            STENCIL_INDEX8 => {}
            STENCIL_INDEX16 => {}
            DEPTH24_STENCIL8 => {}
        }
    }

    #[test]
    fn test_format_last_color_and_last_exists() {
        let _ = Format::Last;
        let _ = Format::LastColorFormat;
    }
}
