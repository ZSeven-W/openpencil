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

#[cfg(feature = "mcp-debug-tools")]
use super::schemas::DEBUG_TOOL_SCHEMAS;
use super::schemas::TOOL_SCHEMAS;

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
}

impl ToolRefusal {
    /// Stable machine-readable code. It leads the wire message so a client
    /// can branch on it without parsing prose.
    pub const fn code(self) -> &'static str {
        match self {
            Self::LocalResourceDenied => "tool-not-available",
            Self::UserTemplateDenied => "user-template-not-available",
            Self::ScopeInsufficient => "scope-insufficient",
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
}

impl McpAccessProfile {
    /// What the local and managed daemons run under: the whole catalog, full
    /// authority. Byte-for-byte the behaviour that predates this module.
    pub const UNRESTRICTED: Self = Self {
        deny_unshareable_tools: false,
        scopes: McpScopes::FULL,
    };

    /// The public multi-account profile.
    pub const fn online(scopes: McpScopes) -> Self {
        Self {
            deny_unshareable_tools: true,
            scopes,
        }
    }

    /// Whether `tool` may appear in this profile's `tools/list`.
    ///
    /// Scope is deliberately NOT consulted here: a read-only token should
    /// still see that a write tool exists, and be told why when it calls one.
    /// Hiding it would look like the tool had been removed.
    pub fn lists(&self, tool: &str) -> bool {
        !self.deny_unshareable_tools || surface_of(tool).is_shareable()
    }

    /// Why `tool` may not be called, if it may not.
    ///
    /// Denial ranks above scope: a tool that is off for everyone should say
    /// so rather than suggesting a bigger token would help.
    pub fn refuse(&self, tool: &str) -> Option<ToolRefusal> {
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

/// Every tool in `schemas::TOOL_SCHEMAS`, classified.
///
/// Kept in the same alphabetical order as the catalog so the two are
/// diffable side by side.
pub const TOOL_PROFILES: &[ToolProfile] = &[
    ToolProfile::new("ToolSearch", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("add_node_effect", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("add_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("align_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "apply_design_system",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("batch_design", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("batch_get", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("clear_selection", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "codegen_assemble",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new(
        "codegen_clean",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new("codegen_plan", ToolAccess::Read, ToolSurface::ProcessGlobal),
    ToolProfile::new(
        "codegen_submit_chunk",
        ToolAccess::Read,
        ToolSurface::ProcessGlobal,
    ),
    ToolProfile::new("conversion_status", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("copy_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("copy_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("count_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("create_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("create_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("cut_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "cycle_active_axis_value",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "debug_logs_tail",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "debug_screenshot",
        ToolAccess::Read,
        ToolSurface::HostDiagnostics,
    ),
    ToolProfile::new(
        "debug_validation_report",
        ToolAccess::Read,
        ToolSurface::HostDiagnostics,
    ),
    ToolProfile::new("delete_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("delete_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_content", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_refine", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("design_skeleton", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "enrich_images",
        ToolAccess::Write,
        // Stock-photo search dials the product-constant Openverse / Wikimedia
        // hosts from the daemon — outbound network, like import_html_url.
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new("finalize_design", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("duplicate_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "duplicate_selected",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("export_design_md", ToolAccess::Read, ToolSurface::InMemory),
    // Writes a deck file at a caller-chosen path, so it is a local-filesystem
    // surface rather than an in-memory one: a hosted tenant must not be able
    // to place bytes anywhere on the daemon host.
    // Writes a directory of images at a caller-chosen path.
    ToolProfile::new(
        "export_frames",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("get_deck_boards", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "export_deck",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    // Reads the user's imported DESIGN.md files off the daemon host, not
    // just the embedded corpus, so it is not an in-memory surface.
    ToolProfile::new(
        "list_style_guides",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    // Mixed surface: every profile may list the immutable shipped catalogue;
    // registry construction injects whether the local-only, process-global
    // saved half may be included. The profile-aware snapshot is what keeps the
    // online result in-memory instead of hiding the whole discovery tool.
    ToolProfile::new(
        "list_scene_templates",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "use_scene_template",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("export_item", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("export_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("find_empty_space", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("find_node_by_name", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_active_theme", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_canvas_bounds", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_component", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_design_md", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_design_prompt", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_document_info", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_editor_state", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_guidelines", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_history_depth", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node_children", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_node_parent", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_screenshot", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_selection", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_selection_set", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_style_guide", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "get_style_guide_tags",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("get_variables", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("get_viewport", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("group_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "import_html",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "import_html_url",
        ToolAccess::Write,
        ToolSurface::OutboundNetwork,
    ),
    ToolProfile::new(
        "import_svg",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "import_web_snapshot",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("insert_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "instantiate_component",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("lint_document", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_components", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_node_kinds", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("list_pages", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "list_theme_presets",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("list_variables", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new(
        "load_theme_preset",
        ToolAccess::Write,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("move_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("nudge_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "open_document",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new("paste_clipboard", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("read_nodes", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("redo", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "remove_node_effect",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("remove_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("rename_variable", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("reorder_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("reorder_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "replace_all_matching_properties",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("replace_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "save_document",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "save_theme_preset",
        ToolAccess::Read,
        ToolSurface::LocalFilesystem,
    ),
    ToolProfile::new(
        "search_all_unique_properties",
        ToolAccess::Read,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_active_axis_value",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_active_page", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_active_tool", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_design_md", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_ellipse_arc", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_collapsed",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_corner_radius",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_fill_hex",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_flip", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_font_size",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_font_weight",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_hidden", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_node_locked", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_node_name", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_node_rotation",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_hex",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_side_width",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_node_stroke_width",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_node_text", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_selection", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_selection_set",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_themes", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new(
        "set_variable_boolean",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_color",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_number",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new(
        "set_variable_string",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("set_variables", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("set_viewport", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("snapshot_layout", ToolAccess::Read, ToolSurface::InMemory),
    ToolProfile::new("spawn_agents", ToolAccess::Read, ToolSurface::HostProcess),
    ToolProfile::new(
        "toggle_node_selection",
        ToolAccess::Write,
        ToolSurface::InMemory,
    ),
    ToolProfile::new("undo", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("ungroup_selected", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("update_node", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_component", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_screen", ToolAccess::Write, ToolSurface::InMemory),
    ToolProfile::new("upsert_variables", ToolAccess::Write, ToolSurface::InMemory),
];

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

/// Names that only exist in a build with the debug-tool feature.
///
/// They stay classified in every build so the deny decision cannot be lost
/// by flipping a feature flag; the parity test knows they are absent from the
/// catalog when the feature is off.
pub const DEBUG_ONLY_TOOLS: &[&str] = &[
    "debug_logs_tail",
    "debug_screenshot",
    "debug_validation_report",
];

pub fn is_debug_only_tool(name: &str) -> bool {
    DEBUG_ONLY_TOOLS.contains(&name)
}

/// Every catalog name this build advertises.
///
/// Only the parity tests consume this outside a debug-tool build; it stays
/// compiled either way so the two builds cannot drift.
#[cfg_attr(not(feature = "mcp-debug-tools"), allow(dead_code))]
pub(crate) fn catalog_tool_names() -> Vec<String> {
    #[cfg_attr(not(feature = "mcp-debug-tools"), allow(unused_mut))]
    let mut names: Vec<String> = TOOL_SCHEMAS
        .iter()
        .filter_map(|schema| schema_name(schema))
        .collect();
    #[cfg(feature = "mcp-debug-tools")]
    names.extend(
        DEBUG_TOOL_SCHEMAS
            .iter()
            .filter_map(|schema| schema_name(schema)),
    );
    names
}

/// Pull `"name":"…"` out of a schema entry.
///
/// The schemas are pre-serialized JSON string constants, and the name is
/// always the first member, so this reads it without a JSON parse.
pub(crate) fn schema_name(schema: &str) -> Option<String> {
    let rest = schema.split_once(r#""name":"#)?.1.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
#[path = "tool_profile_tests.rs"]
mod tests;
