use crate::{prelude::*, ColorType, Data, ImageInfo, Pixmap, YUVAInfo, YUVColorSpace};
use skia_bindings::{self as sb, SkYUVAPixmapInfo, SkYUVAPixmaps};
use std::{ffi::c_void, fmt, ptr};
use yuva_pixmap_info::SupportedDataTypes;

/// Data type for Y, U, V, and possibly A channels independent of how values are packed into planes.
pub use yuva_pixmap_info::DataType;
variant_name!(DataType::Float16);

/// [YUVAInfo] combined with per-plane [ColorType]s and row bytes. Fully specifies the [Pixmap]`s
/// for a YUVA image without the actual pixel memory and data.
pub type YUVAPixmapInfo = Handle<SkYUVAPixmapInfo>;
unsafe_send_sync!(YUVAPixmapInfo);

impl NativeDrop for SkYUVAPixmapInfo {
    fn drop(&mut self) {
        unsafe { sb::C_SkYUVAPixmapInfo_destruct(self) }
    }
}

impl NativePartialEq for SkYUVAPixmapInfo {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkYUVAPixmapInfo_equals(self, rhs) }
    }
}

impl fmt::Debug for YUVAPixmapInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plane_infos: Vec<_> = self.plane_infos().collect();
        let row_bytes: Vec<_> = self.row_bytes_iter().collect();
        f.debug_struct("YUVAPixmapInfo")
            .field("yuva_info", &self.yuva_info())
            .field("plane_infos", &plane_infos)
            .field("row_bytes", &row_bytes)
            .field("data_type", &self.data_type())
            .finish()
    }
}

impl YUVAPixmapInfo {
    pub const MAX_PLANES: usize = sb::SkYUVAInfo_kMaxPlanes as _;
    pub const DATA_TYPE_CNT: usize = DataType::Last as _;

    /// Initializes the [YUVAPixmapInfo] from a [YUVAInfo] with per-plane color types and row bytes.
    /// This will return [None] if the colorTypes aren't compatible with the [YUVAInfo] or if a
    /// rowBytes entry is not valid for the plane dimensions and color type. Color type and
    /// row byte values beyond the number of planes in [YUVAInfo] are ignored. All [ColorType]s
    /// must have the same [DataType] or this will return [None].
    ///
    /// If `rowBytes` is [None] then bpp*width is assumed for each plane.
    pub fn new(
        info: &YUVAInfo,
        color_types: &[ColorType],
        row_bytes: Option<&[usize]>,
    ) -> Option<Self> {
        if color_types.len() != info.num_planes() {
            return None;
        }
        if let Some(rb) = row_bytes {
            if rb.len() != color_types.len() {
                return None;
            }
        }
        let mut color_types_array = [ColorType::Unknown; Self::MAX_PLANES];
        color_types_array[..color_types.len()].copy_from_slice(color_types);

        let mut row_bytes_array = [0; Self::MAX_PLANES];
        let row_bytes_ptr = {
            if let Some(row_bytes) = row_bytes {
                row_bytes_array[..row_bytes.len()].copy_from_slice(row_bytes);
                row_bytes_array.as_ptr()
            } else {
                ptr::null()
            }
        };

        let info = unsafe {
            SkYUVAPixmapInfo::new(
                info.native(),
                color_types_array.native().as_ptr(),
                row_bytes_ptr,
            )
        };
        Self::native_is_valid(&info).then(|| Self::from_native_c(info))
    }

    /// Like above but uses [yuva_pixmap_info::default_color_type_for_data_type] to determine each plane's [ColorType]. If
    /// `rowBytes` is [None] then bpp*width is assumed for each plane.
    pub fn from_data_type(
        info: &YUVAInfo,
        data_type: DataType,
        row_bytes: Option<&[usize]>,
    ) -> Option<Self> {
        let mut row_bytes_array = [0; Self::MAX_PLANES];
        let row_bytes_ptr = {
            if let Some(row_bytes) = row_bytes {
                row_bytes_array[..row_bytes.len()].copy_from_slice(row_bytes);
                row_bytes_array.as_ptr()
            } else {
                ptr::null()
            }
        };

        let info = unsafe { SkYUVAPixmapInfo::new1(info.native(), data_type, row_bytes_ptr) };

        Self::native_is_valid(&info).then(|| Self::from_native_c(info))
    }

    pub fn yuva_info(&self) -> &YUVAInfo {
        YUVAInfo::from_native_ref(&self.native().fYUVAInfo)
    }

    pub fn yuv_color_space(&self) -> YUVColorSpace {
        self.yuva_info().yuv_color_space()
    }

    /// The number of [Pixmap] planes.
    pub fn num_planes(&self) -> usize {
        self.yuva_info().num_planes()
    }

    /// The per-YUV`[A]` channel data type.
    pub fn data_type(&self) -> DataType {
        self.native().fDataType
    }

    /// Row bytes for the ith plane. Returns `None` if `i` >= [`Self::num_planes()`] or this
    /// [YUVAPixmapInfo] is invalid.
    pub fn row_bytes(&self, i: usize) -> Option<usize> {
        (i < self.num_planes()).then(|| unsafe {
            sb::C_SkYUVAPixmapInfo_rowBytes(self.native(), i.try_into().unwrap())
        })
    }

    /// Row bytes for all planes.
    pub fn row_bytes_iter(&self) -> impl Iterator<Item = usize> + use<'_> {
        (0..self.num_planes()).map(move |i| self.row_bytes(i).unwrap())
    }

    /// Image info for the ith plane, or `None` if `i` >= [`Self::num_planes()`]
    pub fn plane_info(&self, i: usize) -> Option<&ImageInfo> {
        (i < self.num_planes()).then(|| {
            ImageInfo::from_native_ref(unsafe {
                &*sb::C_SkYUVAPixmapInfo_planeInfo(self.native(), i.try_into().unwrap())
            })
        })
    }

    /// An iterator of all planes' image infos.
    pub fn plane_infos(&self) -> impl Iterator<Item = &ImageInfo> {
        (0..self.num_planes()).map(move |i| self.plane_info(i).unwrap())
    }

    /// Determine size to allocate for all planes. Optionally retrieves the per-plane sizes in
    /// planeSizes if not [None]. If total size overflows will return SIZE_MAX and set all
    /// `plane_sizes` to SIZE_MAX.
    pub fn compute_total_bytes(
        &self,
        plane_sizes: Option<&mut [usize; Self::MAX_PLANES]>,
    ) -> usize {
        unsafe {
            self.native().computeTotalBytes(
                plane_sizes
                    .map(|ps| ps.as_mut_ptr())
                    .unwrap_or(ptr::null_mut()),
            )
        }
    }

    /// Takes an allocation that is assumed to be at least [compute_total_bytes(&self)] in size and
    /// configures the first [numPlanes(&self)] entries in pixmaps array to point into that memory.
    /// The remaining entries of pixmaps are default initialized. Returns [None] if this
    /// [YUVAPixmapInfo] not valid.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn init_pixmaps_from_single_allocation(
        &self,
        memory: *mut c_void,
    ) -> Option<[Pixmap; Self::MAX_PLANES]> {
        // Can't return a Vec<Pixmap> because Pixmaps can't be cloned.
        let mut pixmaps: [Pixmap; Self::MAX_PLANES] = Default::default();
        self.native()
            .initPixmapsFromSingleAllocation(memory, pixmaps[0].native_mut())
            .then_some(pixmaps)
    }

    /// Is this valid and does it use color types allowed by the passed [SupportedDataTypes]?
    pub fn is_supported(&self, data_types: &SupportedDataTypes) -> bool {
        unsafe { self.native().isSupported(data_types.native()) }
    }

    pub(crate) fn new_if_valid(
        set_pixmap_info: impl Fn(&mut SkYUVAPixmapInfo) -> bool,
    ) -> Option<Self> {
        let mut pixmap_info = Self::new_invalid();
        let r = set_pixmap_info(&mut pixmap_info);
        (r && Self::native_is_valid(&pixmap_info))
            .then(|| YUVAPixmapInfo::from_native_c(pixmap_info))
    }

    /// Returns `true` if this has been configured with a non-empty dimensioned [YUVAInfo] with
    /// compatible color types and row bytes.
    fn native_is_valid(info: *const SkYUVAPixmapInfo) -> bool {
        unsafe { sb::C_SkYUVAPixmapInfo_isValid(info) }
    }

    /// Creates a native default instance that is invalid.
    fn new_invalid() -> SkYUVAPixmapInfo {
        construct(|pi| unsafe { sb::C_SkYUVAPixmapInfo_Construct(pi) })
    }
}

/// Helper to store [Pixmap] planes as described by a [YUVAPixmapInfo]. Can be responsible for
/// allocating/freeing memory for pixmaps or use external memory.
pub type YUVAPixmaps = Handle<SkYUVAPixmaps>;
unsafe_send_sync!(YUVAPixmaps);

impl NativeDrop for SkYUVAPixmaps {
    fn drop(&mut self) {
        unsafe { sb::C_SkYUVAPixmaps_destruct(self) }
    }
}

impl NativeClone for SkYUVAPixmaps {
    fn clone(&self) -> Self {
        construct(|pixmaps| unsafe { sb::C_SkYUVAPixmaps_MakeCopy(self, pixmaps) })
    }
}

impl fmt::Debug for YUVAPixmaps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YUVAPixmaps")
            .field("planes", &self.planes())
            .field("yuva_info", &self.yuva_info())
            .field("data_type", &self.data_type())
            .finish()
    }
}

impl YUVAPixmaps {
    pub const MAX_PLANES: usize = YUVAPixmapInfo::MAX_PLANES;

    pub fn recommended_rgba_color_type(dt: DataType) -> ColorType {
        ColorType::from_native_c(unsafe { sb::SkYUVAPixmaps::RecommendedRGBAColorType(dt) })
    }

    /// Allocate space for pixmaps' pixels in the [YUVAPixmaps].
    pub fn allocate(info: &YUVAPixmapInfo) -> Option<Self> {
        Self::try_construct(|pixmaps| unsafe {
            sb::C_SkYUVAPixmaps_Allocate(pixmaps, info.native());
            Self::native_is_valid(pixmaps)
        })
    }

    /// Use storage in [Data] as backing store for pixmaps' pixels. [Data] is retained by the
    /// [YUVAPixmaps].
    pub fn from_data(info: &YUVAPixmapInfo, data: impl Into<Data>) -> Option<Self> {
        Self::try_construct(|pixmaps| unsafe {
            sb::C_SkYUVAPixmaps_FromData(pixmaps, info.native(), data.into().into_ptr());
            Self::native_is_valid(pixmaps)
        })
    }

    /// Use passed in memory as backing store for pixmaps' pixels. Caller must ensure memory remains
    /// allocated while pixmaps are in use. There must be at least
    /// [YUVAPixmapInfo::computeTotalBytes(&self)] allocated starting at memory.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn from_external_memory(info: &YUVAPixmapInfo, memory: *mut c_void) -> Option<Self> {
        Self::try_construct(|pixmaps| {
            sb::C_SkYUVAPixmaps_FromExternalMemory(pixmaps, info.native(), memory);
            Self::native_is_valid(pixmaps)
        })
    }

    /// Wraps existing `Pixmap`s. The [YUVAPixmaps] will have no ownership of the [Pixmap]s' pixel
    /// memory so the caller must ensure it remains valid. Will return [None] if
    /// the [YUVAInfo] isn't compatible with the [Pixmap] array (number of planes, plane dimensions,
    /// sufficient color channels in planes, ...).
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn from_external_pixmaps(
        info: &YUVAInfo,
        pixmaps: &[Pixmap; Self::MAX_PLANES],
    ) -> Option<Self> {
        Self::try_construct(|pms| {
            sb::C_SkYUVAPixmaps_FromExternalPixmaps(pms, info.native(), pixmaps[0].native());
            Self::native_is_valid(pms)
        })
    }

    pub fn yuva_info(&self) -> &YUVAInfo {
        YUVAInfo::from_native_ref(&self.native().fYUVAInfo)
    }

    pub fn data_type(&self) -> DataType {
        self.native().fDataType
    }

    pub fn pixmaps_info(&self) -> YUVAPixmapInfo {
        YUVAPixmapInfo::construct(|info| unsafe {
            sb::C_SkYUVAPixmaps_pixmapsInfo(self.native(), info)
        })
    }

    /// Number of pixmap planes.
    pub fn num_planes(&self) -> usize {
        self.yuva_info().num_planes()
    }

    /// Access the [Pixmap] planes.
    pub fn planes(&self) -> &[Pixmap] {
        unsafe {
            let planes = Pixmap::from_native_ptr(sb::C_SkYUVAPixmaps_planes(self.native()));
            safer::from_raw_parts(planes, self.num_planes())
        }
    }

    /// Get the ith [Pixmap] plane. `Pixmap` will be default initialized if i >= numPlanes.
    pub fn plane(&self, i: usize) -> &Pixmap {
        &self.planes()[i]
    }

    pub(crate) fn native_is_valid(pixmaps: *const SkYUVAPixmaps) -> bool {
        unsafe { sb::C_SkYUVAPixmaps_isValid(pixmaps) }
    }
}

pub mod yuva_pixmap_info {
    use crate::{prelude::*, ColorType};
    use skia_bindings::{self as sb, SkYUVAPixmapInfo_SupportedDataTypes};
    use std::fmt;

    pub use crate::yuva_info::PlaneConfig;
    pub use crate::yuva_info::Subsampling;

    /// Data type for Y, U, V, and possibly A channels independent of how values are packed into
    /// planes.
    pub use skia_bindings::SkYUVAPixmapInfo_DataType as DataType;

    pub type SupportedDataTypes = Handle<SkYUVAPixmapInfo_SupportedDataTypes>;
    unsafe_send_sync!(SupportedDataTypes);

    impl NativeDrop for SkYUVAPixmapInfo_SupportedDataTypes {
        fn drop(&mut self) {
            unsafe { sb::C_SkYUVAPixmapInfo_SupportedDataTypes_destruct(self) }
        }
    }

    impl Default for SupportedDataTypes {
        /// Defaults to nothing supported.
        fn default() -> Self {
            Self::construct(|sdt| unsafe {
                sb::C_SkYUVAPixmapInfo_SupportedDataTypes_Construct(sdt)
            })
        }
    }

    impl fmt::Debug for SupportedDataTypes {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("SupportedDataType")
                .field("data_type_support", &self.native().fDataTypeSupport)
                .finish()
        }
    }

    impl SupportedDataTypes {
        /// All legal combinations of [PlaneConfig] and [DataType] are supported.
        pub fn all() -> Self {
            Self::construct(|sdt| unsafe { sb::C_SkYUVAPixmapInfo_SupportedDataTypes_All(sdt) })
        }

        /// Checks whether there is a supported combination of color types for planes structured
        /// as indicated by [PlaneConfig] with channel data types as indicated by [DataType].
        pub fn supported(&self, pc: PlaneConfig, dt: DataType) -> bool {
            unsafe { sb::C_SkYUVAPixmapInfo_SupportedDataTypes_supported(self.native(), pc, dt) }
        }

        /// Update to add support for pixmaps with `num_channels` channels where each channel is
        /// represented as [DataType].
        pub fn enable_data_type(&mut self, dt: DataType, num_channels: usize) {
            unsafe {
                self.native_mut()
                    .enableDataType(dt, num_channels.try_into().unwrap())
            }
        }
    }

    /// Gets the default [ColorType] to use with `num_channels` channels, each represented as [DataType].
    /// Returns [ColorType::Unknown] if no such color type.
    pub fn default_color_type_for_data_type(dt: DataType, num_channels: usize) -> ColorType {
        ColorType::from_native_c(unsafe {
            sb::C_SkYUVAPixmapInfo_DefaultColorTypeForDataType(dt, num_channels.try_into().unwrap())
        })
    }

    /// If the [ColorType] is supported for YUVA pixmaps this will return the number of YUVA channels
    /// that can be stored in a plane of this color type and what the [DataType] is of those channels.
    /// If the [ColorType] is not supported as a YUVA plane the number of channels is reported as 0
    /// and the [DataType] returned should be ignored.
    pub fn num_channels_and_data_type(color_type: ColorType) -> (usize, DataType) {
        let mut data_type = DataType::Float16;
        let channels = unsafe {
            sb::C_SkYUVAPixmapInfo_NumChannelsAndDataType(color_type.into_native(), &mut data_type)
        };
        (channels.try_into().unwrap(), data_type)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ColorType, YUVAPixmaps};

    #[test]
    fn recommended_color_type() {
        assert_eq!(
            YUVAPixmaps::recommended_rgba_color_type(super::DataType::Float16),
            ColorType::RGBAF16
        );
    }
}
