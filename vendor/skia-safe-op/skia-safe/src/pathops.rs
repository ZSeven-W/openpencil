//! Wrapper for pathops/SkPathOps.h
use crate::{prelude::*, Path, Rect};
use skia_bindings::{self as sb, SkOpBuilder};
use std::fmt;

pub type PathOp = skia_bindings::SkPathOp;
variant_name!(PathOp::XOR);

// TODO: I am not so sure if we should export these global functions.

pub fn op(one: &Path, two: &Path, op: PathOp) -> Option<Path> {
    Path::try_construct(|p| unsafe { sb::C_SkPathOps_Op(one.native(), two.native(), op, p) })
}

pub fn simplify(path: &Path) -> Option<Path> {
    Path::try_construct(|p| unsafe { sb::C_SkPathOps_Simplify(path.native(), p) })
}

#[deprecated(
    since = "0.83.0",
    note = "Use Path::compute_tight_bounds() and test if the resulting Rect::is_finite()"
)]
pub fn tight_bounds(path: &Path) -> Option<Rect> {
    let rect = path.compute_tight_bounds();
    rect.is_finite().then_some(rect)
}

pub fn as_winding(path: &Path) -> Option<Path> {
    Path::try_construct(|p| unsafe { sb::C_SkPathOps_AsWinding(path.native(), p) })
}

pub type OpBuilder = Handle<SkOpBuilder>;
unsafe_send_sync!(OpBuilder);

impl NativeDrop for SkOpBuilder {
    fn drop(&mut self) {
        unsafe { sb::C_SkOpBuilder_destruct(self) }
    }
}

impl Default for Handle<SkOpBuilder> {
    fn default() -> Self {
        Self::construct(|opb| unsafe { sb::C_SkOpBuilder_Construct(opb) })
    }
}

impl fmt::Debug for OpBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpBuilder").finish()
    }
}

impl OpBuilder {
    pub fn add(&mut self, path: &Path, operator: PathOp) -> &mut Self {
        unsafe {
            self.native_mut().add(path.native(), operator);
        }
        self
    }

    pub fn resolve(&mut self) -> Option<Path> {
        Path::try_construct(|p| unsafe { sb::C_SkOpBuilder_resolve(self.native_mut(), p) })
    }
}

impl Path {
    pub fn op(&self, path: &Path, path_op: PathOp) -> Option<Self> {
        op(self, path, path_op)
    }

    pub fn simplify(&self) -> Option<Self> {
        simplify(self)
    }

    #[deprecated(
        since = "0.83.0",
        note = "Use Path::compute_tight_bounds() and test if the resulting Rect::is_finite()"
    )]
    pub fn tight_bounds(&self) -> Option<Rect> {
        #[allow(deprecated)]
        tight_bounds(self)
    }

    pub fn as_winding(&self) -> Option<Path> {
        as_winding(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Path, PathBuilder, PathOp, Rect};

    #[test]
    fn test_tight_bounds() {
        let mut builder = PathBuilder::new();
        builder.add_rect(
            Rect::from_point_and_size((10.0, 10.0), (10.0, 10.0)),
            None,
            None,
        );
        builder.add_rect(
            Rect::from_point_and_size((15.0, 15.0), (10.0, 10.0)),
            None,
            None,
        );
        let path = builder.detach();

        let tight_bounds: Rect = Rect::from_point_and_size((10.0, 10.0), (15.0, 15.0));
        assert_eq!(path.compute_tight_bounds(), tight_bounds);
    }

    #[test]
    fn test_union() {
        let path = Path::rect(Rect::from_point_and_size((10.0, 10.0), (10.0, 10.0)), None);
        let path2 = Path::rect(Rect::from_point_and_size((15.0, 15.0), (10.0, 10.0)), None);
        let union = path.op(&path2, PathOp::Union).unwrap();
        let expected: Rect = Rect::from_point_and_size((10.0, 10.0), (15.0, 15.0));
        assert_eq!(union.compute_tight_bounds(), expected);
    }

    #[test]
    fn test_intersect() {
        let path = Path::rect(Rect::from_point_and_size((10.0, 10.0), (10.0, 10.0)), None);
        let path2 = Path::rect(Rect::from_point_and_size((15.0, 15.0), (10.0, 10.0)), None);
        let intersected = path.op(&path2, PathOp::Intersect).unwrap();
        let expected: Rect = Rect::from_point_and_size((15.0, 15.0), (5.0, 5.0));
        assert_eq!(intersected.compute_tight_bounds(), expected);
    }
}
