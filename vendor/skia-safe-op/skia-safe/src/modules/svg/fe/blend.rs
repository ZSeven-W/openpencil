use super::{DebugAttributes, Input, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type BlendMode = sb::SkSVGFeBlend_Mode;
variant_name!(BlendMode::Multiply);

pub type Blend = RCHandle<sb::SkSVGFeBlend>;

impl NodeSubtype for sb::SkSVGFeBlend {
    type Base = sb::SkSVGFe;
}

impl_default_make!(Blend, sb::C_SkSVGFeBlend_Make);

impl DebugAttributes for Blend {
    const NAME: &'static str = "FeBlend";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("input2", self.input2())
                .field("mode", self.mode()),
        );
    }
}

impl Blend {
    skia_svg_macros::attrs! {
        SkSVGFeBlend => {
            "in2" as input2: Input [get(value) => Input::from_native_ref(value), set(value) => value.into_native()],
            mode: BlendMode [get(value) => value, set(value) => value]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Blend;

    #[test]
    pub fn create() {
        let blend = Blend::default();
        println!("{blend:?}");
    }
}
