use super::{DebugAttributes, Node, NodeSubtype, TypedNode};
use crate::prelude::*;
use skia_bindings as sb;

pub type Container = RCHandle<sb::SkSVGContainer>;

impl NodeSubtype for sb::SkSVGContainer {
    type Base = sb::SkSVGTransformableNode;
}

impl DebugAttributes for Container {
    const NAME: &'static str = "Container";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()
            ._dbg(builder.field("children", &self.children_typed()));
    }
}

impl Container {
    pub fn append_child(&mut self, node: impl Into<Node>) {
        unsafe { sb::C_SkSVGContainer_appendChild(self.native_mut(), node.into().into_ptr()) }
    }

    pub fn children(&self) -> &[Node] {
        unsafe {
            let sp_slice = safer::from_raw_parts(
                sb::C_SkSVGContainer_children(self.native()),
                self.children_count(),
            );

            RCHandle::from_non_null_sp_slice(sp_slice)
        }
    }

    pub fn children_typed(&self) -> Vec<TypedNode> {
        self.children().iter().cloned().map(|n| n.typed()).collect()
    }

    pub(crate) fn children_count(&self) -> usize {
        unsafe {
            usize::try_from(sb::C_SkSVGContainer_childrenCount(self.native())).unwrap_or_default()
        }
    }
}
