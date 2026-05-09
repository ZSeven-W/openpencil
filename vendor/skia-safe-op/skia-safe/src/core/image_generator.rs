use std::fmt;

use crate::{prelude::*, yuva_pixmap_info, Data, ImageInfo, Recorder, YUVAPixmapInfo};
use skia_bindings::{self as sb, SkImageGenerator};

pub type ImageGenerator = RefHandle<SkImageGenerator>;
unsafe_send_sync!(ImageGenerator);

impl NativeDrop for SkImageGenerator {
    fn drop(&mut self) {
        unsafe { sb::C_SkImageGenerator_delete(self) }
    }
}

impl fmt::Debug for ImageGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageGenerator")
            .field("unique_id", &self.unique_id())
            .field("info", &self.info())
            .finish()
    }
}

impl ImageGenerator {
    pub fn unique_id(&self) -> u32 {
        self.native().fUniqueID
    }

    pub fn encoded_data(&mut self) -> Option<Data> {
        Data::from_ptr_const(unsafe { sb::C_SkImageGenerator_refEncodedData(self.native_mut()) })
    }

    pub fn info(&self) -> &ImageInfo {
        ImageInfo::from_native_ref(&self.native().fInfo)
    }

    pub fn is_valid(&self, recorder: Option<&mut dyn Recorder>) -> bool {
        unsafe {
            sb::C_SkImageGenerator_isValid(
                self.native(),
                recorder
                    .map(|r| r.as_recorder_ref())
                    .native_ptr_or_null_mut(),
            )
        }
    }

    pub fn is_protected(self) -> bool {
        unsafe { sb::C_SkImageGenerator_isProtected(self.native()) }
    }

    #[must_use]
    pub fn get_pixels(&mut self, info: &ImageInfo, pixels: &mut [u8], row_bytes: usize) -> bool {
        assert!(info.valid_pixels(row_bytes, pixels));
        unsafe {
            self.native_mut()
                .getPixels(info.native(), pixels.as_mut_ptr() as _, row_bytes)
        }
    }

    // TODO: m86: get_pixels(&Pixmap)

    pub fn query_yuva_info(
        &self,
        supported_data_types: &yuva_pixmap_info::SupportedDataTypes,
    ) -> Option<YUVAPixmapInfo> {
        YUVAPixmapInfo::new_if_valid(|info| unsafe {
            self.native()
                .queryYUVAInfo(supported_data_types.native(), info)
        })
    }

    // TODO: getYUVAPlanes()

    pub fn is_texture_generator(&self) -> bool {
        unsafe { sb::C_SkImageGenerator_isTextureGenerator(self.native()) }
    }

    #[deprecated(
        since = "0.64.0",
        note = "Removed, will return `None`. Use Image::deferred_from_encoded_data() or Codec::from_data()"
    )]
    pub fn from_encoded(_encoded: impl Into<Data>) -> Option<Self> {
        debug_assert!(false, "Removed, will return `None` in release builds");
        None
    }
}
