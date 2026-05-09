use crate::{Canvas, OwnedCanvas};
use skia_bindings as sb;

impl Canvas {
    pub fn new_null() -> OwnedCanvas<'static> {
        new_null_canvas()
    }
}

pub fn new_null_canvas() -> OwnedCanvas<'static> {
    Canvas::own_from_native_ptr(unsafe { sb::C_SkMakeNullCanvas() }).unwrap()
}

#[test]
fn test_create_null_canvas() {
    let nc = new_null_canvas();
    drop(nc);
}
