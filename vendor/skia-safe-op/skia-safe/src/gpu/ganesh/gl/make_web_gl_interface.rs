pub mod interfaces {
    use skia_bindings as sb;

    use crate::gpu::gl;

    pub fn make_web_gl() -> Option<gl::Interface> {
        gl::Interface::from_ptr(unsafe { sb::C_GrGLInterfaces_MakeWebGL() } as *mut _)
    }
}
