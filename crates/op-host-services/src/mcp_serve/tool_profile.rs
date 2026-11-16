//! What each MCP tool needs, and what a deployment is willing to give it.
//!
//! The tool catalog grew up serving one local operator, so a tool that writes
//! a caller-named path or reads the daemon's own config directory was simply a
//! feature. A public multi-account deployment shares one process — and one
//! filesystem — between mutually untrusting accounts, so those tools have to
//! be off, and a token that was issued read-only must not be able to drive a
//! write tool.
//!
//! Both questions are answered from ONE table, [`TOOL_PROFILES`], because the
//! failure mode of two tables is that they disagree. A test pins the table
//! against `schemas::TOOL_SCHEMAS` in both directions, so a tool added to the
//! catalog without being classified fails the build rather than defaulting to
//! something permissive.
//!
//! ## Where this is enforced
//!
//! Two places, and it must be both:
//!
//! - `tools/list` — a denied tool is filtered out of the catalog, so a client
//!   never learns it exists or writes a plan around it.
//! - `tools/call` — a denied tool is answered with a refusal even when the
//!   client asks for it by name anyway. Filtering the catalog is discovery,
//!   not enforcement; this is enforcement.
//!
//! The refusal happens BEFORE the tool runs, so a path-traversal argument on
//! a denied tool never reaches the code that would open it.

use super::schemas::TOOL_SCHEMAS;
use super::tool_catalog::{self, McpToolCatalog};

/// Whether a tool mutates the document.
///
/// Determined from the tool's own outcome contract: a `Write` tool is one
/// whose `call` can return `ToolOutcome::OkWithCommand` / `OkJsonWithCommand`,
/// i.e. one that hands the host an `EditorCommand` to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolAccess {
    Read,
    Write,
}

/// What the tool touches besides the in-memory document.
///
/// Only [`ToolSurface::InMemory`] is safe to expose on a shared deployment;
/// every other variant names a resource that belongs to the daemon's host or
/// is shared process-wide with no tenant dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSurface {
    /// Only the in-memory document and editor state.
    InMemory,
    /// Reads or writes the daemon host's filesystem, usually at a path the
    /// caller chooses.
    LocalFilesystem,
    /// Performs outbound network from the daemon host.
    OutboundNetwork,
    /// Reads or writes process-global state that carries no tenant
    /// dimension, so one account can observe or destroy another's.
    ProcessGlobal,
    /// Reports on the daemon host itself rather than on a document.
    HostDiagnostics,
    /// Spawns, or is specified to grow into spawning, host processes.
    HostProcess,
}

impl ToolSurface {
    /// Whether this surface is safe to expose to an untrusted account.
    pub const fn is_shareable(self) -> bool {
        matches!(self, Self::InMemory)
    }
}

/// One tool's classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolProfile {
    pub name: &'static str,
    pub access: ToolAccess,
    pub surface: ToolSurface,
}

impl ToolProfile {
    const fn new(name: &'static str, access: ToolAccess, surface: ToolSurface) -> Self {
        Self {
            name,
            access,
            surface,
        }
    }
}

/// Why a tool call was refused before it ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRefusal {
    /// The tool reaches a host or process-global resource that a shared
    /// deployment cannot partition between accounts.
    LocalResourceDenied,
    /// `use_scene_template` was asked to resolve a process-global user-saved
    /// template. Shipped templates remain safe and available online.
    UserTemplateDenied,
    /// The caller's credential does not carry the scope this tool needs.
    ScopeInsufficient,
    /// The tool exists but the session serves a narrower tool catalog.
    NotInCatalog(McpToolCatalog),
}

impl ToolRefusal {
    /// Stable machine-readable code. It leads the wire message so a client
    /// can branch on it without parsing prose.
    pub const fn code(self) -> &'static str {
        match self {
            Self::LocalResourceDenied => "tool-not-available",
            Self::UserTemplateDenied => "user-template-not-available",
            Self::ScopeInsufficient => "scope-insufficient",
            Self::NotInCatalog(_) => "tool-not-in-profile",
        }
    }

    /// The full refusal text a client receives.
    pub fn message(self, tool: &str) -> String {
        match self {
            Self::LocalResourceDenied => format!(
                "{}: the tool '{tool}' is not available on this deployment",
                self.code()
            ),
            Self::UserTemplateDenied => format!(
                "{}: user-saved scene templates are not available on this deployment",
                self.code()
            ),
            Self::ScopeInsufficient => format!(
                "{}: the tool '{tool}' requires the '{}' scope",
                self.code(),
                MCP_WRITE_SCOPE
            ),
            Self::NotInCatalog(catalog) => {
                tool_catalog::not_in_catalog_message(self.code(), catalog, tool)
            }
        }
    }
}

impl std::fmt::Display for ToolRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LocalResourceDenied => "tool is not available on this deployment",
            Self::UserTemplateDenied => {
                "user-saved scene templates are not available on this deployment"
            }
            Self::ScopeInsufficient => "credential lacks the required scope",
            Self::NotInCatalog(_) => "tool is not part of this session's tool profile",
        })
    }
}

impl std::error::Error for ToolRefusal {}

/// Scope names, matching what op-hub issues on an API token.
pub const MCP_READ_SCOPE: &str = "mcp:read";
pub const MCP_WRITE_SCOPE: &str = "mcp:write";

/// What a credential is allowed to do over MCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpScopes {
    read: bool,
    write: bool,
}

impl McpScopes {
    /// Unrestricted — what a local operator, a managed supervisor, and a
    /// browser session all get.
    pub const FULL: Self = Self {
        read: true,
        write: true,
    };

    /// Read-only.
    pub const READ_ONLY: Self = Self {
        read: true,
        write: false,
    };

    /// Derive from the scope list on a hub token.
    ///
    /// **Fail-closed.** A token that names no `mcp:*` scope gets nothing: it
    /// can neither read nor write. An earlier version treated an unscoped
    /// token as unrestricted, for compatibility with a token store that did
    /// not yet issue scopes — but that store still issues no tokens, so
    /// tightening this costs nothing today and removes a default that would
    /// have been very hard to tighten later.
    ///
    /// **op-hub must issue explicit `mcp:read` / `mcp:write` scopes** on every
    /// token intended to drive the canvas; one without them is inert.
    pub fn from_scope_list<S: AsRef<str>>(scopes: &[S]) -> Self {
        let has = |wanted: &str| scopes.iter().any(|scope| scope.as_ref().trim() == wanted);
        Self {
            read: has(MCP_READ_SCOPE),
            write: has(MCP_WRITE_SCOPE),
        }
    }

    /// A credential that may do nothing.
    pub const NONE: Self = Self {
        read: false,
        write: false,
    };

    pub const fn can_read(self) -> bool {
        self.read
    }

    pub const fn allows(self, access: ToolAccess) -> bool {
        match access {
            ToolAccess::Read => self.read,
            ToolAccess::Write => self.write,
        }
    }

    pub const fn can_write(self) -> bool {
        self.write
    }
}

impl Default for McpScopes {
    fn default() -> Self {
        Self::FULL
    }
}

/// The MCP capability profile one request is served under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpAccessProfile {
    /// Refuse every tool whose surface is not shareable.
    pub deny_unshareable_tools: bool,
    pub scopes: McpScopes,
    /// Which slice of the catalog is offered (see `tool_catalog`).
    pub catalog: McpToolCatalog,
}

impl McpAccessProfile {
    /// What the local and managed daemons run under: the whole catalog, full
    /// authority. Byte-for-byte the behaviour that predates this module.
    pub const UNRESTRICTED: Self = Self {
        deny_unshareable_tools: false,
        scopes: McpScopes::FULL,
        catalog: McpToolCatalog::Full,
    };

    /// A local operator on the lean six-tool catalog.
    pub const LEAN: Self = Self::UNRESTRICTED.with_catalog(McpToolCatalog::Lean);

    /// The public multi-account profile.
    pub const fn online(scopes: McpScopes) -> Self {
        Self {
            deny_unshareable_tools: true,
            scopes,
            catalog: McpToolCatalog::Full,
        }
    }

    /// The same deployment and scope decisions over a different catalog.
    pub const fn with_catalog(self, catalog: McpToolCatalog) -> Self {
        Self { catalog, ..self }
    }

    /// Whether `tool` may appear in this profile's `tools/list`.
    ///
    /// Scope is deliberately NOT consulted here: a read-only token should
    /// still see that a write tool exists, and be told why when it calls one.
    /// Hiding it would look like the tool had been removed.
    pub fn lists(&self, tool: &str) -> bool {
        self.catalog.admits(tool)
            && (!self.deny_unshareable_tools || surface_of(tool).is_shareable())
    }

    /// Why `tool` may not be called, if it may not.
    ///
    /// Denial ranks above scope: a tool that is off for everyone should say
    /// so rather than suggesting a bigger token would help.
    pub fn refuse(&self, tool: &str) -> Option<ToolRefusal> {
        // Outside the catalog first: the tool may well be allowed, just not
        // offered here, and saying so points the caller at the right fix.
        if !self.catalog.admits(tool) {
            return Some(ToolRefusal::NotInCatalog(self.catalog));
        }
        if self.deny_unshareable_tools && !surface_of(tool).is_shareable() {
            return Some(ToolRefusal::LocalResourceDenied);
        }
        if !self.scopes.allows(access_of(tool)) {
            return Some(ToolRefusal::ScopeInsufficient);
        }
        None
    }

    /// Why one concrete tool invocation may not run.
    ///
    /// Most capability decisions depend only on the tool name and are handled
    /// by [`Self::refuse`]. `use_scene_template` is intentionally mixed: bare
    /// ids resolve immutable shipped assets, while `user:` ids resolve the
    /// process-global saved-template registry. Shared deployments may keep the
    /// shipped half without exposing that unpartitioned user state.
    pub fn refuse_call(
        &self,
        tool: &str,
        arguments: &std::collections::BTreeMap<String, String>,
    ) -> Option<ToolRefusal> {
        if self.deny_unshareable_tools
            && tool == "use_scene_template"
            && arguments
                .get("templateId")
                .is_some_and(|id| id.trim().starts_with("user:"))
        {
            return Some(ToolRefusal::UserTemplateDenied);
        }
        self.refuse(tool)
    }

    /// Whether `list_scene_templates` may include the process-global saved
    /// half in addition to the immutable shipped catalogue.
    pub const fn includes_user_scene_templates(self) -> bool {
        !self.deny_unshareable_tools
    }
}

impl Default for McpAccessProfile {
    fn default() -> Self {
        Self::UNRESTRICTED
    }
}

/// Whether calling `name` will mutate the document.
///
/// Used by the connection tier to decide, BEFORE dispatch, whether a `/mcp`
/// call needs a shutdown write pass. Unclassified names default to `Write`
/// (see [`access_of`]), which is the safe direction here too: an unknown tool
/// is admitted through the barrier rather than slipping past it.
pub fn tool_writes(name: &str) -> bool {
    matches!(access_of(name), ToolAccess::Write)
}

/// The classification for `name`, if it is in the static catalog.
pub fn profile_for(name: &str) -> Option<&'static ToolProfile> {
    TOOL_PROFILES.iter().find(|profile| profile.name == name)
}

/// The surface of `name`, defaulting to the shareable one.
///
/// A name outside the static catalog is an `element_tools` insert tool:
/// those are generated per-document from the document's OWN component kits,
/// so they touch nothing but the in-memory document. The table-parity test
/// is what keeps this default from silently covering a real omission — a
/// static tool that is never classified fails that test.
fn surface_of(name: &str) -> ToolSurface {
    profile_for(name).map_or(ToolSurface::InMemory, |profile| profile.surface)
}

/// The access level of `name`, defaulting to `Write`.
///
/// Fail-closed on purpose, and in the direction that matters: an
/// unclassified tool is assumed to mutate, so a read-only credential cannot
/// reach it. The dynamic insert tools this covers really are writes.
fn access_of(name: &str) -> ToolAccess {
    profile_for(name).map_or(ToolAccess::Write, |profile| profile.access)
}

#[path = "tool_profile_catalog.rs"]
mod catalog;
pub use catalog::TOOL_PROFILES;

/// Names denied on a shared deployment, for diagnostics and tests.
pub fn denied_tool_names() -> Vec<&'static str> {
    TOOL_PROFILES
        .iter()
        .filter(|profile| !profile.surface.is_shareable())
        .map(|profile| profile.name)
        .collect()
}

/// The static schema catalog that discovery helpers may expose in `profile`.
///
/// `tools/list` filters each response directly because it also appends
/// document-specific element tools. `ToolSearch` needs a `'static` catalog,
/// so the one public-deployment variant is built once from the same
/// [`McpAccessProfile::lists`] decision and then shared by every request.
/// Scope does not participate in discovery listing, hence there are exactly
/// two variants: unrestricted and public/shared.
pub(crate) fn tool_search_schemas(profile: McpAccessProfile) -> &'static [&'static str] {
    if !profile.deny_unshareable_tools {
        return TOOL_SCHEMAS;
    }

    static ONLINE_SCHEMAS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    ONLINE_SCHEMAS
        .get_or_init(|| {
            let online = McpAccessProfile::online(McpScopes::FULL);
            TOOL_SCHEMAS
                .iter()
                .copied()
                .filter(|schema| schema_name(schema).is_none_or(|name| online.lists(&name)))
                .collect()
        })
        .as_slice()
}

#[path = "tool_profile_names.rs"]
mod names;
#[allow(unused_imports)]
pub(crate) use names::catalog_tool_names;
pub(crate) use names::schema_name;
pub use names::{is_debug_only_tool, DEBUG_ONLY_TOOLS};

#[cfg(test)]
#[path = "tool_profile_tests.rs"]
mod tests;
