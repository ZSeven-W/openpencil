pub mod backend_formats {
    use skia_bindings as sb;

    use crate::{
        gpu::{gl, BackendFormat},
        prelude::*,
    };

    #[deprecated(
        since = "0.92.0",
        note = "Prefer make_gl_format(format) for GL_TEXTURE_2D targets and make_gn_external() for GL_TEXTURE_EXTERNAL targets."
    )]
    pub fn make_gl(format: gl::Enum, target: gl::Enum) -> BackendFormat {
        BackendFormat::construct(|bf| unsafe {
            sb::C_GrBackendFormats_ConstructGL(bf, format, target)
        })
        .assert_valid()
    }

    pub fn make_gl_format(format: gl::Enum) -> BackendFormat {
        BackendFormat::construct(|bf| unsafe {
            sb::C_GrBackendFormats_ConstructGLFormat(bf, format)
        })
        .assert_valid()
    }

    pub fn make_gl_external() -> BackendFormat {
        BackendFormat::construct(|bf| unsafe { sb::C_GrBackendFormats_ConstructGLExternal(bf) })
            .assert_valid()
    }

    pub fn as_gl_format(format: &BackendFormat) -> gl::Format {
        unsafe { sb::C_GrBackendFormats_AsGLFormat(format.native()) }
    }

    pub fn as_gl_format_enum(format: &BackendFormat) -> gl::Enum {
        unsafe { sb::C_GrBackendFormats_AsGLFormatEnum(format.native()) }
    }
}

pub mod backend_textures {
    use skia_bindings as sb;

    use crate::{
        gpu::{gl, BackendTexture, Mipmapped},
        prelude::*,
    };

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn make_gl(
        (width, height): (i32, i32),
        mipmapped: Mipmapped,
        gl_info: gl::TextureInfo,
        label: impl AsRef<str>,
    ) -> BackendTexture {
        let str = label.as_ref().as_bytes();
        BackendTexture::from_ptr(sb::C_GrBackendTextures_newGL(
            width,
            height,
            mipmapped,
            gl_info.native(),
            str.as_ptr() as _,
            str.len(),
        ))
        .unwrap()
    }

    pub fn get_gl_texture_info(texture: &BackendTexture) -> Option<gl::TextureInfo> {
        let mut texture_info = gl::TextureInfo::default();
        unsafe {
            sb::C_GrBackendTextures_GetGLTextureInfo(texture.native(), texture_info.native_mut())
        }
        .then_some(texture_info)
    }

    pub fn gl_texture_parameters_modified(texture: &mut BackendTexture) {
        unsafe { sb::C_GrBackendTextures_GLTextureParametersModified(texture.native_mut()) }
    }
}

pub mod backend_render_targets {
    use skia_bindings as sb;

    use crate::{
        gpu::{gl, BackendRenderTarget},
        prelude::*,
    };

    pub fn make_gl(
        (width, height): (i32, i32),
        sample_count: impl Into<Option<usize>>,
        stencil_bits: usize,
        info: gl::FramebufferInfo,
    ) -> BackendRenderTarget {
        BackendRenderTarget::construct(|target| unsafe {
            sb::C_GrBackendRenderTargets_ConstructGL(
                target,
                width,
                height,
                sample_count.into().unwrap_or(0).try_into().unwrap(),
                stencil_bits.try_into().unwrap(),
                info.native(),
            )
        })
    }

    pub fn get_gl_framebuffer_info(
        render_target: &BackendRenderTarget,
    ) -> Option<gl::FramebufferInfo> {
        let mut info = gl::FramebufferInfo::default();
        unsafe {
            sb::C_GrBackendRenderTargets_GetGLFramebufferInfo(
                render_target.native(),
                info.native_mut(),
            )
        }
        .then_some(info)
    }
}
