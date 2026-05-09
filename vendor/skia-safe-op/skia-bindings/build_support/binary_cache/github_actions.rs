#![allow(dead_code)]
use crate::build_support::cargo;
use std::path::PathBuf;

/// Are we running on github?
pub fn is_active() -> bool {
    artifact_staging_directory().is_some()
}

/// Returns the artifact staging directory.
pub fn artifact_staging_directory() -> Option<PathBuf> {
    cargo::env_var("BUILD_ARTIFACTSTAGINGDIRECTORY").map(PathBuf::from)
}
