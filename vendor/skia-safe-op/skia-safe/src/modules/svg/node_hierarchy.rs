use crate::prelude::*;
use std::{
    fmt::{Debug, DebugStruct, Formatter, Result},
    ops::{Deref, DerefMut},
};

use super::{Node, TypedNode};

pub trait NodeSubtype {
    type Base: NativeRefCounted;
}

/// All [`NodeSubType`] are defined to be a [`RcHandle<N>`] with `N` deriving from
/// [`sb::SkRefCntBase`]
impl<T: NodeSubtype> NativeRefCountedBase for T {
    type Base = skia_bindings::SkRefCntBase;
}

impl<T: NativeRefCounted + NodeSubtype> RCHandle<T> {
    pub fn as_base(&self) -> &RCHandle<T::Base> {
        unsafe { transmute_ref(self) }
    }

    pub fn as_base_mut(&mut self) -> &mut RCHandle<T::Base> {
        unsafe { transmute_ref_mut(self) }
    }

    /// All concrete node types can be converted to the supertype [`Node`].
    pub fn into_node(self) -> Node {
        unsafe { std::mem::transmute(self) }
    }

    /// All concrete node types can be converted to a [`TypedNode`]
    pub fn typed(self) -> TypedNode {
        self.into_node().typed()
    }
}

impl<T: NativeRefCounted + NodeSubtype> Deref for RCHandle<T> {
    type Target = RCHandle<T::Base>;

    fn deref(&self) -> &Self::Target {
        self.as_base()
    }
}

/// This implementation of [`DerefMut`] causes subsequent UB when the containing
/// [`RCHandle`] gets overwritten by a base type that does not match the actual
/// underlying type.
impl<T: NativeRefCounted + NodeSubtype> DerefMut for RCHandle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_base_mut()
    }
}

impl<N: NativeRefCounted> Debug for RCHandle<N>
where
    Self: DebugAttributes,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut builder = f.debug_struct(Self::NAME);

        self._dbg(&mut builder);

        builder.finish()
    }
}

pub trait DebugAttributes {
    const NAME: &'static str;

    fn _dbg(&self, builder: &mut DebugStruct);
}

#[cfg(test)]
mod tests {
    use crate::svg::{Circle, Shape, TypedNode};

    #[test]
    fn subtype_can_access_supertype_attributes() {
        let circle = Circle::default();
        _ = circle.opacity();
    }

    #[test]
    fn subtype_can_set_supertype_attributes() {
        let mut circle = Circle::default();
        circle.set_opacity(0.1);
    }

    #[test]
    fn supertype_can_be_retrieved_and_contains_the_same_attributes() {
        let mut circle = Circle::default();
        circle.set_opacity(0.1);
        let base: &Shape = circle.as_base();
        assert_eq!(base.opacity(), Some(0.1));
    }

    #[test]
    fn supertype_can_be_modified_and_affects_subtype() {
        let mut circle = Circle::default();
        let base: &mut Shape = circle.as_base_mut();
        base.set_opacity(0.1);
        assert_eq!(circle.opacity(), Some(0.1));
    }

    #[test]
    fn concrete_node_can_be_converted_to_a_node() {
        Circle::default().into_node();
    }

    #[test]
    fn concrete_node_can_be_typed() {
        let circle = Circle::default().typed();
        assert!(matches!(circle, TypedNode::Circle(_)));
    }
}
