use skia_safe::{color_filters, BlendMode, Color, ColorFilter, ConditionallySend, Sendable};

/// Test if RCHandle<> types can be wrapped into a Sendable and unwrapped.
#[test]
fn sendable() {
    let color = Color::CYAN;
    let mode = BlendMode::ColorBurn;
    let cf = color_filters::blend(color, mode).unwrap();
    let sendable = cf.wrap_send().ok().unwrap();
    let _unwrapped = sendable.into_inner();
}

/// Test if Sendable<> is actually sendable for RCHandle types.
#[test]
fn sendable_implements_send() {
    assert::send::<Sendable<ColorFilter>>();
}

mod codec {
    use skia_safe::{codec, codecs, Codec};
    use static_assertions::*;

    // Codec seems to call into SkPngChunkReader*
    assert_not_impl_any!(Codec: Send, Sync);

    assert_impl_all!(codec::Result: Send, Sync);
    assert_impl_all!(codec::SelectionPolicy: Send, Sync);
    assert_impl_all!(codec::ZeroInitialized: Send, Sync);
    assert_impl_all!(codec::ScanlineOrder: Send, Sync);

    assert_impl_all!(codecs::Decoder: Send, Sync);
}

#[allow(deprecated)]
mod core {
    use contour_measure::{ForwardVerbIterator, VerbMeasure};
    use skia_safe::{recorder::RecorderRef, *};
    use static_assertions::*;

    assert_impl_all!(Arc: Send, Sync);
    // SkBitmap is not thread safe. Each thread must have its own copy of SkBitmap fields,
    // although threads may share the underlying pixel array.
    assert_not_impl_any!(Bitmap: Send, Sync);
    assert_impl_all!(Blender: Send, Sync);
    assert_not_impl_any!(Canvas: Send, Sync);
    assert_impl_all!(canvas::TopLayerPixels: Send, Sync);
    assert_impl_all!(canvas::GlyphPositions: Send, Sync);
    assert_not_impl_any!(OwnedCanvas: Send, Sync);
    assert_impl_all!(Color: Send, Sync);
    assert_impl_all!(ColorFilter: Send, Sync);
    assert_impl_all!(ColorSpacePrimaries: Send, Sync);
    assert_impl_all!(named_primaries::CicpId: Send, Sync);
    assert_impl_all!(ColorSpaceTransferFn: Send, Sync);
    assert_impl_all!(named_transfer_fn::CicpId: Send, Sync);
    assert_impl_all!(ColorSpace: Send, Sync);
    assert_impl_all!(ColorTable: Send, Sync);
    assert_impl_all!(ContourMeasure: Send, Sync);
    assert_impl_all!(ContourMeasureIter: Send, Sync);
    assert_impl_all!(ForwardVerbIterator: Send, Sync);
    assert_impl_all!(VerbMeasure: Send, Sync);
    assert_impl_all!(CubicMap: Send, Sync);
    assert_impl_all!(CubicResampler: Send, Sync);
    assert_impl_all!(Data: Send, Sync);
    assert_impl_all!(DataTable: Send, Sync);
    assert_not_impl_any!(Document: Send, Sync);
    assert_not_impl_any!(Drawable: Send, Sync);
    assert_impl_all!(Font: Send, Sync);
    assert_not_impl_any!(FontArguments: Send, Sync);
    assert_impl_all!(FontMetrics: Send, Sync);
    assert_not_impl_any!(FontStyleSet: Send, Sync);
    assert_not_impl_any!(FontMgr: Send, Sync);
    assert_impl_all!(font_parameters::variation::Axis: Send, Sync);
    assert_impl_all!(FontStyle: Send, Sync);
    // core/image.rs
    // SkImage cannot be modified after it is created. SkImage may allocate additional storage as
    // needed; for instance, an encoded SkImage may decode when drawn.
    // > So far the implementation seems to handle the "allocate additional storage as needed" in a
    // > thread safe way.
    assert_impl_all!(Image: Send, Sync);
    assert_impl_all!(image::CubicResampler: Send, Sync);
    assert_impl_all!(image::BitDepth: Send, Sync);
    assert_impl_all!(image::RequiredProperties: Send, Sync);

    assert_impl_all!(ImageFilter: Send, Sync);
    assert_impl_all!(ImageGenerator: Send, Sync);
    assert_impl_all!(ColorInfo: Send, Sync);
    assert_impl_all!(ImageInfo: Send, Sync);
    assert_impl_all!(M44: Send, Sync);
    assert_impl_all!(MaskFilter: Send, Sync);

    assert_impl_all!(Matrix: Send, Sync);
    assert_impl_all!(matrix::ScaleToFit: Send, Sync);
    assert_impl_all!(matrix::TypeMask: Send, Sync);

    assert_impl_all!(Paint: Send, Sync);

    assert_impl_all!(Path: Send);
    assert_not_impl_any!(Path: Sync);
    assert_not_impl_any!(path::Iter: Send, Sync);
    assert_not_impl_any!(path::RawIter: Send, Sync);
    assert_impl_all!(path::SegmentMask: Send, Sync);
    assert_impl_all!(path::Verb: Send, Sync);

    assert_impl_all!(PathBuilder: Send, Sync);
    assert_impl_all!(path_builder::ArcSize: Send, Sync);
    assert_impl_all!(path_builder::DumpFormat: Send, Sync);

    assert_impl_all!(PathDirection: Send, Sync);
    assert_impl_all!(PathEffect: Send, Sync);
    assert_impl_all!(PathFillType: Send, Sync);

    assert_not_impl_all!(PathIter: Send, Sync);
    assert_not_impl_all!(PathIterRec: Send, Sync);
    assert_not_impl_all!(PathContourIter: Send, Sync);
    assert_not_impl_all!(PathContourIterRec: Send, Sync);

    assert_not_impl_any!(PathMeasure: Send, Sync);
    assert_impl_all!(PathVerb: Send, Sync);

    assert_impl_all!(Picture: Send, Sync);
    assert_not_impl_any!(PictureRecorder: Send, Sync);
    assert_impl_all!(PixelRef: Send, Sync);
    assert_not_impl_any!(Pixmap: Send, Sync);

    assert_not_impl_any!(dyn Recorder: Send, Sync);
    assert_not_impl_any!(RecorderRef: Send, Sync);

    assert_impl_all!(Region: Send, Sync);
    assert_not_impl_any!(region::Iterator: Send, Sync);
    assert_not_impl_any!(region::Cliperator: Send, Sync);
    assert_not_impl_any!(region::Spanerator: Send, Sync);
    assert_impl_all!(RRect: Send, Sync);
    assert_impl_all!(RSXform: Send, Sync);
    assert_impl_all!(Shader: Send, Sync);
    assert_not_impl_any!(Surface: Send, Sync);
    assert_impl_all!(SurfaceProps: Send, Sync);
    assert_impl_all!(TextBlob: Send, Sync);
    assert_impl_all!(TextBlobBuilder: Send, Sync);
    assert_not_impl_any!(TextBlobIter: Send, Sync);
    assert_not_impl_any!(TextBlobRun: Send, Sync);
    assert_impl_all!(typeface::LocalizedString: Send, Sync);
    assert_impl_all!(Typeface: Send, Sync);
    assert_not_impl_any!(typeface::LocalizedStringsIter: Send, Sync);
    assert_impl_all!(Vertices: Send, Sync);
    assert_impl_all!(vertices::Builder: Send, Sync);
    // core/sampling_options.rs
    assert_impl_all!(CubicResampler: Send, Sync);
    assert_impl_all!(FilterMode: Send, Sync);
    assert_impl_all!(SamplingOptions: Send, Sync);
    // core/yuva_info.rs
    assert_impl_all!(YUVAInfo: Send, Sync);
    assert_impl_all!(yuva_info::PlaneConfig: Send, Sync);
    assert_impl_all!(yuva_info::Subsampling: Send, Sync);
    assert_impl_all!(yuva_info::Siting: Send, Sync);
    // core/yuva_pixmaps.rs
    assert_impl_all!(YUVAPixmapInfo: Send, Sync);
    assert_impl_all!(yuva_pixmap_info::PlaneConfig: Send, Sync);
    assert_impl_all!(yuva_pixmap_info::Subsampling: Send, Sync);
    assert_impl_all!(yuva_pixmap_info::DataType: Send, Sync);
    assert_impl_all!(yuva_pixmap_info::SupportedDataTypes: Send, Sync);
    assert_impl_all!(YUVAPixmaps: Send, Sync);
    assert_impl_all!(yuva_pixmaps::DataType: Send, Sync);
    assert_impl_all!(runtime_effect::ChildPtr: Send, Sync);
}

#[cfg(feature = "pdf")]
mod docs {
    use skia_safe::pdf;
    use static_assertions::*;

    assert_impl_all!(pdf::AttributeList: Send, Sync);
    assert_not_impl_any!(pdf::StructureElementNode: Send, Sync);
    assert_impl_all!(pdf::DateTime: Send, Sync);
    assert_not_impl_any!(pdf::Metadata: Send, Sync);
    assert_impl_all!(pdf::Outline: Send, Sync);
    assert_impl_all!(pdf::CompressionLevel: Send, Sync);
}

mod effects {
    use skia_safe::{gradient, image_filters, runtime_effect, RuntimeEffect};
    use static_assertions::*;

    assert_impl_all!(gradient::Interpolation: Send, Sync);
    assert_impl_all!(gradient::interpolation::InPremul: Send, Sync);
    assert_impl_all!(gradient::interpolation::ColorSpace: Send, Sync);
    assert_impl_all!(gradient::interpolation::HueMethod: Send, Sync);
    assert_impl_all!(gradient::Colors: Send, Sync);
    assert_impl_all!(gradient::Gradient: Send, Sync);

    assert_impl_all!(image_filters::CropRect: Send, Sync);
    assert_impl_all!(image_filters::Dither: Send, Sync);

    assert_not_impl_any!(RuntimeEffect: Send, Sync);
    assert_impl_all!(runtime_effect::Uniform: Send, Sync);
    assert_impl_all!(runtime_effect::Child: Send, Sync);
    assert_impl_all!(runtime_effect::ChildType: Send, Sync);
    assert_impl_all!(runtime_effect::Options: Send, Sync);
}

#[cfg(feature = "gpu")]
mod gpu {
    use ganesh::MarkFrameBoundary;
    use skia_safe::gpu::*;
    use static_assertions::*;
    assert_impl_all!(BackendFormat: Send, Sync);
    assert_impl_all!(BackendSemaphore: Send, Sync);
    assert_impl_all!(BackendTexture: Send, Sync);
    assert_impl_all!(BackendRenderTarget: Send, Sync);
    assert_impl_all!(ContextOptions: Send, Sync);
    assert_impl_all!(DriverBugWorkarounds: Send, Sync);
    // The Context* implementations check for single ownership before mutation, so no Send and Sync
    // can be supported.
    // If RC is 1, it can be sent to other threads with `Sendable` / `ConditionallySend`.
    assert_not_impl_any!(DirectContext: Send, Sync);
    assert_impl_all!(DirectContextId: Send, Sync);
    assert_not_impl_any!(RecordingContext: Send, Sync);
    // gpu/yuva_backend_textures.rs
    assert_impl_all!(YUVABackendTextureInfo: Send, Sync);
    assert_impl_all!(YUVABackendTextures: Send, Sync);

    assert_impl_all!(MutableTextureState: Send, Sync);
    assert_impl_all!(BackendApi: Send, Sync);

    // gpu/types.rs
    assert_impl_all!(BackendAPI: Send, Sync);
    assert_impl_all!(Budgeted: Send, Sync);
    assert_impl_all!(Mipmapped: Send, Sync);
    assert_impl_all!(Protected: Send, Sync);
    assert_impl_all!(Renderable: Send, Sync);
    assert_impl_all!(Origin: Send, Sync);
    assert_impl_all!(GpuStatsFlags: Send, Sync);
    assert_impl_all!(GpuStats: Send, Sync);

    // gpu/ganesh/types.rs
    assert_impl_all!(SurfaceOrigin: Send, Sync);
    assert_not_impl_any!(FlushInfo: Send, Sync);
    assert_impl_all!(SemaphoresSubmitted: Send, Sync);
    assert_impl_all!(PurgeResourceOptions: Send, Sync);
    assert_impl_all!(SyncCpu: Send, Sync);
    assert_impl_all!(MarkFrameBoundary: Send, Sync);
    assert_impl_all!(SubmitInfo: Send, Sync);

    #[cfg(feature = "gl")]
    mod gl {
        use skia_safe::gpu::gl::*;
        use static_assertions::*;
        assert_impl_all!(Extensions: Send, Sync);
        // RC & mutable (extensions_mut() ... we could make this function unsafe)
        assert_not_impl_any!(Interface: Send, Sync);
        assert_impl_all!(TextureInfo: Send, Sync);
        assert_impl_all!(FramebufferInfo: Send, Sync);
        assert_impl_all!(SurfaceInfo: Send, Sync);
    }

    #[cfg(feature = "metal")]
    mod mtl {
        use skia_safe::gpu::mtl::*;
        use static_assertions::*;
        assert_impl_all!(TextureInfo: Send, Sync);
        assert_impl_all!(SurfaceInfo: Send, Sync);
        assert_impl_all!(BackendContext: Send, Sync);
    }

    #[cfg(feature = "vulkan")]
    mod vulkan {
        use skia_safe::gpu::vk::*;
        use static_assertions::*;
        // TODO: BackendContext is referencing get_proc and is used only temporarily for building
        //       the context.
        assert_not_impl_any!(BackendContext: Send, Sync);
        // already Copy & Clone, and highly unsafe.
        assert_impl_all!(Alloc: Send, Sync);
        assert_impl_all!(YcbcrConversionInfo: Send, Sync);
        assert_impl_all!(ImageInfo: Send, Sync);
        // GetProc could be Send & Sync , but does it make sense (it's already Copy & Clone)
        assert_not_impl_any!(GetProcOf: Send, Sync);
        assert_impl_all!(DrawableInfo: Send, Sync);
        assert_impl_all!(SurfaceInfo: Send, Sync);
        assert_impl_all!(BackendDrawableInfo: Send, Sync);
        // Note that we can't make most of vk.rs re-export of native Vulkan types Send nor Sync,
        // because they are just re-exports of simple pointers, which already implement
        // !Send & !Sync that can not be overridden...
    }

    #[cfg(feature = "d3d")]
    mod d3d {
        use skia_safe::gpu::d3d::*;
        use static_assertions::*;
        assert_impl_all!(BackendContext: Send, Sync);
        assert_impl_all!(TextureResourceInfo: Send, Sync);
        assert_impl_all!(FenceInfo: Send, Sync);
        assert_impl_all!(SurfaceInfo: Send, Sync);
        assert_impl_all!(Alloc: Send, Sync);
        assert_impl_all!(MemoryAllocator: Send, Sync);
    }
}

#[cfg(feature = "textlayout")]
mod textlayout {
    use skia_safe::textlayout::*;
    use static_assertions::*;

    // RC _and_ mutable, forbid shared mutability.
    assert_not_impl_any!(FontCollection: Send, Sync);
    // ParagraphCache seems to be fully thread safe, but I don't think it is itself meant to be shared between threads.
    assert_not_impl_any!(ParagraphCache: Send, Sync);
    assert_not_impl_any!(Paragraph: Send, Sync);
    assert_impl_all!(paragraph::GlyphInfo: Send, Sync);
    assert_impl_all!(paragraph::FontInfo: Send, Sync);
    assert_not_impl_any!(paragraph::VisitorInfo: Send, Sync);
    assert_not_impl_any!(paragraph::ExtendedVisitorInfo: Send, Sync);
    assert_impl_all!(paragraph::VisitorFlags: Send, Sync);
    assert_impl_all!(paragraph::GlyphClusterInfo: Send, Sync);

    assert_impl_all!(ParagraphBuilder: Send, Sync);
    assert_impl_all!(StrutStyle: Send, Sync);
    assert_impl_all!(TextShadow: Send, Sync);
    assert_impl_all!(Decoration: Send, Sync);
    assert_impl_all!(FontFeature: Send, Sync);
    assert_impl_all!(PlaceholderStyle: Send, Sync);
    assert_impl_all!(TextStyle: Send, Sync);
    assert_impl_all!(Block: Send, Sync);
    assert_impl_all!(Placeholder: Send, Sync);
    assert_impl_all!(FontArguments: Send, Sync);
    assert_not_impl_any!(TypefaceFontStyleSet: Send, Sync);
    assert_not_impl_any!(TypefaceFontProvider: Send, Sync);
    assert_not_impl_any!(FontFamilies: Send, Sync);
}

#[cfg(feature = "textlayout")]
mod shaper {
    use skia_safe::shaper::*;
    use static_assertions::*;

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    assert_impl_all!(skia_safe::shapers::ct::LineBreakMode: Send, Sync);

    assert_impl_all!(Shaper: Send, Sync);
    assert_not_impl_any!(FontRunIterator: Send, Sync);
    assert_not_impl_any!(BiDiRunIterator: Send, Sync);
    assert_not_impl_any!(ScriptRunIterator: Send, Sync);
    assert_not_impl_any!(LanguageRunIterator: Send, Sync);
    assert_not_impl_any!(TextBlobBuilderRunHandler: Send, Sync);
}

mod pathops {
    use skia_safe::OpBuilder;
    use static_assertions::*;
    assert_impl_all!(OpBuilder: Send, Sync);
}

mod svg {
    use skia_safe::svg::*;
    use static_assertions::*;
    assert_not_impl_any!(Canvas: Send, Sync);
}

#[cfg(feature = "svg")]
mod render_svg {
    use skia_safe::svg::*;
    use static_assertions::*;

    assert_not_impl_any!(Dom: Send, Sync);
    assert_impl_all!(LoadError: Send, Sync);
}

mod utils {
    use skia_safe::utils::*;
    use static_assertions::*;
    assert_impl_all!(CustomTypefaceBuilder: Send, Sync);
    assert_not_impl_any!(OrderedFontMgr: Send, Sync);
    assert_impl_all!(parse_path::PathEncoding: Send, Sync);
}

pub mod assert {
    pub fn send<T: Send>() {}
    pub fn sync<T: Sync>() {}
}
