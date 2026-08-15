//! DeepSeek Harness (dsh) MCP patch-file editing.
//!
//! dsh's MCP bridge plugin (`@deepseek-ai/dsh-mcp-client`) is configured
//! through the home-level patch layer `$DSH_HOME/cordis.patch.yml` (default
//! `~/.dsh/cordis.patch.yml`), which applies after every profile's own patch
//! and before `--patch` — one file, effective across all profiles. The file
//! is a top-level YAML array of patches; a new plugin instance is an
//! `insert` entry WITHOUT an `id` (verified against dsh's
//! `applyEntryPatches`, which pushes id-less inserts onto the top level).
//!
//! The writer is line-based on purpose — like `update_codex_config` it edits
//! only the marker block it owns and never re-serializes the file, so no
//! YAML dependency and no comment/formatting loss in the user's own
//! entries.

use std::fs;
use std::io;
use std::path::Path;

use crate::mcp_config_error::McpConfigError;
use crate::mcp_config_io::atomic_write;

/// Managed-block begin marker (a YAML comment line).
pub(crate) const DSH_PATCH_BEGIN: &str =
    "# openpencil-mcp-begin (managed by OpenPencil; do not edit)";
/// Managed-block end marker (a YAML comment line).
pub(crate) const DSH_PATCH_END: &str = "# openpencil-mcp-end";

/// The endpoint the settings panel's live server exposes over
/// streamable HTTP. Same shape as `mcp_integrations::endpoint`; duplicated
/// here so this module stays self-contained.
fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// The exact nine-line managed block, markers included. In the
/// top-level-array context this is valid YAML on its own.
fn dsh_patch_block(port: u16) -> String {
    format!(
        "{DSH_PATCH_BEGIN}\n- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: {}\n{DSH_PATCH_END}\n",
        endpoint(port)
    )
}

/// Line index of the first line whose trimmed content is `[]`, if any.
fn empty_array_line(lines: &[&str]) -> Option<usize> {
    lines.iter().position(|line| line.trim() == "[]")
}

/// `(begin, end)` line indices of a complete managed-marker pair.
fn marker_pair(lines: &[&str]) -> Option<(usize, usize)> {
    let begin = lines
        .iter()
        .position(|line| line.contains(DSH_PATCH_BEGIN))?;
    let end = lines.iter().position(|line| line.contains(DSH_PATCH_END))?;
    Some((begin, end))
}

/// Whether exactly one of the two managed markers is present — the block
/// was hand-edited, so refuse to guess where it ends.
fn lone_marker(lines: &[&str]) -> bool {
    let begin = lines.iter().any(|line| line.contains(DSH_PATCH_BEGIN));
    let end = lines.iter().any(|line| line.contains(DSH_PATCH_END));
    begin ^ end
}

fn dsh_text_has_openpencil(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.contains("id: mcp-openpencil") || trimmed.contains("serverName: openpencil")
    })
}

/// Whether the dsh patch layer already points at an OpenPencil server —
/// either our managed block or a hand-written entry (detection covers both
/// so a hand-wired dsh still lights the toggle up).
pub(crate) fn dsh_config_has_openpencil(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|text| dsh_text_has_openpencil(&text))
        .unwrap_or(false)
}

/// Install / update / remove the OpenPencil managed block in a dsh
/// `cordis.patch.yml`.
///
/// Enable:
/// - managed block present → rewrite the block in place (the port may have
///   changed, nothing else is touched);
/// - lone marker → error ([`McpConfigError::DshPatchMarkersMismatched`]);
/// - file missing → create it with the block alone (valid YAML at the
///   top-level-array position);
/// - a `[]` line exists (the default patch file shape, comments allowed
///   around it) → replace that line with the block, keeping the comments;
/// - otherwise (user entries present, or comments only) → append the block
///   at the end after one blank line.
///
/// Disable:
/// - managed block present → remove the block including both markers,
///   leaving everything else byte-identical;
/// - lone marker or a marker-less `mcp-openpencil` / `serverName: openpencil`
///   entry → error ([`McpConfigError::DshPatchMarkersMismatched`] /
///   [`McpConfigError::DshManualEntry`]) — never delete hand-written or
///   hand-edited content silently;
/// - nothing OpenPencil-related → no-op.
pub(crate) fn update_dsh_patch_config(
    path: &Path,
    enabled: bool,
    port: u16,
) -> Result<(), McpConfigError> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        }
    };
    let lines: Vec<&str> = existing.split_inclusive('\n').collect();

    if !enabled {
        match marker_pair(&lines) {
            Some((begin, end)) => {
                let mut out = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i < begin || i > end {
                        out.push_str(line);
                    }
                }
                atomic_write(path, out.as_bytes())
            }
            None => {
                if lone_marker(&lines) {
                    return Err(McpConfigError::DshPatchMarkersMismatched {
                        path: path.to_path_buf(),
                    });
                }
                if dsh_text_has_openpencil(&existing) {
                    return Err(McpConfigError::DshManualEntry {
                        path: path.to_path_buf(),
                    });
                }
                Ok(())
            }
        }
    } else {
        let text = match marker_pair(&lines) {
            Some((begin, end)) => {
                if end < begin {
                    return Err(McpConfigError::DshPatchMarkersMismatched {
                        path: path.to_path_buf(),
                    });
                }
                let mut out = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i < begin || i > end {
                        out.push_str(line);
                    } else if i == begin {
                        out.push_str(&dsh_patch_block(port));
                    }
                }
                out
            }
            None => {
                if lone_marker(&lines) {
                    return Err(McpConfigError::DshPatchMarkersMismatched {
                        path: path.to_path_buf(),
                    });
                }
                let block = dsh_patch_block(port);
                if !path.exists() {
                    block
                } else if let Some(empty) = empty_array_line(&lines) {
                    let mut out = String::new();
                    for (i, line) in lines.iter().enumerate() {
                        if i == empty {
                            out.push_str(&block);
                        } else {
                            out.push_str(line);
                        }
                    }
                    out
                } else {
                    let mut out = String::from(existing.trim_end());
                    if !out.is_empty() {
                        out.push_str("\n\n");
                    }
                    out.push_str(&block);
                    out
                }
            }
        };
        atomic_write(path, text.as_bytes())
    }
}
