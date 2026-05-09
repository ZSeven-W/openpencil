use skia_bindings::{
    self as sb, skgpu_VulkanAlloc, skgpu_VulkanBackendMemory, skgpu_VulkanYcbcrConversionInfo,
};

use crate::{gpu::vk, prelude::*};

#[deprecated(since = "0.76.0", note = "Use BackendMemory")]
pub type GraphicsBackendMemory = skgpu_VulkanBackendMemory;
pub type BackendMemory = skgpu_VulkanBackendMemory;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Alloc {
    pub memory: vk::DeviceMemory,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
    pub flags: AllocFlag,
    pub backend_memory: BackendMemory,
    uses_system_heap: bool,
}
unsafe_send_sync!(Alloc);

native_transmutable!(skgpu_VulkanAlloc, Alloc);

impl Default for Alloc {
    fn default() -> Self {
        Self {
            memory: vk::NULL_HANDLE.into(),
            offset: 0,
            size: 0,
            flags: AllocFlag::empty(),
            backend_memory: 0,
            uses_system_heap: false,
        }
    }
}

impl PartialEq for Alloc {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_VulkanAlloc_Equals(self.native(), other.native()) }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AllocFlag : u32 {
        const NONCOHERENT = sb::skgpu_VulkanAlloc_Flag::kNoncoherent_Flag as _;
        const MAPPABLE = sb::skgpu_VulkanAlloc_Flag::kMappable_Flag as _;
        const LAZILY_ALLOCATED = sb::skgpu_VulkanAlloc_Flag::kLazilyAllocated_Flag as _;
    }
}

impl Alloc {
    /// # Safety
    /// The memory's lifetime is expected to outlive the lifetime of the returned object.
    pub unsafe fn from_device_memory(
        memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
        flags: AllocFlag,
    ) -> Alloc {
        Alloc {
            memory,
            offset,
            size,
            flags,
            backend_memory: 0,
            uses_system_heap: false,
        }
    }
}

// Robustness: All fields turned private in m144, so it's probably best to convert this to a Handle.
#[derive(Copy, Clone, Eq, Debug)]
#[repr(C)]
pub struct YcbcrConversionInfo {
    format: vk::Format,
    external_format: u64,
    ycbcr_model: vk::SamplerYcbcrModelConversion,
    ycbcr_range: vk::SamplerYcbcrRange,
    x_chroma_offset: vk::ChromaLocation,
    y_chroma_offset: vk::ChromaLocation,
    chroma_filter: vk::Filter,
    force_explicit_reconstruction: vk::Bool32,
    components: vk::ComponentMapping,
    sampler_filter_must_match_chroma_filter: bool,
    supports_linear_filter: bool,
}

native_transmutable!(skgpu_VulkanYcbcrConversionInfo, YcbcrConversionInfo);

impl PartialEq for YcbcrConversionInfo {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sb::C_VulkanYcbcrConversionInfo_Equals(self.native(), other.native()) }
    }
}

impl Default for YcbcrConversionInfo {
    fn default() -> Self {
        Self {
            format: vk::Format::UNDEFINED,
            external_format: 0,
            ycbcr_model: vk::SamplerYcbcrModelConversion::RGB_IDENTITY,
            ycbcr_range: vk::SamplerYcbcrRange::ITU_FULL,
            x_chroma_offset: vk::ChromaLocation::COSITED_EVEN,
            y_chroma_offset: vk::ChromaLocation::COSITED_EVEN,
            chroma_filter: vk::Filter::NEAREST,
            force_explicit_reconstruction: 0,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                g: vk::ComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                b: vk::ComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
                a: vk::ComponentSwizzle::VK_COMPONENT_SWIZZLE_IDENTITY,
            },
            sampler_filter_must_match_chroma_filter: true,
            supports_linear_filter: false,
        }
    }
}

impl YcbcrConversionInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_external_format(
        external_format: u64,
        ycbcr_model: vk::SamplerYcbcrModelConversion,
        ycbcr_range: vk::SamplerYcbcrRange,
        x_chroma_offset: vk::ChromaLocation,
        y_chroma_offset: vk::ChromaLocation,
        chroma_filter: vk::Filter,
        force_explicit_reconstruction: vk::Bool32,
        components: vk::ComponentMapping,
        external_format_features: vk::FormatFeatureFlags,
    ) -> Self {
        Self::construct(|ci| unsafe {
            sb::C_VulkanYcbcrConversionInfo_Construct_ExternalFormat(
                ci,
                external_format,
                ycbcr_model,
                ycbcr_range,
                x_chroma_offset,
                y_chroma_offset,
                chroma_filter,
                force_explicit_reconstruction,
                components,
                external_format_features,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_format(
        format: vk::Format,
        ycbcr_model: vk::SamplerYcbcrModelConversion,
        ycbcr_range: vk::SamplerYcbcrRange,
        x_chroma_offset: vk::ChromaLocation,
        y_chroma_offset: vk::ChromaLocation,
        chroma_filter: vk::Filter,
        force_explicit_reconstruction: vk::Bool32,
        components: vk::ComponentMapping,
        format_features: vk::FormatFeatureFlags,
    ) -> Self {
        Self::construct(|ci| unsafe {
            sb::C_VulkanYcbcrConversionInfo_Construct_Format(
                ci,
                format,
                ycbcr_model,
                ycbcr_range,
                x_chroma_offset,
                y_chroma_offset,
                chroma_filter,
                force_explicit_reconstruction,
                components,
                format_features,
            )
        })
    }
    pub fn is_valid(&self) -> bool {
        self.ycbcr_model != vk::SamplerYcbcrModelConversion::RGB_IDENTITY
            || self.has_external_format()
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn has_external_format(&self) -> bool {
        self.external_format != 0
    }

    pub fn external_format(&self) -> u64 {
        self.external_format
    }

    pub fn model(&self) -> vk::SamplerYcbcrModelConversion {
        self.ycbcr_model
    }

    pub fn range(&self) -> vk::SamplerYcbcrRange {
        self.ycbcr_range
    }

    pub fn x_chroma_offset(&self) -> vk::ChromaLocation {
        self.x_chroma_offset
    }

    pub fn y_chroma_offset(&self) -> vk::ChromaLocation {
        self.y_chroma_offset
    }

    pub fn chroma_filter(&self) -> vk::Filter {
        self.chroma_filter
    }

    pub fn force_explicit_reconstruction(&self) -> vk::Bool32 {
        self.force_explicit_reconstruction
    }

    pub fn components(&self) -> vk::ComponentMapping {
        self.components
    }

    pub fn sampler_filter_must_match_chroma_filter(&self) -> bool {
        self.sampler_filter_must_match_chroma_filter
    }

    pub fn supports_linear_filter(&self) -> bool {
        self.supports_linear_filter
    }
}
