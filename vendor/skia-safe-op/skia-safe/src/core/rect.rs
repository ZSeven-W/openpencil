use std::{
    cmp::{max, min},
    mem,
};

use crate::{
    interop,
    prelude::*,
    private::{
        is_finite,
        safe32::{sk32, sk64},
    },
    Contains, IPoint, ISize, IVector, PathDirection, Point, Size, Vector,
};
use skia_bindings::{self as sb, SkIRect, SkRect};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IRect {
    /// The x coordinate of the rectangle's left edge.
    pub left: i32,
    /// The y coordinate of the rectangle's top edge.
    pub top: i32,
    /// The x coordinate of the rectangle's right edge.
    pub right: i32,
    /// The y coordinate of the rectangle's bottom edge.
    pub bottom: i32,
}

native_transmutable!(SkIRect, IRect);

impl AsRef<IRect> for IRect {
    fn as_ref(&self) -> &IRect {
        self
    }
}

impl IRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[must_use]
    pub const fn new_empty() -> Self {
        Self::new(0, 0, 0, 0)
    }

    #[must_use]
    pub fn from_wh(w: i32, h: i32) -> Self {
        Self::from_size((w, h))
    }

    #[must_use]
    pub fn from_size(size: impl Into<ISize>) -> Self {
        let size = size.into();
        Self::new(0, 0, size.width, size.height)
    }

    #[must_use]
    pub fn from_pt_size(pt: impl Into<IPoint>, size: impl Into<ISize>) -> Self {
        let pt = pt.into();
        let size = size.into();
        Self::from_xywh(pt.x, pt.y, size.width, size.height)
    }

    #[must_use]
    pub const fn from_ltrb(l: i32, t: i32, r: i32, b: i32) -> Self {
        Self::new(l, t, r, b)
    }

    #[must_use]
    pub fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        IRect {
            left: x,
            top: y,
            right: sk32::sat_add(x, w),
            bottom: sk32::sat_add(y, h),
        }
    }

    pub const fn left(&self) -> i32 {
        self.left
    }

    pub const fn top(&self) -> i32 {
        self.top
    }

    pub const fn right(&self) -> i32 {
        self.right
    }

    pub const fn bottom(&self) -> i32 {
        self.bottom
    }

    pub const fn x(&self) -> i32 {
        self.left
    }

    pub const fn y(&self) -> i32 {
        self.top
    }

    pub const fn width(&self) -> i32 {
        sk32::can_overflow_sub(self.right, self.left)
    }

    pub const fn height(&self) -> i32 {
        sk32::can_overflow_sub(self.bottom, self.top)
    }

    pub const fn size(&self) -> ISize {
        ISize::new(self.width(), self.height())
    }

    pub const fn width_64(&self) -> i64 {
        self.right as i64 - self.left as i64
    }

    pub const fn height_64(&self) -> i64 {
        self.bottom as i64 - self.top as i64
    }

    pub fn is_empty_64(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    pub fn is_empty(&self) -> bool {
        unsafe { sb::C_SkIRect_isEmpty(self.native()) }
    }

    pub fn set_empty(&mut self) {
        *self = Self::new_empty()
    }

    pub fn set_ltrb(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        *self = Self::new(left, top, right, bottom);
    }

    pub fn set_xywh(&mut self, x: i32, y: i32, w: i32, h: i32) {
        *self = Self::from_xywh(x, y, w, h);
    }

    pub fn set_wh(&mut self, width: i32, height: i32) {
        self.left = 0;
        self.top = 0;
        self.right = width;
        self.bottom = height;
    }

    pub fn set_size(&mut self, size: impl Into<ISize>) {
        let size = size.into();
        self.left = 0;
        self.top = 0;
        self.right = size.width;
        self.bottom = size.height;
    }

    #[must_use]
    pub fn with_offset(&self, delta: impl Into<IVector>) -> Self {
        let mut copied = *self;
        copied.offset(delta);
        copied
    }

    #[must_use]
    pub fn with_inset(&self, delta: impl Into<IVector>) -> Self {
        self.with_outset(-delta.into())
    }

    #[must_use]
    pub fn with_outset(&self, delta: impl Into<IVector>) -> Self {
        let delta = delta.into();
        let (dx, dy) = (delta.x, delta.y);
        IRect::new(
            sk32::sat_sub(self.left, dx),
            sk32::sat_sub(self.top, dy),
            sk32::sat_add(self.right, dx),
            sk32::sat_add(self.bottom, dy),
        )
    }

    pub fn offset(&mut self, delta: impl Into<IPoint>) {
        let delta = delta.into();
        let (dx, dy) = (delta.x, delta.y);

        self.left = sk32::sat_add(self.left, dx);
        self.top = sk32::sat_add(self.top, dy);
        self.right = sk32::sat_add(self.right, dx);
        self.bottom = sk32::sat_add(self.bottom, dy);
    }

    pub fn offset_to(&mut self, new_p: impl Into<IPoint>) {
        *self = self.with_offset_to(new_p)
    }

    #[must_use]
    pub fn with_offset_to(&self, new_p: impl Into<IPoint>) -> Self {
        let new_p = new_p.into();
        let (new_x, new_y) = (new_p.x, new_p.y);

        IRect::new(
            sk64::pin_to_s32(i64::from(self.right) + i64::from(new_x) - i64::from(self.left)),
            sk64::pin_to_s32(i64::from(self.bottom) + i64::from(new_y) - i64::from(self.top)),
            new_x,
            new_y,
        )
    }

    pub fn inset(&mut self, delta: impl Into<IVector>) {
        *self = self.with_inset(delta)
    }

    pub fn outset(&mut self, delta: impl Into<IVector>) {
        *self = self.with_outset(delta)
    }

    #[must_use]
    pub fn with_adjustment(&self, d_l: i32, d_t: i32, d_r: i32, d_b: i32) -> Self {
        IRect::new(
            sk32::sat_add(self.left, d_l),
            sk32::sat_add(self.top, d_t),
            sk32::sat_add(self.right, d_r),
            sk32::sat_add(self.bottom, d_b),
        )
    }

    pub fn adjust(&mut self, d_l: i32, d_t: i32, d_r: i32, d_b: i32) {
        *self = self.with_adjustment(d_l, d_t, d_r, d_b)
    }

    // contains() is implemented through a trait below.

    pub fn contains_no_empty_check(&self, r: &Self) -> bool {
        debug_assert!(self.left < self.right && self.top < self.bottom);
        debug_assert!(r.left < r.right && r.top < r.bottom);

        self.left <= r.left && self.top <= r.top && self.right >= r.right && self.bottom >= r.bottom
    }

    #[must_use]
    pub fn intersect(a: &Self, b: &Self) -> Option<Self> {
        let mut r = Self::default();
        unsafe { r.native_mut().intersect(a.native(), b.native()) }.then_some(r)
    }

    pub fn intersects(a: &Self, b: &Self) -> bool {
        Self::intersect(a, b).is_some()
    }

    pub fn intersect_no_empty_check_(a: &Self, b: &Self) -> Option<Self> {
        debug_assert!(!a.is_empty_64() && !b.is_empty_64());
        let r = IRect::new(
            max(a.left, b.left),
            max(a.top, b.top),
            min(a.right, b.right),
            min(a.bottom, b.bottom),
        );
        (!r.is_empty()).then_some(r)
    }

    pub fn join(a: &Self, b: &Self) -> Self {
        let mut copied = *a;
        unsafe { copied.native_mut().join(b.native()) }
        copied
    }

    pub fn sort(&mut self) {
        *self = self.sorted()
    }

    #[must_use]
    pub fn sorted(&self) -> Self {
        Self::new(
            min(self.left, self.right),
            min(self.top, self.bottom),
            max(self.left, self.right),
            max(self.top, self.bottom),
        )
    }

    pub fn as_i32s(&self) -> &[i32] {
        unsafe { safer::from_raw_parts(&self.left, 4) }
    }

    #[deprecated(since = "0.27.0", note = "removed without replacement")]
    #[must_use]
    pub fn empty() -> &'static Self {
        &EMPTY_IRECT
    }
}

pub const EMPTY_IRECT: IRect = IRect {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};

impl Contains<IPoint> for IRect {
    fn contains(&self, other: IPoint) -> bool {
        let (x, y) = (other.x, other.y);
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

impl Contains<&IRect> for IRect {
    fn contains(&self, r: &IRect) -> bool {
        !r.is_empty()
            && !self.is_empty()
            && self.left <= r.left
            && self.top <= r.top
            && self.right >= r.right
            && self.bottom >= r.bottom
    }
}

impl Contains<&Rect> for IRect {
    fn contains(&self, other: &Rect) -> bool {
        unsafe { sb::C_SkIRect_contains(self.native(), other.native()) }
    }
}

impl Contains<IRect> for IRect {
    fn contains(&self, other: IRect) -> bool {
        self.contains(&other)
    }
}

impl Contains<Rect> for IRect {
    fn contains(&self, other: Rect) -> bool {
        self.contains(&other)
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Rect {
    /// The x coordinate of the rectangle's left edge.
    pub left: f32,
    /// The y coordinate of the rectangle's top edge.
    pub top: f32,
    /// The x coordinate of the rectangle's right edge.
    pub right: f32,
    /// The y coordinate of the rectangle's bottom edge.
    pub bottom: f32,
}

native_transmutable!(SkRect, Rect);

impl AsRef<Rect> for Rect {
    fn as_ref(&self) -> &Rect {
        self
    }
}

impl Rect {
    #[must_use]
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn new_empty() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    #[must_use]
    pub fn from_wh(w: f32, h: f32) -> Self {
        Self::new(0.0, 0.0, w, h)
    }

    #[must_use]
    pub fn from_iwh(w: i32, h: i32) -> Self {
        Self::from_wh(w as f32, h as f32)
    }

    #[must_use]
    pub fn from_size(size: impl Into<Size>) -> Self {
        (Point::default(), size.into()).into()
    }

    #[must_use]
    pub fn from_ltrb(l: f32, t: f32, r: f32, b: f32) -> Self {
        Self::new(l, t, r, b)
    }

    #[must_use]
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new(x, y, x + w, y + h)
    }

    #[must_use]
    pub fn from_point_and_size(p: impl Into<Point>, sz: impl Into<Size>) -> Self {
        (p.into(), sz.into()).into()
    }

    #[must_use]
    pub fn from_isize(isize: impl Into<ISize>) -> Self {
        let isize = isize.into();
        Self::from_iwh(isize.width, isize.height)
    }

    #[must_use]
    pub fn from_irect(irect: impl AsRef<IRect>) -> Self {
        let irect = irect.as_ref();
        Self::new(
            irect.left as f32,
            irect.top as f32,
            irect.right as f32,
            irect.bottom as f32,
        )
    }

    pub fn is_empty(&self) -> bool {
        // We write it as the NOT of a non-empty rect, so we will return true if any values
        // are NaN.
        !(self.left < self.right && self.top < self.bottom)
    }

    pub fn is_sorted(&self) -> bool {
        self.left <= self.right && self.top <= self.bottom
    }

    pub fn is_finite(&self) -> bool {
        is_finite(&[self.left, self.top, self.right, self.bottom])
    }

    pub const fn x(&self) -> f32 {
        self.left
    }

    pub const fn y(&self) -> f32 {
        self.top
    }

    pub const fn left(&self) -> f32 {
        self.left
    }

    pub const fn top(&self) -> f32 {
        self.top
    }

    pub const fn right(&self) -> f32 {
        self.right
    }

    pub const fn bottom(&self) -> f32 {
        self.bottom
    }

    pub fn size(&self) -> Size {
        (self.width(), self.height()).into()
    }

    pub fn width(&self) -> f32 {
        self.native().fRight - self.native().fLeft
    }

    pub fn height(&self) -> f32 {
        self.native().fBottom - self.native().fTop
    }

    pub fn center_x(&self) -> f32 {
        // don't use (fLeft + fBottom) * 0.5 as that might overflow before the 0.5
        self.left * 0.5 + self.right * 0.5
    }

    pub fn center_y(&self) -> f32 {
        // don't use (fTop + fBottom) * 0.5 as that might overflow before the 0.5
        self.top * 0.5 + self.bottom * 0.5
    }

    pub fn center(&self) -> Point {
        Point::from((self.center_x(), self.center_y()))
    }

    pub fn tl(&self) -> Point {
        Point::from((self.left, self.top))
    }

    pub fn tr(&self) -> Point {
        Point::from((self.right, self.top))
    }

    pub fn bl(&self) -> Point {
        Point::from((self.left, self.bottom))
    }

    pub fn br(&self) -> Point {
        Point::from((self.right, self.bottom))
    }

    pub fn to_quad(&self, dir: impl Into<Option<PathDirection>>) -> [Point; 4] {
        let mut pts = [Point::default(); 4];
        self.copy_to_quad(&mut pts, dir);
        pts
    }

    pub fn copy_to_quad(&self, pts: &mut [Point], dir: impl Into<Option<PathDirection>>) {
        debug_assert!(pts.len() >= 4);
        pts[0] = self.tl();
        pts[2] = self.br();
        match dir.into().unwrap_or(PathDirection::CW) {
            PathDirection::CW => {
                pts[1] = self.tr();
                pts[3] = self.bl();
            }
            PathDirection::CCW => {
                pts[1] = self.bl();
                pts[3] = self.tr();
            }
        }
    }

    pub fn set_empty(&mut self) {
        *self = Self::new_empty()
    }

    // TODO: deprecate and rename to set() as soon the other set() variant is removed.
    pub fn set_irect(&mut self, irect: impl AsRef<IRect>) {
        *self = Self::from_irect(irect)
    }

    pub fn set_ltrb(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        *self = Self::new(left, top, right, bottom)
    }

    pub fn bounds(pts: &[Point]) -> Option<Rect> {
        let mut bounds = Rect::default();
        unsafe { sb::C_SkRect_Bounds(pts.native().as_ptr(), pts.len(), bounds.native_mut()) }
            .then_some(bounds)
    }

    pub fn bounds_or_empty(pts: &[Point]) -> Rect {
        Self::bounds(pts).unwrap_or_else(Self::new_empty)
    }

    pub fn set_bounds(&mut self, points: &[Point]) {
        self.set_bounds_check(points);
    }

    pub fn set_bounds_check(&mut self, points: &[Point]) -> bool {
        unsafe {
            sb::C_SkRect_setBoundsCheck(self.native_mut(), points.native().as_ptr(), points.len())
        }
    }

    pub fn set_bounds_no_check(&mut self, points: &[Point]) {
        unsafe {
            sb::C_SkRect_setBoundsNoCheck(self.native_mut(), points.native().as_ptr(), points.len())
        }
    }

    pub fn set_bounds2(&mut self, p0: impl Into<Point>, p1: impl Into<Point>) {
        let (p0, p1) = (p0.into(), p1.into());
        self.left = p0.x.min(p1.x);
        self.right = p0.x.max(p1.x);
        self.top = p0.y.min(p1.y);
        self.bottom = p0.y.max(p1.y);
    }

    pub fn from_bounds(points: &[Point]) -> Option<Self> {
        let mut r = Self::default();
        r.set_bounds_check(points).then_some(r)
    }

    pub fn set_xywh(&mut self, x: f32, y: f32, width: f32, height: f32) {
        *self = Self::from_xywh(x, y, width, height)
    }

    pub fn set_wh(&mut self, w: f32, h: f32) {
        *self = Self::from_wh(w, h)
    }

    pub fn set_iwh(&mut self, width: i32, height: i32) {
        *self = Self::from_iwh(width, height)
    }

    #[must_use]
    pub fn with_offset(&self, d: impl Into<Vector>) -> Self {
        let d = d.into();
        Self::new(
            self.left + d.x,
            self.top + d.y,
            self.right + d.x,
            self.bottom + d.y,
        )
    }

    #[must_use]
    pub fn with_inset(&self, d: impl Into<Vector>) -> Self {
        let d = d.into();
        Self::new(
            self.left + d.x,
            self.top + d.y,
            self.right - d.x,
            self.bottom - d.y,
        )
    }

    #[must_use]
    pub fn with_outset(&self, d: impl Into<Vector>) -> Self {
        let d = d.into();
        Self::new(
            self.left - d.x,
            self.top - d.y,
            self.right + d.x,
            self.bottom + d.y,
        )
    }

    pub fn offset(&mut self, d: impl Into<Vector>) {
        *self = self.with_offset(d)
    }

    pub fn offset_to(&mut self, new_p: impl Into<Point>) {
        *self = self.with_offset_to(new_p)
    }

    #[must_use]
    pub fn with_offset_to(&self, new_p: impl Into<Point>) -> Self {
        let new_p = new_p.into();
        Self::new(new_p.x, new_p.y, new_p.x - self.left, new_p.y - self.top)
    }

    pub fn inset(&mut self, d: impl Into<Vector>) {
        *self = self.with_inset(d)
    }

    pub fn outset(&mut self, d: impl Into<Vector>) {
        *self = self.with_outset(d)
    }

    pub fn intersect(&mut self, r: impl AsRef<Rect>) -> bool {
        unsafe { self.native_mut().intersect(r.as_ref().native()) }
    }

    #[must_use]
    pub fn intersect2(&mut self, a: impl AsRef<Rect>, b: impl AsRef<Rect>) -> bool {
        unsafe {
            self.native_mut()
                .intersect1(a.as_ref().native(), b.as_ref().native())
        }
    }

    pub fn intersects(&self, r: impl AsRef<Rect>) -> bool {
        let r = r.as_ref();
        Self::intersects_(
            self.left,
            self.top,
            self.right,
            self.bottom,
            r.left,
            r.top,
            r.right,
            r.bottom,
        )
    }

    pub fn intersects2(a: impl AsRef<Rect>, b: impl AsRef<Rect>) -> bool {
        a.as_ref().intersects(b)
    }

    #[allow(clippy::too_many_arguments)]
    fn intersects_(al: f32, at: f32, ar: f32, ab: f32, bl: f32, bt: f32, br: f32, bb: f32) -> bool {
        let l = al.max(bl);
        let r = ar.min(br);
        let t = at.max(bt);
        let b = ab.min(bb);
        l < r && t < b
    }

    pub fn join(&mut self, r: impl AsRef<Rect>) {
        let r = r.as_ref();
        unsafe { self.native_mut().join(r.native()) }
    }

    pub fn join2(a: impl AsRef<Rect>, b: impl AsRef<Rect>) -> Rect {
        let mut result = *a.as_ref();
        result.join(b);
        result
    }

    pub fn join_non_empty_arg(&mut self, r: impl AsRef<Rect>) {
        let r = r.as_ref();
        debug_assert!(!r.is_empty());
        if self.left >= self.right || self.top >= self.bottom {
            *self = *r;
        } else {
            self.join_possibly_empty_rect(r);
        }
    }

    pub fn join_possibly_empty_rect(&mut self, r: impl AsRef<Rect>) {
        let r = r.as_ref();
        self.left = self.left.min(r.left);
        self.top = self.top.min(r.top);
        self.right = self.right.max(r.right);
        self.bottom = self.bottom.max(r.bottom);
    }

    // The set of contains() functions are defined as a trait below.

    #[must_use]
    pub fn round(&self) -> IRect {
        let mut r = IRect::default();
        unsafe { sb::C_SkRect_round(self.native(), r.native_mut()) };
        r
    }

    // The functions round_out() are defined as a trait below.

    #[must_use]
    pub fn round_in(&self) -> IRect {
        let mut r = IRect::default();
        unsafe { sb::C_SkRect_roundIn(self.native(), r.native_mut()) };
        r
    }

    pub fn sort(&mut self) {
        if self.left > self.right {
            mem::swap(&mut self.left, &mut self.right);
        }

        if self.top > self.bottom {
            mem::swap(&mut self.top, &mut self.bottom);
        }
    }

    #[must_use]
    pub fn sorted(&self) -> Rect {
        Rect::new(
            self.left.min(self.right),
            self.top.min(self.bottom),
            self.left.max(self.right),
            self.top.max(self.bottom),
        )
    }

    pub fn as_scalars(&self) -> &[f32; 4] {
        unsafe { transmute_ref(&self.left) }
    }

    pub fn dump(&self, as_hex: impl Into<Option<bool>>) {
        unsafe { self.native().dump(as_hex.into().unwrap_or_default()) }
    }

    pub fn dump_to_string(&self, as_hex: bool) -> String {
        let mut str = interop::String::default();
        unsafe { sb::C_SkRect_dumpToString(self.native(), as_hex, str.native_mut()) }
        str.to_string()
    }

    pub fn dump_hex(&self) {
        self.dump(true)
    }
}

impl Contains<Point> for Rect {
    fn contains(&self, p: Point) -> bool {
        self.contains(&p)
    }
}

impl Contains<&Point> for Rect {
    fn contains(&self, p: &Point) -> bool {
        p.x >= self.left && p.x < self.right && p.y >= self.top && p.y < self.bottom
    }
}

impl Contains<Rect> for Rect {
    fn contains(&self, r: Rect) -> bool {
        self.contains(&r)
    }
}

impl Contains<&Rect> for Rect {
    fn contains(&self, r: &Rect) -> bool {
        // TODO: can we eliminate the this->is_empty check?
        !r.is_empty()
            && !self.is_empty()
            && self.left <= r.left
            && self.top <= r.top
            && self.right >= r.right
            && self.bottom >= r.bottom
    }
}

impl Contains<IRect> for Rect {
    fn contains(&self, r: IRect) -> bool {
        self.contains(&r)
    }
}

impl Contains<&IRect> for Rect {
    fn contains(&self, r: &IRect) -> bool {
        // TODO: can we eliminate the this->isEmpty check?
        !r.is_empty()
            && !self.is_empty()
            && self.left <= r.left as f32
            && self.top <= r.top as f32
            && self.right >= r.right as f32
            && self.bottom >= r.bottom as f32
    }
}

#[test]
fn contains_overloads_compile() {
    let r = Rect::default();
    r.contains(Point::default());
    r.contains(Rect::default());
    r.contains(IRect::default());
}

pub trait RoundOut<R> {
    fn round_out(&self) -> R;
}

impl RoundOut<IRect> for Rect {
    fn round_out(&self) -> IRect {
        let mut r = IRect::default();
        unsafe { sb::C_SkRect_roundOut(self.native(), r.native_mut()) };
        r
    }
}

impl RoundOut<Rect> for Rect {
    fn round_out(&self) -> Rect {
        Rect::new(
            self.left.floor(),
            self.top.floor(),
            self.right.ceil(),
            self.bottom.ceil(),
        )
    }
}

//
// From
//

impl From<(IPoint, ISize)> for IRect {
    fn from((point, size): (IPoint, ISize)) -> Self {
        IRect::new(
            point.x,
            point.y,
            point.x + size.width,
            point.y + size.height,
        )
    }
}

impl From<(Point, Size)> for Rect {
    fn from((point, size): (Point, Size)) -> Self {
        Rect::new(
            point.x,
            point.y,
            point.x + size.width,
            point.y + size.height,
        )
    }
}

impl From<ISize> for Rect {
    fn from(isize: ISize) -> Self {
        Self::from_isize(isize)
    }
}
impl From<IRect> for Rect {
    fn from(irect: IRect) -> Self {
        Self::from_irect(irect)
    }
}
