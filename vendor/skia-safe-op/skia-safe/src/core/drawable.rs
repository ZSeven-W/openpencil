use std::fmt;

use skia_bindings::{self as sb, SkDrawable, SkFlattenable, SkRefCntBase};

use crate::{prelude::*, Canvas, Matrix, NativeFlattenable, Picture, Point, Rect};

pub type Drawable = RCHandle<SkDrawable>;

impl NativeRefCountedBase for SkDrawable {
    type Base = SkRefCntBase;
}

impl NativeFlattenable for SkDrawable {
    fn native_flattenable(&self) -> &SkFlattenable {
        unsafe { &*(self as *const SkDrawable as *const SkFlattenable) }
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkDrawable_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl fmt::Debug for Drawable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Drawable")
            // TODO: clarify why &self has to be mut here.
            // .field("generation_id", &self.generation_id())
            // .field("bounds", &self.bounds())
            .finish()
    }
}

impl Drawable {
    pub fn draw(&mut self, canvas: &Canvas, matrix: Option<&Matrix>) {
        unsafe {
            self.native_mut()
                .draw(canvas.native_mut(), matrix.native_ptr_or_null())
        }
    }

    pub fn draw_at(&mut self, canvas: &Canvas, point: impl Into<Point>) {
        let point = point.into();
        unsafe {
            self.native_mut()
                .draw1(canvas.native_mut(), point.x, point.y)
        }
    }

    #[cfg(feature = "gpu")]
    pub fn snap_gpu_draw_handler(
        &mut self,
        api: crate::gpu::BackendAPI,
        matrix: &Matrix,
        clip_bounds: impl Into<crate::IRect>,
        buffer_info: &crate::ImageInfo,
    ) -> Option<gpu_draw_handler::GPUDrawHandler> {
        gpu_draw_handler::GPUDrawHandler::from_ptr(unsafe {
            sb::C_SkDrawable_snapGpuDrawHandler(
                self.native_mut(),
                api,
                matrix.native(),
                clip_bounds.into().native(),
                buffer_info.native(),
            )
        })
    }

    pub fn make_picture_snapshot(&mut self) -> Picture {
        Picture::from_ptr(unsafe { sb::C_SkDrawable_makePictureSnapshot(self.native_mut()) })
            .expect("Internal error: SkDrawable::makePictureSnapshot returned null")
    }

    pub fn generation_id(&mut self) -> u32 {
        unsafe { self.native_mut().getGenerationID() }
    }

    pub fn bounds(&mut self) -> Rect {
        Rect::construct(|r| unsafe { sb::C_SkDrawable_getBounds(self.native_mut(), r) })
    }

    pub fn approximate_bytes_used(&mut self) -> usize {
        unsafe { self.native_mut().approximateBytesUsed() }
    }

    pub fn notify_drawing_changed(&mut self) {
        unsafe { self.native_mut().notifyDrawingChanged() }
    }
}

#[cfg(feature = "gpu")]
pub use gpu_draw_handler::*;

#[cfg(feature = "gpu")]
pub mod gpu_draw_handler {
    use std::fmt;

    use skia_bindings::{self as sb, SkDrawable_GpuDrawHandler};

    use crate::prelude::*;

    pub type GPUDrawHandler = RefHandle<SkDrawable_GpuDrawHandler>;

    impl NativeDrop for SkDrawable_GpuDrawHandler {
        fn drop(&mut self) {
            unsafe { sb::C_SkDrawable_GpuDrawHandler_delete(self) }
        }
    }

    impl fmt::Debug for GPUDrawHandler {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("GPUDrawHandler").finish()
        }
    }

    #[cfg(feature = "vulkan")]
    impl GPUDrawHandler {
        pub fn draw(&mut self, info: &crate::gpu::vk::BackendDrawableInfo) {
            unsafe {
                sb::C_SkDrawable_GpuDrawHandler_draw(self.native_mut(), info.native());
            }
        }
    }
}
