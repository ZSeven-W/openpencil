//! DeepSeek Harness (dsh) MCP patch-file editing.
//!
//! DSH supports both home and profile patch layers. Profile discovery and
//! migration live in `mcp_integrations_dsh_profiles`. This module edits only
//! OpenPencil-owned marker blocks without reserializing the user's YAML.

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

/// The exact nine-line managed block, markers included. In the
/// top-level-array context this is valid YAML on its own. `url` is the
/// live server endpoint (`mcp_integrations::endpoint_url`).
fn dsh_patch_block(url: &str) -> String {
    format!(
        "{DSH_PATCH_BEGIN}\n- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: {url}\n{DSH_PATCH_END}\n"
    )
}

/// Reject incomplete, repeated, reordered, or modified marker pairs.
fn marker_pair(path: &Path, lines: &[&str]) -> Result<Option<(usize, usize)>, McpConfigError> {
    let begin: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("openpencil-mcp-begin"))
        .map(|(index, _)| index)
        .collect();
    let end: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("openpencil-mcp-end"))
        .map(|(index, _)| index)
        .collect();
    match (begin.as_slice(), end.as_slice()) {
        ([], []) => Ok(None),
        ([begin], [end])
            if begin < end
                && lines[*begin].trim() == DSH_PATCH_BEGIN
                && lines[*end].trim() == DSH_PATCH_END =>
        {
            Ok(Some((*begin, *end)))
        }
        _ => Err(McpConfigError::DshPatchMarkersMismatched {
            path: path.to_path_buf(),
        }),
    }
}

fn dsh_text_has_openpencil(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return false;
        }
        // Conservatively recognize block/flow YAML and quoted keys/values.
        // This is only a refusal guard, not a YAML parser or an edit recipe.
        let unquoted = line.replace(['\'', '"'], "");
        yaml_named_value(&unquoted, "id", "mcp-openpencil")
            || yaml_named_value(&unquoted, "serverName", "openpencil")
    })
}

fn yaml_named_value(mut line: &str, key: &str, value: &str) -> bool {
    while let Some((before, after)) = line.split_once(key) {
        let key_boundary = before
            .chars()
            .next_back()
            .is_none_or(|ch| ch.is_whitespace() || matches!(ch, '{' | '[' | ',' | '-'));
        if key_boundary {
            if let Some(after_colon) = after.trim_start().strip_prefix(':') {
                if let Some(tail) = after_colon.trim_start().strip_prefix(value) {
                    if tail
                        .chars()
                        .next()
                        .is_none_or(|ch| ch.is_whitespace() || matches!(ch, ',' | '}' | ']' | '#'))
                    {
                        return true;
                    }
                }
            }
        }
        line = after;
    }
    false
}

/// Whether the dsh patch layer already points at an OpenPencil server —
/// either our managed block or a hand-written entry (detection covers both
/// so a hand-wired dsh still lights the toggle up).
pub(crate) fn dsh_config_has_openpencil(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|text| dsh_text_has_openpencil(&text))
        .unwrap_or(false)
}

pub(crate) fn read_dsh_patch(path: &Path) -> Result<String, McpConfigError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(McpConfigError::Read {
                path: path.to_path_buf(),
                message: "refusing to replace a symbolic-link DSH patch file".into(),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(read_error(path, error)),
    }
    fs::read_to_string(path).map_err(|error| read_error(path, error))
}

fn read_error(path: &Path, error: io::Error) -> McpConfigError {
    McpConfigError::Read {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

fn ensure_patch_array(mut text: String) -> String {
    // Empty/comment-only YAML parses as null; DSH requires an array.
    if text
        .lines()
        .all(|line| line.trim().is_empty() || line.trim().starts_with('#'))
    {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str("[]\n");
    }
    text
}

fn empty_array_comment(line: &str) -> Option<&str> {
    let (inside, tail) = line.trim().strip_prefix('[')?.split_once(']')?;
    if !inside.trim().is_empty() {
        return None;
    }
    let tail = tail.trim();
    (tail.is_empty() || tail.starts_with('#')).then_some(tail)
}

/// Editing a block list cannot safely append to a flow collection or YAML
/// document stream. Refuse those forms without reserializing user content.
fn editable_array_root(path: &Path, lines: &[&str]) -> Result<Option<usize>, McpConfigError> {
    let meaningful: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect();
    if let [(index, line)] = meaningful.as_slice() {
        if empty_array_comment(line).is_some() {
            return Ok(Some(*index));
        }
    }
    let is_block_entry = |line: &str| {
        line.strip_prefix('-')
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    };
    let supported = meaningful
        .first()
        .is_none_or(|(_, line)| is_block_entry(line))
        && meaningful
            .iter()
            .all(|(_, line)| line.starts_with(char::is_whitespace) || is_block_entry(line));
    if !supported {
        return Err(McpConfigError::Parse {
            path: path.to_path_buf(),
            message: "automatic DSH integration requires a block-style YAML patch list or an empty array; convert this layer or configure OpenPencil manually".into(),
        });
    }
    Ok(None)
}

/// Prepare edits before any file in the multi-profile transaction changes.
/// Never override manual entries, even when they sit beside our own block.
pub(crate) fn render_dsh_patch(
    path: &Path,
    existing: &str,
    enabled: bool,
    url: &str,
) -> Result<String, McpConfigError> {
    let lines: Vec<&str> = existing.split_inclusive('\n').collect();
    let pair = marker_pair(path, &lines)?;
    let unmanaged: String = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| pair.is_none_or(|(begin, end)| *index < begin || *index > end))
        .map(|(_, line)| *line)
        .collect();
    if dsh_text_has_openpencil(&unmanaged) {
        return Err(McpConfigError::DshManualEntry {
            path: path.to_path_buf(),
        });
    }
    let empty_array = editable_array_root(path, &lines)?;
    if let Some((begin, end)) = pair {
        let mut out = String::new();
        for (index, line) in lines.iter().enumerate() {
            if index < begin || index > end {
                out.push_str(line);
            } else if index == begin && enabled {
                out.push_str(&dsh_patch_block(url));
            }
        }
        return Ok(if enabled {
            out
        } else {
            ensure_patch_array(out)
        });
    }
    if !enabled {
        return Ok(ensure_patch_array(existing.to_owned()));
    }
    let block = dsh_patch_block(url);
    if let Some(empty) = empty_array {
        let mut out = String::new();
        for (index, line) in lines.iter().enumerate() {
            if index == empty {
                if let Some(comment) = empty_array_comment(line).filter(|value| !value.is_empty()) {
                    out.push_str(comment);
                    out.push('\n');
                }
                out.push_str(&block);
            } else {
                out.push_str(line);
            }
        }
        return Ok(out);
    }
    let mut out = existing.to_owned();
    if !out.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&block);
    Ok(out)
}

pub(crate) fn update_dsh_patch_config(
    path: &Path,
    enabled: bool,
    url: &str,
) -> Result<(), McpConfigError> {
    let existing = read_dsh_patch(path)?;
    if !enabled && !path.exists() {
        return Ok(());
    }
    let rendered = render_dsh_patch(path, &existing, enabled, url)?;
    if rendered != existing {
        atomic_write(path, rendered.as_bytes())?;
    }
    Ok(())
}
