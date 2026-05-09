use std::fmt;

use skia_bindings::{self as sb, GrBackendSemaphore};

use crate::prelude::*;

/// Wrapper type for passing into and receiving data from Ganesh about a
/// backend semaphore object.
pub type BackendSemaphore = Handle<GrBackendSemaphore>;
unsafe_send_sync!(BackendSemaphore);

impl NativeDrop for GrBackendSemaphore {
    fn drop(&mut self) {
        unsafe { sb::C_GrBackendSemaphore_destruct(self) }
    }
}

impl NativeClone for GrBackendSemaphore {
    fn clone(&self) -> Self {
        construct(|semaphore| unsafe { sb::C_GrBackendSemaphore_CopyConstruct(semaphore, self) })
    }
}

impl fmt::Debug for BackendSemaphore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackendSemaphore")
            .field("backend", &self.backend())
            .field("is_initialized", &self.is_initialized())
            .finish()
    }
}

impl BackendSemaphore {
    pub fn backend(&self) -> sb::GrBackendApi {
        unsafe { sb::C_GrBackendSemaphore_backend(self.native()) }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        unsafe { sb::C_GrBackendSemaphore_isInitialized(self.native()) }
    }
}
