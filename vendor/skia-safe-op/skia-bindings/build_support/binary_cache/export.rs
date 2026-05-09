use crate::build_support::{
    binaries_config::BinariesConfiguration,
    binary_cache::{binaries, SKIA_LICENSE},
};
use std::path::Path;

/// Publish the binaries to Azure.
pub fn publish(binaries_config: &BinariesConfiguration, staging_directory: &Path) {
    println!(
        "DETECTED AZURE, exporting binaries to {}",
        staging_directory.to_str().unwrap()
    );

    println!("EXPORTING BINARIES");
    let source_files = &[(SKIA_LICENSE, "LICENSE_SKIA")];
    binaries::export(binaries_config, source_files, staging_directory)
        .expect("EXPORTING BINARIES FAILED")
}
