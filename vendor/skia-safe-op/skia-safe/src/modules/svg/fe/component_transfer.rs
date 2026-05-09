use super::{DebugAttributes, NodeSubtype};
use crate::{impl_default_make, prelude::*, scalar};
use skia_bindings as sb;

pub type FuncKind = sb::SkSVGFeFuncType;
variant_name!(FuncKind::Identity);

pub type Func = RCHandle<sb::SkSVGFeFunc>;

impl NodeSubtype for sb::SkSVGFeFunc {
    type Base = sb::SkSVGFe;
}

impl DebugAttributes for Func {
    const NAME: &'static str = "FeFunc";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("amplitude", &self.amplitude())
                .field("exponent", &self.exponent())
                .field("intercept", &self.intercept())
                .field("offset", &self.offset())
                .field("slope", &self.slope())
                .field("table_values", &self.table_values())
                .field("kind", self.kind()),
        );
    }
}

impl Func {
    pub fn func_a() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGFeFunc_MakeFuncA() }).unwrap()
    }

    pub fn func_r() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGFeFunc_MakeFuncR() }).unwrap()
    }

    pub fn func_g() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGFeFunc_MakeFuncG() }).unwrap()
    }

    pub fn func_b() -> Self {
        Self::from_ptr(unsafe { sb::C_SkSVGFeFunc_MakeFuncB() }).unwrap()
    }

    pub fn table_values(&self) -> &[scalar] {
        unsafe {
            safer::from_raw_parts(
                sb::C_SkSVGFeFunc_getTableValues(self.native()),
                sb::C_SkSVGFeFunc_getTableValuesCount(self.native()),
            )
        }
    }

    // TODO: wrap getTable()

    skia_svg_macros::attrs! {
        SkSVGFeFunc => {
            *amplitude: scalar [get(value) => value, set(value) => value],
            *exponent: scalar [get(value) => value, set(value) => value],
            *intercept: scalar [get(value) => value, set(value) => value],
            *offset: scalar [get(value) => value, set(value) => value],
            *slope: scalar [get(value) => value, set(value) => value],
            "type" as kind: FuncKind [get(value) => value, set(value) => value]
        }
    }
}

pub type ComponentTransfer = RCHandle<sb::SkSVGFeComponentTransfer>;

impl NodeSubtype for sb::SkSVGFeComponentTransfer {
    type Base = sb::SkSVGFe;
}

impl_default_make!(ComponentTransfer, sb::C_SkSVGFeComponentTransfer_Make);

impl DebugAttributes for ComponentTransfer {
    const NAME: &'static str = "FeComponentTransfer";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(builder);
    }
}
