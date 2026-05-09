use std::ptr;

use crate::gpu;
use crate::gpu::GpuStatsFlags;
use skia_bindings as sb;

pub use skia_bindings::GrBackendApi as BackendApi;
variant_name!(BackendAPI::OpenGL);

#[deprecated(since = "0.80.0", note = "use BackendApi")]
pub use BackendApi as BackendAPI;

pub const METAL_BACKEND: BackendApi = BackendApi::Metal;
pub const VULKAN_BACKEND: BackendApi = BackendApi::Vulkan;
pub const MOCK_BACKEND: BackendApi = BackendApi::Mock;

pub use gpu::Renderable;

pub use gpu::Protected;

pub use skia_bindings::GrSurfaceOrigin as SurfaceOrigin;
variant_name!(SurfaceOrigin::BottomLeft);

// Note: BackendState is in gl/types.rs/

#[repr(C)]
#[allow(dead_code)]
#[derive(Debug)]
pub struct FlushInfo {
    // TODO: wrap access to the following fields in a safe way:
    num_semaphores: usize,
    gpu_stats_flags: GpuStatsFlags,
    signal_semaphores: *mut sb::GrBackendSemaphore,
    finished_proc: sb::GrGpuFinishedProc,
    finished_with_stats_proc: sb::GrGpuFinishedWithStatsProc,
    finished_context: sb::GrGpuFinishedContext,
    submitted_proc: sb::GrGpuSubmittedProc,
    submitted_context: sb::GrGpuSubmittedContext,
}

impl Default for FlushInfo {
    fn default() -> Self {
        Self {
            num_semaphores: 0,
            gpu_stats_flags: GpuStatsFlags::NONE,
            signal_semaphores: ptr::null_mut(),
            finished_proc: None,
            finished_with_stats_proc: None,
            finished_context: ptr::null_mut(),
            submitted_proc: None,
            submitted_context: ptr::null_mut(),
        }
    }
}

native_transmutable!(sb::GrFlushInfo, FlushInfo);

pub use sb::GrSemaphoresSubmitted as SemaphoresSubmitted;
variant_name!(SemaphoresSubmitted::Yes);

pub use sb::GrPurgeResourceOptions as PurgeResourceOptions;
variant_name!(PurgeResourceOptions::AllResources);

pub use sb::GrSyncCpu as SyncCpu;
variant_name!(SyncCpu::Yes);

pub use sb::GrMarkFrameBoundary as MarkFrameBoundary;
variant_name!(MarkFrameBoundary::Yes);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SubmitInfo {
    pub sync: SyncCpu,
    pub mark_boundary: MarkFrameBoundary,
    pub frame_id: u64,
}
native_transmutable!(sb::GrSubmitInfo, SubmitInfo);

impl Default for SubmitInfo {
    fn default() -> Self {
        Self {
            sync: SyncCpu::No,
            mark_boundary: MarkFrameBoundary::No,
            frame_id: 0,
        }
    }
}

impl From<SyncCpu> for SubmitInfo {
    fn from(sync: SyncCpu) -> Self {
        Self {
            sync,
            ..Self::default()
        }
    }
}

impl From<Option<SyncCpu>> for SubmitInfo {
    fn from(sync_cpu: Option<SyncCpu>) -> Self {
        match sync_cpu {
            Some(sync_cpu) => sync_cpu.into(),
            None => Self::default(),
        }
    }
}
