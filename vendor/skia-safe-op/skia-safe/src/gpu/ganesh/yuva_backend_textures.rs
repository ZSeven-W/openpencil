use crate::gpu::{BackendFormat, BackendTexture, Mipmapped, SurfaceOrigin};
use crate::{prelude::*, YUVAInfo, YUVColorSpace};
use skia_bindings::{self as sb, GrYUVABackendTextureInfo, GrYUVABackendTextures};
use std::{fmt, iter};

/// A description of a set [BackendTexture]s that hold the planar data described by a [YUVAInfo].
pub type YUVABackendTextureInfo = Handle<GrYUVABackendTextureInfo>;
unsafe_send_sync!(YUVABackendTextureInfo);

impl NativeDrop for GrYUVABackendTextureInfo {
    fn drop(&mut self) {
        unsafe { sb::C_GrYUVABackendTextureInfo_destruct(self) }
    }
}

impl NativeClone for GrYUVABackendTextureInfo {
    fn clone(&self) -> Self {
        construct(|cloned| unsafe { sb::C_GrYUVABackendTextureInfo_CopyConstruct(cloned, self) })
    }
}

impl NativePartialEq for GrYUVABackendTextureInfo {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_GrYUVABackendTextureInfo_equals(self, rhs) }
    }
}

impl fmt::Debug for YUVABackendTextureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YUVABackendTextureInfo")
            .field("yuva_info", &self.yuva_info())
            .field("yuv_color_space", &self.yuv_color_space())
            .field("mipmapped", &self.mipmapped())
            .field("texture_origin", &self.texture_origin())
            .field("plane_formats", &self.plane_formats())
            .finish()
    }
}

impl YUVABackendTextureInfo {
    pub const MAX_PLANES: usize = YUVAInfo::MAX_PLANES;

    /// Initializes a [YUVABackendTextureInfo] to describe a set of textures that can store the
    /// planes indicated by the [YUVAInfo]. The texture dimensions are taken from the [YUVAInfo]'s
    /// plane dimensions. All the described textures share a common origin. The planar image this
    /// describes will be mip mapped if all the textures are individually mip mapped as indicated
    /// by [Mipmapped]. This will return [None] if the passed formats' channels don't agree with [YUVAInfo].
    pub fn new(
        info: &YUVAInfo,
        formats: &[BackendFormat],
        mip_mapped: Mipmapped,
        origin: SurfaceOrigin,
    ) -> Option<Self> {
        if formats.len() != info.num_planes() {
            return None;
        }

        let mut formats = formats.to_vec();
        formats.extend(
            iter::repeat_with(BackendFormat::new_invalid).take(Self::MAX_PLANES - formats.len()),
        );
        assert_eq!(formats.len(), Self::MAX_PLANES);

        let n = unsafe {
            GrYUVABackendTextureInfo::new(info.native(), formats[0].native(), mip_mapped, origin)
        };
        Self::native_is_valid(&n).then(|| Self::from_native_c(n))
    }

    pub fn yuva_info(&self) -> &YUVAInfo {
        YUVAInfo::from_native_ref(&self.native().fYUVAInfo)
    }

    pub fn yuv_color_space(&self) -> YUVColorSpace {
        self.yuva_info().yuv_color_space()
    }

    pub fn mipmapped(&self) -> Mipmapped {
        self.native().fMipmapped
    }

    pub fn texture_origin(&self) -> SurfaceOrigin {
        self.native().fTextureOrigin
    }

    /// The number of [crate::Pixmap] planes.
    pub fn num_planes(&self) -> usize {
        self.yuva_info().num_planes()
    }

    /// Format of the ith plane, or `None` if `i >= Self::num_planes()`
    pub fn plane_format(&self, i: usize) -> Option<&BackendFormat> {
        (i < self.num_planes())
            .then(|| BackendFormat::from_native_ref(&self.native().fPlaneFormats[i]))
    }

    /// All plane formats.
    pub fn plane_formats(&self) -> &[BackendFormat] {
        unsafe {
            let formats = BackendFormat::from_native_ref(&self.native().fPlaneFormats[0]);
            safer::from_raw_parts(formats, self.num_planes())
        }
    }

    /// Returns `true` if this has been configured with a valid [YUVAInfo] with compatible texture.
    pub(crate) fn native_is_valid(info: &GrYUVABackendTextureInfo) -> bool {
        YUVAInfo::native_is_valid(&info.fYUVAInfo)
    }
}

/// A set of [BackendTexture]s that hold the planar data for an image described a [YUVAInfo].
///
// Because BackendTexture itself is a RefHandle, we need to put GrYUVABackendTextures on the heap, too.
// See <https://github.com/rust-skia/rust-skia/issues/1246>
pub type YUVABackendTextures = RefHandle<GrYUVABackendTextures>;
unsafe_send_sync!(YUVABackendTextures);

impl NativeDrop for GrYUVABackendTextures {
    fn drop(&mut self) {
        unsafe { sb::C_GrYUVABackendTextures_delete(self) }
    }
}

impl fmt::Debug for YUVABackendTextures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YUVABackendTextures")
            .field("yuva_info", &self.yuva_info())
            .field("texture_origin", &self.texture_origin())
            .field("textures", &self.textures())
            .finish()
    }
}

impl YUVABackendTextures {
    pub fn new(
        info: &YUVAInfo,
        textures: &[BackendTexture],
        texture_origin: SurfaceOrigin,
    ) -> Option<Self> {
        if textures.len() != info.num_planes() {
            return None;
        }
        let new_invalid = BackendTexture::new_invalid();
        let new_invalid_ptr = new_invalid.native() as *const _;

        let mut texture_handles = textures
            .iter()
            .map(|tex| tex.native() as *const _)
            .collect::<Vec<_>>();
        texture_handles.extend(iter::repeat_n(
            new_invalid_ptr,
            YUVAInfo::MAX_PLANES - textures.len(),
        ));
        assert_eq!(texture_handles.len(), YUVAInfo::MAX_PLANES);

        let textures = Self::from_ptr(unsafe {
            sb::C_GrYUVABackendTextures_new(info.native(), texture_handles.as_ptr(), texture_origin)
        })?;
        Self::native_is_valid(textures.native()).then_some(textures)
    }

    pub fn textures(&self) -> Vec<BackendTexture> {
        unsafe {
            let textures_ptr = sb::C_GrYUVABackendTextures_textures(self.native());
            let textures = safer::from_raw_parts(textures_ptr, self.num_planes());
            textures
                .iter()
                .map(|native_texture_ref| {
                    BackendTexture::from_ptr(sb::C_GrBackendTexture_Clone(native_texture_ref))
                        .unwrap()
                })
                .collect()
        }
    }

    pub fn texture(&self, i: usize) -> Option<BackendTexture> {
        self.textures().get(i).cloned()
    }

    pub fn yuva_info(&self) -> &YUVAInfo {
        YUVAInfo::from_native_ref(unsafe { &*sb::C_GrYUVABackendTextures_yuvaInfo(self.native()) })
    }

    pub fn num_planes(&self) -> usize {
        self.yuva_info().num_planes()
    }

    pub fn texture_origin(&self) -> SurfaceOrigin {
        unsafe { sb::C_GrYUVABackendTextures_textureOrigin(self.native()) }
    }

    pub(crate) fn native_is_valid(n: &GrYUVABackendTextures) -> bool {
        YUVAInfo::native_is_valid(unsafe { &*sb::C_GrYUVABackendTextures_yuvaInfo(n) })
    }
}
