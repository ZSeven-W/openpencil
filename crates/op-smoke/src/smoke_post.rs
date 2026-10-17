//! Optional durable generation checkpoint and bounded image-stage completion.
//!
//! OPENPENCIL_SMOKE_CHECKPOINT_OUT creates a new, separate pre-image `.op`.
//! OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS bounds the real image stage. A deadline
//! with unfilled slots exits 5 after saving final output; ordinary no-hit
//! searches retain their existing exit semantics and appear in the receipt.

use std::io::Write;
use std::path::{Path, PathBuf};

use op_editor_core::EditorState;

use crate::image_fill::{self, ImageFillConfig};
use crate::smoke_post_error::PostGenerationError;

pub struct PostGeneration {
    images: ImageFillConfig,
    output: Option<PathBuf>,
    checkpoint: Option<PathBuf>,
}

impl PostGeneration {
    pub fn from_env() -> Result<Self, PostGenerationError> {
        let images = ImageFillConfig::from_env()?;
        let output = env_path("OPENPENCIL_SMOKE_OUT");
        let checkpoint = env_path("OPENPENCIL_SMOKE_CHECKPOINT_OUT");
        if let Some(path) = &checkpoint {
            validate_checkpoint(path, output.as_deref())?;
        }
        Ok(Self {
            images,
            output,
            checkpoint,
        })
    }

    /// Called on a plain thread because the shared image fetch bridge blocks.
    /// The final output is written even when the image deadline is exhausted.
    pub fn run(&self, state: &mut EditorState, dump: bool) -> u8 {
        if let Some(path) = &self.checkpoint {
            if let Err(error) = save_checkpoint(state, path, self.output.as_deref()) {
                eprintln!("[SMOKE] pre-image checkpoint failed: {error}");
                return 4;
            }
            eprintln!(
                "[SMOKE] pre-image checkpoint saved (images pending) → {}",
                path.display()
            );
        }
        let report = image_fill::fill_images(state, &self.images, dump);
        if let Some(path) = &self.output {
            if let Err(error) = save_final(state, path) {
                eprintln!("[SMOKE] save failed ({}): {error}", path.display());
                return 4;
            }
            eprintln!("[SMOKE] saved doc → {}", path.display());
        }
        if report.deadline_incomplete() {
            eprintln!("[SMOKE] image deadline exhausted with {} unfilled slot(s); image stage incomplete, exit 5", report.unfilled);
            5
        } else {
            0
        }
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn path_identity(path: &Path) -> Result<PathBuf, PostGenerationError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(PostGenerationError::CurrentDirectory)?
            .join(path)
    };
    if let Ok(existing) = absolute.canonicalize() {
        return Ok(existing);
    }
    // A dangling final symlink could begin pointing at the newly-created
    // checkpoint and overwrite it when the final output is written.
    if std::fs::symlink_metadata(&absolute).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(PostGenerationError::UnresolvedSymlink(absolute));
    }
    let parent = absolute
        .parent()
        .ok_or_else(|| PostGenerationError::MissingParent(absolute.clone()))?;
    let name = absolute
        .file_name()
        .ok_or_else(|| PostGenerationError::MissingFileName(absolute.clone()))?;
    Ok(parent
        .canonicalize()
        .map_err(|source| PostGenerationError::PathIo {
            path: parent.to_path_buf(),
            source,
        })?
        .join(name))
}

fn validate_checkpoint(
    checkpoint: &Path,
    output: Option<&Path>,
) -> Result<(), PostGenerationError> {
    let checkpoint_identity = path_identity(checkpoint)?;
    if output
        .map(path_identity)
        .transpose()?
        .is_some_and(|path| path == checkpoint_identity)
    {
        return Err(PostGenerationError::CheckpointAliasesOutput);
    }
    // create_new below closes the race and rejects symlinks/hardlinks as well.
    // Thus no existing input, checkpoint or final artifact can be overwritten.
    if std::fs::symlink_metadata(checkpoint).is_ok() {
        return Err(PostGenerationError::CheckpointExists(
            checkpoint.to_path_buf(),
        ));
    }
    Ok(())
}

fn save_checkpoint(
    state: &EditorState,
    path: &Path,
    output: Option<&Path>,
) -> Result<(), PostGenerationError> {
    validate_checkpoint(path, output)?;
    let bytes = crate::smoke_save::bytes(state)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| PostGenerationError::PathIo {
            path: path.to_path_buf(),
            source,
        })?;
    let result = file.write_all(&bytes).and_then(|()| file.sync_all());
    if let Err(error) = result {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(PostGenerationError::PathIo {
            path: path.to_path_buf(),
            source: error,
        });
    }
    Ok(())
}

fn save_final(state: &EditorState, path: &Path) -> Result<(), PostGenerationError> {
    let bytes = crate::smoke_save::bytes(state)?;
    std::fs::write(path, bytes).map_err(|source| PostGenerationError::PathIo {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[path = "smoke_post_tests.rs"]
mod tests;
