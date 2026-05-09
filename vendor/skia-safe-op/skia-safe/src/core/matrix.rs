use std::{
    ops::{Index, IndexMut, Mul},
    slice,
};

use super::scalar_;
use crate::{prelude::*, scalar, Point, Point3, RSXform, Rect, Scalar, Size, Vector};
use skia_bindings::{self as sb, SkMatrix};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Matrix {
    mat: [scalar; 9usize],
    type_mask: u32,
}

native_transmutable!(SkMatrix, Matrix);

impl PartialEq for Matrix {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkMatrix_Equals(self.native(), rhs.native()) }
    }
}

impl Mul for Matrix {
    type Output = Self;
    fn mul(self, rhs: Matrix) -> Self::Output {
        Matrix::concat(&self, &rhs)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Member {
    ScaleX = 0,
    SkewX = 1,
    TransX = 2,
    SkewY = 3,
    ScaleY = 4,
    TransY = 5,
    Persp0 = 6,
    Persp1 = 7,
    Persp2 = 8,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AffineMember {
    ScaleX = 0,
    SkewY = 1,
    SkewX = 2,
    ScaleY = 3,
    TransX = 4,
    TransY = 5,
}

impl Index<Member> for Matrix {
    type Output = scalar;

    fn index(&self, index: Member) -> &Self::Output {
        &self[index as usize]
    }
}

impl Index<AffineMember> for Matrix {
    type Output = scalar;

    fn index(&self, index: AffineMember) -> &Self::Output {
        &self[index as usize]
    }
}

impl Index<usize> for Matrix {
    type Output = scalar;

    fn index(&self, index: usize) -> &Self::Output {
        &self.native().fMat[index]
    }
}

impl IndexMut<Member> for Matrix {
    fn index_mut(&mut self, index: Member) -> &mut Self::Output {
        self.index_mut(index as usize)
    }
}

impl IndexMut<AffineMember> for Matrix {
    fn index_mut(&mut self, index: AffineMember) -> &mut Self::Output {
        self.index_mut(index as usize)
    }
}

impl IndexMut<usize> for Matrix {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut *sb::C_SkMatrix_SubscriptMut(self.native_mut(), index) }
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::new()
    }
}

impl Matrix {
    const fn new() -> Self {
        Self {
            mat: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            type_mask: TypeMask::IDENTITY.bits() | 0x10,
        }
    }

    #[must_use]
    pub fn scale((sx, sy): (scalar, scalar)) -> Self {
        let mut m = Self::new();
        m.set_scale((sx, sy), None);
        m
    }

    #[must_use]
    pub fn translate(d: impl Into<Vector>) -> Self {
        let mut m = Self::new();
        m.set_translate(d);
        m
    }

    #[must_use]
    pub fn scale_translate((sx, sy): (scalar, scalar), t: impl Into<Vector>) -> Self {
        let t = t.into();
        Self::construct(|m| unsafe { sb::C_SkMatrix_ScaleTranslate(sx, sy, t.x, t.y, m) })
    }

    #[must_use]
    pub fn rotate_deg(deg: scalar) -> Self {
        let mut m = Self::new();
        m.set_rotate(deg, None);
        m
    }

    #[must_use]
    pub fn rotate_deg_pivot(deg: scalar, pivot: impl Into<Point>) -> Self {
        let mut m = Self::new();
        m.set_rotate(deg, pivot.into());
        m
    }

    #[must_use]
    pub fn rotate_rad(rad: scalar) -> Self {
        Self::rotate_deg(scalar_::radians_to_degrees(rad))
    }

    #[must_use]
    pub fn skew((kx, ky): (scalar, scalar)) -> Self {
        let mut m = Self::new();
        m.set_skew((kx, ky), None);
        m
    }
}

pub type ScaleToFit = skia_bindings::SkMatrix_ScaleToFit;
variant_name!(ScaleToFit::Fill);

impl Matrix {
    #[deprecated(since = "0.89.0", note = "Use rect_2_rect")]
    #[must_use]
    pub fn rect_to_rect(
        src: impl AsRef<Rect>,
        dst: impl AsRef<Rect>,
        scale_to_fit: impl Into<Option<ScaleToFit>>,
    ) -> Option<Self> {
        Self::rect_2_rect(src, dst, scale_to_fit.into().unwrap_or(ScaleToFit::Fill))
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_all(
        scale_x: scalar,
        skew_x: scalar,
        trans_x: scalar,
        skew_y: scalar,
        scale_y: scalar,
        trans_y: scalar,
        pers_0: scalar,
        pers_1: scalar,
        pers_2: scalar,
    ) -> Self {
        let mut m = Self::new();
        m.set_all(
            scale_x, skew_x, trans_x, skew_y, scale_y, trans_y, pers_0, pers_1, pers_2,
        );
        m
    }
}

bitflags! {
    // m85: On Windows the SkMatrix_TypeMask is defined as i32,
    // but we stick to u32 (macOS / Linux), because there is no need to leak
    // the platform difference to the Rust side.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TypeMask: u32 {
        const IDENTITY = sb::SkMatrix_TypeMask_kIdentity_Mask as _;
        const TRANSLATE = sb::SkMatrix_TypeMask_kTranslate_Mask as _;
        const SCALE = sb::SkMatrix_TypeMask_kScale_Mask as _;
        const AFFINE = sb::SkMatrix_TypeMask_kAffine_Mask as _;
        const PERSPECTIVE = sb::SkMatrix_TypeMask_kPerspective_Mask as _;
    }
}

impl TypeMask {
    const UNKNOWN: u32 = sb::SkMatrix_kUnknown_Mask as _;
}

impl Matrix {
    pub fn get_type(&self) -> TypeMask {
        TypeMask::from_bits_truncate(unsafe { sb::C_SkMatrix_getType(self.native()) } as _)
    }

    pub fn is_identity(&self) -> bool {
        self.get_type() == TypeMask::IDENTITY
    }

    pub fn is_scale_translate(&self) -> bool {
        (self.get_type() & !(TypeMask::SCALE | TypeMask::TRANSLATE)).is_empty()
    }

    pub fn is_translate(&self) -> bool {
        (self.get_type() & !TypeMask::TRANSLATE).is_empty()
    }

    pub fn rect_stays_rect(&self) -> bool {
        unsafe { sb::C_SkMatrix_rectStaysRect(self.native()) }
    }

    pub fn preserves_axis_alignment(&self) -> bool {
        self.rect_stays_rect()
    }

    pub fn has_perspective(&self) -> bool {
        unsafe { sb::C_SkMatrix_hasPerspective(self.native()) }
    }

    pub fn is_similarity(&self) -> bool {
        unsafe { self.native().isSimilarity(scalar::NEARLY_ZERO) }
    }

    pub fn preserves_right_angles(&self) -> bool {
        unsafe { self.native().preservesRightAngles(scalar::NEARLY_ZERO) }
    }

    pub fn rc(&self, r: usize, c: usize) -> scalar {
        assert!(r <= 2);
        assert!(c <= 2);
        self[r * 3 + c]
    }

    pub fn scale_x(&self) -> scalar {
        self[Member::ScaleX]
    }

    pub fn scale_y(&self) -> scalar {
        self[Member::ScaleY]
    }

    pub fn skew_y(&self) -> scalar {
        self[Member::SkewY]
    }

    pub fn skew_x(&self) -> scalar {
        self[Member::SkewX]
    }

    pub fn translate_x(&self) -> scalar {
        self[Member::TransX]
    }

    pub fn translate_y(&self) -> scalar {
        self[Member::TransY]
    }

    pub fn persp_x(&self) -> scalar {
        self[Member::Persp0]
    }

    pub fn persp_y(&self) -> scalar {
        self[Member::Persp1]
    }

    pub fn set_scale_x(&mut self, v: scalar) -> &mut Self {
        self.set(Member::ScaleX, v)
    }

    pub fn set_scale_y(&mut self, v: scalar) -> &mut Self {
        self.set(Member::ScaleY, v)
    }

    pub fn set_skew_y(&mut self, v: scalar) -> &mut Self {
        self.set(Member::SkewY, v)
    }

    pub fn set_skew_x(&mut self, v: scalar) -> &mut Self {
        self.set(Member::SkewX, v)
    }

    pub fn set_translate_x(&mut self, v: scalar) -> &mut Self {
        self.set(Member::TransX, v)
    }

    pub fn set_translate_y(&mut self, v: scalar) -> &mut Self {
        self.set(Member::TransY, v)
    }

    pub fn set_persp_x(&mut self, v: scalar) -> &mut Self {
        self.set(Member::Persp0, v)
    }

    pub fn set_persp_y(&mut self, v: scalar) -> &mut Self {
        self.set(Member::Persp1, v)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_all(
        &mut self,
        scale_x: scalar,
        skew_x: scalar,
        trans_x: scalar,
        skew_y: scalar,
        scale_y: scalar,
        trans_y: scalar,
        persp_0: scalar,
        persp_1: scalar,
        persp_2: scalar,
    ) -> &mut Self {
        self[Member::ScaleX] = scale_x;
        self[Member::SkewX] = skew_x;
        self[Member::TransX] = trans_x;
        self[Member::SkewY] = skew_y;
        self[Member::ScaleY] = scale_y;
        self[Member::TransY] = trans_y;
        self[Member::Persp0] = persp_0;
        self[Member::Persp1] = persp_1;
        self[Member::Persp2] = persp_2;
        self.type_mask = TypeMask::UNKNOWN;
        self
    }

    pub fn get_9(&self, buffer: &mut [scalar; 9]) {
        buffer.copy_from_slice(&self.mat)
    }

    pub fn set_9(&mut self, buffer: &[scalar; 9]) -> &mut Self {
        unsafe {
            self.native_mut().set9(buffer.as_ptr());
        }
        self
    }

    pub fn reset(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().reset();
        }
        self
    }

    pub fn set_identity(&mut self) -> &mut Self {
        self.reset();
        self
    }

    pub fn set_translate(&mut self, v: impl Into<Vector>) -> &mut Self {
        let v = v.into();
        unsafe {
            self.native_mut().setTranslate(v.x, v.y);
        }
        self
    }

    pub fn set_scale(
        &mut self,
        (sx, sy): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().setScale(sx, sy, pivot.x, pivot.y);
        }
        self
    }

    pub fn set_rotate(&mut self, degrees: scalar, pivot: impl Into<Option<Point>>) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().setRotate(degrees, pivot.x, pivot.y);
        }
        self
    }

    pub fn set_sin_cos(
        &mut self,
        (sin_value, cos_value): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut()
                .setSinCos(sin_value, cos_value, pivot.x, pivot.y);
        }
        self
    }

    pub fn set_rsxform(&mut self, rsxform: &RSXform) -> &mut Self {
        unsafe {
            self.native_mut().setRSXform(rsxform.native());
        }
        self
    }

    pub fn set_skew(
        &mut self,
        (kx, ky): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().setSkew(kx, ky, pivot.x, pivot.y);
        }
        self
    }

    pub fn set_concat(&mut self, a: &Self, b: &Self) -> &mut Self {
        unsafe {
            self.native_mut().setConcat(a.native(), b.native());
        }
        self
    }

    pub fn pre_translate(&mut self, delta: impl Into<Vector>) -> &mut Self {
        let delta = delta.into();
        unsafe {
            self.native_mut().preTranslate(delta.x, delta.y);
        }
        self
    }

    pub fn pre_scale(
        &mut self,
        (sx, sy): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().preScale(sx, sy, pivot.x, pivot.y);
        }
        self
    }

    pub fn pre_rotate(&mut self, degrees: scalar, pivot: impl Into<Option<Point>>) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().preRotate(degrees, pivot.x, pivot.y);
        }
        self
    }

    pub fn pre_skew(
        &mut self,
        (kx, ky): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().preSkew(kx, ky, pivot.x, pivot.y);
        }
        self
    }

    pub fn pre_concat(&mut self, other: &Self) -> &mut Self {
        unsafe {
            self.native_mut().preConcat(other.native());
        }
        self
    }

    pub fn post_translate(&mut self, delta: impl Into<Vector>) -> &mut Self {
        let delta = delta.into();
        unsafe {
            self.native_mut().postTranslate(delta.x, delta.y);
        }
        self
    }

    pub fn post_scale(
        &mut self,
        (sx, sy): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().postScale(sx, sy, pivot.x, pivot.y);
        }
        self
    }

    #[deprecated(
        since = "0.27.0",
        note = "use post_scale((1.0 / x as scalar, 1.0 / y as scalar), None)"
    )]
    pub fn post_idiv(&mut self, (div_x, div_y): (i32, i32)) -> bool {
        if div_x == 0 || div_y == 0 {
            return false;
        }
        self.post_scale((1.0 / div_x as scalar, 1.0 / div_y as scalar), None);
        true
    }

    pub fn post_rotate(&mut self, degrees: scalar, pivot: impl Into<Option<Point>>) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().postRotate(degrees, pivot.x, pivot.y);
        }
        self
    }

    pub fn post_skew(
        &mut self,
        (kx, ky): (scalar, scalar),
        pivot: impl Into<Option<Point>>,
    ) -> &mut Self {
        let pivot = pivot.into().unwrap_or_default();
        unsafe {
            self.native_mut().postSkew(kx, ky, pivot.x, pivot.y);
        }
        self
    }

    pub fn post_concat(&mut self, other: &Matrix) -> &mut Self {
        unsafe {
            self.native_mut().postConcat(other.native());
        }
        self
    }

    #[deprecated(since = "0.88.0", note = "Use rect_2_rect")]
    pub fn from_rect_to_rect(
        src: impl AsRef<Rect>,
        dst: impl AsRef<Rect>,
        stf: ScaleToFit,
    ) -> Option<Self> {
        Self::rect_2_rect(src, dst, stf)
    }

    pub fn rect_2_rect(
        src: impl AsRef<Rect>,
        dst: impl AsRef<Rect>,
        stf: impl Into<Option<ScaleToFit>>,
    ) -> Option<Self> {
        let mut m = Self::new_identity();
        unsafe {
            sb::C_SkMatrix_Rect2Rect(
                src.as_ref().native(),
                dst.as_ref().native(),
                stf.into().unwrap_or(ScaleToFit::Fill),
                m.native_mut(),
            )
        }
        .then_some(m)
    }

    pub fn rect_to_rect_or_identity(
        src: impl AsRef<Rect>,
        dst: impl AsRef<Rect>,
        stf: impl Into<Option<ScaleToFit>>,
    ) -> Self {
        Self::rect_2_rect(src, dst, stf).unwrap_or(Matrix::new_identity())
    }

    pub fn poly_to_poly(src: &[Point], dst: &[Point]) -> Option<Matrix> {
        let mut m = Matrix::new();
        m.set_poly_to_poly(src, dst).then_some(m)
    }

    pub fn set_poly_to_poly(&mut self, src: &[Point], dst: &[Point]) -> bool {
        unsafe {
            sb::C_SkMatrix_setPolyToPoly(
                self.native_mut(),
                src.native().as_ptr(),
                src.len(),
                dst.native().as_ptr(),
                dst.len(),
            )
        }
    }

    pub fn from_poly_to_poly(src: &[Point], dst: &[Point]) -> Option<Matrix> {
        let mut m = Matrix::new_identity();
        m.set_poly_to_poly(src, dst).then_some(m)
    }

    pub fn invert(&self) -> Option<Matrix> {
        let mut m = Matrix::new_identity();
        unsafe { sb::C_SkMatrix_invert(self.native(), m.native_mut()) }.then_some(m)
    }

    pub fn set_affine_identity(affine: &mut [scalar; 6]) {
        unsafe { SkMatrix::SetAffineIdentity(affine.as_mut_ptr()) }
    }

    #[must_use]
    pub fn to_affine(self) -> Option<[scalar; 6]> {
        let mut affine = [scalar::default(); 6];
        unsafe { self.native().asAffine(affine.as_mut_ptr()) }.then_some(affine)
    }

    pub fn set_affine(&mut self, affine: &[scalar; 6]) -> &mut Self {
        unsafe { self.native_mut().setAffine(affine.as_ptr()) };
        self
    }

    pub fn from_affine(affine: &[scalar; 6]) -> Matrix {
        let mut m = Matrix::new_identity();
        unsafe {
            m.native_mut().setAffine(affine.as_ptr());
        }
        m
    }

    pub fn normalize_perspective(&mut self) {
        unsafe { sb::C_SkMatrix_normalizePerspective(self.native_mut()) }
    }

    pub fn map_points(&self, dst: &mut [Point], src: &[Point]) {
        assert!(dst.len() >= src.len());

        unsafe {
            sb::C_SkMatrix_mapPoints(
                self.native(),
                dst.native_mut().as_mut_ptr(),
                src.native().as_ptr(),
                src.len(),
            )
        };
    }

    pub fn map_points_inplace(&self, pts: &mut [Point]) {
        let ptr = pts.native_mut().as_mut_ptr();
        unsafe { sb::C_SkMatrix_mapPoints(self.native(), ptr, ptr, pts.len()) };
    }

    pub fn map_homogeneous_points(&self, dst: &mut [Point3], src: &[Point3]) {
        assert!(dst.len() >= src.len());

        unsafe {
            sb::C_SkMatrix_mapHomogeneousPoints(
                self.native(),
                dst.native_mut().as_mut_ptr(),
                src.native().as_ptr(),
                src.len(),
            )
        };
    }

    pub fn map_homogeneous_point(&self, src: Point3) -> Point3 {
        let mut dst = Point3::default();
        self.map_homogeneous_points(slice::from_mut(&mut dst), &[src]);
        dst
    }

    #[deprecated(since = "0.88.0", note = "use map_points_to_homogeneous()")]
    pub fn map_homogeneous_points_2d(&self, dst: &mut [Point3], src: &[Point]) {
        self.map_points_to_homogeneous(dst, src);
    }

    pub fn map_points_to_homogeneous(&self, dst: &mut [Point3], src: &[Point]) {
        assert!(dst.len() >= src.len());

        unsafe {
            sb::C_SkMatrix_mapPointsToHomogeneous(
                self.native(),
                dst.native_mut().as_mut_ptr(),
                src.native().as_ptr(),
                src.len(),
            )
        };
    }

    pub fn map_point_to_homogeneous(&self, src: Point) -> Point3 {
        let mut dst = Point3::default();
        self.map_points_to_homogeneous(slice::from_mut(&mut dst), &[src]);
        dst
    }

    #[deprecated(since = "0.88.0", note = "use map_point((x, y))")]
    pub fn map_xy(&self, x: scalar, y: scalar) -> Point {
        self.map_point((x, y))
    }

    pub fn map_point(&self, point: impl Into<Point>) -> Point {
        Point::from_native_c(unsafe {
            sb::C_SkMatrix_mapPoint(self.native(), point.into().into_native())
        })
    }

    pub fn map_point_affine(&self, point: impl Into<Point>) -> Point {
        Point::from_native_c(unsafe {
            sb::C_SkMatrix_mapPointAffine(self.native(), point.into().into_native())
        })
    }

    pub fn map_origin(&self) -> Point {
        let mut x = self.translate_x();
        let mut y = self.translate_y();
        if self.has_perspective() {
            let mut w = self[Member::Persp2];
            if w != 0.0 {
                w = 1.0 / w;
            }
            x *= w;
            y *= w;
        }
        Point::new(x, y)
    }

    pub fn map_vectors(&self, dst: &mut [Vector], src: &[Vector]) {
        assert!(dst.len() >= src.len());
        unsafe {
            sb::C_SkMatrix_mapVectors(
                self.native(),
                dst.native_mut().as_mut_ptr(),
                src.native().as_ptr(),
                src.len(),
            )
        }
    }

    pub fn map_vectors_inplace(&self, vecs: &mut [Vector]) {
        let ptr = vecs.native_mut().as_mut_ptr();
        unsafe { sb::C_SkMatrix_mapVectors(self.native(), ptr, ptr, vecs.len()) }
    }

    pub fn map_vector(&self, vec: impl Into<Vector>) -> Vector {
        let mut vec = vec.into();
        self.map_vectors_inplace(slice::from_mut(&mut vec));
        vec
    }

    pub fn map_rect(&self, src: impl AsRef<Rect>) -> (Rect, bool) {
        let mut dst = Rect::default();
        let rect_stays_rect = unsafe {
            self.native()
                .mapRect(dst.native_mut(), src.as_ref().native())
        };
        (dst, rect_stays_rect)
    }

    pub fn map_rect_to_quad(&self, rect: impl AsRef<Rect>) -> [Point; 4] {
        let mut quad = rect.as_ref().to_quad(None);
        self.map_points_inplace(quad.as_mut());
        quad
    }

    pub fn map_rect_scale_translate(&self, src: impl AsRef<Rect>) -> Option<Rect> {
        if self.is_scale_translate() {
            let mut rect = Rect::default();
            unsafe {
                self.native()
                    .mapRectScaleTranslate(rect.native_mut(), src.as_ref().native())
            };
            Some(rect)
        } else {
            None
        }
    }

    pub fn map_radius(&self, radius: scalar) -> Option<scalar> {
        if !self.has_perspective() {
            Some(unsafe { self.native().mapRadius(radius) })
        } else {
            None
        }
    }

    #[deprecated(since = "0.27.0", note = "removed without replacement")]
    pub fn is_fixed_step_in_x(&self) -> ! {
        unimplemented!("removed without replacement")
    }

    #[deprecated(since = "0.27.0", note = "removed without replacement")]
    pub fn fixed_step_in_x(&self, _y: scalar) -> ! {
        unimplemented!("removed without replacement")
    }

    #[deprecated(since = "0.27.0", note = "removed without replacement")]
    pub fn cheap_equal_to(&self, _other: &Matrix) -> ! {
        unimplemented!("removed without replacement")
    }

    pub fn dump(&self) {
        unsafe { self.native().dump() }
    }

    pub fn min_scale(&self) -> scalar {
        unsafe { self.native().getMinScale() }
    }

    pub fn max_scale(&self) -> scalar {
        unsafe { self.native().getMaxScale() }
    }

    #[must_use]
    pub fn min_max_scales(&self) -> (scalar, scalar) {
        let mut r: [scalar; 2] = Default::default();
        unsafe { self.native().getMinMaxScales(r.as_mut_ptr()) };
        #[allow(clippy::tuple_array_conversions)]
        (r[0], r[1])
    }

    pub fn decompose_scale(&self, mut remaining: Option<&mut Matrix>) -> Option<Size> {
        let mut size = Size::default();
        unsafe {
            self.native()
                .decomposeScale(size.native_mut(), remaining.native_ptr_or_null_mut())
        }
        .then_some(size)
    }

    pub fn i() -> &'static Matrix {
        &IDENTITY
    }

    pub fn invalid_matrix() -> &'static Matrix {
        Self::from_native_ref(unsafe { &*sb::C_SkMatrix_InvalidMatrix() })
    }

    pub fn concat(a: &Matrix, b: &Matrix) -> Matrix {
        let mut m = Matrix::new_identity();
        unsafe { m.native_mut().setConcat(a.native(), b.native()) };
        m
    }

    pub fn dirty_matrix_type_cache(&mut self) {
        self.native_mut().fTypeMask = 0x80;
    }

    pub fn set_scale_translate(&mut self, s: (scalar, scalar), t: impl Into<Vector>) -> &mut Self {
        *self = Self::scale_translate(s, t);
        self
    }

    pub fn is_finite(&self) -> bool {
        unsafe { sb::C_SkMatrix_isFinite(self.native()) }
    }

    pub const fn new_identity() -> Self {
        Self::new()
    }
}

impl IndexGet for Matrix {}
impl IndexSet for Matrix {}

pub const IDENTITY: Matrix = Matrix::new_identity();

#[cfg(test)]
mod tests {
    use super::{AffineMember, Matrix, TypeMask};
    use crate::prelude::*;

    #[test]
    fn test_get_set_trait_compilation() {
        let mut m = Matrix::new_identity();
        let _x = m.get(AffineMember::ScaleX);
        m.set(AffineMember::ScaleX, 1.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_tuple_to_vector() {
        let mut m = Matrix::new_identity();
        m.set_translate((10.0, 11.0));
        assert_eq!(10.0, m.translate_x());
        assert_eq!(11.0, m.translate_y());
    }

    #[test]
    fn setting_a_matrix_component_recomputes_typemask() {
        let mut m = Matrix::default();
        assert_eq!(TypeMask::IDENTITY, m.get_type());
        m.set_persp_x(0.1);
        assert_eq!(
            TypeMask::TRANSLATE | TypeMask::SCALE | TypeMask::AFFINE | TypeMask::PERSPECTIVE,
            m.get_type()
        );
    }
}
