use crate::{
    interop::{self, AsStr},
    prelude::*,
    Blender, ColorFilter, Data, Matrix, Shader,
};
use core::ffi;
use sb::{SkFlattenable, SkRuntimeEffect_Child};
use skia_bindings::{
    self as sb, ShaderBuilderUniformResult, SkRefCntBase, SkRuntimeEffect, SkRuntimeEffect_Uniform,
};
use std::{fmt, marker::PhantomData, ops::DerefMut, ptr};

pub type Uniform = Handle<SkRuntimeEffect_Uniform>;
unsafe_send_sync!(Uniform);

#[deprecated(since = "0.35.0", note = "Use Uniform instead")]
pub type Variable = Uniform;

impl NativeDrop for SkRuntimeEffect_Uniform {
    fn drop(&mut self) {
        panic!("native type SkRuntimeEffect::Uniform can't be owned by Rust");
    }
}

impl fmt::Debug for Uniform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.native().fmt(f)
    }
}

impl Uniform {
    pub fn name(&self) -> &str {
        self.native().name.as_str()
    }

    pub fn offset(&self) -> usize {
        self.native().offset
    }

    pub fn ty(&self) -> uniform::Type {
        self.native().type_
    }

    pub fn count(&self) -> i32 {
        self.native().count
    }

    pub fn flags(&self) -> uniform::Flags {
        uniform::Flags::from_bits(self.native().flags).unwrap()
    }

    pub fn is_array(&self) -> bool {
        self.flags().contains(uniform::Flags::ARRAY)
    }

    pub fn is_color(&self) -> bool {
        self.flags().contains(uniform::Flags::COLOR)
    }

    pub fn size_in_bytes(&self) -> usize {
        unsafe { self.native().sizeInBytes() }
    }
}

pub mod uniform {
    use skia_bindings as sb;

    pub use sb::SkRuntimeEffect_Uniform_Type as Type;
    variant_name!(Type::Float2x2);

    bitflags! {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct Flags : u32 {
            const ARRAY = sb::SkRuntimeEffect_Uniform_Flags_kArray_Flag as _;
            const COLOR = sb::SkRuntimeEffect_Uniform_Flags_kColor_Flag as _;
            const VERTEX = sb::SkRuntimeEffect_Uniform_Flags_kVertex_Flag as _;
            const FRAGMENT = sb::SkRuntimeEffect_Uniform_Flags_kFragment_Flag as _;
            const HALF_PRECISION = sb::SkRuntimeEffect_Uniform_Flags_kHalfPrecision_Flag as _;
        }
    }
}

pub use sb::SkRuntimeEffect_ChildType as ChildType;
variant_name!(ChildType::Shader);

#[deprecated(since = "0.41.0", note = "Use Child")]
pub type Varying = Child;

pub type Child = Handle<SkRuntimeEffect_Child>;
unsafe_send_sync!(Child);

impl NativeDrop for SkRuntimeEffect_Child {
    fn drop(&mut self) {
        panic!("native type SkRuntimeEffect::Child can't be owned in Rust");
    }
}

impl fmt::Debug for Child {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Child")
            .field("name", &self.name())
            .field("type", &self.ty())
            .field("index", &self.index())
            .finish()
    }
}

impl Child {
    pub fn name(&self) -> &str {
        self.native().name.as_str()
    }

    pub fn ty(&self) -> ChildType {
        self.native().type_
    }

    pub fn index(&self) -> usize {
        self.native().index.try_into().unwrap()
    }
}

pub type RuntimeEffect = RCHandle<SkRuntimeEffect>;

impl NativeRefCountedBase for SkRuntimeEffect {
    type Base = SkRefCntBase;
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Options<'a> {
    pub force_unoptimized: bool,
    pub name: &'a str,
}

impl fmt::Debug for RuntimeEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeEffect")
            .field("uniform_size", &self.uniform_size())
            .field("uniforms", &self.uniforms())
            .field("children", &self.children())
            .field("allow_shader", &self.allow_shader())
            .field("allow_color_filter", &self.allow_color_filter())
            .field("allow_blender", &self.allow_blender())
            .finish()
    }
}

impl RuntimeEffect {
    pub fn make_for_color_filter(
        sksl: impl AsRef<str>,
        options: Option<&Options<'_>>,
    ) -> Result<RuntimeEffect, String> {
        let str = interop::String::from_str(sksl);
        let options = options.copied().unwrap_or_default();
        let options = Self::construct_native_options(&options);
        let mut error = interop::String::default();
        RuntimeEffect::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_MakeForColorFilter(str.native(), &options, error.native_mut())
        })
        .ok_or_else(|| error.to_string())
    }

    pub fn make_for_shader(
        sksl: impl AsRef<str>,
        options: Option<&Options<'_>>,
    ) -> Result<RuntimeEffect, String> {
        let str = interop::String::from_str(sksl);
        let options = options.copied().unwrap_or_default();
        let options = Self::construct_native_options(&options);
        let mut error = interop::String::default();
        RuntimeEffect::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_MakeForShader(str.native(), &options, error.native_mut())
        })
        .ok_or_else(|| error.to_string())
    }

    pub fn make_for_blender(
        sksl: impl AsRef<str>,
        options: Option<&Options<'_>>,
    ) -> Result<RuntimeEffect, String> {
        let str = interop::String::from_str(sksl);
        let options = options.copied().unwrap_or_default();
        let options = Self::construct_native_options(&options);
        let mut error = interop::String::default();
        RuntimeEffect::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_MakeForBlender(str.native(), &options, error.native_mut())
        })
        .ok_or_else(|| error.to_string())
    }

    fn construct_native_options(options: &Options<'_>) -> sb::SkRuntimeEffect_Options {
        construct(|opt| unsafe {
            sb::C_SkRuntimeEffect_Options_Construct(
                opt,
                options.force_unoptimized,
                options.name.as_ptr() as *const ffi::c_char,
                options.name.len(),
            )
        })
    }

    pub fn make_shader<'a>(
        &self,
        uniforms: impl Into<Data>,
        children: &[ChildPtr],
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        let mut children: Vec<_> = children
            .iter()
            .map(|child_ptr| child_ptr.native())
            .collect();
        let children_ptr = children
            .first_mut()
            .map(|c| c.deref_mut() as *mut _)
            .unwrap_or(ptr::null_mut());
        Shader::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_makeShader(
                self.native(),
                uniforms.into().into_ptr(),
                children_ptr,
                children.len(),
                local_matrix.into().native_ptr_or_null(),
            )
        })
    }

    pub fn make_color_filter<'a>(
        &self,
        inputs: impl Into<Data>,
        children: impl Into<Option<&'a [ChildPtr]>>,
    ) -> Option<ColorFilter> {
        let mut children: Vec<_> = children
            .into()
            .map(|c| c.iter().map(|child_ptr| child_ptr.native()).collect())
            .unwrap_or_default();
        let children_ptr = children
            .first_mut()
            .map(|c| c.deref_mut() as *mut _)
            .unwrap_or(ptr::null_mut());
        ColorFilter::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_makeColorFilter(
                self.native(),
                inputs.into().into_ptr(),
                children_ptr,
                children.len(),
            )
        })
    }

    pub fn make_blender<'a>(
        &self,
        uniforms: impl Into<Data>,
        children: impl Into<Option<&'a [ChildPtr]>>,
    ) -> Option<Blender> {
        let mut children: Vec<_> = children
            .into()
            .map(|c| c.iter().map(|child_ptr| child_ptr.native()).collect())
            .unwrap_or_default();
        let children_ptr = children
            .first_mut()
            .map(|c| c.deref_mut() as *mut _)
            .unwrap_or(ptr::null_mut());
        Blender::from_ptr(unsafe {
            sb::C_SkRuntimeEffect_makeBlender(
                self.native(),
                uniforms.into().into_ptr(),
                children_ptr,
                children.len(),
            )
        })
    }

    // TODO: wrap MakeTraced

    pub fn source(&self) -> &str {
        let mut len = 0;
        let ptr = unsafe { sb::C_SkRuntimeEffect_source(self.native(), &mut len) };
        std::str::from_utf8(unsafe { safer::from_raw_parts(ptr, len) }).unwrap()
    }

    #[deprecated(since = "0.35.0", note = "Use uniform_size() instead")]
    pub fn input_size(&self) -> usize {
        self.uniform_size()
    }

    pub fn uniform_size(&self) -> usize {
        unsafe { self.native().uniformSize() }
    }

    #[deprecated(since = "0.35.0", note = "Use uniforms() instead")]
    pub fn inputs(&self) -> &[Uniform] {
        self.uniforms()
    }

    pub fn uniforms(&self) -> &[Uniform] {
        unsafe {
            let mut count: usize = 0;
            let ptr = sb::C_SkRuntimeEffect_uniforms(self.native(), &mut count);
            safer::from_raw_parts(Uniform::from_native_ptr(ptr), count)
        }
    }

    pub fn children(&self) -> &[Child] {
        unsafe {
            let mut count: usize = 0;
            let ptr = sb::C_SkRuntimeEffect_children(self.native(), &mut count);
            safer::from_raw_parts(Child::from_native_ptr(ptr), count)
        }
    }

    #[deprecated(since = "0.35.0", note = "Use find_uniform()")]
    pub fn find_input(&self, name: impl AsRef<str>) -> Option<&Uniform> {
        self.find_uniform(name)
    }

    pub fn find_uniform(&self, name: impl AsRef<str>) -> Option<&Uniform> {
        let name = name.as_ref().as_bytes();
        unsafe { sb::C_SkRuntimeEffect_findUniform(self.native(), name.as_ptr() as _, name.len()) }
            .into_non_null()
            .map(|ptr| Uniform::from_native_ref(unsafe { ptr.as_ref() }))
    }

    pub fn find_child(&self, name: impl AsRef<str>) -> Option<&Child> {
        let name = name.as_ref().as_bytes();
        unsafe { sb::C_SkRuntimeEffect_findChild(self.native(), name.as_ptr() as _, name.len()) }
            .into_non_null()
            .map(|ptr| Child::from_native_ref(unsafe { ptr.as_ref() }))
    }

    pub fn allow_shader(&self) -> bool {
        unsafe { sb::C_SkRuntimeEffect_allowShader(self.native()) }
    }

    pub fn allow_color_filter(&self) -> bool {
        unsafe { sb::C_SkRuntimeEffect_allowColorFilter(self.native()) }
    }

    pub fn allow_blender(&self) -> bool {
        unsafe { sb::C_SkRuntimeEffect_allowBlender(self.native()) }
    }
}

#[derive(Clone, Debug)]
pub enum ChildPtr {
    Shader(Shader),
    ColorFilter(ColorFilter),
    Blender(Blender),
}

impl From<Shader> for ChildPtr {
    fn from(shader: Shader) -> Self {
        Self::Shader(shader)
    }
}

impl From<ColorFilter> for ChildPtr {
    fn from(color_filter: ColorFilter) -> Self {
        Self::ColorFilter(color_filter)
    }
}

impl From<Blender> for ChildPtr {
    fn from(blender: Blender) -> Self {
        Self::Blender(blender)
    }
}

// TODO: Create `ChildPtr` from a Flattenable?

impl ChildPtr {
    pub fn ty(&self) -> ChildType {
        match self {
            ChildPtr::Shader(_) => ChildType::Shader,
            ChildPtr::ColorFilter(_) => ChildType::ColorFilter,
            ChildPtr::Blender(_) => ChildType::Blender,
        }
    }

    // We are treating [`ChildPtr`]s as a _reference_ to a smart pointer: no reference counters are
    // changed (no drop() is called either).
    //
    // Skia will copy the pointers and increase the reference counters if it uses the actual
    // objects.
    pub(self) fn native(&self) -> Borrows<sb::SkRuntimeEffect_ChildPtr> {
        let flattenable: *mut SkFlattenable = match self {
            // casting to &T &mut T is UB, so we don't use the base() indirection and directly cast
            // to a pointer.
            ChildPtr::Shader(shader) => unsafe { shader.native_mut_force() as _ },
            ChildPtr::ColorFilter(color_filter) => unsafe { color_filter.native_mut_force() as _ },
            ChildPtr::Blender(blender) => unsafe { blender.native_mut_force() as _ },
        };

        sb::SkRuntimeEffect_ChildPtr {
            fChild: sb::sk_sp {
                fPtr: flattenable,
                _phantom_0: PhantomData,
            },
        }
        .borrows(self)
    }
}

// TODO: wrap SkRuntimeEffectBuilder, SkRuntimeColorFilterBuilder,
// SkRuntimeBlendBuilder

pub type RuntimeShaderBuilder = Handle<sb::SkRuntimeShaderBuilder>;
unsafe_send_sync!(RuntimeShaderBuilder);

#[derive(Debug)]
pub enum ShaderBuilderError {
    UniformSizeNotSupported,
}
impl fmt::Display for ShaderBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderBuilderError::UniformSizeNotSupported => write!(f, "Unsupported uniform size"),
        }
    }
}
impl std::error::Error for ShaderBuilderError {}

impl NativeDrop for sb::SkRuntimeShaderBuilder {
    fn drop(&mut self) {
        unsafe {
            sb::C_SkRuntimeShaderBuilder_destruct(self);
        }
    }
}

impl RuntimeShaderBuilder {
    pub fn new(effect: RuntimeEffect) -> Self {
        Self::construct(|builder| unsafe {
            let effect: *mut SkRuntimeEffect = effect.into_ptr() as _;
            sb::C_SkRuntimeShaderBuilder_Construct(builder, effect)
        })
    }

    pub fn make_shader(&self, local_matrix: &Matrix) -> Option<Shader> {
        unsafe {
            let instance = self.native_mut_force();
            let shader =
                sb::C_SkRuntimeShaderBuilder_makeShader(instance, local_matrix.native() as _);
            Shader::from_ptr(shader)
        }
    }
    /// Set float uniform values by name.
    ///
    /// Supported types are `float`, `float2`, `float3`, `float4`, `float2x2`, `float3x3`, `float4x4`.
    ///
    /// The data array must have the correct length for the corresponding uniform type:
    /// - `float`: `[f32; 1]`
    /// - `float2`: `[f32; 2]`
    /// - `float3`: `[f32; 3]`
    /// - `float4`: `[f32; 4]`
    /// - `float2x2`: `[f32; 4]`
    /// - `float3x3`: `[f32; 9]`
    /// - `float4x4`: `[f32; 16]`
    ///
    pub fn set_uniform_float(
        &mut self,
        name: impl AsRef<str>,
        data: &[f32],
    ) -> Result<(), ShaderBuilderError> {
        let name = name.as_ref();
        let result = unsafe {
            sb::C_SkRuntimeShaderBuilder_setUniformFloat(
                self.native_mut() as _,
                name.as_bytes().as_ptr() as _,
                name.len(),
                data.as_ptr() as _,
                data.len(),
            )
        };
        match result {
            ShaderBuilderUniformResult::Ok => Ok(()),
            ShaderBuilderUniformResult::Error => Err(ShaderBuilderError::UniformSizeNotSupported),
        }
    }
    /// Set int uniform values by name.
    ///
    /// Supported types are `int`, `int2`, `int3`, `int4`.
    ///
    /// The data array must have the correct length for the corresponding uniform type:
    /// - `int`: `[i32; 1]`
    /// - `int2`: `[i32; 2]`
    /// - `int3`: `[i32; 3]`
    /// - `int4`: `[i32; 4]`
    ///
    ///
    pub fn set_uniform_int(
        &mut self,
        name: impl AsRef<str>,
        data: &[i32],
    ) -> Result<(), ShaderBuilderError> {
        let name = name.as_ref();
        let result = unsafe {
            sb::C_SkRuntimeShaderBuilder_setUniformInt(
                self.native_mut() as _,
                name.as_bytes().as_ptr() as _,
                name.len(),
                data.as_ptr() as _,
                data.len(),
            )
        };
        match result {
            ShaderBuilderUniformResult::Ok => Ok(()),
            ShaderBuilderUniformResult::Error => Err(ShaderBuilderError::UniformSizeNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // <https://github.com/rust-skia/rust-skia/discussions/1133>
    #[allow(unused)]
    fn none_cases_compile() {
        RuntimeEffect::make_for_color_filter("", None);
        RuntimeEffect::make_for_shader("", None);
        RuntimeEffect::make_for_blender("", None);
    }
}
