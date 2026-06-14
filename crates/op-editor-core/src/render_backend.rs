//! Compatibility re-exports for the widget painter facade.
//!
//! The implementation moved to `jian-widgets` as part of the
//! cross-platform component-library extraction. Existing OP code keeps
//! importing this module during migration; new shared widget code should
//! import from `jian_widgets` directly.

pub use jian_widgets::geometry::{Color, Point2D, Rect};
pub use jian_widgets::painter::{
    ImageAdjustments, ImageDrawMode, Painter as RenderBackend, TextLayout,
};
