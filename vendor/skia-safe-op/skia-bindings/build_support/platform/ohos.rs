use std::path::PathBuf;

use super::prelude::*;

pub struct OpenHarmony;

/// For OpenHarmony, we recommend using API12 as the minimum API level
impl PlatformDetails for OpenHarmony {
    fn uses_freetype(&self) -> bool {
        false
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        // linux::gn_args(config, builder);

        // disable fontconfig
        builder.arg("skia_use_egl", yes());
        builder.arg("skia_gl_standard", quote("gles"));
        builder.arg("skia_use_gl", yes());
        builder.arg("skia_enable_vulkan_debug_layers", no());
        builder.arg("skia_use_x11", no());
        builder.arg("skia_use_fontconfig", no());
        builder.arg("skia_use_dng_sdk", no());
        builder.arg("skia_enable_tools", no());
        builder.arg("skia_use_system_freetype2", no());
        builder.arg("skia_use_system_libwebp", no());
        builder.arg("skia_use_system_libpng", no());

        builder.target_os_and_default_cpu(&config.target.system);

        builder.cflags(extra_skia_cflags());
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        builder.args(additional_clang_args(
            &target.to_string(),
            &target.architecture,
        ));
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        link_libraries(features)
            .iter()
            .map(|l| l.to_string())
            .collect()
    }
}

fn ndk() -> String {
    // We can accpet the NDK path from the ohos_sdk_native or OHOS_NDK_HOME environment variable
    // OHOS_NDK_HOME should be the parent directory of the ndk directory
    if let Some(native) = cargo::env_var("ohos_sdk_native") {
        return native;
    }

    let home = cargo::env_var("OHOS_NDK_HOME").expect("ohos_sdk_native variable not set");
    PathBuf::from(home)
        .join("native")
        .to_string_lossy()
        .to_string()
}

pub fn additional_clang_args(target: &str, target_arch: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    match target_arch {
        "arm" => {
            args.push("-march=armv7-a".into());
            args.push("-mfloat-abi=softfp".into());
            args.push("-mtune=generic-armv7-a".into());
            args.push("-mthumb".into());
        }
        "x86_64" => args.push("-m64".into()),
        _ => {}
    };

    let ndk = ndk();

    args.push(format!("--sysroot={ndk}/sysroot"));

    args.push(format!("--target={target}"));
    args.extend(extra_skia_cflags());
    args
}

pub fn extra_skia_cflags() -> Vec<String> {
    vec![format!("-D__MUSL__")]
}

pub fn link_libraries(features: &Features) -> Vec<&str> {
    let mut libs = vec!["c++_static", "c++abi"];
    if features[feature::GL] {
        libs.extend(vec!["EGL", "GLESv3"])
    };
    libs
}
