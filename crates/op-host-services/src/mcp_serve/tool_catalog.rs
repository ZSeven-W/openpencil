//! Opt-in tool catalogs — which slice of the MCP tool surface a session
//! advertises and accepts.
//!
//! The full catalog (135 static schemas plus the per-document kit insert
//! tools) is what every existing client is configured against, so it stays
//! the default. It is also expensive: an external agent pays the schema text
//! in context on every turn and has to pick among near-duplicates. The
//! **lean** catalog is six tools that still cover the whole design loop —
//! read, learn, write, verify, finish — with every capability delegated to the
//! existing tool implementations (see `lean_profile`).
//!
//! A catalog is orthogonal to [`super::tool_profile::McpAccessProfile`]'s
//! deployment and scope checks: it narrows what is offered, it never widens
//! what a deployment allows.
//!
//! Selection surfaces:
//!
//! - `--mcp <path> --mcp-profile lean` / `--mcp-http <port> <path> --mcp-profile lean`
//! - `OPENPENCIL_MCP_PROFILE=lean` (the flag wins when both are present)
//! - the live desktop server's [`LEAN_MCP_PATH`] endpoint, which is what the
//!   terminal-integration writer installs when the lean toggle is on.

use std::fmt;
use std::str::FromStr;

/// Environment variable naming the catalog for `--mcp` / `--mcp-http`.
pub const MCP_PROFILE_ENV: &str = "OPENPENCIL_MCP_PROFILE";

/// Command-line flag naming the catalog for `--mcp` / `--mcp-http`. Accepted
/// as `--mcp-profile lean` or `--mcp-profile=lean`.
pub const MCP_PROFILE_FLAG: &str = "--mcp-profile";

/// Streamable-HTTP path that serves the lean catalog on the live server.
/// `/mcp` (and `/`) keep serving the full catalog. Defined next to
/// `local_mcp_url` so the integration writer and this router share it.
pub const LEAN_MCP_PATH: &str = op_editor_core::agent_settings_connection::LEAN_MCP_PATH;

/// The lean catalog, in workflow order: read → learn → write → verify →
/// finish. `tools/list` advertises them in this order.
pub const LEAN_TOOLS: &[&str] = &[
    "get_editor_state",
    "get_guidelines",
    "batch_design",
    "snapshot_layout",
    "get_screenshot",
    "finalize_design",
];

/// Which tool catalog a session serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpToolCatalog {
    /// Every tool — the behaviour that predates catalogs.
    #[default]
    Full,
    /// The six-tool design loop in [`LEAN_TOOLS`].
    Lean,
}

impl McpToolCatalog {
    /// Every catalog, for help text and parse errors.
    pub const ALL: [Self; 2] = [Self::Full, Self::Lean];

    /// The name users type (`--mcp-profile <name>`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Lean => "lean",
        }
    }

    /// Whether `tool` belongs to this catalog.
    pub fn admits(self, tool: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Lean => LEAN_TOOLS.contains(&tool),
        }
    }

    /// The HTTP path a live-server client should target for this catalog.
    pub const fn http_path(self) -> &'static str {
        match self {
            Self::Full => "/mcp",
            Self::Lean => LEAN_MCP_PATH,
        }
    }

    /// The catalog a live-server request path selects, or `None` when the
    /// path is not a JSON-RPC endpoint at all.
    pub fn for_http_path(path: &str) -> Option<Self> {
        match path {
            "/mcp" | "/" => Some(Self::Full),
            LEAN_MCP_PATH => Some(Self::Lean),
            _ => None,
        }
    }

    /// Parse a user-supplied catalog name. Case-insensitive; surrounding
    /// whitespace is ignored. `default` is an alias for `full`.
    pub fn parse(raw: &str, source: ProfileSource) -> Result<Self, McpProfileError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "full" | "default" => Ok(Self::Full),
            "lean" => Ok(Self::Lean),
            _ => Err(McpProfileError::UnknownProfile {
                value: raw.to_string(),
                source,
            }),
        }
    }
}

impl fmt::Display for McpToolCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for McpToolCatalog {
    type Err = McpProfileError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::parse(raw, ProfileSource::Flag)
    }
}

/// Where a catalog name came from, so a parse error can point at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    Flag,
    Env,
}

/// Why a catalog selection could not be honoured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProfileError {
    /// The name is not one of [`McpToolCatalog::ALL`].
    UnknownProfile {
        value: String,
        source: ProfileSource,
    },
    /// `--mcp-profile` was the last argument, with no name after it.
    MissingValue,
}

impl fmt::Display for McpProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expected = McpToolCatalog::ALL.map(McpToolCatalog::name).join(", ");
        match self {
            Self::UnknownProfile { value, source } => {
                let origin = match source {
                    ProfileSource::Flag => MCP_PROFILE_FLAG,
                    ProfileSource::Env => MCP_PROFILE_ENV,
                };
                write!(
                    f,
                    "unknown MCP tool profile {value:?} from {origin}; expected one of: {expected}"
                )
            }
            Self::MissingValue => write!(
                f,
                "{MCP_PROFILE_FLAG} needs a profile name; expected one of: {expected}"
            ),
        }
    }
}

impl std::error::Error for McpProfileError {}

/// Pull `--mcp-profile <name>` / `--mcp-profile=<name>` out of the argv that
/// follows a server mode, returning the remaining positional arguments in
/// order plus the flag value (the last occurrence wins).
pub fn extract_profile_flag(
    args: impl Iterator<Item = String>,
) -> Result<(Vec<String>, Option<String>), McpProfileError> {
    let mut positional = Vec::new();
    let mut profile = None;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if arg == MCP_PROFILE_FLAG {
            let value = args.next().ok_or(McpProfileError::MissingValue)?;
            profile = Some(value);
        } else if let Some(value) = arg
            .strip_prefix(MCP_PROFILE_FLAG)
            .and_then(|rest| rest.strip_prefix('='))
        {
            if value.trim().is_empty() {
                return Err(McpProfileError::MissingValue);
            }
            profile = Some(value.to_string());
        } else {
            positional.push(arg);
        }
    }
    Ok((positional, profile))
}

/// Resolve the catalog from an explicit flag value and the environment
/// value. The flag wins; an unset or blank environment value means full.
/// Pure (both values are passed in) so tests never touch process env.
pub fn resolve_catalog(
    flag: Option<&str>,
    env: Option<&str>,
) -> Result<McpToolCatalog, McpProfileError> {
    if let Some(flag) = flag {
        return McpToolCatalog::parse(flag, ProfileSource::Flag);
    }
    match env.map(str::trim).filter(|value| !value.is_empty()) {
        Some(env) => McpToolCatalog::parse(env, ProfileSource::Env),
        None => Ok(McpToolCatalog::Full),
    }
}

/// [`resolve_catalog`] against the real `OPENPENCIL_MCP_PROFILE`.
pub fn resolve_catalog_from_env(flag: Option<&str>) -> Result<McpToolCatalog, McpProfileError> {
    let env = std::env::var(MCP_PROFILE_ENV).ok();
    resolve_catalog(flag, env.as_deref())
}

/// The refusal text for a tool that exists but is outside `catalog`.
///
/// Names the tools that ARE available so an agent can recover in the same
/// turn instead of guessing, and says how to reach the full catalog.
pub(crate) fn not_in_catalog_message(code: &str, catalog: McpToolCatalog, tool: &str) -> String {
    let available = match catalog {
        McpToolCatalog::Full => String::new(),
        McpToolCatalog::Lean => LEAN_TOOLS.join(", "),
    };
    format!(
        "{code}: the tool '{tool}' is not part of the '{catalog}' MCP tool profile. \
         Available tools: {available}. Use batch_design operations for node edits and \
         get_editor_state nodeIds for reads; connect to /mcp (or drop {MCP_PROFILE_FLAG}) \
         for the full catalog."
    )
}

#[cfg(test)]
#[path = "tool_catalog_tests.rs"]
mod tests;
