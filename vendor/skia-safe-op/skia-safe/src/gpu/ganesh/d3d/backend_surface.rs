pub mod backend_formats {
    use skia_bindings as sb;

    use crate::{
        gpu::{d3d::DXGI_FORMAT, BackendFormat},
        prelude::*,
    };

    pub fn make_d3d(format: DXGI_FORMAT) -> BackendFormat {
        BackendFormat::construct(|bf| unsafe {
            sb::C_GrBackendFormat_ConstructD3D(bf, format.into_native())
        })
        .assert_valid()
    }

    pub fn as_dxgi_format(backend_format: &BackendFormat) -> Option<DXGI_FORMAT> {
        let mut format = sb::DXGI_FORMAT::DXGI_FORMAT_UNKNOWN;
        unsafe { sb::C_GrBackendFormats_AsDxgiFormat(backend_format.native(), &mut format) }
            .then_some(DXGI_FORMAT::from_native_c(format))
    }
}

pub mod backend_textures {
    use skia_bindings as sb;

    use crate::{
        gpu::{d3d, BackendTexture},
        prelude::*,
    };

    pub fn make_d3d(
        (width, height): (i32, i32),
        d3d_info: &d3d::TextureResourceInfo,
        label: impl AsRef<str>,
    ) -> BackendTexture {
        let label = label.as_ref().as_bytes();
        unsafe {
            BackendTexture::from_native_if_valid(sb::C_GrBackendTexture_newD3D(
                width,
                height,
                d3d_info.native(),
                label.as_ptr() as _,
                label.len(),
            ))
        }
        .unwrap()
    }

    /// Return a snapshot of the [`d3d::TextureResourceInfo`] struct.
    /// This snapshot will set `resource_state` to the current resource state.
    pub fn get_d3d_texture_resource_info(
        texture: &BackendTexture,
    ) -> Option<d3d::TextureResourceInfo> {
        unsafe {
            let mut info = sb::GrD3DTextureResourceInfo::default();
            sb::C_GrBackendTextures_GetD3DTextureResourceInfo(texture.native(), &mut info).then(
                || {
                    assert!(!info.fResource.fObject.is_null());
                    d3d::TextureResourceInfo::from_native_c(info)
                },
            )
        }
    }

    /// Anytime the client changes the `D3D12_RESOURCE_STATES` of the `ID3D12_RESOURCE`
    /// captured by this [`BackendTexture`], they must call this function to notify
    /// Skia of the changed layout.
    pub fn set_d3d_resource_state(
        texture: &mut BackendTexture,
        resource_state: d3d::ResourceStateEnum,
    ) -> &mut BackendTexture {
        unsafe { sb::C_GrBackendTextures_SetD3DResourceState(texture.native_mut(), resource_state) }
        texture
    }
}

pub mod backend_render_targets {
    use skia_bindings as sb;

    use crate::{
        gpu::{d3d, BackendRenderTarget},
        prelude::*,
    };

    pub fn make_d3d(
        (width, height): (i32, i32),
        d3d_info: &d3d::TextureResourceInfo,
    ) -> BackendRenderTarget {
        BackendRenderTarget::construct(|brt| unsafe {
            sb::C_GrBackendRenderTarget_ConstructD3D(brt, width, height, d3d_info.native())
        })
    }

    /// Return a snapshot of the [`d3d::TextureResourceInfo`] struct.
    /// This snapshot will set `resource_state` to the current resource state.
    pub fn get_d3d_texture_resource_info(
        target: &BackendRenderTarget,
    ) -> Option<d3d::TextureResourceInfo> {
        let mut info = sb::GrD3DTextureResourceInfo::default();
        unsafe {
            sb::C_GrBackendRenderTargets_GetD3DTextureResourceInfo(target.native(), &mut info)
        }
        .then(|| {
            assert!(!info.fResource.fObject.is_null());
            d3d::TextureResourceInfo::from_native_c(info)
        })
    }

    /// Anytime the client changes the `D3D12_RESOURCE_STATES` of the `ID3D12_RESOURCE`
    /// captured by this [`BackendRenderTarget`], they must call this function to notify
    /// Skia of the changed layout.
    pub fn set_d3d_resource_state(
        target: &mut BackendRenderTarget,
        resource_state: d3d::ResourceStateEnum,
    ) -> &mut BackendRenderTarget {
        unsafe {
            sb::C_GrBackendRenderTargets_SetD3DResourceState(target.native_mut(), resource_state)
        }
        target
    }
}
