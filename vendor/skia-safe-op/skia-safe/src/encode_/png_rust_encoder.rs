use std::io;

use crate::{encode, interop::RustWStream, prelude::*, Pixmap};
use skia_bindings as sb;

pub type CompressionLevel = sb::SkPngRustEncoder_CompressionLevel;
variant_name!(CompressionLevel::Medium);

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    pub compression_level: CompressionLevel,
    pub comments: Vec<encode::Comment>,
}

pub fn encode<W: io::Write>(pixmap: &Pixmap, writer: &mut W, options: &Options) -> bool {
    let Some(comments) = encode::comments::to_data_table(&options.comments) else {
        return false;
    };

    let mut stream = RustWStream::new(writer);

    unsafe {
        sb::C_SkPngRustEncoder_Encode(
            stream.stream_mut(),
            pixmap.native(),
            options.compression_level,
            comments.into_ptr(),
        )
    }
}

// TODO: Wrap `Make`
