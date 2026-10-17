//! Typed configuration and persistence errors for post-generation stages.

use std::path::PathBuf;

#[derive(Debug)]
pub enum PostGenerationError {
    InvalidImageBudget,
    CurrentDirectory(std::io::Error),
    MissingParent(PathBuf),
    MissingFileName(PathBuf),
    UnresolvedSymlink(PathBuf),
    CheckpointAliasesOutput,
    CheckpointExists(PathBuf),
    PathIo {
        path: PathBuf,
        source: std::io::Error,
    },
    Serialize(op_host_services::doc_io::DocIoError),
}

impl std::fmt::Display for PostGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidImageBudget => write!(f, "OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS must be a positive, representable integer number of seconds"),
            Self::CurrentDirectory(error) => write!(f, "current directory: {error}"),
            Self::MissingParent(path) => write!(f, "output has no parent directory: {}", path.display()),
            Self::MissingFileName(path) => write!(f, "output has no file name: {}", path.display()),
            Self::UnresolvedSymlink(path) => write!(f, "cannot resolve output symlink: {}", path.display()),
            Self::CheckpointAliasesOutput => write!(f, "OPENPENCIL_SMOKE_CHECKPOINT_OUT must differ from OPENPENCIL_SMOKE_OUT"),
            Self::CheckpointExists(path) => write!(f, "checkpoint already exists: {}", path.display()),
            Self::PathIo { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Serialize(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PostGenerationError {}

impl From<op_host_services::doc_io::DocIoError> for PostGenerationError {
    fn from(error: op_host_services::doc_io::DocIoError) -> Self {
        Self::Serialize(error)
    }
}
