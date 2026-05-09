use crate::{prelude::*, scalar};
use skia_bindings::{self as sb, SkISize, SkSize};
use std::ops::{Div, DivAssign, Mul, MulAssign};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ISize {
    pub width: i32,
    pub height: i32,
}

native_transmutable!(SkISize, ISize);

impl ISize {
    pub const fn new(w: i32, h: i32) -> ISize {
        ISize {
            width: w,
            height: h,
        }
    }

    pub const fn new_empty() -> ISize {
        Self::new(0, 0)
    }

    pub fn set(&mut self, w: i32, h: i32) {
        *self = Self::new(w, h);
    }

    pub fn is_zero(self) -> bool {
        self.width == 0 && self.height == 0
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub fn set_empty(&mut self) {
        *self = Self::new_empty();
    }

    pub const fn area(self) -> i64 {
        self.width as i64 * self.height as i64
    }

    // TODO: should the functions with() and height() be supported?

    pub fn equals(self, w: i32, h: i32) -> bool {
        self == Self::new(w, h)
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Size {
    pub width: scalar,
    pub height: scalar,
}

native_transmutable!(SkSize, Size);

impl Size {
    pub const fn new(w: scalar, h: scalar) -> Size {
        Size {
            width: w,
            height: h,
        }
    }

    pub const fn from_isize(src: ISize) -> Size {
        Self::new(src.width as _, src.height as _)
    }

    pub const fn new_empty() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn set(&mut self, w: scalar, h: scalar) {
        *self = Self::new(w, h);
    }

    pub fn is_zero(self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn set_empty(&mut self) {
        *self = Self::new_empty()
    }

    // TODO: should width() and height() be supported?

    pub fn equals(self, w: scalar, h: scalar) -> bool {
        self == Self::new(w, h)
    }

    pub fn to_round(self) -> ISize {
        ISize::from_native_c(unsafe { sb::C_SkSize_toRound(self.native()) })
    }

    pub fn to_ceil(self) -> ISize {
        ISize::from_native_c(unsafe { sb::C_SkSize_toCeil(self.native()) })
    }

    pub fn to_floor(self) -> ISize {
        ISize::from_native_c(unsafe { sb::C_SkSize_toFloor(self.native()) })
    }
}

//
// From
//

impl From<(i32, i32)> for ISize {
    fn from(source: (i32, i32)) -> Self {
        Self::new(source.0, source.1)
    }
}

impl From<(scalar, scalar)> for Size {
    fn from(source: (scalar, scalar)) -> Self {
        Self::new(source.0, source.1)
    }
}

impl From<ISize> for Size {
    fn from(size: ISize) -> Self {
        Self::new(size.width as _, size.height as _)
    }
}

// TODO: this is experimental.
impl From<(i32, i32)> for Size {
    fn from(source: (i32, i32)) -> Self {
        (source.0 as scalar, source.1 as scalar).into()
    }
}

impl Div<scalar> for Size {
    type Output = Self;
    fn div(self, rhs: scalar) -> Self {
        Self::new(self.width / rhs, self.height / rhs)
    }
}

impl DivAssign<scalar> for Size {
    fn div_assign(&mut self, rhs: scalar) {
        *self = *self / rhs
    }
}

impl Mul<scalar> for Size {
    type Output = Self;
    fn mul(self, rhs: scalar) -> Self {
        Self::new(self.width * rhs, self.height * rhs)
    }
}

impl MulAssign<scalar> for Size {
    fn mul_assign(&mut self, rhs: scalar) {
        *self = *self * rhs
    }
}
