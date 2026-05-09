use std::io;

use crate::{encode, interop::RustWStream, prelude::*, Pixmap};
use skia_bindings as sb;

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FilterFlag: u32 {
        const ZERO = sb::SkPngEncoder_FilterFlag::kZero as _;
        const NONE = sb::SkPngEncoder_FilterFlag::kNone as _;
        const SUB = sb::SkPngEncoder_FilterFlag::kSub as _;
        const UP = sb::SkPngEncoder_FilterFlag::kUp as _;
        const AVG = sb::SkPngEncoder_FilterFlag::kAvg as _;
        const PAETH = sb::SkPngEncoder_FilterFlag::kPaeth as _;
        const ALL = Self::NONE.bits() | Self::SUB.bits() | Self::UP.bits() | Self::AVG.bits() | Self::PAETH.bits();
    }
}
native_transmutable!(sb::SkPngEncoder_FilterFlag, FilterFlag);

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    pub filter_flags: FilterFlag,
    pub z_lib_level: i32,
    pub comments: Vec<encode::Comment>,
    // TODO: fHdrMetadata
    // TODO: If SkGainmapInfo get out of private/ : fGainmap fGainmapInfo
}

impl Default for Options {
    fn default() -> Self {
        Self {
            filter_flags: FilterFlag::ALL,
            z_lib_level: 6,
            comments: vec![],
        }
    }
}

#[deprecated(since = "0.90.0", note = "use encode::Comment")]
pub type Comment = encode::Comment;

pub fn encode<W: io::Write>(pixmap: &Pixmap, writer: &mut W, options: &Options) -> bool {
    let Some(comments) = encode::comments::to_data_table(&options.comments) else {
        return false;
    };

    let mut stream = RustWStream::new(writer);

    unsafe {
        sb::C_SkPngEncoder_Encode(
            stream.stream_mut(),
            pixmap.native(),
            comments.into_ptr(),
            options.filter_flags.into_native(),
            options.z_lib_level,
        )
    }
}

pub fn encode_pixmap(src: &Pixmap, options: &Options) -> Option<crate::Data> {
    crate::Data::from_ptr(unsafe {
        sb::C_SkPngEncoder_EncodePixmap(
            src.native(),
            encode::comments::to_data_table(&options.comments)?.into_ptr(),
            options.filter_flags.into_native(),
            options.z_lib_level,
        )
    })
}

pub fn encode_image<'a>(
    context: impl Into<Option<&'a mut crate::gpu::DirectContext>>,
    img: &crate::Image,
    options: &Options,
) -> Option<crate::Data> {
    crate::Data::from_ptr(unsafe {
        sb::C_SkPngEncoder_EncodeImage(
            context.into().native_ptr_or_null_mut(),
            img.native(),
            encode::comments::to_data_table(&options.comments)?.into_ptr(),
            options.filter_flags.into_native(),
            options.z_lib_level,
        )
    })
}

// TODO: Make
