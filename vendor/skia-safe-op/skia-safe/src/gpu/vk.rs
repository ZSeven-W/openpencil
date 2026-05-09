use std::{ops::Deref, ptr};

use skia_bindings as sb;

mod vulkan_backend_context;
mod vulkan_mutable_texture_state;
mod vulkan_types;

pub use super::ganesh::vk::vk_types::*;
pub use vulkan_backend_context::*;
pub use vulkan_mutable_texture_state::*;
pub use vulkan_types::*;

pub use crate::gpu::ganesh::vk::BackendDrawableInfo;

//
// Additional Vulkan re-exports and definitions.
//

pub use sb::VkBool32 as Bool32;
pub use sb::VkBuffer as Buffer;
pub use sb::VkChromaLocation as ChromaLocation;
pub use sb::VkCommandBuffer as CommandBuffer;
pub use sb::VkComponentMapping as ComponentMapping;
pub use sb::VkComponentSwizzle as ComponentSwizzle;
pub use sb::VkDevice as Device;
pub use sb::VkDeviceMemory as DeviceMemory;
pub use sb::VkDeviceSize as DeviceSize;
pub use sb::VkExtent2D as Extent2D;
pub use sb::VkFilter as Filter;
pub use sb::VkFlags as Flags;
pub use sb::VkFormat as Format;
pub use sb::VkFormatFeatureFlags as FormatFeatureFlags;
pub use sb::VkImage as Image;
pub use sb::VkImageLayout as ImageLayout;
pub use sb::VkImageTiling as ImageTiling;
pub use sb::VkImageUsageFlags as ImageUsageFlags;
pub use sb::VkInstance as Instance;
pub use sb::VkOffset2D as Offset2D;
pub use sb::VkPhysicalDevice as PhysicalDevice;
pub use sb::VkPhysicalDeviceFeatures as PhysicalDeviceFeatures;
pub use sb::VkPhysicalDeviceFeatures2 as PhysicalDeviceFeatures2;
pub use sb::VkQueue as Queue;
pub use sb::VkRect2D as Rect2D;
pub use sb::VkRenderPass as RenderPass;
pub use sb::VkSamplerYcbcrModelConversion as SamplerYcbcrModelConversion;
pub use sb::VkSamplerYcbcrRange as SamplerYcbcrRange;
pub use sb::VkSharingMode as SharingMode;

pub const QUEUE_FAMILY_IGNORED: u32 = !0;

//
// VK_NULL_HANDLE and conversions.
//

#[derive(Debug)]
pub struct NullHandle;
pub const NULL_HANDLE: NullHandle = NullHandle;

#[cfg(target_pointer_width = "64")]
impl From<NullHandle> for Buffer {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

impl From<NullHandle> for CommandBuffer {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

impl From<NullHandle> for Device {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

#[cfg(target_pointer_width = "64")]
impl From<NullHandle> for DeviceMemory {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

impl From<NullHandle> for Instance {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

impl From<NullHandle> for PhysicalDevice {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

#[cfg(target_pointer_width = "64")]
impl From<NullHandle> for Image {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

impl From<NullHandle> for Queue {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

#[cfg(target_pointer_width = "64")]
impl From<NullHandle> for RenderPass {
    fn from(_: NullHandle) -> Self {
        ptr::null_mut()
    }
}

#[cfg(not(target_pointer_width = "64"))]
impl From<NullHandle> for u64 {
    fn from(_: NullHandle) -> Self {
        0
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Version(u32);

impl Version {
    pub fn new(major: usize, minor: usize, patch: usize) -> Self {
        ((((major & 0x3ff) << 22) | ((minor & 0x3ff) << 12) | (patch & 0xfff)) as u32).into()
    }

    pub fn major(&self) -> usize {
        (self.deref() >> 22) as _
    }

    pub fn minor(&self) -> usize {
        ((self.deref() >> 12) & 0x3ff) as _
    }

    pub fn patch(&self) -> usize {
        ((self.deref()) & 0xfff) as _
    }
}

impl From<u32> for Version {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<(usize, usize, usize)> for Version {
    fn from((major, minor, patch): (usize, usize, usize)) -> Self {
        Self::new(major, minor, patch)
    }
}

impl Deref for Version {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
