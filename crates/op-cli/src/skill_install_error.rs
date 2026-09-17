//! Typed failures for `op install` / `op uninstall` (see `skill_install_cli`).
//!
//! Same hand-rolled style as `op_orchestrator::OrchestratorError` — a plain
//! enum with a `Display` impl, no `thiserror`. Every variant reproduces the
//! exact sentence the stringly-typed version produced, so the JSON `error`
//! field each per-target result carries is unchanged.

use std::fmt;
use std::path::PathBuf;

/// The filesystem verb that failed. Doubles as the message prefix, which is
/// why the messages read `create <path>: <io error>` etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FsAction {
    Create,
    Write,
    Read,
    Remove,
    Inspect,
    Parse,
    Serialize,
}

impl FsAction {
    fn label(self) -> &'static str {
        match self {
            FsAction::Create => "create",
            FsAction::Write => "write",
            FsAction::Read => "read",
            FsAction::Remove => "remove",
            FsAction::Inspect => "inspect",
            FsAction::Parse => "parse",
            FsAction::Serialize => "serialize",
        }
    }
}

/// Something is wrong with the skill bundle compiled into the binary — a
/// build-time packaging fault, never a user mistake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BundleError {
    /// The template did not carry the expected number of version sentinels.
    SentinelCount { expected: usize, found: usize },
    /// A sentinel survived rendering (so a shipped file would advertise the
    /// placeholder as its version).
    SentinelRemains,
    /// The rendered bundle is not valid JSON.
    Parse(String),
    /// A required top-level field (`version` / `files`) is absent.
    MissingField(&'static str),
    /// The bundle carries no files at all.
    Empty,
    /// A bundle file entry is not a JSON string.
    FileNotString(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::SentinelCount { expected, found } => write!(
                f,
                "embedded skill bundle template expected {expected} version sentinels {:?}, \
                 found {found}",
                crate::skill_install_cli::VERSION_SENTINEL
            ),
            BundleError::SentinelRemains => f.write_str(
                "embedded skill bundle still contains the version sentinel after rendering",
            ),
            BundleError::Parse(e) => write!(f, "parse skill bundle: {e}"),
            BundleError::MissingField(field) => write!(f, "skill bundle missing {field}"),
            BundleError::Empty => f.write_str("embedded skill bundle is empty"),
            BundleError::FileNotString(path) => write!(f, "bundle file {path:?} is not a string"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillInstallError {
    /// `--target <name>` named an agent this build does not support.
    UnknownTarget(String),
    /// Nothing to install into and no `--target` to disambiguate.
    NoTargetsDetected,
    /// Neither `$HOME` nor `%USERPROFILE%` is set.
    HomeUnavailable,
    /// A plain filesystem / serde operation against `path` failed.
    Fs {
        action: FsAction,
        path: PathBuf,
        detail: String,
    },
    /// Neither symlinking nor copying could create the discovery entry.
    Link {
        link: PathBuf,
        target: PathBuf,
        detail: String,
    },
    /// A discovery entry the installer would write is already occupied by
    /// something it did not create (e.g. the user's own skill of that name).
    NotOwned(PathBuf),
    /// An agent config file exists but its root is not a JSON object.
    NotAJsonObject(PathBuf),
    /// A config key that must hold an object holds something else.
    NotAnObject(String),
    /// The bundle compiled into this binary is unusable.
    Bundle(BundleError),
}

impl SkillInstallError {
    pub(crate) fn fs(
        action: FsAction,
        path: impl Into<PathBuf>,
        detail: impl fmt::Display,
    ) -> Self {
        SkillInstallError::Fs {
            action,
            path: path.into(),
            detail: detail.to_string(),
        }
    }

    pub(crate) fn link(
        link: impl Into<PathBuf>,
        target: impl Into<PathBuf>,
        detail: impl fmt::Display,
    ) -> Self {
        SkillInstallError::Link {
            link: link.into(),
            target: target.into(),
            detail: detail.to_string(),
        }
    }
}

impl fmt::Display for SkillInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillInstallError::UnknownTarget(raw) => write!(
                f,
                "unknown target {raw:?}; available: claude, codex, cursor, opencode, cline"
            ),
            SkillInstallError::NoTargetsDetected => f.write_str(
                "no supported AI coding agents detected; pass --target claude|codex|cursor|opencode|cline",
            ),
            SkillInstallError::HomeUnavailable => f.write_str("home directory not available"),
            SkillInstallError::Fs {
                action,
                path,
                detail,
            } => write!(f, "{} {}: {detail}", action.label(), path.display()),
            SkillInstallError::Link {
                link,
                target,
                detail,
            } => write!(
                f,
                "link {} -> {}: {detail}",
                link.display(),
                target.display()
            ),
            SkillInstallError::NotAJsonObject(path) => {
                write!(f, "{} must contain a JSON object", path.display())
            }
            SkillInstallError::NotOwned(path) => write!(
                f,
                "{} already exists and was not created by openpencil; move it aside and retry",
                path.display()
            ),
            SkillInstallError::NotAnObject(key) => write!(f, "{key} is not an object"),
            SkillInstallError::Bundle(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SkillInstallError {}

impl From<BundleError> for SkillInstallError {
    fn from(error: BundleError) -> Self {
        SkillInstallError::Bundle(error)
    }
}
