//! Typed failures for the auto-saved user settings file
//! (`settings_io.rs` + its strict validator `settings_io_checked.rs`).
//!
//! Style follows `op_orchestrator::OrchestratorError`: a plain enum plus a
//! hand-written `Display`, no `thiserror` and no new dependency. Unlike the
//! transport enums in this crate, every variant carries STRUCTURED fields and
//! `Display` re-formats the sentence, so the user-visible text — which the
//! `--serve-web` daemon logs and `settings_io_tests.rs` asserts on — is
//! reproduced byte for byte while callers can match on the reason instead of
//! the prose.
//!
//! What the enum adds is a phase classification the flat strings could not
//! express: whether a failure happened while LOCATING the file
//! ([`SettingsIoError::PathUnresolved`] / [`SettingsIoError::PathUnavailable`]),
//! while READING it ([`SettingsIoError::Read`] / [`SettingsIoError::Parse`]),
//! while VALIDATING it (`UnsupportedVersion` / `UnknownField` / `Lossy` /
//! `UnsupportedCredentialEntry` — the "the daemon would silently drop settings
//! it cannot round-trip" family that `load_checked` exists to produce, and the
//! difference between it and the best-effort `settings_io::load`), or while
//! WRITING it (the `CreateDir` … `Replace` family, all of which leave the
//! previous file intact).
//!
//! Every seam is typed: `save_checked` / `load_checked` and the private
//! `*_to_path` / `*_from_path` twins the tests drive all report this enum, and
//! the three `--serve-web` call sites that used to pin a
//! `FnOnce(&EditorState) -> Result<(), String>` closure bound
//! (`web_canvas_server::persist_api_settings`,
//! `web_canvas_server::serve_options::startup_editor_for_web_canvas_with_loader`,
//! `web_canvas_server::run_loop::enforce_credential_persistence_policy`) now
//! name this type instead. `settings_io::load` / `save` stay infallible
//! best-effort wrappers, so the desktop host — which only calls those — is
//! untouched by the conversion.
//!
//! One inbound seam still speaks `String`: `serde_json`'s and `std::io`'s own
//! messages, which are interpolated verbatim into the `detail` fields below
//! exactly as the pre-conversion `format!` calls did.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsIoError {
    /// No usable platform config base, so the strict loader cannot even name
    /// the file it was asked to read.
    PathUnresolved,
    /// Same condition on the save side. Worded differently from
    /// [`SettingsIoError::PathUnresolved`] because the two sentences predate
    /// this enum and are logged verbatim.
    PathUnavailable,
    /// The settings file exists but could not be read off disk.
    Read { detail: String },
    /// The bytes are not the JSON this build expects. Both the raw-`Value`
    /// pass and the typed `SettingsPayload` pass report this — they produced
    /// the same sentence before the conversion, so they share a variant.
    Parse { detail: String },
    /// The file declares a schema version this build does not write, so
    /// loading it would silently drop or mangle fields.
    UnsupportedVersion { found: u32, expected: u32 },
    /// A field this build does not know about appeared inside `context`
    /// ("root", "Openverse OAuth", "built-in agent", …). Saving over it would
    /// drop the unknown key, so the strict loader refuses instead.
    UnknownField { context: String },
    /// The file parses but cannot be re-serialized without losing or changing
    /// a value (out-of-range port, unknown theme/locale/preset, a dangling
    /// active image profile, untrimmed Openverse credentials, …).
    Lossy,
    /// A credential entry (built-in agent, ACP agent, image profile) is not
    /// representable at all — distinct from [`SettingsIoError::Lossy`], which
    /// means "representable, but not losslessly".
    UnsupportedCredentialEntry,
    /// The settings directory could not be created before the write.
    CreateDir { detail: String },
    /// The live preferences could not be encoded to JSON.
    Encode { detail: String },
    /// The sibling temporary file could not be created.
    CreateTemp { detail: String },
    /// The temporary file was created but could not be restricted to `0600`,
    /// so it is removed rather than left holding API keys world-readable.
    SecureTemp { detail: String },
    /// 128 candidate temporary names were all taken — a stuck directory
    /// rather than a transient collision.
    TempAllocExhausted,
    /// Writing the encoded JSON into the temporary file failed.
    WriteTemp { detail: String },
    /// The completed temporary file could not be renamed over the real
    /// settings file. The previous file is still intact.
    Replace { detail: String },
}

impl fmt::Display for SettingsIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsIoError::PathUnresolved => f.write_str("failed to resolve settings file path"),
            SettingsIoError::PathUnavailable => f.write_str("settings path is unavailable"),
            SettingsIoError::Read { detail } => {
                write!(f, "failed to read settings file: {detail}")
            }
            SettingsIoError::Parse { detail } => {
                write!(f, "failed to parse settings file: {detail}")
            }
            SettingsIoError::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported settings file version {found}; expected {expected}"
            ),
            SettingsIoError::UnknownField { context } => {
                write!(f, "unknown settings field in {context}")
            }
            SettingsIoError::Lossy => f.write_str("settings file cannot be loaded losslessly"),
            SettingsIoError::UnsupportedCredentialEntry => {
                f.write_str("unsupported settings credential entry")
            }
            SettingsIoError::CreateDir { detail } => {
                write!(f, "failed to create settings directory: {detail}")
            }
            SettingsIoError::Encode { detail } => {
                write!(f, "failed to encode settings: {detail}")
            }
            SettingsIoError::CreateTemp { detail } => {
                write!(f, "failed to create temporary settings file: {detail}")
            }
            SettingsIoError::SecureTemp { detail } => {
                write!(f, "failed to secure temporary settings file: {detail}")
            }
            SettingsIoError::TempAllocExhausted => {
                f.write_str("failed to allocate a unique temporary settings file")
            }
            SettingsIoError::WriteTemp { detail } => {
                write!(f, "failed to write temporary settings: {detail}")
            }
            SettingsIoError::Replace { detail } => {
                write!(f, "failed to replace settings file: {detail}")
            }
        }
    }
}

impl std::error::Error for SettingsIoError {}
