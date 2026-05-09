use crate::{gpu::gl::Extensions, prelude::*};
use skia_bindings::{self as sb, GrGLInterface, SkRefCntBase};
use std::{ffi::c_void, fmt, os::raw};

pub type Interface = RCHandle<GrGLInterface>;
require_type_equality!(sb::GrGLInterface_INHERITED, sb::SkRefCnt);

impl NativeRefCountedBase for GrGLInterface {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interface")
            .field("extensions", &self.extensions())
            .finish()
    }
}

impl Interface {
    pub fn new_native() -> Option<Self> {
        Self::from_ptr(unsafe { sb::C_GrGLInterface_MakeNativeInterface() as _ })
    }

    pub fn new_load_with<F>(load_fn: F) -> Option<Self>
    where
        F: FnMut(&str) -> *const c_void,
    {
        Self::from_ptr(unsafe {
            sb::C_GrGLInterface_MakeAssembledInterface(
                &load_fn as *const _ as *mut c_void,
                Some(gl_get_proc_fn_wrapper::<F>),
            ) as _
        })
    }

    pub fn new_load_with_cstr<F>(load_fn: F) -> Option<Self>
    where
        F: FnMut(&std::ffi::CStr) -> *const c_void,
    {
        Self::from_ptr(unsafe {
            sb::C_GrGLInterface_MakeAssembledInterface(
                &load_fn as *const _ as *mut c_void,
                Some(gl_get_proc_fn_wrapper_cstr::<F>),
            ) as _
        })
    }

    pub fn validate(&self) -> bool {
        unsafe { self.native().validate() }
    }

    pub fn extensions(&self) -> &Extensions {
        Extensions::from_native_ref(unsafe {
            &*sb::C_GrGLInterface_extensions(self.native_mut_force())
        })
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        Extensions::from_native_ref_mut(unsafe {
            &mut *sb::C_GrGLInterface_extensions(self.native_mut())
        })
    }

    pub fn has_extension(&self, extension: impl AsRef<str>) -> bool {
        self.extensions().has(extension)
    }
}

unsafe extern "C" fn gl_get_proc_fn_wrapper<F>(
    ctx: *mut c_void,
    name: *const raw::c_char,
) -> *const c_void
where
    F: FnMut(&str) -> *const c_void,
{
    (*(ctx as *mut F))(std::ffi::CStr::from_ptr(name).to_str().unwrap())
}

unsafe extern "C" fn gl_get_proc_fn_wrapper_cstr<F>(
    ctx: *mut c_void,
    name: *const raw::c_char,
) -> *const c_void
where
    F: FnMut(&std::ffi::CStr) -> *const c_void,
{
    (*(ctx as *mut F))(std::ffi::CStr::from_ptr(name))
}
