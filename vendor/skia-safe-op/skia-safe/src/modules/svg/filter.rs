use super::{BoundingBoxUnits, DebugAttributes, Length, NodeSubtype};
use crate::{impl_default_make, prelude::*};
use skia_bindings as sb;

pub type Filter = RCHandle<sb::SkSVGFilter>;

impl DebugAttributes for Filter {
    const NAME: &'static str = "Filter";

    fn _dbg(&self, builder: &mut std::fmt::DebugStruct) {
        self.as_base()._dbg(
            builder
                .field("x", &self.x())
                .field("y", &self.y())
                .field("width", &self.width())
                .field("height", &self.height())
                .field("filter_units", self.filter_units())
                .field("primitive_units", self.primitive_units()),
        );
    }
}

impl NodeSubtype for sb::SkSVGFilter {
    type Base = sb::SkSVGContainer;
}

impl_default_make!(Filter, sb::C_SkSVGFilter_Make);

impl Filter {
    // TODO: wrap applyProperties()
    // TODO: wrap buildFilterDAG

    skia_svg_macros::attrs! {
        SkSVGFilter => {
            x: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            y: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            width: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            height: Length [get(value) => Length::from_native_ref(value), set(value) => value.into_native()],
            filter_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }],
            primitive_units: BoundingBoxUnits [get(value) => &value.fType, set(value) => sb::SkSVGObjectBoundingBoxUnits { fType: value }]
        }
    }
}
