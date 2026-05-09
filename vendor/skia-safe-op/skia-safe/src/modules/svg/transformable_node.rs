use super::{DebugAttributes, NodeSubtype};
use crate::{prelude::*, Matrix};
use skia_bindings as sb;

pub type TransformableNode = RCHandle<sb::SkSVGTransformableNode>;

impl NodeSubtype for sb::SkSVGTransformableNode {
    type Base = sb::SkSVGNode;
}

impl DebugAttributes for TransformableNode {
    const NAME: &'static str = "TransformableNode";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()
            ._dbg(builder.field("transform", self.transform()));
    }
}

impl TransformableNode {
    pub fn set_transform(&mut self, transform: &Matrix) {
        unsafe { sb::C_SkSVGTransformableNode_setTransform(self.native_mut(), transform.native()) }
    }

    pub fn transform(&self) -> &Matrix {
        Matrix::from_native_ref(&self.native().fTransform)
    }
}
