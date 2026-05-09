use super::{DebugAttributes, Input, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

pub type CompositeOperator = sb::SkSVGFeCompositeOperator;
variant_name!(CompositeOperator::Out);

pub type Composite = RCHandle<sb::SkSVGFeComposite>;

impl NodeSubtype for sb::SkSVGFeComposite {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Composite, sb::C_SkSVGFeComposite_Make);

impl DebugAttributes for Composite {
    const NAME: &'static str = "FeComposite";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("input2", self.input2())
                .field("k1", &self.k1())
                .field("k2", &self.k2())
                .field("k3", &self.k3())
                .field("k4", &self.k4())
                .field("operator", self.operator()),
        );
    }
}

impl Composite {
    skia_svg_macros::attrs! {
        SkSVGFeComposite => {
            "in2" as input2: Input [get(value) => Input::from_native_ref(value), set(value) => value.into_native()],
            *k1: scalar [get(value) => value, set(value) => value],
            *k2: scalar [get(value) => value, set(value) => value],
            *k3: scalar [get(value) => value, set(value) => value],
            *k4: scalar [get(value) => value, set(value) => value],
            operator: CompositeOperator [get(value) => value, set(value) => value]
        }
    }
}
