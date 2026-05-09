use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::platform;
use crate::build_support::{
    cargo,
    features::{self, feature, Features},
};

pub const SKIA_OUTPUT_DIR: &str = "skia";
const ICUDTL_DAT: &str = "icudtl.dat";

/// The libraries to link with.
pub mod lib {
    pub const SKIA: &str = "skia";
    pub const SKIA_BINDINGS: &str = "skia-bindings";
    pub const SK_SHAPER: &str = "skshaper";
    pub const SK_PARAGRAPH: &str = "skparagraph";
    pub const SVG: &str = "svg";
    pub const SKOTTIE: &str = "skottie";
    pub const SKSG: &str = "sksg";
    pub const JSON_READER: &str = "jsonreader";
    pub const SK_RESOURCES: &str = "skresources";
    pub const SK_UNICODE_CORE: &str = "skunicode_core";
    pub const SK_UNICODE_ICU: &str = "skunicode_icu";
}

/// The configuration of the resulting binaries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BinariesConfiguration {
    /// The features we built with.
    pub features: Features,

    /// The output directory of the libraries we build and we need to inform cargo about.
    pub output_directory: PathBuf,

    /// The TARGET specific link libraries we need to inform cargo about.
    pub link_libraries: Vec<String>,

    /// The static Skia libraries built by ninja that dependent projects need to link with.
    pub ninja_built_libraries: Vec<String>,

    /// Static libraries that are generated in the binding process dependent projects need to link
    /// with.
    pub binding_libraries: Vec<String>,

    /// Files that are generated in the binding process (this is `bindings.rs`).
    pub binding_files: Vec<PathBuf>,

    /// Additional files relative to the output_directory
    /// that are needed to build dependent projects.
    pub additional_files: Vec<PathBuf>,

    /// `true` if the skia libraries are built with debugging information.
    pub skia_debug: bool,
}

impl BinariesConfiguration {
    /// Build a binaries configuration from a set of Skia features.
    pub fn from_features(features: &features::Features, skia_debug: bool) -> Self {
        let target = cargo::target();

        let mut ninja_built_libraries = Vec::new();
        let mut binding_libraries = Vec::new();
        let binding_files = vec!["bindings.rs".into()];
        let mut additional_files = Vec::new();

        if features[feature::TEXTLAYOUT] {
            if target.is_windows() {
                additional_files.push(ICUDTL_DAT.into());
            }
            ninja_built_libraries.push(lib::SK_PARAGRAPH.into());
            ninja_built_libraries.push(lib::SK_SHAPER.into());
            ninja_built_libraries.push(lib::SK_UNICODE_CORE.into());
            ninja_built_libraries.push(lib::SK_UNICODE_ICU.into());
        }
        if features[feature::SVG] {
            ninja_built_libraries.push(lib::SVG.into());
            ninja_built_libraries.push(lib::SK_RESOURCES.into());
        }
        if features[feature::SKOTTIE] {
            ninja_built_libraries.push(lib::SKOTTIE.into());
            ninja_built_libraries.push(lib::SKSG.into());
            ninja_built_libraries.push(lib::JSON_READER.into());
            // skottie depends on skresources, add it if not already added by SVG
            if !features[feature::SVG] {
                ninja_built_libraries.push(lib::SK_RESOURCES.into());
            }
        }

        let link_libraries = platform::link_libraries(features, &target);

        let output_directory = cargo::output_directory()
            .join(SKIA_OUTPUT_DIR)
            .to_str()
            .unwrap()
            .into();

        ninja_built_libraries.push(lib::SKIA.into());
        binding_libraries.push(lib::SKIA_BINDINGS.into());

        BinariesConfiguration {
            features: features.clone(),
            output_directory,
            link_libraries,
            ninja_built_libraries,
            binding_libraries,
            binding_files,
            additional_files,
            skia_debug,
        }
    }

    pub fn built_libraries(&self, include_bindings: bool) -> impl Iterator<Item = &str> {
        self.ninja_built_libraries
            .iter()
            .chain(if include_bindings {
                self.binding_libraries.iter()
            } else {
                [].iter()
            })
            .map(|x| x.as_str())
    }

    /// Inform cargo that the library files of the given configuration are available and
    /// can be used as dependencies.
    pub fn commit_to_cargo(&self) {
        cargo::add_link_search(self.output_directory.to_str().unwrap());

        let target = cargo::target();

        if target.is_emscripten() {
            // Since Skia milestone 148, the wasm GN toolchain emits static archives as
            // `*.wasm.a`.
            for lib in &self.ninja_built_libraries {
                let from = self.output_directory.join(format!("lib{lib}.wasm.a"));
                let to = self.output_directory.join(format!("lib{lib}.a"));
                fs::copy(&from, &to).unwrap_or_else(|e| {
                    panic!(
                        "failed to prepare emscripten archive for linking: from '{}' to '{}': {}",
                        from.display(),
                        to.display(),
                        e
                    )
                });
            }
        }

        // On Linux, the order is significant, first the static libraries we built, and then
        // the system libraries.
        cargo::add_static_link_libs(&target, self.built_libraries(true));
        cargo::add_link_libs(&self.link_libraries);
    }

    /// Import library and additional files from `from_dir` to the output directory.
    pub fn import(&self, from_dir: &Path, import_bindings: bool) -> io::Result<()> {
        let output_directory = &self.output_directory;
        self.copy_libs_and_additional_files(from_dir, output_directory, import_bindings)
    }

    /// Export library and additional files from the output directory to a `to_dir`.
    #[allow(unused)]
    pub fn export(&self, to_dir: &Path) -> io::Result<()> {
        let output_directory = &self.output_directory;
        self.copy_libs_and_additional_files(output_directory, to_dir, true)
    }

    fn copy_libs_and_additional_files(
        &self,
        from_dir: &Path,
        to_dir: &Path,
        copy_bindings: bool,
    ) -> io::Result<()> {
        fs::create_dir_all(to_dir)?;

        let target = cargo::target();

        for lib in self.built_libraries(copy_bindings) {
            let filename = &target.library_to_filename(lib);
            copy(filename, from_dir, to_dir)?;
        }

        for filename in &self.additional_files {
            copy(filename, from_dir, to_dir)?;
        }

        if copy_bindings {
            for filename in &self.binding_files {
                copy(filename, from_dir, to_dir)?;
            }
        }

        return Ok(());

        fn copy(file_name: &Path, from_dir: &Path, to_dir: &Path) -> io::Result<()> {
            let from_path = from_dir.join(file_name);
            let to_path = to_dir.join(file_name);
            fs::copy(&from_path, &to_path).map(|_| ()).map_err(|e| {
                eprintln!(
                    "COPY OPERATION FAILED: from '{}' to '{}': {}",
                    from_path.display(),
                    to_path.display(),
                    e
                );
                e
            })
        }
    }
}
