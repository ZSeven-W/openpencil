pub mod direct_contexts {
    use skia_bindings as sb;

    use crate::{
        gpu::{gl, ContextOptions, DirectContext},
        prelude::*,
    };

    pub fn make_gl<'a>(
        interface: impl Into<gl::Interface>,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        DirectContext::from_ptr(unsafe {
            sb::C_GrDirectContext_MakeGL(
                interface.into().into_ptr(),
                options.into().native_ptr_or_null(),
            )
        })
    }
}
