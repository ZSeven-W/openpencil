use std::{fmt, os::raw::c_uint};

use skia_bindings::{
    GrD3DAlloc, GrD3DFenceInfo, GrD3DMemoryAllocator, GrD3DSurfaceInfo, GrD3DTextureResourceInfo,
    SkRefCntBase,
};
use windows::Win32::Graphics::{
    Direct3D12::{ID3D12Fence, D3D12_RESOURCE_STATE_COMMON},
    Dxgi::Common::{DXGI_FORMAT_UNKNOWN, DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN},
};

use crate::{
    gpu::{
        self,
        d3d::{ID3D12Resource, D3D12_RESOURCE_STATES, DXGI_FORMAT},
    },
    prelude::*,
};

// TODO: add remaining cp functions to ComPtr via traits (get, reset, retain).

pub type Alloc = RCHandle<GrD3DAlloc>;
unsafe_send_sync!(Alloc);

impl NativeRefCountedBase for GrD3DAlloc {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Alloc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Alloc").finish()
    }
}

// TODO: support the implementation of custom D3D memory allocator's
// virtual createResource() and createAliasingResource() functions.
pub type MemoryAllocator = RCHandle<GrD3DMemoryAllocator>;
unsafe_send_sync!(MemoryAllocator);

impl NativeRefCountedBase for GrD3DMemoryAllocator {
    type Base = SkRefCntBase;
}

impl fmt::Debug for MemoryAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryAllocator").finish()
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct TextureResourceInfo {
    pub resource: ID3D12Resource,
    pub alloc: Option<Alloc>,
    pub resource_state: D3D12_RESOURCE_STATES,
    pub format: DXGI_FORMAT,
    pub sample_count: u32,
    pub level_count: u32,
    pub sample_quality_pattern: std::os::raw::c_uint,
    pub protected: gpu::Protected,
}

native_transmutable!(GrD3DTextureResourceInfo, TextureResourceInfo);
unsafe_send_sync!(TextureResourceInfo);

impl TextureResourceInfo {
    pub fn from_resource(resource: ID3D12Resource) -> Self {
        Self {
            resource,
            alloc: None,
            resource_state: D3D12_RESOURCE_STATE_COMMON,
            format: DXGI_FORMAT_UNKNOWN,
            sample_count: 1,
            level_count: 0,
            sample_quality_pattern: DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN,
            protected: gpu::Protected::No,
        }
    }

    #[must_use]
    pub fn with_state(self, resource_state: D3D12_RESOURCE_STATES) -> Self {
        Self {
            resource_state,
            ..self
        }
    }
}

impl From<ID3D12Resource> for TextureResourceInfo {
    fn from(resource: ID3D12Resource) -> Self {
        Self::from_resource(resource)
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FenceInfo {
    pub fence: ID3D12Fence,
    pub value: u64,
}

unsafe_send_sync!(FenceInfo);
native_transmutable!(GrD3DFenceInfo, FenceInfo);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct SurfaceInfo {
    pub sample_count: u32,
    pub level_count: u32,
    pub protected: gpu::Protected,

    pub format: DXGI_FORMAT,
    pub sample_quality_pattern: c_uint,
}

native_transmutable!(GrD3DSurfaceInfo, SurfaceInfo);

impl Default for SurfaceInfo {
    fn default() -> Self {
        Self {
            sample_count: 1,
            level_count: 0,
            protected: gpu::Protected::No,
            format: DXGI_FORMAT_UNKNOWN,
            sample_quality_pattern: DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN,
        }
    }
}
