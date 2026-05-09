use skia_bindings as sb;

pub use super::ganesh::d3d::{backend_context::*, types::*};

// re-export D3D types we use

pub use sb::GrD3DResourceStateEnum as ResourceStateEnum;

pub use windows::Win32::Graphics::Direct3D12::ID3D12CommandQueue;
pub use windows::Win32::Graphics::Direct3D12::ID3D12Device;
pub use windows::Win32::Graphics::Direct3D12::ID3D12Resource;
pub use windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATES;
pub use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
pub use windows::Win32::Graphics::Dxgi::IDXGIAdapter1;

native_transmutable!(sb::DXGI_FORMAT, DXGI_FORMAT);
