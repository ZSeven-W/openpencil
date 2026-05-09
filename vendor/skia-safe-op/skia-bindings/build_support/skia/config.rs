//! Full build support for the Skia library.

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::build_support::{
    binaries_config, features,
    platform::{self, prelude::*},
};

/// The build configuration for Skia.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildConfiguration {
    /// Do we build _on_ a Windows OS?
    pub on_windows: bool,

    /// Set the optimization level (0-3, s or z). Clang and GCC use the same notation
    /// as Rust, so we just pass this option through from Cargo.
    pub opt_level: Option<String>,

    /// Build Skia in a debug configuration?
    pub skia_debug: bool,

    /// The Skia feature set to compile.
    pub features: features::Features,

    /// C compiler to use
    pub cc: String,

    /// C++ compiler to use
    pub cxx: String,

    /// The target (arch-vendor-os-abi)
    pub target: Target,
}

/// Builds a Skia configuration from a Features set.
impl BuildConfiguration {
    pub fn from_features(features: features::Features, skia_debug: bool) -> Self {
        // Yocto SDKs set CLANGCC/CLANGCXX, which is a better choice to determine clang,
        // as CC/CXX are likely referring to gcc.
        let cc = cargo::env_var("CLANGCC")
            .or_else(|| cargo::env_var("CC"))
            .unwrap_or_else(|| "clang".to_string());
        let cxx = cargo::env_var("CLANGCXX")
            .or_else(|| cargo::env_var("CXX"))
            .unwrap_or_else(|| "clang++".to_string());

        // It's possible that the provided command line for the compiler already includes --target.
        // We assume that it's most specific/appropriate, extract and use is. It might for example include
        // a vendor infix, while cargo targets usually don't.
        let mut target = cc
            .find("--target=")
            .map(|target_option_offset| {
                let target_tail = &cc[(target_option_offset + "--target=".len())..];
                let target_str = target_tail
                    .split_once(' ')
                    .map_or(target_tail, |(target_str, ..)| target_str);
                cargo::parse_target(target_str)
            })
            .unwrap_or_else(cargo::target);

        if target.architecture == "riscv64gc" {
            target.architecture = "riscv64".to_string();
        }

        BuildConfiguration {
            on_windows: cargo::host().is_windows(),
            // `OPT_LEVEL` is set by Cargo itself.
            opt_level: cargo::env_var("OPT_LEVEL"),
            features,
            skia_debug,
            cc,
            cxx,
            target,
        }
    }
}

/// This is the final, low level build configuration.
#[derive(Debug)]
pub struct FinalBuildConfiguration {
    /// The Skia source directory.
    pub skia_source_dir: PathBuf,

    /// Arguments passed to GN.
    pub gn_args: Vec<(String, String)>,

    /// Whether to use system libraries or not.
    pub use_system_libraries: bool,

    /// The target (arch-vendor-os-abi)
    pub target: Target,

    /// An optional target sysroot
    pub sysroot: Option<String>,
}

impl FinalBuildConfiguration {
    pub fn from_build_configuration(
        build: &BuildConfiguration,
        use_system_libraries: bool,
        skia_source_dir: &Path,
    ) -> FinalBuildConfiguration {
        let features =
            platform::filter_features(&build.target, use_system_libraries, build.features.clone());

        // `SDKROOT` is the environment variable used on macOS to specify the sysroot.
        // `SDKTARGETSYSROOT` is the environment variable set in Yocto Linux SDKs when
        // cross-compiling.
        let sysroot = cargo::env_var("SDKTARGETSYSROOT").or_else(|| cargo::env_var("SDKROOT"));

        let mut builder = GnArgsBuilder::new(&build.target);

        let gn_args = {
            builder
                .arg("is_official_build", yes_if(!build.skia_debug))
                .arg("is_debug", yes_if(build.skia_debug))
                .arg("skia_enable_svg", yes_if(features[feature::SVG]))
                .arg("skia_enable_ganesh", yes_if(features.gpu()))
                .arg("skia_enable_skottie", yes_if(features[feature::SKOTTIE]))
                .arg("skia_enable_pdf", yes_if(features[feature::PDF]))
                .arg("skia_use_gl", yes_if(features[feature::GL]))
                .arg("skia_use_egl", yes_if(features[feature::EGL]))
                .arg("skia_use_x11", yes_if(features[feature::X11]))
                .arg("skia_use_system_libpng", yes_if(use_system_libraries))
                .arg(
                    "skia_use_libwebp_encode",
                    yes_if(features[feature::WEBP_ENCODE]),
                )
                .arg(
                    "skia_use_libwebp_decode",
                    yes_if(features[feature::WEBP_DECODE]),
                )
                .arg("skia_use_system_zlib", yes_if(use_system_libraries))
                .arg("skia_use_xps", no())
                .arg("skia_use_dng_sdk", no())
                .arg(
                    "skia_use_libjpeg_turbo_decode",
                    yes_if(features[feature::JPEG_DECODE]),
                )
                .arg(
                    "skia_use_libjpeg_turbo_encode",
                    yes_if(features[feature::JPEG_ENCODE]),
                )
                .arg("cc", quote(&build.cc))
                .arg("cxx", quote(&build.cxx));
            if features[feature::VULKAN] {
                builder
                    .arg("skia_use_vulkan", yes())
                    .arg("skia_enable_spirv_validation", no());
            }

            if features[feature::METAL] {
                builder.arg("skia_use_metal", yes());
            }

            if features[feature::D3D] {
                builder.arg("skia_use_direct3d", yes());
            }

            // further flags that limit the components of Skia debug builds.
            if build.skia_debug {
                builder
                    .arg("skia_enable_spirv_validation", no())
                    .arg("skia_enable_tools", no())
                    .arg("skia_enable_vulkan_debug_layers", no())
                    .arg("skia_use_lua", no());
            }

            if features[feature::TEXTLAYOUT] {
                builder
                    .arg("skia_enable_skshaper", yes())
                    .arg("skia_use_icu", yes())
                    .arg("skia_use_system_icu", yes_if(use_system_libraries))
                    .arg("skia_use_harfbuzz", yes())
                    .arg("skia_pdf_subset_harfbuzz", yes())
                    .arg("skia_use_system_harfbuzz", yes_if(use_system_libraries))
                    .arg("skia_enable_skparagraph", yes());
                // note: currently, tests need to be enabled, because modules/skparagraph
                // is not included in the default dependency configuration.
                // ("paragraph_tests_enabled", no()),
            } else {
                builder
                    .arg("skia_use_icu", no())
                    .arg("skia_use_harfbuzz", no());
            }

            if features[feature::WEBP_ENCODE] || features[feature::WEBP_DECODE] {
                builder.arg("skia_use_system_libwebp", yes_if(use_system_libraries));
            }

            let use_freetype = platform::uses_freetype(&build.target);
            builder.arg("skia_use_freetype", yes_if(use_freetype));
            if use_freetype {
                builder.arg(
                    "skia_use_freetype_woff2",
                    yes_if(features[feature::FREETYPE_WOFF2]),
                );

                if features[feature::EMBED_FREETYPE] {
                    builder.arg("skia_use_system_freetype2", no());
                } else {
                    // third_party/freetype2/BUILD.gn hard-codes /usr/include/freetype2
                    // as include path. When cross-compiling against a sysroot, we don't
                    // want the host directory, we want the path from the sysroot, so prepend
                    // a `=` to substitute the sysroot if present.
                    // Ideally we'd overwrite the skia_system_freetype2_include_path
                    // argument, but somehow that doesn't accept a `=`. So change it to
                    // a non-existent path, append a sysroot prefixed include path, as well
                    // as the previous fallback that's used if no sysroot is specified.
                    builder.arg("skia_system_freetype2_include_path", "\"/does/not/exist\"");
                    builder.cflag("-I=/usr/include/freetype2");
                    builder.cflag("-I/usr/include/freetype2");
                }
            }

            // target specific gn args.
            let target = &build.target;

            if let Some(sysroot) = &sysroot {
                builder.cflag(format!("--sysroot={sysroot}"));
            }

            let jpeg_sys_cflags: Vec<String>;
            if cfg!(feature = "use-system-jpeg-turbo") {
                let paths = cargo::env_var("DEP_JPEG_INCLUDE").expect("mozjpeg-sys include path");
                jpeg_sys_cflags = std::env::split_paths(&paths)
                    .map(|arg| format!("-I{}", arg.display()))
                    .collect();
                builder.cflags(jpeg_sys_cflags);
                builder.arg("skia_use_system_libjpeg_turbo", yes());
            } else {
                builder.arg(
                    "skia_use_system_libjpeg_turbo",
                    yes_if(use_system_libraries),
                );
            }

            if let Some(opt_level) = &build.opt_level {
                /* LTO generates corrupt libraries on the host platforms when building with --release
                if opt_level.parse::<usize>() != Ok(0) {
                    builder.skia_cflag("-flto");
                }
                */
                // When targeting windows `-O` isn't supported.
                if !target.is_windows() {
                    builder.cflag(format!("-O{opt_level}"));
                }
            }

            // Always compile expat
            builder.arg("skia_use_expat", yes());
            builder.arg("skia_use_system_expat", yes_if(use_system_libraries));

            // Add platform specific args
            platform::gn_args(build, builder)
        };

        FinalBuildConfiguration {
            skia_source_dir: skia_source_dir.into(),
            gn_args,
            use_system_libraries,
            target: build.target.clone(),
            sysroot,
        }
    }
}

/// Orchestrates the entire build of Skia based on the arguments provided.
pub fn build(
    build: &FinalBuildConfiguration,
    config: &binaries_config::BinariesConfiguration,
    ninja_command: Option<PathBuf>,
    gn_command: Option<PathBuf>,
    offline: bool,
) {
    let python = &prerequisites::locate_python3_cmd();
    println!("Python 3 found: {python:?}");
    let ninja = ninja_command.unwrap_or_else(ninja::exe_name);

    if !offline {
        println!("Synchronizing Skia dependencies");
        #[cfg(feature = "binary-cache")]
        crate::build_support::binary_cache::resolve_dependencies();
        assert!(
            Command::new(python)
                // Explicitly providing `GIT_SYNC_DEPS_PATH` fixes a problem with `git-sync-deps`
                // accidentally resolving an absolute directory for `GIT_SYNC_DEPS_PATH` when MingW
                // Python 3 runs on Windows under MSys.
                .env("GIT_SYNC_DEPS_PATH", "skia/DEPS")
                .env("GIT_SYNC_DEPS_SKIP_EMSDK", "1")
                .arg("skia/tools/git-sync-deps")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .unwrap()
                .success(),
            "`skia/tools/git-sync-deps` failed"
        );
    }

    configure_skia(build, config, python, gn_command.as_deref());
    build_skia(config, &ninja);
}

/// Configures Skia by calling gn
pub fn configure_skia(
    build: &FinalBuildConfiguration,
    config: &binaries_config::BinariesConfiguration,
    python: &Path,
    gn_command: Option<&Path>,
) {
    let mut gn_args = build
        .gn_args
        .iter()
        .map(|(name, value)| name.clone() + "=" + value)
        .collect::<Vec<String>>()
        .join(" ");

    if let Some(args) = cargo::env_var("SKIA_GN_ARGS") {
        gn_args.push_str(format!(" {}", args.as_str()).as_str());
    }

    let gn_command = gn_command
        .map(|p| p.to_owned())
        .unwrap_or_else(|| build.skia_source_dir.join("bin").join("gn"));

    println!("Skia args: {}", &gn_args);

    let output = Command::new(gn_command)
        .args([
            "gen",
            config.output_directory.to_str().unwrap(),
            &format!("--script-executable={}", python.to_str().unwrap()),
            &format!("--args={gn_args}"),
        ])
        .envs(env::vars())
        .current_dir(&build.skia_source_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("gn error");

    assert!(
        output.status.code() == Some(0),
        "{:?}",
        String::from_utf8(output.stdout).unwrap()
    );
}

/// Builds Skia.
///
/// This function assumes that all prerequisites are in place and that the output directory
/// contains a fully configured Skia source tree generated by gn.
pub fn build_skia(config: &binaries_config::BinariesConfiguration, ninja_command: &Path) {
    let ninja_status = Command::new(ninja_command)
        // Order of arguments do matter here: See <https://github.com/rust-skia/rust-skia/pull/643>
        // for details.
        .args(["-C", config.output_directory.to_str().unwrap()])
        .args(&config.ninja_built_libraries)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    assert!(
        ninja_status
            .expect("failed to run `ninja`, does it exist in PATH?")
            .success(),
        "`ninja` returned an error, please check the output for details."
    );
}

mod prerequisites {
    use std::path::PathBuf;
    use std::process::Command;

    /// Resolves the full path
    pub fn locate_python3_cmd() -> PathBuf {
        const PYTHON_CMDS: [&str; 2] = ["python", "python3"];
        for python in PYTHON_CMDS.as_ref() {
            println!("Probing '{python}'");
            if let Some(true) = is_python_version_3(python) {
                return python.into();
            }
        }

        panic!(">>>>> Probing for Python 3 failed, please make sure that it's available in PATH, probed executables are: {PYTHON_CMDS:?} <<<<<");
    }

    /// Returns `true` if the given python executable identifies itself as a python version 3
    /// executable. Returns `None` if the executable was not found.
    fn is_python_version_3(exe: impl AsRef<str>) -> Option<bool> {
        Command::new(exe.as_ref())
            .arg("--version")
            .output()
            .map(|output| {
                let mut str = String::from_utf8(output.stdout).unwrap();
                if str.is_empty() {
                    // Python2 seems to push the version to stderr.
                    str = String::from_utf8(output.stderr).unwrap()
                }
                // Don't parse version output, for example output
                // might be "Python 2.7.15+"
                str.starts_with("Python 3.")
            })
            .ok()
    }
}

mod ninja {
    use std::path::PathBuf;

    use super::cargo;

    pub fn exe_name() -> PathBuf {
        if cargo::host().is_windows() {
            "ninja.exe"
        } else {
            "ninja"
        }
        .into()
    }
}
