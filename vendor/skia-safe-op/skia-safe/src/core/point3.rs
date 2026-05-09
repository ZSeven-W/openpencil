use crate::{prelude::*, private::is_finite, scalar};
use skia_bindings::SkPoint3;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

pub type Vector3 = Point3;
pub type Color3f = Point3;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Point3 {
    pub x: scalar,
    pub y: scalar,
    pub z: scalar,
}

native_transmutable!(SkPoint3, Point3);

impl From<(scalar, scalar, scalar)> for Point3 {
    fn from((x, y, z): (scalar, scalar, scalar)) -> Self {
        Self::new(x, y, z)
    }
}

impl Neg for Point3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Add for Point3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Point3 {
    fn add_assign(&mut self, rhs: Point3) {
        *self = *self + rhs;
    }
}

impl Sub for Point3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Point3 {
    fn sub_assign(&mut self, rhs: Point3) {
        *self = *self - rhs;
    }
}

impl Mul<Point3> for scalar {
    type Output = Point3;

    fn mul(self, p: Point3) -> Self::Output {
        Point3::new(self * p.x, self * p.y, self * p.z)
    }
}

impl Point3 {
    pub const fn new(x: scalar, y: scalar, z: scalar) -> Self {
        Self { x, y, z }
    }

    pub fn set(&mut self, x: scalar, y: scalar, z: scalar) {
        *self = Self::new(x, y, z);
    }

    pub fn length_xyz(x: scalar, y: scalar, z: scalar) -> scalar {
        unsafe { SkPoint3::Length(x, y, z) }
    }

    pub fn length(&self) -> scalar {
        unsafe { SkPoint3::Length(self.x, self.y, self.z) }
    }

    pub fn normalize(&mut self) -> bool {
        unsafe { self.native_mut().normalize() }
    }

    #[must_use]
    pub fn normalized(&self) -> Option<Self> {
        let mut normalized = *self;
        unsafe { normalized.native_mut().normalize() }.then_some(normalized)
    }

    // TODO: with_scale()?
    #[must_use]
    pub fn scaled(&self, scale: scalar) -> Self {
        Self::new(scale * self.x, scale * self.y, scale * self.z)
    }

    pub fn scale(&mut self, value: scalar) {
        *self = self.scaled(value);
    }

    pub fn is_finite(&self) -> bool {
        is_finite(&[self.x, self.y, self.z])
    }

    pub fn dot_product(a: Self, b: Self) -> scalar {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    pub fn dot(&self, vec: Self) -> scalar {
        Self::dot_product(*self, vec)
    }

    #[allow(clippy::many_single_char_names)]
    pub fn cross_product(a: Self, b: Self) -> Self {
        let x = a.y * b.z - a.z * b.y;
        let y = a.z * b.x - a.x * b.z;
        let z = a.x * b.y - a.y * b.x;
        Self { x, y, z }
    }

    #[must_use]
    pub fn cross(&self, vec: Self) -> Self {
        Self::cross_product(*self, vec)
    }
}
