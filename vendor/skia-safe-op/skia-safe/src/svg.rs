pub mod canvas;

pub use self::canvas::Canvas;

#[cfg(feature = "svg")]
pub use crate::modules::svg::*;
