use crate::prelude::*;
use skia_bindings::{SkBBHFactory, SkBBoxHierarchy};

// TODO: complete the wrapper
pub type BBoxHierarchy = RCHandle<SkBBoxHierarchy>;

// TODO: complete the wrapper
pub type BBHFactory = Handle<SkBBHFactory>;

impl NativeDrop for SkBBHFactory {
    fn drop(&mut self) {
        unimplemented!()
    }
}

// TODO: complete the wrapper functions
impl BBHFactory {}
