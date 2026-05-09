use std::{fmt, ptr};

use skia_bindings::{self as sb, GrMtlSurfaceInfo, GrMtlTextureInfo};

use crate::{
    gpu,
    prelude::{self, NativeAccess, NativeDrop, NativePartialEq},
};

pub use skia_bindings::GrMTLHandle as Handle;
pub use skia_bindings::GrMTLPixelFormat as PixelFormat;
pub use skia_bindings::GrMTLStorageMode as StorageMode;
pub use skia_bindings::GrMTLTextureUsage as TextureUsage;

pub type TextureInfo = prelude::Handle<GrMtlTextureInfo>;
unsafe_send_sync!(TextureInfo);

impl NativeDrop for GrMtlTextureInfo {
    fn drop(&mut self) {
        unsafe { sb::C_GrMtlTextureInfo_Destruct(self) }
    }
}

impl NativePartialEq for GrMtlTextureInfo {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_GrMtlTextureInfo_Equals(self, other) }
    }
}

impl Default for TextureInfo {
    fn default() -> Self {
        unsafe { Self::new(ptr::null()) }
    }
}

impl fmt::Debug for TextureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextureInfo")
            .field("texture", &self.texture())
            .finish()
    }
}

impl TextureInfo {
    /// # Safety
    ///
    /// Unsafe because the texture provided must either be `null` or pointing to a Metal texture by
    /// providing a raw pointer.
    ///
    /// This function retains the texture and releases it as soon TextureInfo is dropped.
    pub unsafe fn new(texture: Handle) -> Self {
        Self::construct(|ti| sb::C_GrMtlTextureInfo_Construct(ti, texture))
    }

    pub fn texture(&self) -> Handle {
        self.native().fTexture.fObject
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct SurfaceInfo {
    pub sample_count: u32,
    pub level_count: u32,
    pub protected: gpu::Protected,

    pub format: PixelFormat,
    pub usage: TextureUsage,
    pub storage_mode: StorageMode,
}

native_transmutable!(GrMtlSurfaceInfo, SurfaceInfo);

impl Default for SurfaceInfo {
    fn default() -> Self {
        Self {
            sample_count: 1,
            level_count: 0,
            protected: gpu::Protected::No,
            format: 0,
            usage: 0,
            storage_mode: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextureInfo;

    #[test]
    fn default_texture_info() {
        drop(TextureInfo::default());
    }
}
