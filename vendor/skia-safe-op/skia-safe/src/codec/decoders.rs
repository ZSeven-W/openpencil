pub mod bmp_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkBmpDecoder_Decoder(decoder) })
    }
}

pub mod gif_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkGifDecoder_Decoder(decoder) })
    }
}

pub mod ico_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkIcoDecoder_Decoder(decoder) })
    }
}

#[cfg(feature = "jpeg")]
pub mod jpeg_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkJpegDecoder_Decoder(decoder) })
    }
}

pub mod png_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkPngDecoder_Decoder(decoder) })
    }
}

#[cfg(any())]
pub mod png_rust_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkPngRustDecoder_Decoder(decoder) })
    }
}

pub mod wbmp_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkWbmpDecoder_Decoder(decoder) })
    }
}

#[cfg(feature = "webp-decode")]
pub mod webp_decoder {
    use std::{io, result};

    use crate::{codec::codecs::Decoder, codec::Result, Codec};

    pub fn decode_stream(stream: &mut impl io::Read) -> result::Result<Codec, Result> {
        decoder().from_stream(stream)
    }

    pub fn decoder() -> Decoder {
        Decoder::construct(|decoder| unsafe { skia_bindings::C_SkWebpDecoder_Decoder(decoder) })
    }
}
