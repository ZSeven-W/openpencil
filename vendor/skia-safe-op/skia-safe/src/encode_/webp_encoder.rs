use crate::{interop::RustWStream, prelude::*, Pixmap};
use skia_bindings::SkWebpEncoder_Compression;
use std::io;

pub type Compression = SkWebpEncoder_Compression;
variant_name!(Compression::Lossy);

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub compression: Compression,
    pub quality: f32,
    // TODO: ICCProfile
    // TODO: ICCProfileDescription
}

impl Default for Options {
    fn default() -> Self {
        Self {
            compression: Compression::Lossy,
            quality: 100.0,
        }
    }
}

pub fn encode<W: io::Write>(pixmap: &Pixmap, writer: &mut W, options: &Options) -> bool {
    let mut stream = RustWStream::new(writer);
    unsafe {
        skia_bindings::C_SkWebpEncoder_Encode(
            stream.stream_mut(),
            pixmap.native(),
            options.compression,
            options.quality,
        )
    }
}

pub fn encode_image<'a>(
    context: impl Into<Option<&'a mut crate::gpu::DirectContext>>,
    img: &crate::Image,
    options: &Options,
) -> Option<crate::Data> {
    crate::Data::from_ptr(unsafe {
        skia_bindings::C_SkWebpEncoder_EncodeImage(
            context.into().native_ptr_or_null_mut(),
            img.native(),
            options.compression,
            options.quality,
        )
    })
}

// TODO: EncodeAnimated
