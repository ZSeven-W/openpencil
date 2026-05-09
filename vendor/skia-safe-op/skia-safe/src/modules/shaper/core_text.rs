use crate::Shaper;

pub use skia_bindings::SkShapers_CT_LineBreakMode as LineBreakMode;
variant_name!(LineBreakMode::Default);

pub fn core_text(line_break_mode: LineBreakMode) -> Shaper {
    Shaper::new_core_text(line_break_mode)
}
