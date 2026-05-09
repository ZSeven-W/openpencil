pub mod interfaces {
    use skia_bindings as sb;

    use crate::gpu::gl;

    pub fn make_egl() -> Option<gl::Interface> {
        gl::Interface::from_ptr(unsafe { sb::C_GrGLInterfaces_MakeEGL() } as *mut _)
    }
}
