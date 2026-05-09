use std::{
    fmt,
    ops::{Deref, DerefMut},
    ptr,
    time::Duration,
};

use crate::{
    gpu::{
        BackendFormat, BackendRenderTarget, BackendTexture, ContextOptions, FlushInfo,
        GpuStatsFlags, MutableTextureState, PurgeResourceOptions, RecordingContext,
        SemaphoresSubmitted, SubmitInfo, SyncCpu,
    },
    prelude::*,
    surfaces, Data, Image, Surface, TextureCompressionType,
};
use skia_bindings::{self as sb, GrDirectContext, GrDirectContext_DirectContextID, SkRefCntBase};

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DirectContextId {
    id: u32,
}

native_transmutable!(GrDirectContext_DirectContextID, DirectContextId);

pub type DirectContext = RCHandle<GrDirectContext>;

impl NativeRefCountedBase for GrDirectContext {
    type Base = SkRefCntBase;
}

impl Deref for DirectContext {
    type Target = RecordingContext;

    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}

impl DerefMut for DirectContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_ref_mut(self) }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ResourceCacheLimits {
    pub max_resources: usize,
    pub max_resource_bytes: usize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ResourceCacheUsage {
    pub resource_count: usize,
    pub resource_bytes: usize,
}

impl fmt::Debug for DirectContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirectContext")
            .field("base", self as &RecordingContext)
            .field("resource_cache_limit", &self.resource_cache_limit())
            .field("resource_cache_usage", &self.resource_cache_usage())
            .field(
                "resource_cache_purgeable_bytes",
                &self.resource_cache_purgeable_bytes(),
            )
            .field(
                "supports_distance_field_text",
                &self.supports_distance_field_text(),
            )
            .finish()
    }
}

impl DirectContext {
    // Removed from Skia
    #[cfg(feature = "gl")]
    #[deprecated(since = "0.74.0", note = "use gpu::direct_contexts::make_gl()")]
    pub fn new_gl<'a>(
        interface: impl Into<crate::gpu::gl::Interface>,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        crate::gpu::direct_contexts::make_gl(interface, options)
    }

    // Removed from Skia
    #[cfg(feature = "vulkan")]
    #[deprecated(since = "0.74.0", note = "use gpu::direct_contexts::make_vulkan()")]
    pub fn new_vulkan<'a>(
        backend_context: &crate::gpu::vk::BackendContext,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        crate::gpu::direct_contexts::make_vulkan(backend_context, options)
    }

    #[cfg(feature = "metal")]
    #[deprecated(since = "0.74.0", note = "use gpu::direct_contexts::make_metal()")]
    pub fn new_metal<'a>(
        backend_context: &crate::gpu::mtl::BackendContext,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        crate::gpu::direct_contexts::make_metal(backend_context, options)
    }

    #[cfg(feature = "d3d")]
    #[deprecated(since = "0.95.0", note = "use gpu::direct_contexts::make_d3d()")]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_d3d<'a>(
        backend_context: &crate::gpu::d3d::BackendContext,
        options: impl Into<Option<&'a ContextOptions>>,
    ) -> Option<DirectContext> {
        crate::gpu::direct_contexts::make_d3d(backend_context, options)
    }

    pub fn reset(&mut self, backend_state: Option<u32>) -> &mut Self {
        unsafe {
            self.native_mut()
                .resetContext(backend_state.unwrap_or(sb::kAll_GrBackendState))
        }
        self
    }

    pub fn reset_gl_texture_bindings(&mut self) -> &mut Self {
        unsafe { self.native_mut().resetGLTextureBindings() }
        self
    }

    pub fn abandon(&mut self) -> &mut Self {
        unsafe {
            // self.native_mut().abandonContext()
            sb::GrDirectContext_abandonContext(self.native_mut() as *mut _ as _)
        }
        self
    }

    pub fn is_device_lost(&mut self) -> bool {
        unsafe { self.native_mut().isDeviceLost() }
    }

    // TODO: threadSafeProxy()

    pub fn oomed(&mut self) -> bool {
        unsafe { self.native_mut().oomed() }
    }

    pub fn release_resources_and_abandon(&mut self) -> &mut Self {
        unsafe {
            sb::GrDirectContext_releaseResourcesAndAbandonContext(self.native_mut() as *mut _ as _)
        }
        self
    }

    pub fn resource_cache_limit(&self) -> usize {
        unsafe { self.native().getResourceCacheLimit() }
    }

    pub fn resource_cache_usage(&self) -> ResourceCacheUsage {
        let mut resource_count = 0;
        let mut resource_bytes = 0;
        unsafe {
            self.native()
                .getResourceCacheUsage(&mut resource_count, &mut resource_bytes)
        }
        ResourceCacheUsage {
            resource_count: resource_count.try_into().unwrap(),
            resource_bytes,
        }
    }

    pub fn resource_cache_purgeable_bytes(&self) -> usize {
        unsafe { self.native().getResourceCachePurgeableBytes() }
    }

    pub fn set_resource_cache_limits(&mut self, limits: ResourceCacheLimits) {
        unsafe {
            self.native_mut().setResourceCacheLimits(
                limits.max_resources.try_into().unwrap(),
                limits.max_resource_bytes,
            )
        }
    }

    pub fn set_resource_cache_limit(&mut self, max_resource_bytes: usize) {
        unsafe { self.native_mut().setResourceCacheLimit(max_resource_bytes) }
    }

    pub fn free_gpu_resources(&mut self) -> &mut Self {
        unsafe { sb::GrDirectContext_freeGpuResources(self.native_mut() as *mut _ as _) }
        self
    }

    pub fn perform_deferred_cleanup(
        &mut self,
        not_used: Duration,
        opts: impl Into<Option<PurgeResourceOptions>>,
    ) -> &mut Self {
        unsafe {
            sb::C_GrDirectContext_performDeferredCleanup(
                self.native_mut(),
                not_used.as_millis().try_into().unwrap(),
                opts.into().unwrap_or(PurgeResourceOptions::AllResources),
            )
        }
        self
    }

    pub fn purge_unlocked_resource_bytes(
        &mut self,
        bytes_to_purge: usize,
        prefer_scratch_resources: bool,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .purgeUnlockedResources(bytes_to_purge, prefer_scratch_resources)
        }
        self
    }

    pub fn purge_unlocked_resources(&mut self, opts: PurgeResourceOptions) -> &mut Self {
        unsafe { self.native_mut().purgeUnlockedResources1(opts) }
        self
    }

    pub fn supported_gpu_stats(&self) -> GpuStatsFlags {
        GpuStatsFlags::from_bits_truncate(unsafe { self.native().supportedGpuStats() })
    }

    // TODO: wait()

    pub fn flush_and_submit(&mut self) -> &mut Self {
        unsafe { sb::C_GrDirectContext_flushAndSubmit(self.native_mut()) }
        self
    }

    pub fn flush_submit_and_sync_cpu(&mut self) -> &mut Self {
        self.flush(&FlushInfo::default());
        self.submit(SyncCpu::Yes);
        self
    }

    #[deprecated(since = "0.37.0", note = "Use flush()")]
    pub fn flush_with_info(&mut self, info: &FlushInfo) -> SemaphoresSubmitted {
        self.flush(info)
    }

    pub fn flush<'a>(&mut self, info: impl Into<Option<&'a FlushInfo>>) -> SemaphoresSubmitted {
        let n = self.native_mut();
        if let Some(info) = info.into() {
            unsafe { n.flush(info.native()) }
        } else {
            let info = FlushInfo::default();
            unsafe { n.flush(info.native()) }
        }
    }

    pub fn flush_image_with_info(
        &mut self,
        image: &Image,
        info: &FlushInfo,
    ) -> SemaphoresSubmitted {
        unsafe {
            sb::C_GrDirectContext_flushImageWithInfo(
                self.native_mut(),
                image.clone().into_ptr(),
                info.native(),
            )
        }
    }

    pub fn flush_image(&mut self, image: &Image) {
        unsafe { sb::C_GrDirectContext_flushImage(self.native_mut(), image.clone().into_ptr()) }
    }

    pub fn flush_and_submit_image(&mut self, image: &Image) {
        unsafe {
            sb::C_GrDirectContext_flushAndSubmitImage(self.native_mut(), image.clone().into_ptr())
        }
    }

    pub fn flush_surface_with_access(
        &mut self,
        surface: &mut Surface,
        access: surfaces::BackendSurfaceAccess,
        info: &FlushInfo,
    ) -> SemaphoresSubmitted {
        unsafe {
            self.native_mut()
                .flush3(surface.native_mut(), access, info.native())
        }
    }

    pub fn flush_surface_with_texture_state(
        &mut self,
        surface: &mut Surface,
        info: &FlushInfo,
        new_state: Option<&MutableTextureState>,
    ) -> SemaphoresSubmitted {
        unsafe {
            self.native_mut().flush4(
                surface.native_mut(),
                info.native(),
                new_state.native_ptr_or_null(),
            )
        }
    }

    pub fn flush_and_submit_surface(
        &mut self,
        surface: &mut Surface,
        sync_cpu: impl Into<Option<SyncCpu>>,
    ) {
        unsafe {
            self.native_mut()
                .flushAndSubmit1(surface.native_mut(), sync_cpu.into().unwrap_or(SyncCpu::No))
        }
    }

    pub fn flush_surface(&mut self, surface: &mut Surface) {
        unsafe { self.native_mut().flush5(surface.native_mut()) }
    }

    pub fn submit(&mut self, submit_info: impl Into<SubmitInfo>) -> bool {
        unsafe { self.native_mut().submit(&submit_info.into().into_native()) }
    }

    pub fn check_async_work_completion(&mut self) {
        unsafe { self.native_mut().checkAsyncWorkCompletion() }
    }

    // TODO: dumpMemoryStatistics()

    pub fn supports_distance_field_text(&self) -> bool {
        unsafe { self.native().supportsDistanceFieldText() }
    }
}

#[cfg(feature = "vulkan")]
impl DirectContext {
    pub fn can_detect_new_vk_pipeline_cache_data(&self) -> bool {
        unsafe { self.native().canDetectNewVkPipelineCacheData() }
    }

    pub fn has_new_vk_pipeline_cache_data(&self) -> bool {
        unsafe { self.native().hasNewVkPipelineCacheData() }
    }

    pub fn store_vk_pipeline_cache_data(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().storeVkPipelineCacheData();
        }
        self
    }

    pub fn store_vk_pipeline_cache_data_with_max_size(&mut self, max_size: usize) -> &mut Self {
        unsafe {
            self.native_mut().storeVkPipelineCacheData1(max_size);
        }
        self
    }
}

impl DirectContext {
    // TODO: wrap createBackendTexture (several variants)
    //       introduced in m76, m77, and m79
    //       extended in m84 with finishedProc and finishedContext
    //       extended in m107 with label

    // TODO: wrap updateBackendTexture (several variants)
    //       introduced in m84

    pub fn compressed_backend_format(&self, compression: TextureCompressionType) -> BackendFormat {
        let mut backend_format = BackendFormat::new_invalid();
        unsafe {
            sb::C_GrDirectContext_compressedBackendFormat(
                self.native(),
                compression,
                backend_format.native_mut(),
            )
        };
        backend_format
    }

    // TODO: wrap createCompressedBackendTexture (several variants)
    //       introduced in m81
    //       extended in m84 with finishedProc and finishedContext

    // TODO: wrap updateCompressedBackendTexture (two variants)
    //       introduced in m86

    // TODO: add variant with GpuFinishedProc / GpuFinishedContext
    pub fn set_backend_texture_state(
        &mut self,
        backend_texture: &BackendTexture,
        state: &MutableTextureState,
    ) -> bool {
        self.set_backend_texture_state_and_return_previous(backend_texture, state)
            .is_some()
    }

    pub fn set_backend_texture_state_and_return_previous(
        &mut self,
        backend_texture: &BackendTexture,
        state: &MutableTextureState,
    ) -> Option<MutableTextureState> {
        let mut previous = MutableTextureState::default();
        unsafe {
            self.native_mut().setBackendTextureState(
                backend_texture.native(),
                state.native(),
                previous.native_mut(),
                None,
                ptr::null_mut(),
            )
        }
        .then_some(previous)
    }

    // TODO: add variant with GpuFinishedProc / GpuFinishedContext
    pub fn set_backend_render_target_state(
        &mut self,
        target: &BackendRenderTarget,
        state: &MutableTextureState,
    ) -> bool {
        self.set_backend_render_target_state_and_return_previous(target, state)
            .is_some()
    }

    pub fn set_backend_render_target_state_and_return_previous(
        &mut self,
        target: &BackendRenderTarget,
        state: &MutableTextureState,
    ) -> Option<MutableTextureState> {
        let mut previous = MutableTextureState::default();
        unsafe {
            self.native_mut().setBackendRenderTargetState(
                target.native(),
                state.native(),
                previous.native_mut(),
                None,
                ptr::null_mut(),
            )
        }
        .then_some(previous)
    }

    pub fn delete_backend_texture(&mut self, texture: &BackendTexture) {
        unsafe { self.native_mut().deleteBackendTexture(texture.native()) }
    }

    pub fn precompile_shader(&mut self, key: &Data, data: &Data) -> bool {
        unsafe {
            self.native_mut()
                .precompileShader(key.native(), data.native())
        }
    }

    pub fn id(&self) -> DirectContextId {
        let mut id = DirectContextId { id: 0 };
        unsafe { sb::C_GrDirectContext_directContextId(self.native(), id.native_mut()) }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::DirectContext;
    use crate::gpu::{SubmitInfo, SyncCpu};

    #[allow(unused)]
    fn submit_invocation(direct_context: &mut DirectContext) {
        direct_context.submit(SyncCpu::Yes);
        direct_context.submit(None);
        direct_context.submit(Some(SyncCpu::Yes));
        direct_context.submit(SubmitInfo::default());
    }
}
