use skia_bindings as sb;
use skia_bindings::skgpu_GpuStats;

pub use sb::skgpu_BackendApi as BackendApi;
variant_name!(BackendApi::Metal);

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Budgeted(bool);
native_transmutable!(sb::skgpu_Budgeted, Budgeted);

#[allow(non_upper_case_globals)]
impl Budgeted {
    // we want this look like enum case names.
    pub const No: Budgeted = Budgeted(false);
    pub const Yes: Budgeted = Budgeted(true);
}

// TODO: CallbackResult

pub use skia_bindings::skgpu_Mipmapped as Mipmapped;

pub use skia_bindings::skgpu_Protected as Protected;
variant_name!(Protected::Yes);

pub use skia_bindings::skgpu_Renderable as Renderable;
variant_name!(Renderable::No);

pub use skia_bindings::skgpu_Origin as Origin;
variant_name!(Origin::TopLeft);

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GpuStatsFlags : u32 {
        const NONE = sb::skgpu_GpuStatsFlags_kNone as _;
        const ELAPSED_TIME = sb::skgpu_GpuStatsFlags_kElapsedTime as _;
        const OCCLUSION_PASS_SAMPLES = sb::skgpu_GpuStatsFlags_kOcclusionPassSamples as _;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuStats {
    pub elapsed_time: u64,
    pub num_occlusion_pass_samples: u64,
}

native_transmutable!(skgpu_GpuStats, GpuStats);
