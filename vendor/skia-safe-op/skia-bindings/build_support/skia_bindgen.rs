//! Full build support for the SkiaBindings library, and bindings.rs file.
use std::path::{Path, PathBuf};

use bindgen::{CodegenConfig, EnumVariation};
use cc::Build;

use crate::build_support::{
    binaries_config,
    cargo::{self, Target},
    features,
    platform::{self, prelude::feature},
};

// 20: Since m143
const CPP_VERSION: &str = "20";

pub mod env {
    use crate::build_support::cargo;

    pub fn skia_lib_definitions() -> Option<String> {
        cargo::env_var("SKIA_BUILD_DEFINES")
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Configuration {
    /// The features active.
    pub features: features::Features,

    /// The binding source files to compile.
    pub binding_sources: Vec<PathBuf>,

    /// The Skia source directory.
    pub skia_source_dir: PathBuf,

    /// Further definitions needed for build consistency.
    pub definitions: Definitions,
}

impl Configuration {
    pub fn new(
        features: &features::Features,
        definitions: Definitions,
        skia_source_dir: &Path,
    ) -> Self {
        let binding_sources = {
            let mut sources: Vec<PathBuf> = vec!["src/bindings.cpp".into()];
            if features[feature::GL] {
                sources.push("src/gl.cpp".into());
            }
            if features[feature::EGL] {
                sources.push("src/egl.cpp".into());
            }
            if features[feature::VULKAN] {
                sources.push("src/vulkan.cpp".into());
            }
            if features[feature::METAL] {
                sources.push("src/metal.cpp".into());
            }
            if features[feature::D3D] {
                sources.push("src/d3d.cpp".into());
            }
            if features.gpu() {
                sources.push("src/gpu.cpp".into());
            }
            if features[feature::TEXTLAYOUT] {
                sources.extend(vec!["src/shaper.cpp".into(), "src/paragraph.cpp".into()]);
            }
            if features[feature::SVG] {
                sources.push("src/svg.cpp".into());
            }
            if features[feature::SKOTTIE] {
                sources.push("src/skottie.cpp".into());
            }
            if features[feature::SVG] || features[feature::SKOTTIE] {
                sources.push("src/skresources.cpp".into());
            }
            if features[feature::WEBP_ENCODE] {
                sources.push("src/webp-encode.cpp".into());
            }
            sources
        };

        Self {
            features: features.clone(),
            skia_source_dir: skia_source_dir.into(),
            binding_sources,
            definitions,
        }
    }
}

pub fn generate_bindings(
    build: &Configuration,
    output_directory: &Path,
    target: Target,
    sysroot: Option<&str>,
) {
    let mut builder = bindgen::Builder::default()
        .generate_comments(false)
        .layout_tests(true)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(ParseCallbacks))
        .allowlist_function("C_.*")
        .constified_enum(".*Mask")
        .constified_enum(".*Flags")
        .constified_enum(".*Bits")
        .constified_enum("SkCanvas_SaveLayerFlagsSet")
        .constified_enum("GrVkAlloc_Flag")
        .constified_enum("GrGLBackendState")
        // not used:
        .blocklist_type("SkPathRef_Editor")
        .blocklist_function("SkPathRef_Editor_Editor")
        // private types that pull in inline functions that cannot be linked:
        // https://github.com/rust-skia/rust-skia/issues/318
        .raw_line("pub enum GrContext_Base {}")
        .blocklist_type("GrContext_Base")
        .blocklist_function("GrContext_Base_.*")
        .raw_line("pub enum GrImageContext {}")
        .blocklist_type("GrImageContext")
        .raw_line("pub enum GrImageContextPriv {}")
        .blocklist_type("GrImageContextPriv")
        .raw_line("pub enum GrContextThreadSafeProxy {}")
        .blocklist_type("GrContextThreadSafeProxy")
        .blocklist_type("GrContextThreadSafeProxyPriv")
        .raw_line("pub enum GrContextThreadSafeProxyPriv {}")
        .blocklist_type("GrRecordingContextPriv")
        .raw_line("pub enum GrRecordingContextPriv {}")
        .blocklist_function("GrRecordingContext_priv.*")
        .blocklist_function("GrDirectContext_priv.*")
        .blocklist_type("GrContextPriv")
        .raw_line("pub enum GrContextPriv {}")
        .blocklist_function("GrContext_priv.*")
        .blocklist_function("SkDeferredDisplayList_priv.*")
        .raw_line("pub enum SkVerticesPriv {}")
        .blocklist_type("SkVerticesPriv")
        .blocklist_function("SkVertices_priv.*")
        .blocklist_function("std::bitset_flip.*")
        // Vulkan reexports that got swallowed by making them opaque.
        // (these can not be allowlisted by a extern "C" function)
        .allowlist_type("VkPhysicalDeviceFeatures")
        .allowlist_type("VkPhysicalDeviceFeatures2").
        // m91: These functions are not actually implemented.
        blocklist_function("SkCustomTypefaceBuilder_setGlyph[123].*")
        // m113: `SkUnicode` pulls in an impl block that forwards static functions that may not be
        // linked into the final executable.
        .blocklist_type("SkUnicode")
        .raw_line("pub enum SkUnicode {}")


        // misc
        .allowlist_var("SK_Color.*")
        .allowlist_var("kAll_GrBackendState")
        .use_core()
        .clang_arg(format!("-std=c++{CPP_VERSION}"))
        .clang_args(&["-x", "c++"])
        .clang_arg("-v");

    // gpu builds

    if build.features.gpu() {
        builder = builder
            // bindgen 0.70 alignment problems on i686-linux-android
            .blocklist_type("GrBackendFormat_AnyFormatData")
            .raw_line("#[repr(C, align(8))] pub struct GrBackendFormat_AnyFormatData { data: [u8;GrBackendFormat_kMaxSubclassSize + 1] }")
            .blocklist_type("GrBackendTexture_AnyTextureData")
            .raw_line("#[repr(C, align(8))] pub struct GrBackendTexture_AnyTextureData { data: [u8;GrBackendTexture_kMaxSubclassSize + 1] }")
            .blocklist_type("GrBackendRenderTarget_AnyRenderTargetData")
            .raw_line("#[repr(C, align(8))] pub struct GrBackendRenderTarget_AnyRenderTargetData { data: [u8;GrBackendRenderTarget_kMaxSubclassSize + 1] }")
            .blocklist_type("GrBackendSemaphore_AnySemaphoreData")
            .raw_line("#[repr(C, align(8))] pub struct GrBackendSemaphore_AnySemaphoreData { data: [u8;GrBackendSemaphore_kMaxSubclassSize + 1] }");
    }

    // Don't generate destructors for Windows targets:
    // <https://github.com/rust-skia/rust-skia/issues/318>
    if target.is_windows() {
        builder = builder.with_codegen_config({
            let mut config = CodegenConfig::default();
            config.remove(CodegenConfig::DESTRUCTORS);
            config
        });
    }

    for function in ALLOWLISTED_FUNCTIONS {
        builder = builder.allowlist_function(function)
    }

    for opaque_type in OPAQUE_TYPES {
        builder = builder.opaque_type(opaque_type)
    }

    for t in BLOCKLISTED_TYPES {
        builder = builder.blocklist_type(t);
    }

    let mut cc_build = Build::new();

    for source in &build.binding_sources {
        cc_build.file(source);
        let source = source.to_str().unwrap();
        cargo::rerun_if_file_changed(source);
        builder = builder.header(source);
    }

    let mut bindgen_args = Vec::new();
    let mut cc_defines = Vec::new();
    let mut cc_args = Vec::new();

    let include_path = &build.skia_source_dir;
    cargo::rerun_if_file_changed(include_path.join("include"));

    bindgen_args.push(format!("-I{}", include_path.display()));
    cc_build.include(include_path);

    for (name, value) in &build.definitions {
        match value {
            Some(value) => {
                cc_defines.push((name, value.as_str()));
                bindgen_args.push(format!("-D{name}={value}"));
            }
            None => {
                cc_defines.push((name, ""));
                bindgen_args.push(format!("-D{name}"));
            }
        }
    }

    cc_build.cpp(true).out_dir(output_directory);

    {
        let std_cpp_arg = if target.builds_with_msvc() {
            format!("/std:c++{CPP_VERSION}")
        } else {
            format!("-std=c++{CPP_VERSION}")
        };
        cc_args.push(std_cpp_arg);
    }

    // Disable RTTI. Otherwise RustWStream may cause compilation errors.
    bindgen_args.push("-fno-rtti".into());
    if target.builds_with_msvc() {
        cc_args.push("/GR-".into());
    } else {
        cc_args.push("-fno-rtti".into());
    }

    // Set platform specific arguments and flags and target.
    {
        let args = platform::bindgen_and_cc_args(&target, sysroot);

        bindgen_args.extend(args.args.clone());
        cc_args.extend(args.args);

        let mut target_str = &target.to_string();
        let mut override_target = false;
        if let Some(target) = &args.target_override {
            target_str = target;
            override_target = true;
        }

        // If we use the target() function for override targets, cc will override it based on the
        // environment, for example when targeting the ios simulator.
        if override_target {
            cc_args.push(format!("--target={target_str}"));
        }
        bindgen_args.push(format!("--target={target_str}"));
    }

    {
        println!("COMPILING BINDINGS: {:?}", build.binding_sources);
        println!(
            "  DEFINES: {}",
            cc_defines
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        println!("  ARGS: {}", cc_args.join(" "));

        for (var, val) in cc_defines {
            cc_build.define(var, val);
        }

        for arg in cc_args {
            cc_build.flag(&arg);
        }

        // we add skia-bindings later on.
        cc_build.cargo_metadata(false);
        cc_build.compile(binaries_config::lib::SKIA_BINDINGS);
    }

    {
        println!("GENERATING BINDINGS");
        println!("  ARGS: {}", bindgen_args.join(" "));

        builder = builder.clang_args(bindgen_args);

        let bindings = builder.generate().expect("Unable to generate bindings");
        bindings
            .write_to_file(output_directory.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}

const ALLOWLISTED_FUNCTIONS: &[&str] = &[
    "SkAnnotateRectWithURL",
    "SkAnnotateNamedDestination",
    "SkAnnotateLinkToDestination",
    "SkColorTypeBytesPerPixel",
    "SkColorTypeIsAlwaysOpaque",
    "SkColorTypeValidateAlphaType",
    "SkRGBToHSV",
    "SkHSVToColor",
    "SkPreMultiplyARGB",
    "SkPreMultiplyColor",
    "SkBlendMode_AsCoeff",
    "SkBlendMode_Name",
    "SkSwapRB",
    // pathops/
    "Op",
    "Simplify",
    "TightBounds",
    "AsWinding",
    "SkYUVColorSpaceIsLimitedRange",
];

const OPAQUE_TYPES: &[&str] = &[
    // Types for which the binding generator pulls in stuff that can not be compiled.
    "SkDeferredDisplayList",
    "SkDeferredDisplayList_PendingPathsMap",
    // Types for which a bindgen layout is wrong causing types that contain
    // fields of them to fail their layout test.
    // Windows:
    "std::atomic",
    "std::function",
    "std::unique_ptr",
    "SkTHashMap",
    // Ubuntu 18 LLVM 6: all types derived from SkWeakRefCnt
    "SkWeakRefCnt",
    "GrContext",
    "GrGLInterface",
    "GrSurfaceProxy",
    "Sk2DPathEffect",
    "SkCornerPathEffect",
    "SkDataTable",
    "SkDiscretePathEffect",
    "SkDrawable",
    "SkLine2DPathEffect",
    "SkPath2DPathEffect",
    "SkPathRef_GenIDChangeListener",
    "SkPicture",
    "SkPixelRef",
    "SkSurface",
    // Types not needed (for now):
    "SkDeque",
    "SkDeque_Iter",
    "GrGLInterface_Functions",
    // SkShaper (m77) Trivial*Iterator classes create two vtable pointers.
    "SkShaper_TrivialBiDiRunIterator",
    "SkShaper_TrivialFontRunIterator",
    "SkShaper_TrivialLanguageRunIterator",
    "SkShaper_TrivialScriptRunIterator",
    // skparagraph
    "std::vector",
    "std::u16string",
    // skparagraph (m78), (layout fails on macOS and Linux, not sure why, looks like an obscure alignment problem)
    "skia::textlayout::FontCollection",
    // skparagraph (m79), std::map is used in LineMetrics
    "std::map",
    // Vulkan reexports with the wrong field naming conventions.
    "VkPhysicalDeviceFeatures",
    "VkPhysicalDeviceFeatures2",
    // Since Rust 1.39 beta (TODO: investigate why, and re-test when 1.39 goes stable).
    "GrContextOptions_PersistentCache",
    "GrContextOptions_ShaderErrorHandler",
    "Sk1DPathEffect",
    "SkBBoxHierarchy", // vtable
    "SkBBHFactory",
    "SkBitmap_Allocator",
    "SkBitmap_HeapAllocator",
    "SkColorFilter",
    "SkDeque_F2BIter",
    "SkDrawable_GpuDrawHandler",
    "SkFlattenable",
    "SkFontMgr",
    "SkFontStyleSet",
    "SkMaskFilter",
    "SkPathEffect",
    "SkPicture_AbortCallback",
    "SkPixelRef_GenIDChangeListener",
    "SkRasterHandleAllocator",
    // m114: Must keep `SkRefCnt`, because otherwise bindgen would add an additional vtable because
    // of its newly introduced virtual functions.
    // "SkRefCnt",
    "SkShader",
    "SkStream",
    "SkStreamAsset",
    "SkStreamMemory",
    "SkStreamRewindable",
    "SkStreamSeekable",
    "SkTypeface_LocalizedStrings",
    "SkWStream",
    "GrVkMemoryAllocator",
    "SkShaper",
    "SkShaper_BiDiRunIterator",
    "SkShaper_FontRunIterator",
    "SkShaper_LanguageRunIterator",
    "SkShaper_RunHandler",
    "SkShaper_RunIterator",
    "SkShaper_ScriptRunIterator",
    "SkContourMeasure",
    "SkDocument",
    // m81: tuples:
    "SkRuntimeEffect_EffectResult",
    "SkRuntimeEffect_ByteCodeResult",
    "SkRuntimeEffect_SpecializeResult",
    // m81: derives from std::string
    "SkSL::String",
    // The following two were replaced by a more generic variant further down (LLVM 21)
    // "std::basic_string",
    // "std::basic_string_value_type",
    // m81: wrong size on macOS and Linux
    "SkRuntimeEffect",
    "GrShaderCaps",
    // more stuff we don't need that was tracked down fixing:
    // https://github.com/rust-skia/rust-skia/issues/318
    // referred from SkPath, but not used:
    "SkPathRef",
    "SkMutex",
    // m82: private
    "SkIDChangeListener",
    // m86:
    "GrRecordingContext",
    "GrDirectContext",
    // m87:
    "GrD3DAlloc",
    "GrD3DMemoryAllocator",
    // m87, yuva_pixmaps
    "std::tuple",
    // Homebrew macOS LLVM 13
    "std::tuple_.*",
    // Since 3.1.57 of the emsdk: <https://github.com/rust-skia/rust-skia/issues/975>
    "std::__2::.*",
    // clang 18 / XCode 16
    "std::__1::.*",
    // m93: private, exposed by Paint::asBlendMode(), fails layout tests.
    "skstd::optional",
    // m100
    "std::optional",
    // Feature `svg`:
    "SkSVGProperty",
    "SkSVGNode",
    "SkTLazy",             // causes wrong layouts in SkSVGSVG
    "SkTCopyOnFirstWrite", // causes wrong layouts in SkSVGRenderContext
    "skresources::ResourceProvider",
    // m107 (layout failure)
    "skgpu::VulkanMemoryAllocator",
    // m109 (ParagraphPainter::SkPaintOrID)
    "std::variant",
    // m111 Used in SkTextBlobBuilder
    "skia_private::AutoTMalloc",
    // Pulled in by `SkData`.
    "FILE",
    // m114: Results in wrongly sized template specializations.
    "skia_private::THashMap",
    // m121:
    "skgpu::MutableTextureState",
    // emscripten: Uses SkLRUCache (which is blocklisted)
    "skia::textlayout::ParagraphCache",
    // Fix bindgen 0.70 layout failures
    "skgpu::VulkanBackendContext",
    "GrYUVABackendTextures",
    // LLVM21
    "std::basic_string.*",
    "std::__tree.*",
    // libstdc++ 10 on Linux (since m143, c++20)
    "std::strong_ordering",
    // skottie internal types with layout issues
    "skottie::internal::TextAnimator",
    "skottie::internal::TextAnimator_AnimatedProps",
    "skottie::internal::TextAdapter",
    "skottie::VectorValue",
    "skottie::ColorValue",
    "sksg::PaintNode",
    "sksg::Color",
    "sksg::BlurImageFilter",
    // m147
    "std::unordered_map.*",
];

const BLOCKLISTED_TYPES: &[&str] = &[
    // modules/skparagraph
    //   pulls in a std::map<>, which we treat as opaque, but bindgen creates wrong bindings for
    //   std::_Tree* types
    "std::_Tree.*",
    "std::map.*",
    //   debug builds:
    "SkLRUCache",
    "SkLRUCache_Entry",
    //   not used at all:
    "std::vector.*",
    // too much template magic:
    "SkRuntimeEffect_ConstIterable.*",
    // Linux LLVM9 c++17
    "std::_Rb_tree.*",
    // archlinux
    "std::__rb_tree.*",
    // Linux LLVM9 c++17 with SKIA_DEBUG=1
    "std::__cxx.*",
    "std::array.*",
    // m115 unused Linux
    "std::__uset_hashtable.*",
    "std::unordered_set.*",
    // m115 unused Windows
    "std::_List_unchecked.*",
    "std::_Hash.*",
    "std::_List_const.*",
    "std::list.*",
    "std::list__Unchecked.*",
    "std::_List_iterator.*",
    // <https://github.com/rust-skia/rust-skia/issues/1009> (feature vulkan)
    "PFN_vkVoidFunction",
];

#[derive(Debug)]
struct ParseCallbacks;

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
    /// Allows to rename an enum variant, replacing `_original_variant_name`.
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        enum_name.and_then(|enum_name| {
            ENUM_REWRITES
                .iter()
                .find(|n| n.0 == enum_name)
                .map(|(_, replacer)| replacer(enum_name, original_variant_name))
        })
    }

    fn item_name(&self, original_item_name: bindgen::callbacks::ItemInfo<'_>) -> Option<String> {
        ITEM_RENAMES
            .iter()
            .find(|(original, _)| *original == original_item_name.name)
            .map(|(_, replacement)| replacement.to_string())
    }
}

type EnumEntry = (&'static str, fn(&str, &str) -> String);

const ITEM_RENAMES: &[(&str, &str)] = &[
    ("std___1_string_view", "std_string_view"),
    ("std___2_string_view", "std_string_view"),
    ("std___1_string", "std_string"),
    ("std___2_string", "std_string"),
];

const ENUM_REWRITES: &[EnumEntry] = &[
    //
    // codec/
    //
    ("DocumentStructureType", rewrite::k_xxx),
    ("ZeroInitialized", rewrite::k_xxx_name),
    ("SelectionPolicy", rewrite::k_xxx),
    //
    // core/ effects/
    //
    ("SkApplyPerspectiveClip", rewrite::k_xxx),
    ("SkBlendMode", rewrite::k_xxx),
    ("SkBlendModeCoeff", rewrite::k_xxx),
    ("SkBlurStyle", rewrite::k_xxx_name),
    ("SkClipOp", rewrite::k_xxx),
    ("SkColorChannel", rewrite::k_xxx),
    ("SkCoverageMode", rewrite::k_xxx),
    ("SkEncodedImageFormat", rewrite::k_xxx),
    ("SkEncodedOrigin", rewrite::k_xxx_name),
    ("SkFilterQuality", rewrite::k_xxx_name),
    ("SkFontHinting", rewrite::k_xxx),
    ("SkAlphaType", rewrite::k_xxx_name),
    ("SkYUVColorSpace", rewrite::k_xxx_name),
    ("SkPathFillType", rewrite::k_xxx),
    ("SkPathConvexityType", rewrite::k_xxx),
    ("SkPathDirection", rewrite::k_xxx),
    ("SkPathVerb", rewrite::k_xxx),
    ("SkPathOp", rewrite::k_xxx_name),
    ("SkTileMode", rewrite::k_xxx),
    // svg/
    ("Unit", rewrite::k_xxx),
    ("Scale", rewrite::k_xxx),
    ("SkSVGLineCap", rewrite::k_xxx),
    ("SkSVGXmlSpace", rewrite::k_xxx),
    ("SkSVGColorspace", rewrite::k_xxx),
    ("SkSVGDisplay", rewrite::k_xxx),
    ("SkSVGAttribute", rewrite::k_xxx),
    ("SkSVGTag", rewrite::k_xxx),
    ("LengthType", rewrite::k_xxx),
    // SkPaint_Style
    // SkStrokeRec_Style
    // SkPath1DPathEffect_Style
    ("Style", rewrite::k_xxx_name_opt),
    // SkPaint_Cap
    ("Cap", rewrite::k_xxx_name),
    // SkPaint_Join
    ("Join", rewrite::k_xxx_name),
    // SkStrokeRec_InitStyle
    ("InitStyle", rewrite::k_xxx_name),
    // SkBlurImageFilter_TileMode
    // SkMatrixConvolutionImageFilter_TileMode
    ("TileMode", rewrite::k_xxx_name),
    // SkCanvas_*
    ("PointMode", rewrite::k_xxx_name),
    ("SrcRectConstraint", rewrite::k_xxx_name),
    // SkCanvas_Lattice_RectType
    ("RectType", rewrite::k_xxx),
    // SkDisplacementMapEffect_ChannelSelectorType
    ("ChannelSelectorType", rewrite::k_xxx_name),
    // SkDropShadowImageFilter_ShadowMode
    ("ShadowMode", rewrite::k_xxx_name),
    // SkFont_Edging
    ("Edging", rewrite::k_xxx),
    // SkFont_Slant
    ("Slant", rewrite::k_xxx_name),
    // SkHighContrastConfig_InvertStyle
    ("InvertStyle", rewrite::k_xxx),
    // SkImage_*
    ("BitDepth", rewrite::k_xxx),
    ("CachingHint", rewrite::k_xxx_name),
    ("SkTextureCompressionType", rewrite::k_xxx),
    // SkImageFilter_MapDirection
    ("MapDirection", rewrite::k_xxx_name),
    // SkCodec_Result
    // SkInterpolatorBase_Result
    ("Result", rewrite::k_xxx),
    // SkMatrix_ScaleToFit
    ("ScaleToFit", rewrite::k_xxx_name),
    // SkPathBuilder_*
    ("ArcSize", rewrite::k_xxx_name),
    ("AddPathMode", rewrite::k_xxx_name),
    // SkPathBuilder_*
    ("DumpFormat", rewrite::k_xxx),
    // SkRegion_Op
    // TODO: remove kLastOp?
    ("Op", rewrite::k_xxx_name_opt),
    // SkRRect_*
    // TODO: remove kLastType?
    // SkRuntimeEffect_Uniform_Type
    ("Type", rewrite::k_xxx_name_opt),
    ("Corner", rewrite::k_xxx_name),
    // SkShader_GradientType
    ("GradientType", rewrite::k_xxx_name),
    // SkSurface_*
    ("ContentChangeMode", rewrite::k_xxx_name),
    ("BackendHandleAccess", rewrite::k_xxx),
    // SkTextUtils_Align
    // We need name_opt to cover SkSVGPreserveAspectRatio_Align
    ("Align", rewrite::k_xxx_name_opt),
    // SkTrimPathEffect_Mode
    ("Mode", rewrite::k_xxx),
    // SkTypeface_SerializeBehavior
    ("SerializeBehavior", rewrite::k_xxx),
    // SkVertices_VertexMode
    ("VertexMode", rewrite::k_xxx_name),
    // SkYUVAIndex_Index
    ("Index", rewrite::k_xxx_name),
    // SkRuntimeEffect_Variable_Qualifier
    ("Qualifier", rewrite::k_xxx),
    // private type that leaks through SkRuntimeEffect_Variable
    ("GrSLType", rewrite::k_xxx_name),
    //
    // gpu/
    //
    ("Origin", rewrite::k_xxx),
    ("GrGLStandard", rewrite::k_xxx_name),
    ("GrGLFormat", rewrite::k_xxx),
    ("GrSurfaceOrigin", rewrite::k_xxx_name),
    ("GrBackendApi", rewrite::k_xxx),
    ("Mipmapped", rewrite::k_xxx),
    ("Renderable", rewrite::k_xxx),
    ("Protected", rewrite::k_xxx),
    //
    // DartTypes.h
    //
    ("Affinity", rewrite::k_xxx),
    ("TextAlign", rewrite::k_xxx),
    ("TextDirection", rewrite::k_xxx_uppercase),
    ("TextBaseline", rewrite::k_xxx),
    ("TextHeightBehavior", rewrite::k_xxx),
    ("DrawOptions", rewrite::k_xxx),
    //
    // TextStyle.h
    //
    ("TextDecorationStyle", rewrite::k_xxx),
    ("TextDecorationMode", rewrite::k_xxx),
    ("StyleType", rewrite::k_xxx),
    //
    // Vk*
    //
    ("VkChromaLocation", rewrite::vk),
    ("VkFilter", rewrite::vk),
    ("VkFormat", rewrite::vk),
    ("VkImageLayout", rewrite::vk),
    ("VkImageTiling", rewrite::vk),
    ("VkSamplerYcbcrModelConversion", rewrite::vk),
    ("VkSamplerYcbcrRange", rewrite::vk),
    ("VkStructureType", rewrite::vk),
    // m84: SkPath::Verb
    ("Verb", rewrite::k_xxx_name),
    // m84: SkVertices::Attribute::Usage
    ("Usage", rewrite::k_xxx),
    ("GrSemaphoresSubmitted", rewrite::k_xxx),
    ("BackendSurfaceAccess", rewrite::k_xxx),
    // m85
    ("VkSharingMode", rewrite::vk),
    // m86:
    ("SkFilterMode", rewrite::k_xxx),
    ("SkMipmapMode", rewrite::k_xxx),
    ("Enable", rewrite::k_xxx),
    ("ShaderCacheStrategy", rewrite::k_xxx),
    // m87:
    // SkYUVAInfo_PlanarConfig
    ("PlanarConfig", rewrite::k_xxx),
    ("Siting", rewrite::k_xxx),
    // SkYUVAPixmapInfo
    ("DataType", rewrite::k_xxx),
    // m88:
    // SkYUVAInfo_*
    ("PlaneConfig", rewrite::k_xxx),
    // m89, SkImageFilters::Dither
    ("Dither", rewrite::k_xxx),
    ("SkScanlineOrder", rewrite::k_xxx_name),
    // m94: SkRuntimeEffect::ChildType
    ("ChildType", rewrite::k_xxx_name_opt),
    // m108: SkGradientShader::Interpolation::InPremul
    ("InPremul", rewrite::k_xxx),
    // m108: skgpu::BackendApi
    ("BackendApi", rewrite::k_xxx),
    // m109: SkGradientShader::Interpolation::ColorSpace
    ("ColorSpace", rewrite::k_xxx),
    // m109: SkGradientShader::Interpolation::HueMethod
    ("HueMethod", rewrite::k_xxx),
    // SkCodecAnimation
    ("DisposalMethod", rewrite::k_xxx),
    ("Blend", rewrite::k_xxx),
    // SkJpegEncoder.h
    ("AlphaOption", rewrite::k_xxx),
    // SkWebpEncoder.h
    ("Compression", rewrite::k_xxx),
    // m118:
    ("GrPurgeResourceOptions", rewrite::k_xxx),
    ("GrSyncCpu", rewrite::k_xxx),
    // m129:
    ("Clamp", rewrite::k_xxx), // SkColorFilters
    // svg:
    ("SkSVGFeColorMatrixType", rewrite::k_xxx),
    ("SkSVGFeCompositeOperator", rewrite::k_xxx),
    ("SkSVGFeFuncType", rewrite::k_xxx),
    // SkSVGFeMorphology::Operator
    ("Operator", rewrite::k_xxx),
    // m131:
    ("GrMarkFrameBoundary", rewrite::k_xxx),
    // SkResources.h
    ("ImageDecodeStrategy", rewrite::k_xxx),
    // SkNamedPrimaries::CicpId, SkNamedTransferFn::CicpId
    ("CicpId", rewrite::k_xxx),
    // `SkCodec::IsAnimated`s
    ("IsAnimated", rewrite::k_xxx),
    // m142: PngRustEncoder::CompressionLevel
    ("CompressionLevel", rewrite::k_opt_xxx),
    // m148: SkShapers::CT::LineBreakMode
    ("LineBreakMode", rewrite::k_xxx),
];

pub(crate) mod rewrite {
    use heck::ToShoutySnakeCase;
    use regex::Regex;

    pub fn k_xxx_uppercase(name: &str, variant: &str) -> String {
        k_xxx(name, variant).to_uppercase()
    }

    pub fn k_xxx(name: &str, variant: &str) -> String {
        if let Some(stripped) = variant.strip_prefix('k') {
            stripped.into()
        } else {
            panic!(
                "Variant name '{variant}' of enum type '{name}' is expected to start with a 'k'"
            );
        }
    }

    // For enums that come with and without a `k` prefix.
    pub fn k_opt_xxx(_name: &str, variant: &str) -> String {
        if let Some(stripped) = variant.strip_prefix('k') {
            stripped.into()
        } else {
            variant.into()
        }
    }

    pub fn _k_xxx_enum(name: &str, variant: &str) -> String {
        capture(name, variant, &format!("k(.*)_{name}"))
    }

    pub fn k_xxx_name_opt(name: &str, variant: &str) -> String {
        let suffix = &format!("_{name}");
        let value = if variant.ends_with(suffix) {
            capture(name, variant, &format!("k(.*){suffix}"))
        } else {
            capture(name, variant, "k(.*)")
        };

        if value.parse::<usize>().is_ok() {
            // it's a FontWeight::Type
            format!("W{value}") // W(eight)
        } else {
            value
        }
    }

    pub fn k_xxx_name(name: &str, variant: &str) -> String {
        capture(name, variant, &format!("k(.*)_{name}"))
    }

    pub fn vk(name: &str, variant: &str) -> String {
        let prefix = name.to_shouty_snake_case();
        capture(name, variant, &format!("{prefix}_(.*)"))
    }

    fn capture(name: &str, variant: &str, pattern: &str) -> String {
        let re = Regex::new(pattern).unwrap();
        re.captures(variant).unwrap_or_else(|| {
            panic!("failed to match '{pattern}' on enum variant '{variant}' of enum '{name}'")
        })[1]
            .into()
    }
}

#[allow(unused)]
pub use definitions::{Definition, Definitions};

pub(crate) mod definitions {
    use std::{
        collections::HashSet,
        fs,
        io::Write,
        path::{Path, PathBuf},
    };

    use super::env;
    use crate::build_support::features::{self, feature};

    /// A preprocessor definition.
    pub type Definition = (String, Option<String>);
    /// A container for a number of preprocessor definitions.
    pub type Definitions = Vec<Definition>;

    pub fn from_env() -> Definitions {
        let env_string =
            env::skia_lib_definitions().expect("must include library definition environment");
        from_defines_str(&env_string)
    }

    pub fn save_definitions(
        definitions: &[Definition],
        output_directory: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        fs::create_dir_all(&output_directory)?;
        let mut file = fs::File::create(output_directory.as_ref().join("skia-defines.txt"))?;
        for (name, value) in definitions.iter() {
            if let Some(value) = value {
                writeln!(file, "-D{name}={value}")?;
            } else {
                writeln!(file, "-D{name}")?;
            }
        }
        writeln!(file)
    }

    // Extracts definitions from ninja files that need to be parsed for build consistency.
    pub fn from_ninja_features(
        features: &features::Features,
        use_system_libraries: bool,
        output_directory: &Path,
    ) -> Definitions {
        let ninja_files = ninja_files_for_features(features, use_system_libraries);
        from_ninja_files(&ninja_files, output_directory)
    }

    fn from_ninja_files(ninja_files: &[PathBuf], output_directory: &Path) -> Definitions {
        let mut definitions = Vec::new();

        for ninja_file in ninja_files {
            let ninja_file = output_directory.join(ninja_file);
            let contents = fs::read_to_string(&ninja_file).unwrap_or_else(|err| {
                panic!(
                    "Failed to read ninja file: `{}`: {err}",
                    ninja_file.display()
                )
            });
            definitions = combine(definitions, from_ninja_file_content(&ninja_file, contents))
        }

        definitions
    }

    /// Parse a defines = line from a ninja build file.
    fn from_ninja_file_content(file_path: &Path, contents: impl AsRef<str>) -> Definitions {
        let defines = {
            let prefix = "defines = ";
            let defines = contents
                .as_ref()
                .lines()
                .find(|s| s.starts_with(prefix))
                .unwrap_or_else(|| {
                    panic!(
                        "Missing a line starting with `defines =` in: {}",
                        file_path.display()
                    )
                });
            &defines[prefix.len()..]
        };
        from_defines_str(defines)
    }

    fn ninja_files_for_features(
        features: &features::Features,
        use_system_libraries: bool,
    ) -> Vec<PathBuf> {
        let mut files = vec!["obj/skia.ninja".into()];
        if features.gpu() {
            files.push("obj/gpu.ninja".into());
        }
        if features[feature::TEXTLAYOUT] {
            files.extend(vec![
                "obj/modules/skshaper/skshaper.ninja".into(),
                "obj/modules/skparagraph/skparagraph.ninja".into(),
                "obj/modules/skunicode/skunicode_core.ninja".into(),
                "obj/modules/skunicode/skunicode_icu.ninja".into(),
            ]);
            // shaper.cpp includes SkLoadICU.h
            if !use_system_libraries {
                files.push("obj/third_party/icu/icu.ninja".into())
            }
        }
        if features[feature::SVG] {
            files.push("obj/modules/svg/svg.ninja".into());
        }
        if features[feature::SKOTTIE] {
            files.push("obj/modules/skottie/skottie.ninja".into());
        }
        files
    }

    fn combine(a: Definitions, b: Definitions) -> Definitions {
        remove_duplicates(a.into_iter().chain(b).collect())
    }

    fn remove_duplicates(mut definitions: Definitions) -> Definitions {
        let mut uniques = HashSet::new();
        definitions.retain(|e| uniques.insert(e.0.clone()));
        definitions
    }

    fn from_defines_str(defines: &str) -> Definitions {
        const PREFIX: &str = "-D";
        defines
            .split_whitespace()
            .map(|d| {
                if let Some(stripped) = d.strip_prefix(PREFIX) {
                    stripped
                } else {
                    panic!("Missing '{PREFIX}' prefix from a definition")
                }
            })
            .map(|d| {
                let items: Vec<&str> = d.splitn(2, '=').collect();
                match items.len() {
                    1 => (items[0].to_string(), None),
                    2 => (items[0].to_string(), Some(unescape_ninja(items[1]))),
                    _ => panic!("Internal error"),
                }
            })
            .collect()
    }

    fn unescape_ninja(input: &str) -> String {
        unescape(&unescape(input, '$'), '\\')
    }

    fn unescape(input: &str, escape_character: char) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == escape_character {
                if let Some(&next_ch) = chars.peek() {
                    chars.next();
                    result.push(next_ch);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn properly_unescape_trivial_abi() {
            // This happens if SKIA_DEBUG=1
            let str = r#"\[\[clang$:$:trivial_abi\]\]"#;
            assert_eq!(super::unescape_ninja(str), "[[clang::trivial_abi]]");
        }
    }
}
