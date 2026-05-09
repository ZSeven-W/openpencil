use std::fmt;

use skia_bindings::{self as sb, GrBackendDrawableInfo};

use crate::{
    gpu::{vk, BackendAPI},
    prelude::*,
};

pub type BackendDrawableInfo = Handle<GrBackendDrawableInfo>;
unsafe_send_sync!(BackendDrawableInfo);

impl NativeDrop for GrBackendDrawableInfo {
    fn drop(&mut self) {
        unsafe { sb::C_GrBackendDrawableInfo_destruct(self) }
    }
}

impl fmt::Debug for BackendDrawableInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("BackendDrawableInfo");
        d.field("is_valid", &self.is_valid());
        d.field("backend", &self.backend());
        #[cfg(feature = "vulkan")]
        d.field("vk_drawable_info", &self.get_vk_drawable_info());
        d.finish()
    }
}

impl BackendDrawableInfo {
    pub fn new() -> Self {
        Self::construct(|di| unsafe { sb::C_GrBackendDrawableInfo_Construct(di) })
    }

    #[cfg(feature = "vulkan")]
    pub fn from_vk(info: &vk::DrawableInfo) -> Self {
        Self::construct(|di| unsafe { sb::C_GrBackendDrawableInfo_Construct2(di, info.native()) })
    }

    pub fn is_valid(&self) -> bool {
        unsafe { sb::C_GrBackendDrawableInfo_isValid(self.native()) }
    }

    pub fn backend(&self) -> BackendAPI {
        unsafe { sb::C_GrBackendDrawableInfo_backend(self.native()) }
    }

    #[cfg(feature = "vulkan")]
    pub fn get_vk_drawable_info(&self) -> Option<vk::DrawableInfo> {
        unsafe {
            let mut di = vk::DrawableInfo::default();
            sb::C_GrBackendDrawableInfo_getVkDrawableInfo(self.native(), di.native_mut())
                .then_some(di)
        }
    }
}
