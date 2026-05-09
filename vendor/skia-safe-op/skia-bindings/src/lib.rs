#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]
// <https://github.com/rust-lang/rust-bindgen/issues/1651>
#![allow(unknown_lints)]
#![allow(deref_nullptr)]
// GrVkBackendContext contains u128 fields on macOS
#![allow(improper_ctypes)]
#![allow(dead_code)]

// The following type aliases are needed because of name mangling changes introduced with clang 18,
// (this works together with `ITEM_RENAMES` in `skia_bindgen.rs`)
type std___1_string_view = std_string_view;
type std___2_string_view = std_string_view;
type std___1_string = std_string;
type std___2_string = std_string;

include!(concat!(env!("OUT_DIR"), "/skia/bindings.rs"));

mod defaults;
#[allow(unused_imports)]
pub use defaults::*;

mod impls;

#[cfg(feature = "textlayout")]
pub mod icu;

#[allow(unused_imports)]
#[doc(hidden)]
#[cfg(feature = "use-system-jpeg-turbo")]
use mozjpeg_sys;
