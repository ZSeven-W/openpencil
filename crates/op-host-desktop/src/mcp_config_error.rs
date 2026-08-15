//! Typed failures for terminal-side MCP client configuration — the enum
//! shared by `mcp_integrations.rs` (which CLI writes which key, and the
//! Antigravity two-file transaction) and `mcp_config_io.rs` (the lossless
//! TOML edit + crash-safe write underneath it). They are one failure domain:
//! every path here ends in "read, edit, and atomically rewrite a config file
//! that belongs to another tool", and the settings panel surfaces whatever
//! comes back as one message.
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Every
//! variant carries STRUCTURED fields and `Display` re-formats the sentence,
//! so the text is byte-identical — these messages reach the MCP settings tab
//! and `mcp_integrations`' rollback test asserts on the word "parse".
//!
//! What the enum adds is the distinction the stringly code could not express:
//! a config we REFUSE to touch because its shape is wrong
//! ([`McpConfigError::McpServersNotAnObject`] and friends — the user's file
//! is intact and we stopped on purpose) versus an IO failure mid-write, and —
//! the load-bearing one — [`McpConfigError::Rollback`], which nests the
//! original cause and every failed undo instead of flattening the whole
//! transaction into one pre-joined sentence.
//!
//! One inbound seam still speaks `String`:
//! `op_host_services::doc_io::atomic_file`'s `create_sibling_temp` /
//! `replace_file`, in a crate this pass does not own. Their message is
//! carried verbatim by [`McpConfigError::AtomicFile`], adapted with
//! `e.to_string()` so the bridge survives if that crate later types its own
//! error.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpConfigError {
    /// No home directory is available, so no CLI config path can be derived.
    HomeDirUnavailable,
    /// The Antigravity integration was reached through the explicit-path
    /// entry point, which cannot resolve its second (permissions) file.
    AntigravityNeedsHome,
    /// Reading the existing config failed for a reason other than "not
    /// found" — a missing file is an empty config, not an error.
    Read { path: PathBuf, message: String },
    /// The existing config is not valid JSON / TOML.
    Parse { path: PathBuf, message: String },
    /// The file's metadata (needed to preserve its permission bits) could not
    /// be read while snapshotting it for rollback.
    Metadata { path: PathBuf, message: String },
    /// Deleting a file that did not exist before the transaction failed while
    /// rolling back.
    Remove { path: PathBuf, message: String },
    /// Writing the replacement bytes into the sibling temp failed.
    Write { path: PathBuf, message: String },
    /// `fsync` on the sibling temp failed — the replace is not attempted,
    /// because an unsynced temp could land as a truncated config.
    Sync { path: PathBuf, message: String },
    /// Re-applying the original permission bits to the sibling temp failed.
    Permissions { path: PathBuf, message: String },
    /// Creating the config's parent directory failed.
    CreateDir { path: PathBuf, message: String },
    /// The config path has no parent directory, so no sibling temp can be
    /// placed next to it.
    NoParentDirectory { path: PathBuf },
    /// Re-serializing the edited JSON document failed.
    Serialize { path: PathBuf, message: String },
    /// The config parsed but its root is not a JSON object.
    NotAJsonObject { path: PathBuf },
    /// `permissions` exists but is not an object (Antigravity settings).
    PermissionsNotAnObject,
    /// `permissions.allow` exists but is not an array (Antigravity settings).
    PermissionsAllowNotAnArray,
    /// `mcpServers` exists but is not an object (JSON-config CLIs).
    McpServersNotAnObject,
    /// `mcp_servers` vanished between the insert and the lookup (Grok TOML) —
    /// only reachable if `toml_edit` changed semantics under us.
    GrokMcpServersMissing,
    /// `mcp_servers` exists in the Grok TOML but is not a table.
    GrokMcpServersNotATable,
    /// The dsh patch file carries an `mcp-openpencil` entry the user wrote
    /// by hand (no OpenPencil marker block). Disabling refuses to delete it
    /// rather than silently dropping user content.
    DshManualEntry { path: PathBuf },
    /// The dsh patch file carries only one of the two OpenPencil markers —
    /// the managed block was hand-edited. Refuse to touch it.
    DshPatchMarkersMismatched { path: PathBuf },
    /// The shared crash-safe file primitives refused. Carries their message.
    AtomicFile(String),
    /// A multi-file transaction failed and its undo failed too. Keeps the
    /// original `cause` and each rollback failure as typed errors instead of
    /// pre-joined prose, so a caller can inspect either half; `Display` still
    /// renders the exact sentence the flattened version produced.
    Rollback {
        cause: Box<McpConfigError>,
        failures: Vec<McpConfigError>,
    },
}

impl fmt::Display for McpConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpConfigError::HomeDirUnavailable => f.write_str("home directory not available"),
            McpConfigError::AntigravityNeedsHome => {
                f.write_str("Antigravity configuration requires a home directory")
            }
            McpConfigError::Read { path, message } => {
                write!(f, "read {}: {message}", path.display())
            }
            McpConfigError::Parse { path, message } => {
                write!(f, "parse {}: {message}", path.display())
            }
            McpConfigError::Metadata { path, message } => {
                write!(f, "metadata {}: {message}", path.display())
            }
            McpConfigError::Remove { path, message } => {
                write!(f, "remove {}: {message}", path.display())
            }
            McpConfigError::Write { path, message } => {
                write!(f, "write {}: {message}", path.display())
            }
            McpConfigError::Sync { path, message } => {
                write!(f, "sync {}: {message}", path.display())
            }
            McpConfigError::Permissions { path, message } => {
                write!(f, "permissions {}: {message}", path.display())
            }
            McpConfigError::CreateDir { path, message } => {
                write!(f, "create {}: {message}", path.display())
            }
            McpConfigError::NoParentDirectory { path } => {
                write!(f, "{} has no parent directory", path.display())
            }
            McpConfigError::Serialize { path, message } => {
                write!(f, "serialize {}: {message}", path.display())
            }
            McpConfigError::NotAJsonObject { path } => {
                write!(f, "{} must contain a JSON object", path.display())
            }
            McpConfigError::PermissionsNotAnObject => f.write_str("permissions is not an object"),
            McpConfigError::PermissionsAllowNotAnArray => {
                f.write_str("permissions.allow is not an array")
            }
            McpConfigError::McpServersNotAnObject => f.write_str("mcpServers is not an object"),
            McpConfigError::GrokMcpServersMissing => f.write_str("mcp_servers is missing"),
            McpConfigError::GrokMcpServersNotATable => f.write_str("mcp_servers must be a table"),
            McpConfigError::DshManualEntry { path } => {
                write!(
                    f,
                    "{} contains a manually added mcp-openpencil entry; remove it manually to disable this integration",
                    path.display()
                )
            }
            McpConfigError::DshPatchMarkersMismatched { path } => {
                write!(
                    f,
                    "{} carries only one openpencil-mcp marker; repair or remove the marker block manually",
                    path.display()
                )
            }
            McpConfigError::AtomicFile(message) => f.write_str(message),
            McpConfigError::Rollback { cause, failures } => {
                let joined = failures
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "{cause}; rollback failed: {joined}")
            }
        }
    }
}

impl std::error::Error for McpConfigError {}
