pub mod backend_formats {
    use skia_bindings as sb;

    use crate::{
        gpu::{mtl, BackendFormat},
        prelude::*,
    };

    pub fn make_mtl(format: mtl::PixelFormat) -> BackendFormat {
        BackendFormat::construct(|bf| unsafe { sb::C_GrBackendFormats_ConstructMtl(bf, format) })
            .assert_valid()
    }

    pub fn as_mtl_format(backend_format: &BackendFormat) -> Option<mtl::PixelFormat> {
        let pixel_format = unsafe { sb::C_GrBackendFormats_AsMtlFormat(backend_format.native()) };
        // Mtl's PixelFormat == 0 is invalid.
        (pixel_format != 0).then_some(pixel_format)
    }
}

pub mod backend_textures {
    use skia_bindings as sb;

    use crate::{
        gpu::{self, mtl, BackendTexture},
        prelude::*,
    };

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn make_mtl(
        (width, height): (i32, i32),
        mipmapped: gpu::Mipmapped,
        mtl_info: &mtl::TextureInfo,
        label: impl AsRef<str>,
    ) -> BackendTexture {
        let label = label.as_ref().as_bytes();
        BackendTexture::from_native_if_valid(sb::C_GrBackendTextures_newMtl(
            width,
            height,
            mipmapped,
            mtl_info.native(),
            label.as_ptr() as _,
            label.len(),
        ))
        .unwrap()
    }

    pub fn get_mtl_texture_info(texture: &BackendTexture) -> Option<mtl::TextureInfo> {
        unsafe {
            let mut texture_info = mtl::TextureInfo::default();
            sb::C_GrBackendTextures_GetMtlTextureInfo(texture.native(), texture_info.native_mut())
                .then_some(texture_info)
        }
    }
}

pub mod backend_render_targets {
    use skia_bindings as sb;

    use crate::{
        gpu::{mtl, BackendRenderTarget},
        prelude::*,
    };

    pub fn make_mtl(
        (width, height): (i32, i32),
        mtl_info: &mtl::TextureInfo,
    ) -> BackendRenderTarget {
        BackendRenderTarget::construct(|target| unsafe {
            sb::C_GrBackendRenderTargets_ConstructMtl(target, width, height, mtl_info.native())
        })
    }

    pub fn get_mtl_texture_info(render_target: &BackendRenderTarget) -> Option<mtl::TextureInfo> {
        let mut info = mtl::TextureInfo::default();
        unsafe {
            sb::C_GrBackendRenderTargets_GetMtlTextureInfo(
                render_target.native(),
                info.native_mut(),
            )
        }
        .then_some(info)
    }
}
