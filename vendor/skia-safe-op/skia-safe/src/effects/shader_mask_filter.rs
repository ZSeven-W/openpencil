use crate::{prelude::*, MaskFilter, Shader};
use skia_bindings as sb;

impl MaskFilter {
    #[deprecated(
        since = "0.76.0",
        note = "ShaderMaskFilters will be deleted entirely in an upcoming Skia release."
    )]
    pub fn from_shader(shader: impl Into<Shader>) -> MaskFilter {
        #[allow(deprecated)]
        new(shader)
    }
}

#[deprecated(
    since = "0.76.0",
    note = "ShaderMaskFilters will be deleted entirely in an upcoming Skia release."
)]
pub fn new(shader: impl Into<Shader>) -> MaskFilter {
    MaskFilter::from_ptr(unsafe { sb::C_SkShaderMaskFilter_Make(shader.into().into_ptr()) })
        .unwrap()
}
