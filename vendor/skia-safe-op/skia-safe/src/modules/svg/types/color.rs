use std::fmt;

use crate::{prelude::*, Color as SkColor};
use skia_bindings as sb;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ColorKind {
    CurrentColor,
    Color,
}

pub type Fill = Handle<sb::SkSVGColor>;

impl NativeDrop for sb::SkSVGColor {
    fn drop(&mut self) {
        unsafe { sb::C_SkSVGColor_destruct(self) }
    }
}

impl fmt::Debug for Fill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgColor")
            .field("kind", &self.kind())
            .field("color", &self.color())
            .finish()
    }
}

impl Fill {
    pub fn color(&self) -> Option<SkColor> {
        let color = self.native();

        if matches!(color.fType, sb::SkSVGColor_Type::Color) {
            Some(SkColor::new(color.fColor))
        } else {
            None
        }
    }

    pub fn kind(&self) -> ColorKind {
        match self.native().fType {
            sb::SkSVGColor_Type::Color => ColorKind::Color,
            sb::SkSVGColor_Type::CurrentColor | sb::SkSVGColor_Type::ICCColor => {
                ColorKind::CurrentColor
            }
        }
    }

    pub fn current_color() -> Self {
        Self::construct(|uninitialized| unsafe { sb::C_SkSVGColor_Construct(uninitialized) })
    }

    pub fn from_color(color: SkColor) -> Self {
        Self::construct(|uninitialized| unsafe {
            sb::C_SkSVGColor_Construct1(uninitialized, color.into_native())
        })
    }
}
