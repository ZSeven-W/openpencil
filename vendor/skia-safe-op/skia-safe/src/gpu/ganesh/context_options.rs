use std::os::raw;

use skia_bindings::{self as sb, GrContextOptions};

use crate::{gpu::DriverBugWorkarounds, prelude::*};

pub use skia_bindings::GrContextOptions_Enable as Enable;
variant_name!(Enable::Yes);

pub use skia_bindings::GrContextOptions_ShaderCacheStrategy as ShaderCacheStrategy;
variant_name!(ShaderCacheStrategy::BackendSource);

#[repr(C)]
#[derive(Debug)]
pub struct ContextOptions {
    /// Optional callback that can be passed into the [`DirectContext`] which will be called when the
    /// [`DirectContext`] is about to be destroyed. When this call is made, it will be safe for the
    /// client to delete the GPU backend context that is backing the [`DirectContext`]. The
    /// [`DirectContextDestroyedContext`] will be passed back to the client in the callback.
    context_delete_context: sb::GrDirectContextDestroyedContext,
    context_delete_proc: sb::GrDirectContextDestroyedProc,

    /// Executor to handle threaded work within Ganesh. If this is `None`, then all work will be
    /// done serially on the main thread. To have worker threads assist with various tasks, set this
    /// to a valid [`sb::SkExecutor`] instance. Currently, used for software path rendering, but may
    /// be used for other tasks.
    executor: *mut sb::SkExecutor,

    /// Cache in which to store compiled shader binaries between runs.
    persistent_cache: *mut sb::GrContextOptions_PersistentCache,

    /// If present, use this object to report shader compilation failures. If not, report failures
    /// via [`Debugf`] and assert.
    shader_error_handler: *mut sb::GrContextOptions_ShaderErrorHandler,

    /// Default minimum size to use when allocating buffers for uploading data to textures. The
    /// larger the value the more uploads can be packed into one buffer, but at the cost of
    /// more gpu memory allocated that may not be used. Uploads larger than the minimum will still
    /// work by allocating a dedicated buffer.
    pub minimum_staging_buffer_size: usize,

    /// The maximum size of cache textures used for Skia's Glyph cache.
    pub glyph_cache_texture_maximum_bytes: usize,

    /// Controls whether we check for GL errors after functions that allocate resources (e.g.
    /// `glTexImage2d`), at the end of a GPU submission, or checking framebuffer completeness. The
    /// results of shader compilation and program linking are always checked, regardless of this
    /// option. Ignored on backends other than GL.
    pub skip_gl_error_checks: Enable,

    /// Can the glyph atlas use multiple textures. If allowed, the each texture's size is bound by
    /// `glyph_cache_texture_maximum_bytes`.
    pub allow_multiple_glyph_cache_textures: Enable,

    /// Enables driver workaround to use draws instead of HW clears, e.g. `glClear` on the GL
    /// backend.
    pub use_draw_instead_of_clear: Enable,

    /// Allow Ganesh to more aggressively reorder operations to reduce the number of render passes.
    /// Offscreen draws will be done upfront instead of interrupting the main render pass when
    /// possible. May increase VRAM usage, but still observes the resource cache limit.
    ///
    /// Enabled by default.
    pub reduce_ops_task_splitting: Enable,

    /// This affects the usage of the PersistentCache. We can cache `SL`, backend source (GLSL), or
    /// backend binaries (GL program binaries). By default we cache binaries, but if the driver's
    /// binary loading/storing is believed to have bugs, this can be limited to caching GLSL.
    /// Caching GLSL strings still saves CPU work when a GL program is created.
    pub shader_cache_strategy: ShaderCacheStrategy,

    /// Overrides: These options override feature detection using backend API queries. These
    /// overrides can only reduce the feature set or limits, never increase them beyond the detected
    /// values.
    pub max_texture_size_override: raw::c_int,

    /// The threshold in bytes above which we will use a buffer mapping API to map vertex and index
    /// buffers to CPU memory in order to update them.  A value of -1 means the `Context` should
    /// deduce the optimal value for this platform.
    pub buffer_map_threshold: raw::c_int,

    /// Maximum number of GPU programs or pipelines to keep active in the runtime cache.
    pub runtime_program_cache_size: raw::c_int,

    /// Specifies the number of samples Ganesh should use when performing internal draws with MSAA
    /// (hardware capabilities permitting).
    ///
    /// If 0, Ganesh will disable internal code paths that use multisampling.
    pub internal_multisample_count: raw::c_int,

    /// In Skia's vulkan backend a single `Context` submit equates to the submission of a single
    /// primary command buffer to the VkQueue. This value specifies how many vulkan secondary command
    /// buffers we will cache for reuse on a given primary command buffer. A single submit may use
    /// more than this many secondary command buffers, but after the primary command buffer is
    /// finished on the GPU it will only hold on to this many secondary command buffers for reuse.
    ///
    /// A value of -1 means we will pick a limit value internally.
    pub max_cached_vulkan_secondary_command_buffers: raw::c_int,

    /// Below this threshold size in device space distance field fonts won't be used. Distance field
    /// fonts don't support hinting which is more important at smaller sizes.
    pub min_distance_field_font_size: f32,

    /// Above this threshold size in device space glyphs are drawn as individual paths.
    pub glyphs_as_paths_font_size: f32,

    pub driver_bug_workarounds: DriverBugWorkarounds,

    /// Construct mipmaps manually, via repeated downsampling draw-calls. This is used when
    /// the driver's implementation (`gl_generate_mipmap`) contains bugs. This requires mipmap
    /// level control (ie desktop or ES3).
    pub do_manual_mipmapping: bool,

    /// Disables the use of coverage counting shortcuts to render paths. Coverage counting can cause
    /// artifacts along shared edges if care isn't taken to ensure both contours wind in the same
    /// direction.
    pub disable_coverage_counting_paths: bool,

    /// Disables distance field rendering for paths. Distance field computation can be expensive,
    /// and yields no benefit if a path is not rendered multiple times with different transforms.
    pub disable_distance_field_paths: bool,

    /// If `true` this allows path mask textures to be cached. This is only really useful if paths
    /// are commonly rendered at the same scale and fractional translation.
    pub allow_path_mask_caching: bool,

    /// If `true`, the GPU will not be used to perform YUV -> RGB conversion when generating
    /// textures from codec-backed images.
    pub disable_gpu_yuv_conversion: bool,

    /// Bugs on certain drivers cause stencil buffers to leak. This flag causes Skia to avoid
    /// allocating stencil buffers and use alternate rasterization paths, avoiding the leak.
    pub avoid_stencil_buffers: bool,

    /// If `true`, texture fetches from mip-mapped textures will be biased to read larger MIP levels.
    /// This has the effect of sharpening those textures, at the cost of some aliasing, and possible
    /// performance impact.
    pub sharpen_mipmapped_textures: bool,

    /// Some ES3 contexts report the ES2 external image extension, but not the ES3 version.
    /// If support for external images is critical, enabling this option will cause Ganesh to limit
    /// shaders to the ES2 shading language in that situation.
    pub prefer_external_images_over_es3: bool,

    /// Disables correctness workarounds that are enabled for particular GPUs, OSes, or drivers.
    /// This does not affect code path choices that are made for performance reasons nor does it
    /// override other [`ContextOptions`] settings.
    pub disable_driver_correctness_workarounds: bool,

    /// If `true`, the caps will never support mipmaps.
    pub suppress_mipmap_support: bool,

    /// If `true`, the TessellationPathRenderer will not be used for path rendering.
    /// If `false`, will fallback to any driver workarounds, if set.
    pub disable_tessellation_path_renderer: bool,

    /// If `true`, and if supported, enables hardware tessellation in the caps.
    /// DEPRECATED: This value is ignored; experimental hardware tessellation is always disabled.
    pub enable_experimental_hardware_tessellation: bool,

    /// If `true`, then add 1 pixel padding to all glyph masks in the atlas to support bi-lerp
    /// rendering of all glyphs. This must be set to `true` to use Slugs.
    pub support_bilerp_from_glyph_atlas: bool,

    /// Uses a reduced variety of shaders. May perform less optimally in steady state but can reduce
    /// jank due to shader compilations.
    pub reduced_shader_variations: bool,

    /// If `true`, then allow to enable MSAA on new Intel GPUs.
    pub allow_msaa_on_new_intel: bool,

    /// Currently on ARM Android we disable the use of GL TexStorage because of memory regressions.
    /// However, some clients may still want to use TexStorage. For example, TexStorage support is
    /// required for creating protected textures.
    ///
    /// This flag has no impact on non GL backends.
    pub always_use_text_storage_when_available: bool,

    // Suppress prints for the GrContext.
    pub suppress_prints: bool,
}
unsafe_send_sync!(ContextOptions);

impl Default for ContextOptions {
    fn default() -> Self {
        Self::construct(|ptr| unsafe { sb::C_GrContextOptions_Construct(ptr) })
    }
}

impl ContextOptions {
    pub fn new() -> Self {
        Self::default()
    }
}

native_transmutable!(GrContextOptions, ContextOptions);

// TODO: PersistentCache, ShaderErrorHandler
