//! Support function for communicating with cargo's variables and outputs.

#![allow(dead_code)]

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::{env, fmt, fs, io};

pub fn warning(warning: impl AsRef<str>) {
    println!("cargo:warning={}", warning.as_ref());
}

pub fn output_directory() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

pub fn rerun_if_file_changed(path: impl AsRef<Path>) {
    println!("cargo:rerun-if-changed={}", path.as_ref().to_str().unwrap());
}

/// Returns the value of an environment variable and notify cargo that the build
/// should re-run if it changes.
pub fn env_var(name: impl AsRef<str>) -> Option<String> {
    let name = name.as_ref();
    rerun_if_env_var_changed(name);
    env::var(name).ok()
}

/// Notify cargo that it should rerun the build if the environment
/// variable changes.
pub fn rerun_if_env_var_changed(name: impl AsRef<str>) {
    println!("cargo:rerun-if-env-changed={}", name.as_ref())
}

pub fn add_link_libs<T: AsRef<str>>(libs: impl IntoIterator<Item = T>) {
    libs.into_iter().for_each(|s| add_link_lib(s.as_ref()))
}

pub fn add_link_lib(lib: impl AsRef<str>) {
    println!("cargo:rustc-link-lib={}", lib.as_ref());
}

pub fn add_static_link_libs<T: AsRef<str>>(target: &Target, libs: impl IntoIterator<Item = T>) {
    libs.into_iter()
        .for_each(|s| add_static_link_lib(target, s.as_ref()))
}

pub fn add_static_link_lib(target: &Target, lib: impl AsRef<str>) {
    // Prefixing the libraries we built with `static=` causes linker errors on Windows.
    // https://github.com/rust-skia/rust-skia/pull/354
    if target.is_windows() {
        println!("cargo:rustc-link-lib={}", lib.as_ref());
    } else {
        println!("cargo:rustc-link-lib=static={}", lib.as_ref());
    }
}

pub fn add_link_search(dir: impl AsRef<str>) {
    println!("cargo:rustc-link-search={}", dir.as_ref());
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub architecture: String,
    pub vendor: String,
    pub system: String,
    pub abi: Option<String>,
}

impl Target {
    pub fn is_windows(&self) -> bool {
        self.system == "windows"
    }

    pub fn is_emscripten(&self) -> bool {
        self.system == "emscripten"
    }

    pub fn builds_with_msvc(&self) -> bool {
        self.abi.as_deref() == Some("msvc")
    }

    /// Convert a library name to a filename.
    pub fn library_to_filename(&self, name: impl AsRef<str>) -> PathBuf {
        let name = name.as_ref();
        if self.is_windows() {
            format!("{name}.lib").into()
        } else {
            format!("lib{name}.a").into()
        }
    }

    pub fn as_strs(&self) -> (&str, &str, &str, Option<&str>) {
        (
            self.architecture.as_str(),
            self.vendor.as_str(),
            self.system.as_str(),
            self.abi.as_deref(),
        )
    }

    pub fn arch_abi(&self) -> (&str, Option<&str>) {
        let (arch, _vendor, _system, abi) = self.as_strs();
        (arch, abi)
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}",
            &self.architecture, &self.vendor, &self.system
        )?;

        if let Some(ref abi) = self.abi {
            write!(f, "-{abi}")
        } else {
            Result::Ok(())
        }
    }
}

pub fn target() -> Target {
    let target_str = env::var("TARGET").unwrap();
    parse_target(target_str)
}

pub fn target_crt_static() -> bool {
    env::var("CARGO_CFG_TARGET_FEATURE")
        .map(|features| features.contains("crt-static"))
        .unwrap_or(false)
}

pub fn host() -> Target {
    let host_str = env::var("HOST").unwrap();
    parse_target(host_str)
}

pub fn parse_target(target_str: impl AsRef<str>) -> Target {
    let target_str = target_str.as_ref();
    let target: Vec<String> = target_str.split('-').map(|s| s.into()).collect();

    if target.len() >= 3 {
        let abi = if target.len() > 3 {
            Some(target[3].clone())
        } else {
            None
        };

        Target {
            architecture: target[0].clone(),
            vendor: target[1].clone(),
            system: target[2].clone(),
            abi,
        }
    } else if target.len() == 2 {
        Target {
            architecture: target[0].clone(),
            vendor: String::new(),
            system: target[1].clone(),
            abi: None,
        }
    } else {
        panic!("Failed to parse TARGET {target_str}");
    }
}

/// Returns `true` if the target should be built in release mode, `false`, if in debug mode.
///
/// We can not assume that the build profile of the build.rs script reflects the build
/// profile that the target needs.
pub fn build_release() -> bool {
    match env::var("PROFILE").unwrap().as_str() {
        "release" => true,
        "debug" => false,
        profile => panic!("PROFILE '{profile}' is not supported by this build script"),
    }
}

/// Are we inside a crate?
pub fn is_crate() -> bool {
    crate_repository_hash().is_ok()
}

// If we are building from within a crate, return the full commit hash
// of the repository the crate was packaged from.
pub fn crate_repository_hash() -> io::Result<String> {
    let vcs_info = fs::read_to_string(".cargo_vcs_info.json")?;
    let value: serde_json::Value = serde_json::from_str(&vcs_info)?;
    let git = value.get("git").expect("failed to get 'git' property");
    let sha1 = git.get("sha1").expect("failed to get 'sha1' property");
    Ok(sha1.as_str().unwrap().into())
}

pub fn package_version() -> String {
    env::var("CARGO_PKG_VERSION").unwrap().as_str().into()
}

/// Parses Cargo.toml and returns the metadata specified in the [package.metadata] section.
pub fn get_metadata() -> Vec<(String, String)> {
    use toml::{de, value};

    let cargo_toml = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("missing environment variable CARGO_MANIFEST_DIR"),
    )
    .join("Cargo.toml");
    let str = fs::read_to_string(cargo_toml).expect("Failed to read Cargo.toml");
    let root: value::Table =
        de::from_str::<value::Table>(&str).expect("Failed to parse Cargo.toml");
    let manifest_table: &value::Table = root
        .get("package")
        .expect("section [package] missing")
        .get("metadata")
        .expect("section [package.metadata] missing")
        .as_table()
        .unwrap();

    manifest_table
        .iter()
        .map(|(a, b)| (a.clone(), b.as_str().unwrap().to_owned()))
        .collect()
}

#[test]
fn parse_target_tests() {
    assert_eq!(
        parse_target("aarch64-unknown-linux-gnu"),
        Target {
            architecture: "aarch64".into(),
            vendor: "unknown".into(),
            system: "linux".into(),
            abi: Some("gnu".into())
        }
    );
    assert_eq!(
        parse_target("aarch64-unknown-linux"),
        Target {
            architecture: "aarch64".into(),
            vendor: "unknown".into(),
            system: "linux".into(),
            abi: None
        }
    );
    assert_eq!(
        parse_target("aarch64-linux"),
        Target {
            architecture: "aarch64".into(),
            vendor: String::new(),
            system: "linux".into(),
            abi: None
        }
    );
    assert!(std::panic::catch_unwind(|| parse_target("garbage")).is_err());
}
