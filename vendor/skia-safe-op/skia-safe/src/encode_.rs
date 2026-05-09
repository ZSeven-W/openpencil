use crate::{Bitmap, EncodedImageFormat, Pixmap};

#[cfg(feature = "jpeg")]
pub mod jpeg_encoder;
pub mod png_encoder;
// TODO: May support with an optional feature, gn flag `skia_use_rust_png_encode`.
// pub mod png_rust_encoder;
#[cfg(feature = "webp-encode")]
pub mod webp_encoder;

impl Pixmap<'_> {
    pub fn encode(
        &self,
        format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<Vec<u8>> {
        crate::encode::pixmap(self, format, quality)
    }
}

impl Bitmap {
    pub fn encode(
        &self,
        format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<Vec<u8>> {
        crate::encode::bitmap(self, format, quality)
    }
}

impl crate::Image {
    pub fn encode<'a>(
        &self,
        context: impl Into<Option<&'a mut crate::gpu::DirectContext>>,
        format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<crate::Data> {
        crate::encode::image(context, self, format, quality)
    }
}

pub mod encode {
    #[cfg(feature = "jpeg")]
    use super::jpeg_encoder;
    use super::png_encoder;
    use crate::{Bitmap, EncodedImageFormat, Pixmap};

    pub fn pixmap(
        bitmap: &Pixmap,
        format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<Vec<u8>> {
        let mut data = Vec::new();
        let quality = quality.into().unwrap_or(100).clamp(0, 100);
        let _ = quality;
        match format {
            #[cfg(feature = "jpeg")]
            EncodedImageFormat::JPEG => {
                let opts = jpeg_encoder::Options {
                    quality,
                    ..jpeg_encoder::Options::default()
                };
                jpeg_encoder::encode(bitmap, &mut data, &opts)
            }
            EncodedImageFormat::PNG => {
                let opts = png_encoder::Options::default();
                png_encoder::encode(bitmap, &mut data, &opts)
            }
            #[cfg(feature = "webp-encode")]
            EncodedImageFormat::WEBP => {
                use super::webp_encoder;
                let mut opts = webp_encoder::Options::default();
                if quality == 100 {
                    opts.compression = webp_encoder::Compression::Lossless;
                    opts.quality = 75.0;
                } else {
                    opts.compression = webp_encoder::Compression::Lossy;
                    opts.quality = quality as _;
                }
                webp_encoder::encode(bitmap, &mut data, &opts)
            }
            _ => false,
        }
        .then_some(data)
    }

    pub fn bitmap(
        bitmap: &Bitmap,
        format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<Vec<u8>> {
        let pixels = bitmap.peek_pixels()?;
        pixmap(&pixels, format, quality)
    }

    pub fn image<'a>(
        context: impl Into<Option<&'a mut crate::gpu::DirectContext>>,
        image: &crate::Image,
        image_format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<crate::Data> {
        let quality = quality.into().unwrap_or(100).clamp(0, 100);
        let _ = quality;
        match image_format {
            #[cfg(feature = "jpeg")]
            EncodedImageFormat::JPEG => {
                let opts = jpeg_encoder::Options {
                    quality,
                    ..jpeg_encoder::Options::default()
                };
                jpeg_encoder::encode_image(context, image, &opts)
            }
            EncodedImageFormat::PNG => {
                let opts = png_encoder::Options::default();
                png_encoder::encode_image(context, image, &opts)
            }
            #[cfg(feature = "webp-encode")]
            EncodedImageFormat::WEBP => {
                use super::webp_encoder;
                let mut opts = webp_encoder::Options::default();
                if quality == 100 {
                    opts.compression = webp_encoder::Compression::Lossless;
                    opts.quality = 75.0;
                } else {
                    opts.compression = webp_encoder::Compression::Lossy;
                    opts.quality = quality as _;
                }
                webp_encoder::encode_image(context, image, &opts)
            }
            _ => None,
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Comment {
        pub keyword: String,
        pub text: String,
    }

    impl Comment {
        pub fn new(keyword: impl Into<String>, text: impl Into<String>) -> Self {
            Self {
                keyword: keyword.into(),
                text: text.into(),
            }
        }
    }

    pub(crate) mod comments {
        use std::ffi::CString;

        use crate::DataTable;

        use super::Comment;

        pub fn to_data_table(comments: &[Comment]) -> Option<DataTable> {
            let mut comments_c = Vec::with_capacity(comments.len() * 2);
            for comment in comments {
                comments_c.push(CString::new(comment.keyword.as_str()).ok()?);
                comments_c.push(CString::new(comment.text.as_str()).ok()?);
            }
            let slices: Vec<&[u8]> = comments_c.iter().map(|c| c.as_bytes_with_nul()).collect();
            Some(DataTable::from_slices(&slices))
        }
    }
}
