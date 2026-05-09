#![allow(deprecated)]
use crate::{prelude::*, scalar, Canvas, Matrix, M44, V3};
use skia_bindings::{self as sb, Sk3DView, SkCamera3D, SkPatch3D};
use std::fmt;

#[deprecated(
    since = "0.30.0",
    note = "Skia now has support for a 4x matrix (core::M44) in core::Canvas."
)]
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Patch3D {
    pub u: V3,
    pub v: V3,
    pub origin: V3,
}

native_transmutable!(SkPatch3D, Patch3D);

impl Default for Patch3D {
    fn default() -> Self {
        Patch3D::from_native_c(unsafe { SkPatch3D::new() })
    }
}

impl Patch3D {
    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() }
        self
    }

    #[must_use]
    pub fn transform(&self, m: &M44) -> Self {
        let mut dst = Patch3D::default();
        unsafe { self.native().transform(m.native(), dst.native_mut()) }
        dst
    }

    pub fn dot_with(&self, v: impl Into<V3>) -> scalar {
        let v = v.into();
        unsafe { self.native().dotWith(v.x, v.y, v.z) }
    }
}

#[deprecated(
    since = "0.30.0",
    note = "Skia now has support for a 4x matrix (core::M44) in core::Canvas."
)]
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Camera3D {
    pub location: V3,
    pub axis: V3,
    pub zenith: V3,
    pub observer: V3,
    orientation: Matrix,
    need_to_update: bool,
}

native_transmutable!(SkCamera3D, Camera3D);

impl Default for Camera3D {
    fn default() -> Self {
        Camera3D::from_native_c(unsafe { SkCamera3D::new() })
    }
}

impl Camera3D {
    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() }
        self
    }

    pub fn update(&mut self) -> &mut Self {
        unsafe { self.native_mut().update() }
        self
    }

    pub fn patch_to_matrix(&self, quilt: &Patch3D) -> Matrix {
        let mut matrix = Matrix::default();
        unsafe {
            self.native()
                .patchToMatrix(quilt.native(), matrix.native_mut())
        }
        matrix
    }
}

// Note: original name is Sk3DView not SkView3D
// Also note that the implementation uses interior pointers,
// so we let Skia do the allocation.

#[deprecated(
    since = "0.30.0",
    note = "Skia now has support for a 4x matrix (core::M44) in core::Canvas."
)]
pub type View3D = RefHandle<Sk3DView>;
unsafe_send_sync!(View3D);

impl Default for View3D {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeDrop for Sk3DView {
    fn drop(&mut self) {
        unsafe { sb::C_Sk3DView_delete(self) }
    }
}

impl fmt::Debug for View3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View3D").finish()
    }
}

impl RefHandle<Sk3DView> {
    pub fn new() -> Self {
        View3D::from_ptr(unsafe { sb::C_Sk3DView_new() }).unwrap()
    }

    pub fn save(&mut self) -> &mut Self {
        unsafe { self.native_mut().save() }
        self
    }

    pub fn restore(&mut self) -> &mut Self {
        unsafe { self.native_mut().restore() }
        self
    }

    pub fn translate(&mut self, d: impl Into<V3>) -> &mut Self {
        let d = d.into();
        unsafe { self.native_mut().translate(d.x, d.y, d.z) }
        self
    }

    pub fn rotate_x(&mut self, deg: scalar) -> &mut Self {
        unsafe { self.native_mut().rotateX(deg) }
        self
    }

    pub fn rotate_y(&mut self, deg: scalar) -> &mut Self {
        unsafe { self.native_mut().rotateY(deg) }
        self
    }

    pub fn rotate_z(&mut self, deg: scalar) -> &mut Self {
        unsafe { self.native_mut().rotateZ(deg) }
        self
    }

    pub fn matrix(&self) -> Matrix {
        let mut m = Matrix::default();
        unsafe { self.native().getMatrix(m.native_mut()) }
        m
    }

    pub fn apply_to_canvas(&self, canvas: &Canvas) -> &Self {
        unsafe { self.native().applyToCanvas(canvas.native_mut()) }
        self
    }

    pub fn dot_with_normal(&self, d: impl Into<V3>) -> scalar {
        let d = d.into();
        unsafe { self.native().dotWithNormal(d.x, d.y, d.z) }
    }
}

#[test]
fn test_canvas_passing_syntax() {
    use crate::utils::new_null_canvas;
    use crate::Surface;

    let null_canvas = new_null_canvas();
    let view = View3D::default();
    // as mutable reference
    view.apply_to_canvas(&null_canvas);

    // and one with a mutable reference to a shared Canvas:
    let mut surface = Surface::new_raster_n32_premul((100, 100)).unwrap();
    view.apply_to_canvas(surface.canvas());
}
