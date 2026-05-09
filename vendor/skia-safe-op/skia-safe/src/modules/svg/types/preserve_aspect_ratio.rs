use skia_bindings as sb;

pub type Align = sb::SkSVGPreserveAspectRatio_Align;
variant_name!(Align::XMidYMax);
pub type Scale = sb::SkSVGPreserveAspectRatio_Scale;
variant_name!(Scale::Slice);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PreserveAspectRatio {
    pub align: Align,
    pub scale: Scale,
}

impl PreserveAspectRatio {
    pub fn new(align: Align, scale: Scale) -> Self {
        Self { align, scale }
    }
}

native_transmutable!(sb::SkSVGPreserveAspectRatio, PreserveAspectRatio);
