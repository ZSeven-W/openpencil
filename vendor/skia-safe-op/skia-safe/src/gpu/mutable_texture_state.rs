use std::fmt;

use skia_bindings::{self as sb, skgpu_MutableTextureState, SkRefCntBase};

use super::BackendApi;
use crate::prelude::*;

pub type MutableTextureState = RCHandle<skgpu_MutableTextureState>;
unsafe_send_sync!(MutableTextureState);

impl NativeRefCountedBase for skgpu_MutableTextureState {
    type Base = SkRefCntBase;
}

impl Default for MutableTextureState {
    fn default() -> Self {
        MutableTextureState::from_ptr(unsafe { sb::C_MutableTextureState_Construct() }).unwrap()
    }
}

impl fmt::Debug for MutableTextureState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = f.debug_struct("MutableTextureState");
        #[cfg(feature = "vulkan")]
        {
            str.field(
                "image_layout",
                &crate::gpu::vk::mutable_texture_states::get_vk_image_layout(self),
            )
            .field(
                "queue_family_index",
                &crate::gpu::vk::mutable_texture_states::get_vk_queue_family_index(self),
            );
        }
        str.field("backend", &self.backend()).finish()
    }
}

impl MutableTextureState {
    pub fn copied(&self) -> Self {
        MutableTextureState::from_ptr(unsafe {
            sb::C_MutableTextureState_CopyConstruct(self.native())
        })
        .unwrap()
    }

    #[cfg(feature = "vulkan")]
    #[deprecated(
        since = "0.72.0",
        note = "use gpu::vk::mutable_texture_states::new_vulkan()"
    )]
    pub fn new_vk(layout: crate::gpu::vk::ImageLayout, queue_family_index: u32) -> Self {
        crate::gpu::vk::mutable_texture_states::new_vulkan(layout, queue_family_index)
    }

    #[cfg(feature = "vulkan")]
    #[deprecated(
        since = "0.72.0",
        note = "use gpu::vk::mutable_texture_states::get_vk_image_layout()"
    )]
    pub fn vk_image_layout(&self) -> sb::VkImageLayout {
        crate::gpu::vk::mutable_texture_states::get_vk_image_layout(self)
    }

    #[cfg(feature = "vulkan")]
    #[deprecated(
        since = "0.72.0",
        note = "use gpu::vk::mutable_texture_states::get_vk_queue_family_index()"
    )]
    pub fn queue_family_index(&self) -> u32 {
        crate::gpu::vk::mutable_texture_states::get_vk_queue_family_index(self)
    }

    pub fn backend(&self) -> BackendApi {
        unsafe { sb::C_MutableTextureState_backend(self.native()) }
    }
}
