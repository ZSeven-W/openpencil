use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use super::prelude::*;

pub struct Ios;

// m119: The use of value() in `effects/SkImageFilters.h` requires iOS12
const MIN_IOS_VERSION: &str = "12";
const MIN_IOS_VERSION_M1: &str = "14";
// m100: XCode 13.2 fails to build with version 13
const MIN_IOS_VERSION_CATALYST: &str = "14";

impl PlatformDetails for Ios {
    fn uses_freetype(&self) -> bool {
        false
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        let (arch, abi) = config.target.arch_abi();

        // Set minimum target for consistency (this is actually not required, because it is set it
        // in the extra_cflags, too).
        builder.arg("ios_min_target", quote(&format!("{MIN_IOS_VERSION}.0")));

        builder.target_os_and_default_cpu("ios");

        // m100: Needed for aarch64 simulators, requires cherry Skia pick
        // 0361abf39d1504966799b1cdb5450e07f88b2bc2 (until milestone 102).
        if is_simulator(arch, abi) {
            builder.arg("ios_use_simulator", yes());
        }

        builder.cflags(extra_skia_cflags(arch, abi));

        if let Some(specific_target) = specific_target(arch, abi) {
            builder.target(specific_target);
        }
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        builder.args(additional_clang_args(
            &target.architecture,
            target.abi.as_deref(),
        ));

        // TODO: duplicated from gn_args, target overrides should probably a separated from Gn and
        // bindgen args.
        let (arch, abi) = target.arch_abi();
        if let Some(specific_target) = specific_target(arch, abi) {
            builder.override_target(&specific_target);
        }
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        let abi = cargo::target().abi;
        let mut libs = vec![
            "c++",
            "framework=CoreFoundation",
            "framework=CoreGraphics",
            "framework=CoreText",
            "framework=ImageIO",
        ];

        if abi.as_deref() != Some("macabi") {
            libs.extend(vec!["framework=MobileCoreServices", "framework=UIKit"]);
        }

        if features[feature::METAL] {
            libs.push("framework=Metal");
        }

        libs.iter().map(|s| s.to_string()).collect()
    }
}

fn is_simulator(arch: &str, abi: Option<&str>) -> bool {
    IosPlatform::new(arch, abi) == IosPlatform::Simulator
}

fn specific_target(arch: &str, abi: Option<&str>) -> Option<String> {
    (IosPlatform::new(arch, abi) == IosPlatform::M1Simulator)
        .then(|| format!("arm64-apple-ios{MIN_IOS_VERSION_M1}.0-simulator"))
}

// TODO: add support for 32 bit devices and simulators.
fn extra_skia_cflags(arch: &str, abi: Option<&str>) -> Vec<String> {
    IosPlatform::new(arch, abi).flags()
}

fn additional_clang_args(arch: &str, abi: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    let platform = IosPlatform::new(arch, abi);

    args.extend(platform.flags());

    use IosPlatform::*;
    match platform {
        Simulator | M1Simulator => {
            args.push("-m64".into());
        }
        Device | Catalyst => {
            args.push("-arch".into());
            args.push(clang::target_arch(arch).into());
        }
    }

    args.push("-isysroot".into());
    args.push(platform.sdk_path().to_str().unwrap().into());
    args.push("-fembed-bitcode".into());

    args
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum IosPlatform {
    Simulator,
    M1Simulator,
    Device,
    Catalyst,
}

impl IosPlatform {
    fn new(arch: &str, abi: Option<&str>) -> Self {
        use IosPlatform::*;
        match () {
            () if abi == Some("macabi") => Catalyst,
            () if arch == "x86_64" => Simulator,
            () if arch == "aarch64" && abi == Some("sim") => M1Simulator,
            () => Device,
        }
    }

    fn flags(self) -> Vec<String> {
        let ios_version = self.min_ios_version();

        let platform_variant = if self.is_simulator() {
            "ios-simulator"
        } else {
            "iphoneos"
        };

        // m119: We have to set -m version-min in cflags, otherwise effects/SkImageFilters.h does
        // not compile: `error: 'value' is unavailable: introduced in iOS 12.0`
        let min_version = format!("-m{platform_variant}-version-min={ios_version}.0");
        // Even though version-min is defined, This must be defined, too. Otherwise MAX_ALLOWED gets
        // ignored and set to the highest version, which in turn sets the wrong
        // GR_METAL_SDK_VERSION.
        let min_required = format!("-D__IPHONE_OS_VERSION_MIN_REQUIRED={ios_version}0000");
        let max_version = format!("-D__IPHONE_OS_VERSION_MAX_ALLOWED={ios_version}0000");

        vec![min_version, min_required, max_version]
    }

    fn is_simulator(self) -> bool {
        match self {
            IosPlatform::Device => false,
            IosPlatform::Simulator => true,
            IosPlatform::M1Simulator => true,
            IosPlatform::Catalyst => false,
        }
    }

    fn min_ios_version(self) -> &'static str {
        match self {
            IosPlatform::Device => MIN_IOS_VERSION,
            IosPlatform::Simulator => MIN_IOS_VERSION,
            IosPlatform::M1Simulator => MIN_IOS_VERSION_M1,
            IosPlatform::Catalyst => MIN_IOS_VERSION_CATALYST,
        }
    }

    /// Resolve the iOS SDK path by starting `xcrun`.
    fn sdk_path(self) -> PathBuf {
        let sdk_path = Command::new("xcrun")
            .arg("--show-sdk-path")
            .arg("--sdk")
            .arg(self.sdk_name())
            .stderr(Stdio::inherit())
            .output()
            .expect("Failed to invoke xcrun")
            .stdout;

        let string = String::from_utf8(sdk_path).expect("failed to resolve iOS SDK path");
        PathBuf::from(string.trim())
    }

    fn sdk_name(self) -> &'static str {
        use IosPlatform::*;
        match self {
            Simulator | M1Simulator => "iphonesimulator",
            Device => "iphoneos",
            Catalyst => "macosx",
        }
    }
}
