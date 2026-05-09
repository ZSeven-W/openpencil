use std::process::{Command, Stdio};

use super::{generic, prelude::*};
use pkg_config;

pub struct Linux;

impl PlatformDetails for Linux {
    fn uses_freetype(&self) -> bool {
        true
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        generic::gn_args(config, builder);
        // This makes it possible for clang++ to locate the C++ includes.
        builder.target_str = Some(linux_shortened_target_str(&config.target));

        let target = &config.target;
        builder.cflags(flags(target));
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        builder.args(flags(target))
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        link_libraries(features)
    }
}

pub fn link_libraries(features: &Features) -> Vec<String> {
    let mut libs = vec!["stdc++".to_string()];

    // Use pkg-config for system libraries when available
    add_pkg_config_libs(&mut libs, "freetype2", &["freetype"]);
    add_pkg_config_libs(&mut libs, "fontconfig", &["fontconfig"]);

    if features[feature::GL] {
        if features[feature::EGL] {
            add_pkg_config_libs(&mut libs, "egl", &["EGL"]);
        }

        if features[feature::X11] {
            add_pkg_config_libs(&mut libs, "gl", &["GL"]);
        }

        if features[feature::WAYLAND] {
            add_pkg_config_libs(&mut libs, "wayland-egl", &["wayland-egl"]);
            libs.push("GLESv2".to_string()); // Fallback for GLESv2
        }
    }

    if skia::env::use_system_libraries() {
        // Use pkg-config for these libraries
        add_pkg_config_libs(&mut libs, "libpng", &["png16"]);
        add_pkg_config_libs(&mut libs, "zlib", &["z"]);
        add_pkg_config_libs(&mut libs, "harfbuzz", &["harfbuzz"]);
        add_pkg_config_libs(&mut libs, "expat", &["expat"]);

        // ICU libraries - try pkg-config first, fallback to manual linking
        add_pkg_config_libs(&mut libs, "icu-uc", &["icuuc"]);
        add_pkg_config_libs(&mut libs, "icu-i18n", &["icui18n"]);
        add_pkg_config_libs(&mut libs, "icu-io", &["icuio"]);
        // Note: removed icutest and icutu as they appear to be development/testing utilities

        if features[feature::WEBP_ENCODE] || features[feature::WEBP_DECODE] {
            add_pkg_config_libs(&mut libs, "libwebp", &["webp"]);
        }
    }

    if skia::env::use_system_libraries() || cfg!(feature = "use-system-jpeg-turbo") {
        libs.push("jpeg".to_string());
    }

    libs
}

fn add_pkg_config_libs(libs: &mut Vec<String>, pkg_name: &str, fallback_libs: &[&str]) {
    match pkg_config::probe_library(pkg_name) {
        Ok(library) => {
            for lib in &library.libs {
                libs.push(lib.clone());
            }
        }
        Err(_) => {
            // Fallback to hardcoded library names
            for lib in fallback_libs {
                libs.push(lib.to_string());
            }
        }
    }
}

fn flags(target: &Target) -> Vec<String> {
    if *target != cargo::host() {
        // For cross-compilation.
        // The default "7" is specific to Ubuntu 18+.
        let stdlib_version = get_stdlib_version().unwrap_or("7".into());
        // TODO: Do we need this anymore (we since changed the target to linux_target_str, clang /
        // clang++ should be able to locate the include paths).
        let target_path_component = linux_shortened_target_str(target);
        [
            format!("-I/usr/{target_path_component}/include/c++/{stdlib_version}/{target_path_component}"),
            format!("-I/usr/{target_path_component}/include"),
        ]
        .into()
    } else {
        Vec::new()
    }
}

pub fn get_stdlib_version() -> Option<String> {
    let mut cmd = Command::new("g++");
    cmd.arg("-dumpversion");
    let output = cmd.stderr(Stdio::inherit()).output().ok()?;
    if output.status.code() != Some(0) {
        return None;
    }
    match String::from_utf8(output.stdout).ok()?.trim() {
        "" => None,
        v => Some(v.into()),
    }
}

// A target for building on Linux, separated by `-` without vendor. Also used for include path components.
pub fn linux_shortened_target_str(target: &Target) -> String {
    let abi = target
        .abi
        .as_deref()
        .map(|abi| format!("-{abi}"))
        .unwrap_or_default();
    format!("{}-{}{}", target.architecture, target.system, abi)
}
