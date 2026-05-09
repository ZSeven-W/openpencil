use std::path::PathBuf;

use super::prelude::*;

pub struct Msvc;

impl PlatformDetails for Msvc {
    fn uses_freetype(&self) -> bool {
        false
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        if let Some(win_vc) = resolve_vc() {
            builder.arg(
                "win_vc",
                quote(
                    win_vc
                        .to_str()
                        .expect("Failed to convert Windows compiler path to UTF-8"),
                ),
            );
        }

        // Tell Skia's build system where LLVM is supposed to be located.
        if let Some(llvm_home) = llvm::find_home() {
            let clang_version_dir = llvm::clang_version_dir(&llvm_home)
                .unwrap_or_else(|| panic!("Unable to locate Clang's version directory"));

            builder.arg("clang_win", quote(&llvm_home));
            builder.arg("clang_win_version", quote(&clang_version_dir));
        } else {
            panic!("Unable to locate LLVM installation");
        }

        // Setting `target_cpu` to `i686` or `x86`, nightly builds would lead to
        // > 'C:/Program' is not recognized as an internal or external command
        //
        // Without it, the executables pop out just fine. See the GH job
        // `supplemental-builds/windows-x86`.
        {
            let arch = &config.target.architecture;
            if arch != "i686" {
                builder.arg("target_cpu", quote(clang::target_arch(arch)));
            }
        }

        // Disable `[[clang::trivial_abi]]` because it leads to ABI mismatches if the
        // bindings are compiled with a compiler other than clang (e.g. MSVC).
        // (see <https://groups.google.com/g/skia-discuss/c/3rpeWuPcD9Y/m/CySLakaTAAAJ>)
        builder.arg("is_trivial_abi", no());

        // Code on MSVC needs to be compiled differently (e.g. with /MT or /MD)
        // depending on the runtime being linked. (See
        // <https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes>)
        // When static feature is enabled (target-feature=+crt-static) the C runtime
        // should be statically linked and the compiler has to place the library name
        // LIBCMT.lib into the .obj See
        // <https://docs.microsoft.com/en-us/cpp/build/reference/md-mt-ld-use-run-time-library?view=vs-2019>
        let runtime_library = if cargo::target_crt_static() {
            "/MT"
        } else {
            // otherwise the C runtime should be linked dynamically
            "/MD"
        };

        builder.cflag(runtime_library);

        // <https://github.com/llvm/llvm-project/issues/95133>
        builder.cflag("-D__RTMINTRIN_H");
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        generic_link_libraries(features)
    }
}

pub struct Generic;

impl PlatformDetails for Generic {
    fn uses_freetype(&self) -> bool {
        false
    }

    fn gn_args(&self, _config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        builder.target_os_and_default_cpu("win");
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        generic_link_libraries(features)
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        if let Some(specific_target) = specific_target(target) {
            builder.override_target(specific_target);
        }
    }
}

fn generic_link_libraries(features: &Features) -> Vec<String> {
    // `Advapi32` is needed by `skunicode_icu.lib`.
    let mut libs = vec!["usp10", "ole32", "user32", "gdi32", "fontsub", "Advapi32"];
    if features[feature::GL] {
        libs.push("opengl32");
    }
    if features[feature::D3D] {
        libs.extend(["d3d12", "dxgi", "d3dcompiler"]);
    }

    libs.iter().map(|l| l.to_string()).collect()
}

/// Visual Studio VC detection support
/// TODO: sophisticate: <https://github.com/alexcrichton/cc-rs/blob/master/src/windows_registry.rs0>
fn resolve_vc() -> Option<PathBuf> {
    if let Some(install_dir) = cargo::env_var("VCINSTALLDIR") {
        // vcvars.bat may end up setting VCINSTALLDIR to a path with trailing backslash, we
        // invoke GN  as win_vc="the path", and we end up with "foo\", which erroneously
        // escapes the quote instead of closing.
        return Some(PathBuf::from(install_dir.trim_end_matches('\\')));
    }

    let releases = [("Program Files", "2022"), ("Program Files (x86)", "2019")];
    let editions = ["BuildTools", "Enterprise", "Professional", "Community"];

    releases
        .iter()
        .flat_map(|r| editions.iter().map(move |e| (r, e)))
        .map(|((rp, r), ed)| format!("C:\\{rp}\\Microsoft Visual Studio\\{r}\\{ed}\\VC"))
        .map(PathBuf::from)
        .find(|pb| pb.exists())
}

mod llvm {
    use crate::build_support::cargo;
    use std::{fs, path::PathBuf};

    /// Locate the LLVM installation which can be used to build Skia.
    /// If the environment variable `LLVM_HOME` is present it will
    /// be used. Otherwise we search a set of common paths:
    ///   - C:\Program Files\LLVM
    ///   - C:\LLVM
    ///   - %USERPROFILE%\scoop\apps\llvm\current
    pub fn find_home() -> Option<String> {
        if let Some(llvm_home) = cargo::env_var("LLVM_HOME") {
            validate_home(&llvm_home)
        } else {
            // USERPROFILE *should* equate to HOME and always be defined.
            // TODO: If this isn't defined, should we try c:\Users\%USERNAME%?
            let userprofile =
                cargo::env_var("USERPROFILE").expect("Unable to resolve %USERPROFILE%");
            let common_roots = [
                "C:\\Program Files\\LLVM".into(),
                "C:\\LLVM".into(),
                format!("{userprofile}\\scoop\\apps\\llvm\\current"),
            ];
            for root in &common_roots {
                let root = validate_home(root);
                if root.is_some() {
                    return root;
                }
            }
            None
        }
    }

    /// Return the clang's highest version directory by scanning the directories in
    /// `LLVM_HOME\lib\clang\*`.
    pub fn clang_version_dir(home: &str) -> Option<String> {
        let path: PathBuf = [home, "lib", "clang"].into_iter().collect();
        let mut highest_version = None;
        let mut highest_version_path = None;
        for entry in fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(v) = parse_version(path.file_name()?.to_str()?) {
                    if Some(v) > highest_version {
                        highest_version = Some(v);
                        highest_version_path = Some(path);
                    }
                }
            }
        }
        let path = highest_version_path?;
        Some(path.to_str()?.to_string())
    }

    fn validate_home(home: &str) -> Option<String> {
        let clang_cl: PathBuf = [home, "bin", "clang-cl.exe"].into_iter().collect();
        eprintln!("Checking for {clang_cl:?}");
        clang_cl.exists().then(|| home.to_string())
    }

    fn parse_version(s: &str) -> Option<(usize, usize, usize)> {
        let v: Result<Vec<_>, _> = s.split('.').map(|s| s.parse()).collect();
        let v = v.ok()?;
        match v.len() {
            1 => Some((v[0], 0, 0)),
            2 => Some((v[0], v[1], 0)),
            3 => Some((v[0], v[1], v[2])),
            _ => None,
        }
    }
}

fn specific_target(target: &Target) -> Option<&'static str> {
    if target.vendor == "win7" {
        if target.architecture == "x86_64" {
            return Some("x86_64-pc-windows-msvc");
        }
        if target.architecture == "i686" {
            return Some("i686-pc-windows-msvc");
        }
    }

    None
}
