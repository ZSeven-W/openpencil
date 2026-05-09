use skia_bindings as sb;

use crate::scalar;

pub type LengthUnit = sb::SkSVGLength_Unit;
variant_name!(LengthUnit::Number);

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Length {
    pub value: scalar,
    pub unit: LengthUnit,
}

native_transmutable!(sb::SkSVGLength, Length);

impl Length {
    pub fn new(value: scalar, unit: LengthUnit) -> Self {
        Self { value, unit }
    }
}
