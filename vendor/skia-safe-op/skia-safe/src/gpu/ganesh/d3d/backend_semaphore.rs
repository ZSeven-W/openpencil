use skia_bindings as sb;

use crate::{
    gpu::{d3d, BackendSemaphore},
    prelude::*,
};

pub mod backend_semaphores {
    use super::*;

    pub fn make_d3d(info: &d3d::FenceInfo) -> BackendSemaphore {
        let semaphore = BackendSemaphore::construct(|semaphore| unsafe {
            sb::C_GrBackendSemaphore_ConstructD3D(semaphore, info.native())
        });
        assert!(semaphore.is_initialized());
        semaphore
    }

    pub fn get_d3d_fence_info(semaphore: &BackendSemaphore) -> Option<d3d::FenceInfo> {
        if !semaphore.is_initialized() || semaphore.backend() != sb::GrBackendApi::Direct3D {
            return None;
        }

        let mut info = construct(|info| unsafe { sb::C_GrD3DFenceInfo_Construct(info) });
        unsafe { sb::C_GrBackendSemaphores_GetD3DFenceInfo(semaphore.native(), &mut info) }
        Some(d3d::FenceInfo::from_native_c(info))
    }
}
