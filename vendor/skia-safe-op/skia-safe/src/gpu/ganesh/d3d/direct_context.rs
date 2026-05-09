pub mod direct_contexts {
    use skia_bindings as sb;

    use crate::{
        gpu::{d3d, ContextOptions, DirectContext},
        prelude::*,
    };

    #[allow(clippy::missing_safety_doc)]
    /// Makes a [`DirectContext`] which uses Direct3D as the backend. The Direct3D context
    /// must be kept alive until the returned [`DirectContext`] is first destroyed or abandoned.
    pub unsafe fn make_d3d<'a>(
        backend_context: &d3d::BackendContext,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        DirectContext::from_ptr(sb::C_GrDirectContexts_MakeD3D(
            backend_context.native(),
            options.into().native_ptr_or_null(),
        ))
    }
}
