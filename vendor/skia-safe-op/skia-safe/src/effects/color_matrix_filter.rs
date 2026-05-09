use crate::{color_filters, Color, ColorFilter};

impl ColorFilter {
    #[deprecated(since = "0.56.0", note = "Use color_filters::lighting()")]
    pub fn new_lighting(mul: impl Into<Color>, add: impl Into<Color>) -> Option<Self> {
        color_filters::lighting(mul, add)
    }
}

#[deprecated(since = "0.56.0", note = "Use color_filters::lighting()")]
pub fn new_lighting(mul: impl Into<Color>, add: impl Into<Color>) -> Option<ColorFilter> {
    color_filters::lighting(mul, add)
}
