use std::fmt;

use crate::interop;
use skia_bindings as sb;

pub type InputType = sb::SkSVGFeInputType_Type;
variant_name!("InputType::SourceGraphic");

#[repr(C)]
#[derive(Clone)]
pub struct Input {
    kind: InputType,
    id: interop::String,
}
native_transmutable!(sb::SkSVGFeInputType, Input);

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgFeInput")
            .field("kind", &self.kind())
            .field("id", &self.id())
            .finish()
    }
}

impl Input {
    pub fn kind(&self) -> InputType {
        self.kind
    }

    pub fn id(&self) -> Option<&str> {
        if self.kind == InputType::FilterPrimitiveReference {
            Some(self.id.as_str())
        } else {
            None
        }
    }

    pub fn source_graphic() -> Self {
        Self {
            kind: InputType::SourceGraphic,
            id: interop::String::default(),
        }
    }

    pub fn source_alpha() -> Self {
        Self {
            kind: InputType::SourceAlpha,
            id: interop::String::default(),
        }
    }

    pub fn background_image() -> Self {
        Self {
            kind: InputType::BackgroundImage,
            id: interop::String::default(),
        }
    }

    pub fn background_alpha() -> Self {
        Self {
            kind: InputType::BackgroundAlpha,
            id: interop::String::default(),
        }
    }

    pub fn fill_paint() -> Self {
        Self {
            kind: InputType::FillPaint,
            id: interop::String::default(),
        }
    }

    pub fn stroke_paint() -> Self {
        Self {
            kind: InputType::StrokePaint,
            id: interop::String::default(),
        }
    }

    pub fn unspecified() -> Self {
        Self {
            kind: InputType::Unspecified,
            id: interop::String::default(),
        }
    }

    pub fn new<T: AsRef<str>>(id: T) -> Self {
        Self {
            kind: InputType::FilterPrimitiveReference,
            id: interop::String::from_str(id.as_ref()),
        }
    }
}
