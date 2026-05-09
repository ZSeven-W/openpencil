use std::{
    f32,
    ops::{Add, AddAssign, Div, DivAssign, Index, Mul, MulAssign, Neg, Sub, SubAssign},
    slice,
};

use skia_bindings::{self as sb, SkM44, SkV2, SkV3, SkV4};

use crate::{prelude::*, private::is_finite, scalar, Matrix, Rect};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

native_transmutable!(SkV2, V2);

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, b: Self) -> scalar {
        self.x * b.x + self.y * b.y
    }

    pub fn cross(self, b: Self) -> scalar {
        self.x * b.y - self.y * b.x
    }

    #[must_use]
    pub fn normalize(self) -> Self {
        self * (1.0 / self.length())
    }

    pub fn length_squared(self) -> scalar {
        Self::dot(self, self)
    }

    pub fn length(self) -> scalar {
        self.length_squared().sqrt()
    }

    const COMPONENTS: usize = 2;

    pub fn as_array(&self) -> &[f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts(&self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }

    pub fn as_mut_array(&mut self) -> &mut [f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts_mut(&mut self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }
}

impl Neg for V2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Add for V2 {
    type Output = Self;

    fn add(self, v: Self) -> Self::Output {
        Self::new(self.x + v.x, self.y + v.y)
    }
}

impl Sub for V2 {
    type Output = Self;

    fn sub(self, v: Self) -> Self::Output {
        Self::new(self.x - v.x, self.y - v.y)
    }
}

impl Mul for V2 {
    type Output = Self;

    fn mul(self, v: Self) -> Self::Output {
        Self::new(self.x * v.x, self.y * v.y)
    }
}

impl Mul<scalar> for V2 {
    type Output = Self;

    fn mul(self, s: scalar) -> Self::Output {
        Self::new(self.x * s, self.y * s)
    }
}

impl Mul<V2> for scalar {
    type Output = V2;

    fn mul(self, v: V2) -> Self::Output {
        V2::new(v.x * self, v.y * self)
    }
}

impl Div<V2> for scalar {
    type Output = V2;

    fn div(self, v: V2) -> Self::Output {
        V2::new(self / v.x, self / v.y)
    }
}

impl Div<scalar> for V2 {
    type Output = V2;
    fn div(self, s: scalar) -> Self::Output {
        V2::new(self.x / s, self.y / s)
    }
}

impl AddAssign for V2 {
    fn add_assign(&mut self, v: Self) {
        *self = *self + v
    }
}

impl SubAssign for V2 {
    fn sub_assign(&mut self, v: Self) {
        *self = *self - v
    }
}

impl MulAssign for V2 {
    fn mul_assign(&mut self, v: Self) {
        *self = *self * v
    }
}

impl MulAssign<scalar> for V2 {
    fn mul_assign(&mut self, s: scalar) {
        *self = *self * s
    }
}

impl DivAssign<scalar> for V2 {
    fn div_assign(&mut self, s: scalar) {
        *self = *self / s
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

native_transmutable!(SkV3, V3);

impl V3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(&self, b: &Self) -> scalar {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    #[must_use]
    pub fn cross(&self, b: &Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }

    #[must_use]
    pub fn normalize(&self) -> Self {
        *self * (1.0 / self.length())
    }

    pub fn length_squared(&self) -> scalar {
        Self::dot(self, self)
    }

    pub fn length(&self) -> scalar {
        Self::dot(self, self).sqrt()
    }

    const COMPONENTS: usize = 3;

    pub fn as_array(&self) -> &[f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts(&self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }

    pub fn as_mut_array(&mut self) -> &mut [f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts_mut(&mut self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }
}

impl Neg for V3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Add for V3 {
    type Output = Self;

    fn add(self, v: Self) -> Self::Output {
        Self::new(self.x + v.x, self.y + v.y, self.z + v.z)
    }
}

impl Sub for V3 {
    type Output = Self;

    fn sub(self, v: Self) -> Self::Output {
        Self::new(self.x - v.x, self.y - v.y, self.z - v.z)
    }
}

impl Mul for V3 {
    type Output = Self;

    fn mul(self, v: Self) -> Self::Output {
        Self::new(self.x * v.x, self.y * v.y, self.z * v.z)
    }
}

impl Mul<scalar> for V3 {
    type Output = Self;

    fn mul(self, s: scalar) -> Self::Output {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<V3> for scalar {
    type Output = V3;

    fn mul(self, v: V3) -> Self::Output {
        V3::new(v.x * self, v.y * self, v.z * self)
    }
}

impl AddAssign for V3 {
    fn add_assign(&mut self, v: Self) {
        *self = *self + v
    }
}

impl SubAssign for V3 {
    fn sub_assign(&mut self, v: Self) {
        *self = *self - v
    }
}

impl MulAssign for V3 {
    fn mul_assign(&mut self, v: Self) {
        *self = *self * v
    }
}

impl MulAssign<scalar> for V3 {
    fn mul_assign(&mut self, s: scalar) {
        *self = *self * s
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct V4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

native_transmutable!(SkV4, V4);

impl V4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn length_squared(&self) -> scalar {
        Self::dot(self, self)
    }

    pub fn length(&self) -> scalar {
        scalar::sqrt(Self::dot(self, self))
    }

    pub fn dot(&self, b: &Self) -> scalar {
        self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w
    }

    pub fn normalize(&self) -> Self {
        (*self) * (1.0 / self.length())
    }

    const COMPONENTS: usize = 4;

    pub fn as_array(&self) -> &[f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts(&self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }

    pub fn as_mut_array(&mut self) -> &mut [f32; Self::COMPONENTS] {
        unsafe { slice::from_raw_parts_mut(&mut self.x, Self::COMPONENTS) }
            .try_into()
            .unwrap()
    }
}

impl Neg for V4 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

impl Add for V4 {
    type Output = Self;

    fn add(self, v: Self) -> Self::Output {
        Self::new(self.x + v.x, self.y + v.y, self.z + v.z, self.w + v.w)
    }
}

impl Sub for V4 {
    type Output = Self;

    fn sub(self, v: Self) -> Self::Output {
        Self::new(self.x - v.x, self.y - v.y, self.z - v.z, self.w - v.w)
    }
}

impl Mul for V4 {
    type Output = Self;

    fn mul(self, v: Self) -> Self::Output {
        Self::new(self.x * v.x, self.y * v.y, self.z * v.z, self.w * v.w)
    }
}

impl Mul<scalar> for V4 {
    type Output = Self;

    fn mul(self, s: scalar) -> Self::Output {
        Self::new(self.x * s, self.y * s, self.z * s, self.w * s)
    }
}

impl Mul<V4> for scalar {
    type Output = V4;

    fn mul(self, v: V4) -> Self::Output {
        V4::new(v.x * self, v.y * self, v.z * self, v.w * self)
    }
}

impl AddAssign for V4 {
    fn add_assign(&mut self, v: Self) {
        *self = *self + v
    }
}

impl SubAssign for V4 {
    fn sub_assign(&mut self, v: Self) {
        *self = *self - v
    }
}

impl MulAssign for V4 {
    fn mul_assign(&mut self, v: Self) {
        *self = *self * v
    }
}

impl MulAssign<scalar> for V4 {
    fn mul_assign(&mut self, s: scalar) {
        *self = *self * s
    }
}

impl Index<usize> for V4 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_array()[index]
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct M44 {
    mat: [f32; Self::COMPONENTS],
}

native_transmutable!(SkM44, M44);

impl Default for M44 {
    fn default() -> Self {
        Self::new_identity()
    }
}

impl PartialEq for M44 {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_SkM44_equals(self.native(), other.native()) }
    }
}

impl M44 {
    const COMPONENTS: usize = 16;

    pub const fn new_identity() -> Self {
        Self {
            mat: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn concat(a: &Self, b: &Self) -> Self {
        let mut m = Self::default();
        m.set_concat(a, b);
        m
    }

    pub const fn nan() -> Self {
        Self {
            mat: [f32::NAN; Self::COMPONENTS],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        m0: scalar,
        m4: scalar,
        m8: scalar,
        m12: scalar,
        m1: scalar,
        m5: scalar,
        m9: scalar,
        m13: scalar,
        m2: scalar,
        m6: scalar,
        m10: scalar,
        m14: scalar,
        m3: scalar,
        m7: scalar,
        m11: scalar,
        m15: scalar,
    ) -> Self {
        Self {
            mat: [
                m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
            ],
        }
    }

    pub fn rows(r0: &V4, r1: &V4, r2: &V4, r3: &V4) -> Self {
        let mut m = Self::default();
        m.set_row(0, r0);
        m.set_row(1, r1);
        m.set_row(2, r2);
        m.set_row(3, r3);
        m
    }

    pub fn cols(c0: &V4, c1: &V4, c2: &V4, c3: &V4) -> Self {
        let mut m = Self::default();
        m.set_col(0, c0);
        m.set_col(1, c1);
        m.set_col(2, c2);
        m.set_col(3, c3);
        m
    }

    pub fn row_major(r: &[scalar; Self::COMPONENTS]) -> Self {
        Self::new(
            r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10], r[11], r[12], r[13],
            r[14], r[15],
        )
    }

    pub fn col_major(c: &[scalar; Self::COMPONENTS]) -> Self {
        Self::new(
            c[0], c[4], c[8], c[12], c[1], c[5], c[9], c[13], c[2], c[6], c[10], c[14], c[3], c[7],
            c[11], c[15],
        )
    }

    pub fn translate(x: scalar, y: scalar, z: scalar) -> Self {
        Self::new(
            1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z, 0.0, 0.0, 0.0, 1.0,
        )
    }

    pub fn scale(x: scalar, y: scalar, z: scalar) -> Self {
        Self::new(
            x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
        )
    }

    pub fn rotate(axis: V3, radians: scalar) -> Self {
        let mut m = Self::default();
        m.set_rotate(axis, radians);
        m
    }

    pub fn rect_to_rect(src: impl AsRef<Rect>, dst: impl AsRef<Rect>) -> Self {
        let (src, dst) = (src.as_ref(), dst.as_ref());
        Self::construct(|m| unsafe { sb::C_SkM44_RectToRect(src.native(), dst.native(), m) })
    }

    pub fn look_at(eye: &V3, center: &V3, up: &V3) -> Self {
        Self::construct(|m| unsafe {
            sb::C_SkM44_LookAt(eye.native(), center.native(), up.native(), m)
        })
    }

    pub fn perspective(near: f32, far: f32, angle: f32) -> Self {
        Self::construct(|m| unsafe { sb::C_SkM44_Perspective(near, far, angle, m) })
    }

    pub fn get_col_major(&self, v: &mut [scalar; Self::COMPONENTS]) {
        v.copy_from_slice(&self.mat)
    }

    pub fn get_row_major(&self, v: &mut [scalar; Self::COMPONENTS]) {
        unsafe { self.native().getRowMajor(v.as_mut_ptr()) }
    }

    #[deprecated(since = "0.30.0", note = "use M44::col_major() instead")]
    pub fn set_col_major(&mut self, v: &[scalar; Self::COMPONENTS]) -> &mut Self {
        *self = Self::col_major(v);
        self
    }

    #[deprecated(since = "0.30.0", note = "use M44::row_major() instead")]
    pub fn set_row_major(&mut self, v: &[scalar; Self::COMPONENTS]) -> &mut Self {
        *self = Self::row_major(v);
        self
    }

    #[allow(clippy::too_many_arguments)]
    #[deprecated(since = "0.30.0", note = "use Self::new() instead")]
    pub fn set_44(
        &mut self,
        m0: scalar,
        m1: scalar,
        m2: scalar,
        m3: scalar,
        m4: scalar,
        m5: scalar,
        m6: scalar,
        m7: scalar,
        m8: scalar,
        m9: scalar,
        m10: scalar,
        m11: scalar,
        m12: scalar,
        m13: scalar,
        m14: scalar,
        m15: scalar,
    ) -> &mut Self {
        *self = Self::new(
            m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15,
        );
        self
    }

    pub fn rc(&self, r: usize, c: usize) -> scalar {
        assert!(r <= 3);
        assert!(c <= 3);
        self.mat[c * 4 + r]
    }

    pub fn set_rc(&mut self, r: usize, c: usize, value: scalar) {
        assert!(r <= 3);
        assert!(c <= 3);
        self.mat[c * 4 + r] = value;
    }

    pub fn row(&self, i: usize) -> V4 {
        assert!(i <= 3);
        V4::new(
            self.mat[i],
            self.mat[i + 4],
            self.mat[i + 8],
            self.mat[i + 12],
        )
    }

    pub fn col(&self, i: usize) -> V4 {
        assert!(i <= 3);
        V4::new(
            self.mat[i * 4],
            self.mat[i * 4 + 1],
            self.mat[i * 4 + 2],
            self.mat[i * 4 + 3],
        )
    }

    pub fn set_row(&mut self, i: usize, v: &V4) {
        assert!(i <= 3);
        self.mat[i] = v.x;
        self.mat[i + 4] = v.y;
        self.mat[i + 8] = v.z;
        self.mat[i + 12] = v.w;
    }

    pub fn set_col(&mut self, i: usize, v: &V4) {
        assert!(i <= 3);
        self.mat[(i * 4)..(i * 4 + V4::COMPONENTS)].copy_from_slice(v.as_array());
    }

    pub fn set_identity(&mut self) -> &mut Self {
        *self = Self {
            mat: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        };
        self
    }

    pub fn set_translate(&mut self, x: scalar, y: scalar, z: scalar) -> &mut Self {
        *self = Self {
            mat: [
                1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z, 0.0, 0.0, 0.0, 1.0,
            ],
        };
        self
    }

    pub fn set_scale(&mut self, x: scalar, y: scalar, z: scalar) -> &mut Self {
        *self = Self {
            mat: [
                x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        };
        self
    }

    pub fn set_rotate_unit_sin_cos(
        &mut self,
        axis: V3,
        sin_angle: scalar,
        cos_angle: scalar,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .setRotateUnitSinCos(axis.into_native(), sin_angle, cos_angle)
        };
        self
    }

    pub fn set_rotate_unit(&mut self, axis: V3, radians: scalar) -> &mut Self {
        self.set_rotate_unit_sin_cos(axis, radians.sin(), radians.cos())
    }

    pub fn set_rotate(&mut self, axis: V3, radians: scalar) -> &mut Self {
        unsafe { self.native_mut().setRotate(axis.into_native(), radians) };
        self
    }

    #[deprecated(since = "0.30.0", note = "use M44::col_major() and M44::set_concat()")]
    pub fn set_concat_16(&mut self, a: &M44, col_major: &[scalar; Self::COMPONENTS]) -> &mut Self {
        self.set_concat(a, &Self::col_major(col_major))
    }

    pub fn set_concat(&mut self, a: &M44, b: &M44) -> &mut Self {
        unsafe {
            self.native_mut().setConcat(a.native(), b.native());
        }
        self
    }

    #[deprecated(since = "0.30.0", note = "use M44::col_major() and M44::pre_concat()")]
    #[allow(deprecated)]
    pub fn pre_concat_16(&mut self, col_major: &[scalar; Self::COMPONENTS]) -> &mut Self {
        self.set_concat_16(&self.clone(), col_major)
    }

    pub fn pre_concat(&mut self, m: &M44) -> &mut Self {
        unsafe {
            let self_ptr = self.native() as *const _;
            self.native_mut().setConcat(self_ptr, m.native());
        }
        self
    }

    pub fn post_concat(&mut self, m: &M44) -> &mut Self {
        unsafe {
            let self_ptr = self.native() as *const _;
            self.native_mut().setConcat(m.native(), self_ptr);
        }
        self
    }

    pub fn normalize_perspective(&mut self) {
        unsafe { self.native_mut().normalizePerspective() }
    }

    pub fn is_finite(&self) -> bool {
        is_finite(&self.mat)
    }

    #[must_use]
    pub fn invert(&self) -> Option<M44> {
        let mut m = Self::default();
        unsafe { self.native().invert(m.native_mut()) }.then_some(m)
    }

    #[must_use]
    pub fn transpose(&self) -> Self {
        Self::construct(|m| unsafe { sb::C_SkM44_transpose(self.native(), m) })
    }

    pub fn dump(&self) {
        unsafe { self.native().dump() }
    }

    pub fn map(&self, x: f32, y: f32, z: f32, w: f32) -> V4 {
        V4::from_native_c(unsafe { sb::C_SkM44_map(self.native(), x, y, z, w) })
    }

    pub fn to_m33(&self) -> Matrix {
        let m = &self.mat;
        Matrix::new_all(m[0], m[4], m[12], m[1], m[5], m[13], m[3], m[7], m[15])
    }

    pub fn pre_translate(
        &mut self,
        x: scalar,
        y: scalar,
        z: impl Into<Option<scalar>>,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .preTranslate(x, y, z.into().unwrap_or(0.0))
        };
        self
    }

    pub fn post_translate(
        &mut self,
        x: scalar,
        y: scalar,
        z: impl Into<Option<scalar>>,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .postTranslate(x, y, z.into().unwrap_or(0.0))
        };
        self
    }

    pub fn pre_scale(&mut self, x: scalar, y: scalar) -> &mut Self {
        unsafe { self.native_mut().preScale(x, y) };
        self
    }

    pub fn pre_scale_xyz(&mut self, x: scalar, y: scalar, z: scalar) -> &mut Self {
        unsafe { self.native_mut().preScale1(x, y, z) };
        self
    }
}

impl Mul for &M44 {
    type Output = M44;

    fn mul(self, m: Self) -> Self::Output {
        M44::concat(self, m)
    }
}

impl Mul<V4> for &M44 {
    type Output = V4;

    fn mul(self, v: V4) -> Self::Output {
        self.map(v.x, v.y, v.z, v.w)
    }
}

impl Mul<V3> for &M44 {
    type Output = V3;

    fn mul(self, v: V3) -> Self::Output {
        let v4 = self.map(v.x, v.y, v.z, 0.0);
        V3::new(v4.x, v4.y, v4.z)
    }
}

impl From<&Matrix> for M44 {
    fn from(src: &Matrix) -> Self {
        use crate::matrix::Member::*;

        Self::new(
            src[ScaleX],
            src[SkewX],
            0.0,
            src[TransX],
            src[SkewY],
            src[ScaleY],
            0.0,
            src[TransY],
            0.0,
            0.0,
            1.0,
            0.0,
            src[Persp0],
            src[Persp1],
            0.0,
            src[Persp2],
        )
    }
}

impl From<Matrix> for M44 {
    fn from(m: Matrix) -> Self {
        M44::from(&m)
    }
}

#[cfg(test)]
mod tests {
    use crate::{matrix, Matrix, Rect, M44};

    #[test]
    pub fn convert_from_matrix_and_back() {
        // taken from skulpin's physics example.
        let vr = Rect::new(-4.5, -4.0, 4.5, 2.0);
        let dst = Rect::new(0.0, 0.0, 1350.0, 900.0);

        #[allow(deprecated)]
        let m = Matrix::from_rect_to_rect(vr, dst, matrix::ScaleToFit::Center).unwrap();
        let m44 = M44::from(m);
        let m3 = m44.to_m33();
        assert_eq!(m, m3);
    }
}
