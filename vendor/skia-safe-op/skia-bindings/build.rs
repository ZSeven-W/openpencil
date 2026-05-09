use std::{fs, io};

use build_support::{
    binaries_config,
    cargo::{self, Target},
    features, platform, skia, skia_bindgen,
};

mod build_support;

fn main() -> Result<(), io::Error> {
    if env::is_docs_rs_build() {
        println!("DETECTED DOCS_RS BUILD");
        return fake_bindings();
    }

    let skia_debug = env::is_skia_debug();
    let cargo_target = cargo::target();

    let features = {
        let mut features = features::Features::from_cargo_env();
        let missing_dependencies = features.missing_dependencies();
        if !missing_dependencies.is_empty() {
            return Err(io::Error::other(format!(
                "Missing dependent features: {missing_dependencies}"
            )));
        }

        let redundant_features = platform::redundant_features(&features, &cargo_target);
        if !redundant_features.is_empty() {
            #[cfg(feature = "binary-cache")]
            if build_support::binary_cache::should_export().is_some() {
                return Err(io::Error::other(format!(
                    "Can't produce binaries with redundant features: {redundant_features}"
                )));
            }
            cargo::warning(format!(
                "Redundant features: {redundant_features}. Disabled for the build and binary download."
            ));
            features -= redundant_features;
            cargo::warning(format!("Final features: {features}"));
        }

        features
    };

    let binaries_config =
        binaries_config::BinariesConfiguration::from_features(&features, skia_debug);

    //
    // skip attempting to download?
    //
    if let Some(source_dir) = env::source_dir() {
        if let Some(search_path) = env::skia_lib_search_path() {
            println!("STARTING BIND AGAINST SYSTEM SKIA");

            binaries_config.import(&search_path, false).unwrap();

            let definitions = skia_bindgen::definitions::from_env();
            generate_bindings(
                &features,
                definitions,
                &binaries_config,
                &source_dir,
                cargo_target,
                None,
            );
        } else {
            if cfg!(feature = "no-compile") {
                panic!("Refusing to offline-build skia with no-compile feature");
            }

            println!("STARTING OFFLINE BUILD");

            let final_build_configuration = build_from_source(
                features.clone(),
                &binaries_config,
                &source_dir,
                skia_debug,
                true,
            );
            let definitions = skia_bindgen::definitions::from_ninja_features(
                &features,
                final_build_configuration.use_system_libraries,
                &binaries_config.output_directory,
            );
            generate_bindings(
                &features,
                definitions,
                &binaries_config,
                &source_dir,
                final_build_configuration.target,
                final_build_configuration
                    .sysroot
                    .as_ref()
                    .map(AsRef::as_ref),
            );
        }
    } else {
        //
        // is the download of prebuilt binaries possible?
        //

        #[allow(unused_variables)]
        let build_skia = true;

        #[cfg(feature = "binary-cache")]
        let build_skia = build_support::binary_cache::try_prepare_download(&binaries_config);

        //
        // full build?
        //

        if build_skia {
            if cfg!(feature = "no-compile") {
                panic!("Refusing to full-build skia with no-compile feature");
            }

            println!("STARTING A FULL BUILD");
            println!("HOST: {}", cargo::host());

            let source_dir = std::env::current_dir().unwrap().join("skia");
            let final_build_configuration = build_from_source(
                features.clone(),
                &binaries_config,
                &source_dir,
                skia_debug,
                false,
            );
            let definitions = skia_bindgen::definitions::from_ninja_features(
                &features,
                final_build_configuration.use_system_libraries,
                &binaries_config.output_directory,
            );
            generate_bindings(
                &features,
                definitions,
                &binaries_config,
                &source_dir,
                final_build_configuration.target,
                final_build_configuration
                    .sysroot
                    .as_ref()
                    .map(AsRef::as_ref),
            );
        }
    };

    binaries_config.commit_to_cargo();

    #[cfg(feature = "binary-cache")]
    if let Some(staging_directory) = build_support::binary_cache::should_export() {
        build_support::binary_cache::publish(&binaries_config, &staging_directory);
    }

    Ok(())
}

fn build_from_source(
    features: features::Features,
    binaries_config: &binaries_config::BinariesConfiguration,
    skia_source_dir: &std::path::Path,
    skia_debug: bool,
    offline: bool,
) -> skia::FinalBuildConfiguration {
    let build_config = skia::BuildConfiguration::from_features(features, skia_debug);
    let final_configuration = skia::FinalBuildConfiguration::from_build_configuration(
        &build_config,
        skia::env::use_system_libraries(),
        skia_source_dir,
    );

    skia::build(
        &final_configuration,
        binaries_config,
        skia::env::ninja_command(),
        skia::env::gn_command(),
        offline,
    );

    final_configuration
}

fn generate_bindings(
    features: &features::Features,
    definitions: Vec<skia_bindgen::Definition>,
    binaries_config: &binaries_config::BinariesConfiguration,
    skia_source_dir: &std::path::Path,
    target: Target,
    sysroot: Option<&str>,
) {
    // Emit the ninja definitions, to help debug build consistency.
    skia_bindgen::definitions::save_definitions(&definitions, &binaries_config.output_directory)
        .expect("failed to write Skia defines");

    let bindings_config = skia_bindgen::Configuration::new(features, definitions, skia_source_dir);
    skia_bindgen::generate_bindings(
        &bindings_config,
        &binaries_config.output_directory,
        target,
        sysroot,
    );
}

/// On docs.rs, rustdoc runs inside a container with no networking, so copy a pre-generated
/// `bindings.rs` file.
fn fake_bindings() -> Result<(), io::Error> {
    println!("COPYING bindings_docs.rs to OUT_DIR/skia/bindings.rs");
    let bindings_target = cargo::output_directory()
        .join(binaries_config::SKIA_OUTPUT_DIR)
        .join("bindings.rs");
    fs::copy("bindings_docs.rs", bindings_target).map(|_| ())
}

/// Environment variables used by this build script.
mod env {
    use crate::build_support::cargo;
    use std::path::PathBuf;

    /// The path to the Skia source directory.
    pub fn source_dir() -> Option<PathBuf> {
        cargo::env_var("SKIA_SOURCE_DIR").map(PathBuf::from)
    }

    /// The path to where a pre-built Skia library can be found.
    pub fn skia_lib_search_path() -> Option<PathBuf> {
        cargo::env_var("SKIA_LIBRARY_SEARCH_PATH").map(PathBuf::from)
    }

    pub fn is_skia_debug() -> bool {
        matches!(cargo::env_var("SKIA_DEBUG"), Some(v) if v != "0")
    }

    pub fn is_docs_rs_build() -> bool {
        matches!(cargo::env_var("DOCS_RS"), Some(v) if v != "0")
    }
}
