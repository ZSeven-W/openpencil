//! Desktop font startup delegates to the shared opt-in native bundle.

pub use op_host_native::bundled_design_fonts::register;

#[cfg(test)]
#[path = "bundled_fonts_tests.rs"]
mod tests;
