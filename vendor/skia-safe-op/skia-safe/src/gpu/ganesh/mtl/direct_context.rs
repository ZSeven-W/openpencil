pub mod direct_contexts {

    use skia_bindings as sb;

    use crate::{
        gpu::{mtl, ContextOptions, DirectContext},
        prelude::*,
    };

    /// Makes a [`DirectContext`] which uses Metal as the backend. The [`mtl::BackendContext`] contains a
    /// MTLDevice and MTLCommandQueue which should be used by the backend. These objects must
    /// have their own ref which will be released when the [`mtl::BackendContext`] is destroyed.
    /// Ganesh will take its own ref on the objects which will be released when the [`DirectContext`]
    /// is destroyed.
    pub fn make_metal<'a>(
        backend: &mtl::BackendContext,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        DirectContext::from_ptr(unsafe {
            sb::C_GrContext_MakeMetal(backend.native(), options.into().native_ptr_or_null())
        })
    }
}
