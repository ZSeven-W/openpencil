use crate::{interop, prelude::*, scalar, Matrix, Rect, Vector};
use skia_bindings::{self as sb, SkRRect};
use std::{fmt, mem, ptr};

pub use skia_bindings::SkRRect_Type as Type;
variant_name!(Type::Complex);

pub use skia_bindings::SkRRect_Corner as Corner;
variant_name!(Corner::LowerLeft);

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct RRect(SkRRect);

native_transmutable!(SkRRect, RRect);

impl PartialEq for RRect {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkRRect_Equals(self.native(), rhs.native()) }
    }
}

impl Default for RRect {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RRect")
            .field("rect", &self.rect())
            .field(
                "radii",
                &[
                    self.radii(Corner::UpperLeft),
                    self.radii(Corner::UpperRight),
                    self.radii(Corner::LowerRight),
                    self.radii(Corner::LowerLeft),
                ],
            )
            .field("type", &self.get_type())
            .finish()
    }
}

impl AsRef<RRect> for RRect {
    fn as_ref(&self) -> &RRect {
        self
    }
}

impl RRect {
    pub fn new() -> Self {
        RRect::construct(|rr| unsafe { sb::C_SkRRect_Construct(rr) })
    }

    pub fn get_type(&self) -> Type {
        unsafe { sb::C_SkRRect_getType(self.native()) }
    }

    pub fn is_empty(&self) -> bool {
        self.get_type() == Type::Empty
    }

    pub fn is_rect(&self) -> bool {
        self.get_type() == Type::Rect
    }

    pub fn is_oval(&self) -> bool {
        self.get_type() == Type::Oval
    }

    pub fn is_simple(&self) -> bool {
        self.get_type() == Type::Simple
    }

    pub fn is_nine_patch(&self) -> bool {
        self.get_type() == Type::NinePatch
    }

    pub fn is_complex(&self) -> bool {
        self.get_type() == Type::Complex
    }

    pub fn width(&self) -> scalar {
        self.rect().width()
    }

    pub fn height(&self) -> scalar {
        self.rect().height()
    }

    pub fn simple_radii(&self) -> Vector {
        self.radii(Corner::UpperLeft)
    }

    pub fn set_empty(&mut self) {
        *self = Self::new()
    }

    pub fn set_rect(&mut self, rect: impl AsRef<Rect>) {
        unsafe { sb::C_SkRRect_setRect(self.native_mut(), rect.as_ref().native()) }
    }

    pub fn new_empty() -> Self {
        Self::new()
    }

    // TODO: consider to rename all the following new_* function to from_* functions?
    //       is it possible to find a proper convention here (new_ vs from_?)?

    pub fn new_rect(rect: impl AsRef<Rect>) -> Self {
        let mut rr = Self::default();
        rr.set_rect(rect);
        rr
    }

    pub fn new_oval(oval: impl AsRef<Rect>) -> Self {
        let mut rr = Self::default();
        rr.set_oval(oval);
        rr
    }

    pub fn new_rect_xy(rect: impl AsRef<Rect>, x_rad: scalar, y_rad: scalar) -> Self {
        let mut rr = Self::default();
        rr.set_rect_xy(rect.as_ref(), x_rad, y_rad);
        rr
    }

    pub fn new_rect_radii(rect: impl AsRef<Rect>, radii: &[Vector; 4]) -> Self {
        let mut rr = Self::default();
        rr.set_rect_radii(rect, radii);
        rr
    }

    pub fn new_nine_patch(
        rect: impl AsRef<Rect>,
        left_rad: scalar,
        top_rad: scalar,
        right_rad: scalar,
        bottom_rad: scalar,
    ) -> Self {
        let mut r = Self::default();
        r.set_nine_patch(rect, left_rad, top_rad, right_rad, bottom_rad);
        r
    }

    pub fn set_oval(&mut self, oval: impl AsRef<Rect>) {
        unsafe { self.native_mut().setOval(oval.as_ref().native()) }
    }

    pub fn set_rect_xy(&mut self, rect: impl AsRef<Rect>, x_rad: scalar, y_rad: scalar) {
        unsafe {
            self.native_mut()
                .setRectXY(rect.as_ref().native(), x_rad, y_rad)
        }
    }

    pub fn set_nine_patch(
        &mut self,
        rect: impl AsRef<Rect>,
        left_rad: scalar,
        top_rad: scalar,
        right_rad: scalar,
        bottom_rad: scalar,
    ) {
        unsafe {
            self.native_mut().setNinePatch(
                rect.as_ref().native(),
                left_rad,
                top_rad,
                right_rad,
                bottom_rad,
            )
        }
    }

    pub fn set_rect_radii(&mut self, rect: impl AsRef<Rect>, radii: &[Vector; 4]) {
        unsafe {
            self.native_mut()
                .setRectRadii(rect.as_ref().native(), radii.native().as_ptr())
        }
    }

    pub fn rect(&self) -> &Rect {
        Rect::from_native_ref(&self.native().fRect)
    }

    pub fn radii(&self, corner: Corner) -> Vector {
        Vector::from_native_c(self.native().fRadii[corner as usize])
    }

    pub fn radii_ref(&self) -> &[Vector; 4] {
        Vector::from_native_array_ref(&self.native().fRadii)
    }

    pub fn bounds(&self) -> &Rect {
        self.rect()
    }

    pub fn inset(&mut self, delta: impl Into<Vector>) {
        *self = self.with_inset(delta)
    }

    #[must_use]
    pub fn with_inset(&self, delta: impl Into<Vector>) -> Self {
        let delta = delta.into();
        let mut r = Self::default();
        unsafe { self.native().inset(delta.x, delta.y, r.native_mut()) };
        r
    }

    pub fn outset(&mut self, delta: impl Into<Vector>) {
        *self = self.with_outset(delta)
    }

    #[must_use]
    pub fn with_outset(&self, delta: impl Into<Vector>) -> Self {
        self.with_inset(-delta.into())
    }

    pub fn offset(&mut self, delta: impl Into<Vector>) {
        Rect::from_native_ref_mut(&mut self.native_mut().fRect).offset(delta)
    }

    #[must_use]
    pub fn with_offset(&self, delta: impl Into<Vector>) -> Self {
        let mut copied = *self;
        copied.offset(delta);
        copied
    }

    pub fn contains(&self, rect: impl AsRef<Rect>) -> bool {
        unsafe { self.native().contains(rect.as_ref().native()) }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { self.native().isValid() }
    }

    pub const SIZE_IN_MEMORY: usize = mem::size_of::<Self>();

    pub fn write_to_memory(&self, buffer: &mut Vec<u8>) {
        unsafe {
            let size = self.native().writeToMemory(ptr::null_mut());
            buffer.resize(size, 0);
            let written = self.native().writeToMemory(buffer.as_mut_ptr() as _);
            debug_assert_eq!(written, size);
        }
    }

    pub fn read_from_memory(&mut self, buffer: &[u8]) -> usize {
        unsafe {
            self.native_mut()
                .readFromMemory(buffer.as_ptr() as _, buffer.len())
        }
    }

    #[must_use]
    pub fn transform(&self, matrix: &Matrix) -> Option<Self> {
        let mut r = Self::default();
        unsafe { self.native().transform1(matrix.native(), r.native_mut()) }.then_some(r)
    }

    pub fn dump(&self, as_hex: impl Into<Option<bool>>) {
        unsafe { self.native().dump(as_hex.into().unwrap_or_default()) }
    }

    pub fn dump_to_string(&self, as_hex: bool) -> String {
        let mut str = interop::String::default();
        unsafe { sb::C_SkRRect_dumpToString(self.native(), as_hex, str.native_mut()) }
        str.to_string()
    }

    pub fn dump_hex(&self) {
        self.dump(true)
    }
}
