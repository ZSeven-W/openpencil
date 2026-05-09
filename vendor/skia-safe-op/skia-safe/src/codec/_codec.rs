use std::{
    ffi::{self, CStr},
    fmt, io,
    marker::PhantomData,
    mem, ptr, result,
};

use skia_bindings::{self as sb, SkCodec, SkCodec_FrameInfo, SkCodec_Options};

use super::codec_animation;
use crate::{
    interop::RustStream, prelude::*, yuva_pixmap_info::SupportedDataTypes, AlphaType, Data,
    EncodedImageFormat, EncodedOrigin, IRect, ISize, Image, ImageInfo, Pixmap, YUVAPixmapInfo,
    YUVAPixmaps,
};

pub use sb::SkCodec_Result as Result;
variant_name!(Result::IncompleteInput);

// TODO: implement Display

pub fn result_to_string(result: Result) -> &'static str {
    unsafe { CStr::from_ptr(skia_bindings::SkCodec_ResultToString(result)) }
        .to_str()
        .unwrap()
}

pub use sb::SkCodec_SelectionPolicy as SelectionPolicy;
variant_name!(SelectionPolicy::PreferStillImage);

pub use sb::SkCodec_ZeroInitialized as ZeroInitialized;
variant_name!(ZeroInitialized::Yes);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Options {
    pub zero_initialized: ZeroInitialized,
    pub subset: Option<IRect>,
    pub frame_index: usize,
    pub prior_frame: Option<usize>,
    pub max_decode_memory: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            zero_initialized: ZeroInitialized::No,
            subset: None,
            frame_index: 0,
            prior_frame: None,
            max_decode_memory: None,
        }
    }
}

pub const NO_FRAME: i32 = sb::SkCodec_kNoFrame;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct FrameInfo {
    pub required_frame: i32,
    pub duration: i32,
    pub fully_received: bool,
    pub alpha_type: AlphaType,
    pub has_alpha_within_bounds: bool,
    pub disposal_method: codec_animation::DisposalMethod,
    pub blend: codec_animation::Blend,
    pub rect: IRect,
}

native_transmutable!(SkCodec_FrameInfo, FrameInfo);

impl Default for FrameInfo {
    fn default() -> Self {
        Self::construct(|frame_info| unsafe { sb::C_SkFrameInfo_Construct(frame_info) })
    }
}

pub use sb::SkCodec_SkScanlineOrder as ScanlineOrder;
variant_name!(ScanlineOrder::BottomUp);

pub use sb::SkCodec_IsAnimated as IsAnimated;
variant_name!(IsAnimated::Yes);

pub struct Codec<'a> {
    inner: RefHandle<SkCodec>,
    pd: PhantomData<&'a mut dyn io::Read>,
}

impl NativeDrop for SkCodec {
    fn drop(&mut self) {
        unsafe { sb::C_SkCodec_delete(self) }
    }
}

impl fmt::Debug for Codec<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Codec")
            .field("info", &self.info())
            .field("dimensions", &self.dimensions())
            .field("bounds", &self.bounds())
            .field("origin", &self.origin())
            .field("encoded_format", &self.encoded_format())
            .field("scanline_order", &self.scanline_order())
            .field("next_scanline", &self.next_scanline())
            .finish()
    }
}

impl Codec<'_> {
    pub fn from_stream<'a, T: io::Read + io::Seek>(
        stream: &'a mut T,
        decoders: &[codecs::Decoder],
        selection_policy: impl Into<Option<SelectionPolicy>>,
    ) -> result::Result<Codec<'a>, Result> {
        let stream = RustStream::new_seekable(stream);
        let mut result = Result::Unimplemented;
        let codec = unsafe {
            sb::C_SkCodec_MakeFromStream(
                // Transfer ownership of the SkStream to the Codec.
                stream.into_native(),
                decoders.as_ptr() as _,
                decoders.len(),
                &mut result,
                selection_policy
                    .into()
                    .unwrap_or(SelectionPolicy::PreferStillImage),
            )
        };
        if result != Result::Success {
            return Err(result);
        }
        Ok(Codec::from_ptr(codec).expect("Codec is null"))
    }

    // TODO: wrap from_data with SkPngChunkReader

    // TODO: Deprecated in Skia
    pub fn from_data(data: impl Into<Data>) -> Option<Codec<'static>> {
        Self::from_ptr(unsafe { sb::C_SkCodec_MakeFromData(data.into().into_ptr()) })
    }

    pub fn from_data_with_decoders(
        data: impl Into<Data>,
        decoders: &[codecs::Decoder],
    ) -> Option<Codec<'static>> {
        Self::from_ptr(unsafe {
            sb::C_SkCodec_MakeFromData2(
                data.into().into_ptr(),
                decoders.as_ptr() as _,
                decoders.len(),
            )
        })
    }

    pub fn info(&self) -> ImageInfo {
        let mut info = ImageInfo::default();
        unsafe { sb::C_SkCodec_getInfo(self.native(), info.native_mut()) };
        info
    }

    pub fn dimensions(&self) -> ISize {
        ISize::from_native_c(unsafe { sb::C_SkCodec_dimensions(self.native()) })
    }

    pub fn bounds(&self) -> IRect {
        IRect::construct(|r| unsafe { sb::C_SkCodec_bounds(self.native(), r) })
    }

    // TODO: getICCProfile()
    // TODO: getHdrMetadata()

    pub fn has_high_bit_depth_encoded_data(&self) -> bool {
        unsafe { sb::C_SkCodec_hasHighBitDepthEncodedData(self.native()) }
    }

    pub fn origin(&self) -> EncodedOrigin {
        EncodedOrigin::from_native_c(unsafe { sb::C_SkCodec_getOrigin(self.native()) })
    }

    pub fn get_scaled_dimensions(&self, desired_scale: f32) -> ISize {
        ISize::from_native_c(unsafe {
            sb::C_SkCodec_getScaledDimensions(self.native(), desired_scale)
        })
    }

    pub fn valid_subset(&self, desired_subset: impl AsRef<IRect>) -> Option<IRect> {
        let mut desired_subset = *desired_subset.as_ref();
        unsafe { sb::C_SkCodec_getValidSubset(self.native(), desired_subset.native_mut()) }
            .then_some(desired_subset)
    }

    pub fn encoded_format(&self) -> EncodedImageFormat {
        unsafe { sb::C_SkCodec_getEncodedFormat(self.native()) }
    }

    pub fn get_pixels_with_options(
        &mut self,
        info: &ImageInfo,
        pixels: &mut [u8],
        row_bytes: usize,
        options: Option<&Options>,
    ) -> Result {
        assert_eq!(pixels.len(), info.compute_byte_size(row_bytes));
        unsafe {
            let native_options = options.map(|options| Self::native_options(options));
            self.native_mut().getPixels(
                info.native(),
                pixels.as_mut_ptr() as *mut _,
                row_bytes,
                native_options.as_ptr_or_null(),
            )
        }
    }

    #[deprecated(
        since = "0.33.1",
        note = "Use the safe variant get_pixels_with_options()."
    )]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_pixels(
        &mut self,
        info: &ImageInfo,
        pixels: *mut ffi::c_void,
        row_bytes: usize,
    ) -> Result {
        self.native_mut()
            .getPixels(info.native(), pixels, row_bytes, ptr::null())
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_pixels_to_pixmap(
        &mut self,
        pixmap: &Pixmap,
        options: Option<&Options>,
    ) -> Result {
        let native_options = options.map(|options| Self::native_options(options));
        self.native_mut().getPixels(
            pixmap.info().native(),
            pixmap.writable_addr(),
            pixmap.row_bytes(),
            native_options.as_ptr_or_null(),
        )
    }

    unsafe fn native_options(options: &Options) -> SkCodec_Options {
        SkCodec_Options {
            fZeroInitialized: options.zero_initialized,
            fSubset: options.subset.native().as_ptr_or_null(),
            fFrameIndex: options.frame_index.try_into().unwrap(),
            fPriorFrame: match options.prior_frame {
                None => sb::SkCodec_kNoFrame,
                Some(frame) => frame.try_into().expect("invalid prior frame"),
            },
            fMaxDecodeMemory: options.max_decode_memory.unwrap_or(0),
        }
    }

    pub fn get_image<'a>(
        &mut self,
        info: impl Into<Option<ImageInfo>>,
        options: impl Into<Option<&'a Options>>,
    ) -> std::result::Result<Image, Result> {
        let info = info.into().unwrap_or_else(|| self.info());
        let options = options
            .into()
            .map(|options| unsafe { Self::native_options(options) });
        let mut result = Result::InternalError;
        Image::from_ptr(unsafe {
            sb::C_SkCodec_getImage(
                self.native_mut(),
                info.native(),
                options.as_ptr_or_null(),
                &mut result,
            )
        })
        .ok_or(result)
    }

    pub fn query_yuva_info(
        &self,
        supported_data_types: &SupportedDataTypes,
    ) -> Option<YUVAPixmapInfo> {
        YUVAPixmapInfo::new_if_valid(|pixmap_info| unsafe {
            self.native()
                .queryYUVAInfo(supported_data_types.native(), pixmap_info)
        })
    }

    pub fn get_yuva_planes(&mut self, pixmaps: &YUVAPixmaps) -> Result {
        unsafe { self.native_mut().getYUVAPlanes(pixmaps.native()) }
    }

    pub fn start_incremental_decode<'a>(
        &mut self,
        dst_info: &ImageInfo,
        dst: &mut [u8],
        row_bytes: usize,
        options: impl Into<Option<&'a Options>>,
    ) -> Result {
        if !dst_info.valid_pixels(row_bytes, dst) {
            return Result::InvalidParameters;
        }
        let options = options
            .into()
            .map(|options| unsafe { Self::native_options(options) });
        unsafe {
            self.native_mut().startIncrementalDecode(
                dst_info.native(),
                dst.as_mut_ptr() as _,
                row_bytes,
                options.as_ptr_or_null(),
            )
        }
    }

    pub fn incremental_decode(&mut self) -> (Result, Option<usize>) {
        let mut rows_decoded = Default::default();
        let r = unsafe { sb::C_SkCodec_incrementalDecode(self.native_mut(), &mut rows_decoded) };
        if r == Result::IncompleteInput {
            (r, Some(rows_decoded.try_into().unwrap()))
        } else {
            (r, None)
        }
    }

    pub fn start_scanline_decode<'a>(
        &mut self,
        dst_info: &ImageInfo,
        options: impl Into<Option<&'a Options>>,
    ) -> Result {
        let options = options
            .into()
            .map(|options| unsafe { Self::native_options(options) });
        unsafe {
            self.native_mut()
                .startScanlineDecode(dst_info.native(), options.as_ptr_or_null())
        }
    }

    pub fn get_scanlines(&mut self, dst: &mut [u8], count_lines: usize, row_bytes: usize) -> usize {
        assert!(mem::size_of_val(dst) >= count_lines * row_bytes);
        unsafe {
            self.native_mut().getScanlines(
                dst.as_mut_ptr() as _,
                count_lines.try_into().unwrap(),
                row_bytes,
            )
        }
        .try_into()
        .unwrap()
    }

    pub fn skip_scanlines(&mut self, count_lines: usize) -> bool {
        unsafe {
            self.native_mut()
                .skipScanlines(count_lines.try_into().unwrap())
        }
    }

    pub fn scanline_order(&self) -> ScanlineOrder {
        unsafe { sb::C_SkCodec_getScanlineOrder(self.native()) }
    }

    pub fn next_scanline(&self) -> i32 {
        unsafe { sb::C_SkCodec_nextScanline(self.native()) }
    }

    pub fn outbound_scanline(&self, input_scanline: i32) -> i32 {
        unsafe { self.native().outputScanline(input_scanline) }
    }

    pub fn get_frame_count(&mut self) -> usize {
        unsafe { sb::C_SkCodec_getFrameCount(self.native_mut()) }
            .try_into()
            .unwrap()
    }

    pub fn get_frame_info(&mut self, index: usize) -> Option<FrameInfo> {
        let mut info = FrameInfo::default();
        unsafe {
            sb::C_SkCodec_getFrameInfo(
                self.native_mut(),
                index.try_into().unwrap(),
                info.native_mut(),
            )
        }
        .then_some(info)
    }

    pub fn get_repetition_count(&mut self) -> Option<usize> {
        const REPETITION_COUNT_INFINITE: i32 = -1;
        let count = unsafe { sb::C_SkCodec_getRepetitionCount(self.native_mut()) };
        if count != REPETITION_COUNT_INFINITE {
            Some(count.try_into().unwrap())
        } else {
            None
        }
    }

    pub fn is_animated(&mut self) -> IsAnimated {
        unsafe { sb::C_SkCodec_isAnimated(self.native_mut()) }
    }

    // TODO: Register

    fn native(&self) -> &SkCodec {
        self.inner.native()
    }

    fn native_mut(&mut self) -> &mut SkCodec {
        self.inner.native_mut()
    }

    pub(crate) fn from_ptr<'a>(codec: *mut SkCodec) -> Option<Codec<'a>> {
        RefHandle::from_ptr(codec).map(|inner| Codec {
            inner,
            pd: PhantomData,
        })
    }
}

pub mod codecs {
    use std::{fmt, io, ptr, result, str};

    use skia_bindings::{self as sb, SkCodecs_Decoder};

    use super::Result;
    use crate::{interop::RustStream, prelude::*, AlphaType, Codec, Image};

    pub type Decoder = Handle<SkCodecs_Decoder>;
    unsafe_send_sync!(Decoder);

    impl fmt::Debug for Decoder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Decoder").field("id", &self.id()).finish()
        }
    }

    impl NativeDrop for SkCodecs_Decoder {
        fn drop(&mut self) {
            unsafe { sb::C_SkCodecs_Decoder_destruct(self) }
        }
    }

    impl NativeClone for SkCodecs_Decoder {
        fn clone(&self) -> Self {
            construct(|d| unsafe { sb::C_SkCodecs_Decoder_CopyConstruct(d, self) })
        }
    }

    impl Decoder {
        pub fn id(&self) -> &'static str {
            let mut len: usize = 0;
            let ptr = unsafe { sb::C_SkCodecs_Decoder_getId(self.native(), &mut len) };
            let chars = unsafe { safer::from_raw_parts(ptr as _, len) };
            str::from_utf8(chars).expect("Invalid UTF-8 decoder id")
        }

        pub fn is_format(&self, data: &[u8]) -> bool {
            unsafe {
                (self.native().isFormat.expect("Decoder::isFormat is null"))(
                    data.as_ptr() as _,
                    data.len(),
                )
            }
        }

        pub fn from_stream<'a>(
            &self,
            stream: &'a mut impl io::Read,
        ) -> result::Result<Codec<'a>, Result> {
            let stream = RustStream::new(stream);
            let mut result = Result::Unimplemented;
            let codec = unsafe {
                sb::C_SkCodecs_Decoder_MakeFromStream(
                    self.native(),
                    // Transfer ownership of the SkStream to the Codec.
                    stream.into_native(),
                    &mut result,
                    ptr::null_mut(),
                )
            };
            if result != Result::Success {
                return Err(result);
            }
            Ok(Codec::from_ptr(codec).expect("Codec is null"))
        }
    }

    // TODO: wrap Register()

    pub fn deferred_image(
        codec: Codec<'_>,
        alpha_type: impl Into<Option<AlphaType>>,
    ) -> Option<Borrows<'_, Image>> {
        let alpha_type: Option<AlphaType> = alpha_type.into();
        // Even though codec is getting consumed / moved here, we need to preserve the borrow of the
        // input stream.
        Image::from_ptr(unsafe {
            sb::C_SkCodecs_DeferredImage(codec.inner.into_ptr(), alpha_type.as_ptr_or_null())
        })
        .map(|h| unsafe { Borrows::unchecked_new(h) })
    }
}
