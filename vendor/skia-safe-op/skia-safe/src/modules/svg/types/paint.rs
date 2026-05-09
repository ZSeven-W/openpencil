use super::color::Fill;
use crate::{prelude::*, Color};
use skia_bindings as sb;
use std::fmt;

pub type Paint = Handle<sb::SkSVGPaint>;

impl NativeDrop for sb::SkSVGPaint {
    fn drop(&mut self) {
        unsafe { sb::C_SkSVGPaint_destruct(self) }
    }
}

impl fmt::Debug for Paint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_color() {
            f.debug_tuple("SvgPaint::Color")
                .field(&self.color().unwrap())
                .finish()
        } else if self.is_current_color() {
            f.debug_tuple("SvgPaint::CurrentColor").finish()
        } else {
            f.debug_tuple("SvgPaint::None").finish()
        }
    }
}

impl Paint {
    pub fn color(&self) -> Option<Color> {
        let paint = self.native();

        if self.is_color() {
            Some(Color::new(paint.fColor.fColor))
        } else {
            None
        }
    }

    pub fn is_current_color(&self) -> bool {
        matches!(self.native().fType, sb::SkSVGPaint_Type::Color)
            && matches!(
                self.native().fColor.fType,
                sb::SkSVGColor_Type::CurrentColor
            )
    }

    pub fn is_color(&self) -> bool {
        matches!(self.native().fType, sb::SkSVGPaint_Type::Color)
            && matches!(self.native().fColor.fType, sb::SkSVGColor_Type::Color)
    }

    pub fn is_none(&self) -> bool {
        matches!(self.native().fType, sb::SkSVGPaint_Type::None)
    }

    pub fn from_color(color: Color) -> Self {
        Self::construct(|uninitialized| unsafe {
            let color = Fill::from_color(color);

            sb::C_SkSVGPaint_Construct1(uninitialized, color.native())
        })
    }

    pub fn current_color() -> Self {
        Self::construct(|uninitialized| unsafe {
            let color = Fill::current_color();

            sb::C_SkSVGPaint_Construct1(uninitialized, color.native())
        })
    }

    pub fn none() -> Self {
        Self::construct(|uninitialized| unsafe { sb::C_SkSVGPaint_Construct(uninitialized) })
    }
}
