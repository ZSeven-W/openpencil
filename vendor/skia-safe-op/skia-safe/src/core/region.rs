use std::{fmt, iter, marker::PhantomData, mem, ptr};

use crate::{prelude::*, Contains, IPoint, IRect, IVector, Path, PathBuilder, QuickReject};
use skia_bindings::{
    self as sb, SkRegion, SkRegion_Cliperator, SkRegion_Iterator, SkRegion_RunHead,
    SkRegion_Spanerator,
};

pub type Region = Handle<SkRegion>;
unsafe_send_sync!(Region);

impl NativeDrop for SkRegion {
    fn drop(&mut self) {
        unsafe { sb::C_SkRegion_destruct(self) }
    }
}

impl NativeClone for SkRegion {
    fn clone(&self) -> Self {
        unsafe { SkRegion::new1(self) }
    }
}

impl NativePartialEq for SkRegion {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkRegion_Equals(self, rhs) }
    }
}

impl fmt::Debug for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Region")
            .field("is_empty", &self.is_empty())
            .field("is_rect", &self.is_rect())
            .field("is_complex", &self.is_complex())
            .field("bounds", &self.bounds())
            .finish()
    }
}

pub use skia_bindings::SkRegion_Op as RegionOp;
variant_name!(RegionOp::ReverseDifference);

impl Region {
    pub fn new() -> Region {
        Self::from_native_c(unsafe { SkRegion::new() })
    }

    pub fn from_rect(rect: impl AsRef<IRect>) -> Region {
        Self::from_native_c(unsafe { SkRegion::new2(rect.as_ref().native()) })
    }

    pub fn set(&mut self, src: &Region) -> bool {
        unsafe { sb::C_SkRegion_set(self.native_mut(), src.native()) }
    }

    pub fn swap(&mut self, other: &mut Region) {
        unsafe { self.native_mut().swap(other.native_mut()) }
    }

    const EMPTY_RUN_HEAD_PTR: *mut SkRegion_RunHead = -1 as _;
    const RECT_RUN_HEAD_PTR: *mut SkRegion_RunHead = ptr::null_mut();

    pub fn is_empty(&self) -> bool {
        ptr::eq(self.native().fRunHead, Self::EMPTY_RUN_HEAD_PTR)
    }

    pub fn is_rect(&self) -> bool {
        ptr::eq(self.native().fRunHead, Self::RECT_RUN_HEAD_PTR)
    }

    pub fn is_complex(&self) -> bool {
        !self.is_empty() && !self.is_rect()
    }

    pub fn bounds(&self) -> &IRect {
        IRect::from_native_ref(&self.native().fBounds)
    }

    pub fn compute_region_complexity(&self) -> usize {
        unsafe { self.native().computeRegionComplexity().try_into().unwrap() }
    }

    pub fn add_boundary_path(&self, path: &mut PathBuilder) -> bool {
        unsafe { self.native().addBoundaryPath(path.native_mut()) }
    }

    #[deprecated(since = "0.91.0", note = "Use boundary_path()")]
    pub fn get_boundary_path(&self, path: &mut Path) -> bool {
        unsafe { sb::C_SkRegion_getBoundaryPath(self.native(), path.native_mut()) };
        !path.is_empty()
    }

    pub fn boundary_path(&self) -> Option<Path> {
        let mut path = Path::default();
        unsafe { sb::C_SkRegion_getBoundaryPath(self.native(), path.native_mut()) };
        (!path.is_empty()).then_some(path)
    }

    pub fn set_empty(&mut self) -> bool {
        unsafe { self.native_mut().setEmpty() }
    }

    pub fn set_rect(&mut self, rect: impl AsRef<IRect>) -> bool {
        unsafe { self.native_mut().setRect(rect.as_ref().native()) }
    }

    pub fn set_rects(&mut self, rects: &[IRect]) -> bool {
        unsafe {
            self.native_mut()
                .setRects(rects.native().as_ptr(), rects.len().try_into().unwrap())
        }
    }

    pub fn set_region(&mut self, region: &Region) -> bool {
        unsafe { self.native_mut().setRegion(region.native()) }
    }

    pub fn set_path(&mut self, path: &Path, clip: &Region) -> bool {
        unsafe { self.native_mut().setPath(path.native(), clip.native()) }
    }

    // there is also a trait for intersects() below.

    pub fn intersects_rect(&self, rect: impl AsRef<IRect>) -> bool {
        unsafe { self.native().intersects(rect.as_ref().native()) }
    }

    pub fn intersects_region(&self, other: &Region) -> bool {
        unsafe { self.native().intersects1(other.native()) }
    }

    // contains() trait below.

    pub fn contains_point(&self, point: IPoint) -> bool {
        unsafe { self.native().contains(point.x, point.y) }
    }

    pub fn contains_rect(&self, rect: impl AsRef<IRect>) -> bool {
        unsafe { self.native().contains1(rect.as_ref().native()) }
    }

    pub fn contains_region(&self, other: &Region) -> bool {
        unsafe { self.native().contains2(other.native()) }
    }

    pub fn quick_contains(&self, r: impl AsRef<IRect>) -> bool {
        let r = r.as_ref();
        unsafe { sb::C_SkRegion_quickContains(self.native(), r.native()) }
    }

    // see also the quick_reject() trait below.

    pub fn quick_reject_rect(&self, rect: impl AsRef<IRect>) -> bool {
        let rect = rect.as_ref();
        self.is_empty() || rect.is_empty() || !IRect::intersects(self.bounds(), rect)
    }

    pub fn quick_reject_region(&self, rgn: &Region) -> bool {
        self.is_empty() || rgn.is_empty() || !IRect::intersects(self.bounds(), rgn.bounds())
    }

    pub fn translate(&mut self, d: impl Into<IVector>) {
        let d = d.into();
        let self_ptr = self.native_mut() as *mut _;
        unsafe { self.native().translate(d.x, d.y, self_ptr) }
    }

    #[must_use]
    pub fn translated(&self, d: impl Into<IVector>) -> Self {
        let mut r = self.clone();
        r.translate(d);
        r
    }

    pub fn op_rect(&mut self, rect: impl AsRef<IRect>, op: RegionOp) -> bool {
        let self_ptr = self.native_mut() as *const _;
        unsafe { self.native_mut().op1(self_ptr, rect.as_ref().native(), op) }
    }

    pub fn op_region(&mut self, region: &Region, op: RegionOp) -> bool {
        let self_ptr = self.native_mut() as *const _;
        unsafe { self.native_mut().op2(self_ptr, region.native(), op) }
    }

    pub fn op_rect_region(
        &mut self,
        rect: impl AsRef<IRect>,
        region: &Region,
        op: RegionOp,
    ) -> bool {
        unsafe {
            self.native_mut()
                .op(rect.as_ref().native(), region.native(), op)
        }
    }

    pub fn op_region_rect(
        &mut self,
        region: &Region,
        rect: impl AsRef<IRect>,
        op: RegionOp,
    ) -> bool {
        unsafe {
            self.native_mut()
                .op1(region.native(), rect.as_ref().native(), op)
        }
    }

    pub fn write_to_memory(&self, buf: &mut Vec<u8>) {
        unsafe {
            let size = self.native().writeToMemory(ptr::null_mut());
            buf.resize(size, 0);
            let written = self.native().writeToMemory(buf.as_mut_ptr() as _);
            debug_assert!(written == size);
        }
    }

    pub fn read_from_memory(&mut self, buf: &[u8]) -> usize {
        unsafe {
            self.native_mut()
                .readFromMemory(buf.as_ptr() as _, buf.len())
        }
    }
}

//
// combine overloads (static)
//

pub trait Combine<A, B>: Sized {
    fn combine(a: &A, op: RegionOp, b: &B) -> Self;

    fn difference(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::Difference, b)
    }

    fn intersect(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::Intersect, b)
    }

    fn xor(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::XOR, b)
    }

    fn union(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::Union, b)
    }

    fn reverse_difference(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::ReverseDifference, b)
    }

    fn replace(a: &A, b: &B) -> Self {
        Self::combine(a, RegionOp::Replace, b)
    }
}

impl Combine<IRect, Region> for Handle<SkRegion> {
    fn combine(rect: &IRect, op: RegionOp, region: &Region) -> Self {
        let mut r = Region::new();
        r.op_rect_region(rect, region, op);
        r
    }
}

impl Combine<Region, IRect> for Handle<SkRegion> {
    fn combine(region: &Region, op: RegionOp, rect: &IRect) -> Self {
        let mut r = Region::new();
        r.op_region_rect(region, rect, op);
        r
    }
}

impl Combine<Region, Region> for Handle<SkRegion> {
    fn combine(a: &Region, op: RegionOp, b: &Region) -> Self {
        let mut a = a.clone();
        a.op_region(b, op);
        a
    }
}

//
// intersects overloads
//

pub trait Intersects<T> {
    fn intersects(&self, other: &T) -> bool;
}

impl Intersects<IRect> for Region {
    fn intersects(&self, rect: &IRect) -> bool {
        self.intersects_rect(rect)
    }
}

impl Intersects<Region> for Region {
    fn intersects(&self, other: &Region) -> bool {
        self.intersects_region(other)
    }
}

//
// contains overloads
//

impl Contains<IPoint> for Region {
    fn contains(&self, point: IPoint) -> bool {
        self.contains_point(point)
    }
}

impl Contains<&IRect> for Region {
    fn contains(&self, rect: &IRect) -> bool {
        self.contains_rect(rect)
    }
}

impl Contains<&Region> for Region {
    fn contains(&self, other: &Region) -> bool {
        self.contains_region(other)
    }
}

//
// quick_reject overloads
//

impl QuickReject<IRect> for Region {
    fn quick_reject(&self, rect: &IRect) -> bool {
        self.quick_reject_rect(rect)
    }
}

impl QuickReject<Region> for Region {
    fn quick_reject(&self, other: &Region) -> bool {
        self.quick_reject_region(other)
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Iterator<'a>(SkRegion_Iterator, PhantomData<&'a Region>);

native_transmutable!(SkRegion_Iterator, Iterator<'_>);

impl fmt::Debug for Iterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iterator")
            .field("is_done", &self.is_done())
            .field("rect", self.rect())
            .finish()
    }
}

impl<'a> Iterator<'a> {
    pub fn new_empty() -> Self {
        Iterator::construct(|iterator| unsafe {
            sb::C_SkRegion_Iterator_Construct(iterator);
        })
    }

    pub fn new(region: &'a Region) -> Iterator<'a> {
        Iterator::from_native_c(unsafe { SkRegion_Iterator::new(region.native()) })
    }

    pub fn rewind(&mut self) -> bool {
        unsafe { self.native_mut().rewind() }
    }

    pub fn reset(mut self, region: &Region) -> Iterator {
        unsafe {
            self.native_mut().reset(region.native());
            mem::transmute(self)
        }
    }

    pub fn is_done(&self) -> bool {
        self.native().fDone
    }

    pub fn next(&mut self) {
        unsafe {
            self.native_mut().next();
        }
    }

    pub fn rect(&self) -> &IRect {
        IRect::from_native_ref(&self.native().fRect)
    }

    pub fn rgn(&self) -> Option<&Region> {
        unsafe {
            let r = sb::C_SkRegion_Iterator_rgn(self.native()).into_non_null()?;
            Some(Region::from_native_ref(r.as_ref()))
        }
    }
}

impl iter::Iterator for Iterator<'_> {
    type Item = IRect;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done() {
            return None;
        }
        let r = *self.rect();
        Iterator::next(self);
        Some(r)
    }
}

#[test]
fn test_iterator() {
    let r1 = IRect::new(10, 10, 12, 14);
    let r2 = IRect::new(100, 100, 120, 140);
    let mut r = Region::new();
    r.set_rects(&[r1, r2]);
    let rects: Vec<IRect> = Iterator::new(&r).collect();
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], r1);
    assert_eq!(rects[1], r2);
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Cliperator<'a>(SkRegion_Cliperator, PhantomData<&'a Region>);

native_transmutable!(SkRegion_Cliperator, Cliperator<'_>);

impl Drop for Cliperator<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkRegion_Cliperator_destruct(self.native_mut()) }
    }
}

impl fmt::Debug for Cliperator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cliperator")
            .field("is_done", &self.is_done())
            .field("rect", &self.rect())
            .finish()
    }
}

impl<'a> Cliperator<'a> {
    pub fn new(region: &'a Region, clip: impl AsRef<IRect>) -> Cliperator<'a> {
        Cliperator::from_native_c(unsafe {
            SkRegion_Cliperator::new(region.native(), clip.as_ref().native())
        })
    }

    pub fn is_done(&self) -> bool {
        self.native().fDone
    }

    pub fn next(&mut self) {
        unsafe { self.native_mut().next() }
    }

    pub fn rect(&self) -> &IRect {
        IRect::from_native_ref(&self.native().fRect)
    }
}

impl iter::Iterator for Cliperator<'_> {
    type Item = IRect;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done() {
            return None;
        }
        let rect = *self.rect();
        self.next();
        Some(rect)
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Spanerator<'a>(SkRegion_Spanerator, PhantomData<&'a Region>);

native_transmutable!(SkRegion_Spanerator, Spanerator<'_>);

impl Drop for Spanerator<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkRegion_Spanerator_destruct(self.native_mut()) }
    }
}

impl fmt::Debug for Spanerator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spanerator").finish()
    }
}

impl<'a> Spanerator<'a> {
    pub fn new(region: &'a Region, y: i32, left: i32, right: i32) -> Spanerator<'a> {
        Spanerator::from_native_c(unsafe {
            SkRegion_Spanerator::new(region.native(), y, left, right)
        })
    }
}

impl iter::Iterator for Spanerator<'_> {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut left = 0;
            let mut right = 0;
            self.native_mut()
                .next(&mut left, &mut right)
                .then_some((left, right))
        }
    }
}

#[test]
fn new_clone_drop() {
    let region = Region::new();
    #[allow(clippy::redundant_clone)]
    let _cloned = region.clone();
}

#[test]
fn can_compare() {
    let r1 = Region::new();
    #[allow(clippy::redundant_clone)]
    let r2 = r1.clone();
    assert!(r1 == r2);
}
