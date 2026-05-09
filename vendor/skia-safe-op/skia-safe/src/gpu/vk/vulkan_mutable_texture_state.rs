pub mod mutable_texture_states {
    use skia_bindings as sb;

    use crate::{
        gpu::{vk::ImageLayout, BackendApi, MutableTextureState},
        prelude::*,
    };

    pub fn new_vulkan(layout: ImageLayout, queue_family_index: u32) -> MutableTextureState {
        MutableTextureState::from_ptr(unsafe {
            sb::C_MutableTextureStates_ConstructVulkan(layout, queue_family_index)
        })
        .unwrap()
    }

    pub fn get_vk_image_layout(state: &MutableTextureState) -> sb::VkImageLayout {
        assert_eq!(state.backend(), BackendApi::Vulkan);
        unsafe { sb::C_MutableTextureStates_getVkImageLayout(state.native()) }
    }

    pub fn get_vk_queue_family_index(state: &MutableTextureState) -> u32 {
        assert_eq!(state.backend(), BackendApi::Vulkan);
        unsafe { sb::C_MutableTextureStates_getVkQueueFamilyIndex(state.native()) }
    }
}
