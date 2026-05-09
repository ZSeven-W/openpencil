use std::fmt;

use crate::{
    gpu::mtl::Handle,
    prelude::{self, NativeDrop},
};

use skia_bindings::{self as sb, GrMtlBackendContext};

pub type BackendContext = prelude::Handle<GrMtlBackendContext>;
unsafe_send_sync!(BackendContext);

impl NativeDrop for GrMtlBackendContext {
    fn drop(&mut self) {
        unsafe { sb::C_GrMtlBackendContext_Destruct(self) }
    }
}

impl fmt::Debug for BackendContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackendContext").finish()
    }
}

impl BackendContext {
    /// # Safety
    ///
    /// Unsafe because it expects various objects in form of `c_void` pointers.
    ///
    /// This function retains all the non-`null` handles passed to it and releases them as soon the
    /// [BackendContext] is dropped.
    pub unsafe fn new(device: Handle, queue: Handle) -> Self {
        BackendContext::construct(|bc| sb::C_GrMtlBackendContext_Construct(bc, device, queue))
    }
}
