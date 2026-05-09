//! Tests for the various image encoder and decoders skia-safe supports by default.
use std::io;

use skia_safe::{
    codec::{self, codecs::Decoder},
    Bitmap, Codec, Data, EncodedImageFormat,
};

/// The supported encoders.
const STANDARD_ENCODERS: &[EncodedImageFormat] = &[
    #[cfg(feature = "jpeg")]
    EncodedImageFormat::JPEG,
    EncodedImageFormat::PNG,
];

/// The supported decoders.
const STANDARD_DECODERS: &[EncodedImageFormat] = &[
    EncodedImageFormat::BMP,
    EncodedImageFormat::GIF,
    EncodedImageFormat::ICO,
    #[cfg(feature = "jpeg")]
    EncodedImageFormat::JPEG,
    EncodedImageFormat::PNG,
    EncodedImageFormat::WBMP,
];

fn supported_encoders() -> Vec<EncodedImageFormat> {
    let mut r = STANDARD_ENCODERS.to_vec();
    if cfg!(feature = "webp-encode") {
        r.push(EncodedImageFormat::WEBP);
    }
    r
}

fn supported_decoders() -> Vec<EncodedImageFormat> {
    let mut r = STANDARD_DECODERS.to_vec();
    if cfg!(feature = "webp-decode") {
        r.push(EncodedImageFormat::WEBP);
    }
    r
}

#[test]
fn test_supported_encoders() {
    const DIM: i32 = 16;

    let mut bitmap = Bitmap::new();
    assert!(bitmap.try_alloc_n32_pixels((DIM, DIM), true));

    let supported: Vec<EncodedImageFormat> = ALL
        .iter()
        .copied()
        .filter(|format| bitmap.encode(*format, 100).is_some())
        .collect();

    assert_eq!(supported, supported_encoders());
}

#[test]
fn test_supported_decoders() {
    let supported: Vec<EncodedImageFormat> = DECODER_TESTS
        .iter()
        .filter(|(format, decoder_f, bytes)| {
            test_decoder(decoder_f(), bytes);
            let data = Data::new_copy(bytes);
            if let Some(codec) = codec::Codec::from_data(data) {
                codec.encoded_format() == *format
            } else {
                false
            }
        })
        .map(|(format, ..)| *format)
        .collect();

    assert_eq!(supported, supported_decoders());
}

#[test]
fn test_from_stream() {
    let all_decoders: Vec<_> = DECODER_TESTS
        .iter()
        .map(|(_, decoder_fn, _)| decoder_fn())
        .collect();

    for (format, _, bytes) in DECODER_TESTS {
        let mut cursor = io::Cursor::new(*bytes);
        match Codec::from_stream(&mut cursor, &all_decoders, None) {
            Ok(codec) => {
                assert_eq!(codec.encoded_format(), *format)
            }
            Err(err) => {
                panic!("Stream decoding of {format:?} failed: {err:?}");
            }
        }
    }
}

type DecoderTest = (EncodedImageFormat, fn() -> Decoder, &'static [u8]);

// image files copied from skia/resources/images
const DECODER_TESTS: &[DecoderTest] = &[
    (
        EncodedImageFormat::BMP,
        codec::bmp_decoder::decoder,
        include_bytes!("images/randPixels.bmp"),
    ),
    (
        EncodedImageFormat::GIF,
        codec::gif_decoder::decoder,
        include_bytes!("images/box.gif"),
    ),
    (
        EncodedImageFormat::ICO,
        codec::ico_decoder::decoder,
        include_bytes!("images/color_wheel.ico"),
    ),
    #[cfg(feature = "jpeg")]
    (
        EncodedImageFormat::JPEG,
        codec::jpeg_decoder::decoder,
        include_bytes!("images/color_wheel.jpg"),
    ),
    (
        EncodedImageFormat::PNG,
        codec::png_decoder::decoder,
        include_bytes!("images/mandrill_16.png"),
    ),
    (
        EncodedImageFormat::WBMP,
        codec::wbmp_decoder::decoder,
        include_bytes!("images/mandrill.wbmp"),
    ),
    #[cfg(feature = "webp-decode")]
    (
        EncodedImageFormat::WEBP,
        codec::webp_decoder::decoder,
        include_bytes!("images/color_wheel.webp"),
    ),
];

fn test_decoder(decoder: Decoder, bytes: &[u8]) {
    assert!(decoder.is_format(bytes));
    let stream = &mut io::Cursor::new(bytes);
    let codec = decoder
        .from_stream(stream)
        .expect("decoder.from_stream returned an error");
    let d = codec.dimensions();
    assert!(d.width > 0 && d.height > 0);
}

/// An exhaustive match for proving that we test all formats defined in Skia.
/// If the match is not exhaustive anymore, update [ALL] below.
#[test]
fn test_encoder_enum() {
    match EncodedImageFormat::GIF {
        EncodedImageFormat::BMP => {}
        EncodedImageFormat::GIF => {}
        EncodedImageFormat::ICO => {}
        EncodedImageFormat::JPEG => {}
        EncodedImageFormat::PNG => {}
        EncodedImageFormat::WBMP => {}
        EncodedImageFormat::WEBP => {}
        EncodedImageFormat::PKM => {}
        EncodedImageFormat::KTX => {}
        EncodedImageFormat::ASTC => {}
        EncodedImageFormat::DNG => {}
        EncodedImageFormat::HEIF => {}
        EncodedImageFormat::AVIF => {}
        EncodedImageFormat::JPEGXL => {}
    }
}

const ALL: &[EncodedImageFormat] = &[
    EncodedImageFormat::BMP,
    EncodedImageFormat::GIF,
    EncodedImageFormat::ICO,
    EncodedImageFormat::JPEG,
    EncodedImageFormat::PNG,
    EncodedImageFormat::WBMP,
    EncodedImageFormat::WEBP,
    EncodedImageFormat::PKM,
    EncodedImageFormat::KTX,
    EncodedImageFormat::ASTC,
    EncodedImageFormat::DNG,
    EncodedImageFormat::HEIF,
    EncodedImageFormat::AVIF,
];
