use std::path::Path;

use super::{generic, prelude::*};

pub struct Emscripten;

impl PlatformDetails for Emscripten {
    fn uses_freetype(&self) -> bool {
        true
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        let features = &config.features;
        let emsdk_base_dir = emsdk_base_dir();

        builder
            .arg("skia_gl_standard", quote("webgl"))
            .arg("skia_use_webgl", yes_if(features.gpu()))
            .arg("target_cpu", quote("wasm"))
            .arg("skia_emsdk_dir", quote(&emsdk_base_dir));

        // The custom embedded font manager is enabled by default on WASM, but depends
        // on the undefined symbol `SK_EMBEDDED_FONTS`. Enable the custom empty font
        // manager instead so typeface creation still works.
        // See https://github.com/rust-skia/rust-skia/issues/648
        builder
            .arg("skia_enable_fontmgr_custom_embedded", no())
            .arg("skia_enable_fontmgr_custom_empty", yes());
    }

    fn bindgen_args(&self, _target: &cargo::Target, builder: &mut BindgenArgsBuilder) {
        builder.arg("-nobuiltininc");

        // visibility=default, otherwise some types may be missing:
        // <https://github.com/rust-lang/rust-bindgen/issues/751#issuecomment-555735577>
        builder.arg("-fvisibility=default");

        let emsdk_base_dir = emsdk_base_dir();

        let sysroot_include = format!("{emsdk_base_dir}/upstream/emscripten/cache/sysroot/include");
        if Path::new(&sysroot_include).is_dir() {
            // Newer emsdk versions expose headers via cache/sysroot and reject direct includes
            // from upstream/emscripten/system.
            let libcxx_include = format!("{sysroot_include}/c++/v1");
            if Path::new(&libcxx_include).is_dir() {
                builder.arg(format!("-isystem{libcxx_include}"));
            }
            builder.arg(format!("-isystem{sysroot_include}"));

            // For xlocale.h
            let compat_include = format!("{sysroot_include}/compat");
            if Path::new(&compat_include).is_dir() {
                builder.arg(format!("-isystem{compat_include}"));
            }
            return;
        }

        // Add C++ includes (otherwise build will fail with <cmath> not found)
        let mut add_sys_include = |path: &str| {
            builder.arg(format!(
                "-isystem{emsdk_base_dir}/upstream/emscripten/system/{path}",
            ));
        };

        add_sys_include("lib/libc/musl/arch/emscripten");
        add_sys_include("lib/libc/musl/arch/generic");
        add_sys_include("lib/libcxx/include");
        add_sys_include("lib/libc/musl/include");
        add_sys_include("include");
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        generic::link_libraries(features)
    }

    fn filter_platform_features(
        &self,
        _use_system_libraries: bool,
        mut features: Features,
    ) -> Features {
        features += feature::EMBED_FREETYPE;
        features
    }
}

fn emsdk_base_dir() -> String {
    match std::env::var("EMSDK") {
        Ok(val) => val,
        Err(_e) => panic!(
            "please set the EMSDK environment variable to the root of your Emscripten installation"
        ),
    }
}
