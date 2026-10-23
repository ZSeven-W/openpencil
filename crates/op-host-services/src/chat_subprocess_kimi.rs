//! Kimi Code CLI (`kimi`) as a text-only generation transport.
//!
//! `kimi -p <prompt> --output-format stream-json` runs one prompt and prints
//! one JSON object per line: `{"role":"meta",…}` bookkeeping, then one
//! `{"role":"assistant","content":…}` per assistant message (whole messages,
//! not token deltas; thinking is never written). Errors go to stderr with a
//! non-zero exit, which the shared exit path already reports.
//!
//! Prompt mode runs tool calls under the `auto` permission policy without
//! asking anyone, and the CLI always loads the user's global MCP servers —
//! for this user that is their live OpenPencil editor. A generation turn
//! must only return text, so every turn selects an agent whose `tools: []`
//! hides and blocks all tools, MCP included.
//!
//! Every turn runs in one fixed, empty working directory: the CLI keys its
//! saved sessions by working directory and offers no ephemeral mode, so a
//! fresh directory per turn would scatter one session folder per LLM call
//! across the user's Kimi home, and a project directory would pull its
//! AGENTS.md into the system prompt.

use std::path::{Path, PathBuf};
use std::time::Duration;

use op_ai::chat_provider::ChatDelta;

/// Generation turns plan or build one section; the same wall clock as Codex.
pub const KIMI_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Prefix the CLI requires on model aliases (`kimi-code/k3.1-pro`).
const MODEL_ALIAS_PREFIX: &str = "kimi-code/";

const GENERATION_AGENT: &str = "---\n\
name: openpencil-generation\n\
description: Text-only design generation for OpenPencil.\n\
tools: []\n\
---\n\
Answer with text only. You have no tools; never try to read files, run \
commands or call MCP servers.\n";

fn generation_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("openpencil")
        .join("kimi-generation")
}

/// The fixed, empty working directory every generation turn runs in.
pub fn workspace_dir() -> PathBuf {
    generation_root().join("workspace")
}

fn agent_file() -> PathBuf {
    generation_root().join("generation-agent.md")
}

fn write_generation_files(agent: &Path, workspace: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(workspace)?;
    std::fs::write(agent, GENERATION_AGENT)
}

/// Base argv for a generation turn. Writes the agent file and workspace on
/// every call so a cleared cache directory never leaves a dangling path; a
/// failed write surfaces as the CLI's own "agent file not found" error.
pub fn kimi_args() -> Vec<String> {
    let agent = agent_file();
    if let Err(error) = write_generation_files(&agent, &workspace_dir()) {
        tracing::warn!(%error, "could not write the Kimi generation agent");
    }
    args_for(&agent)
}

fn args_for(agent: &Path) -> Vec<String> {
    vec![
        "--output-format".into(),
        "stream-json".into(),
        "--agent-file".into(),
        agent.to_string_lossy().into_owned(),
    ]
}

/// `kimi-code/<model>` for a bare model id; qualified aliases pass through.
pub fn model_alias(model: &str) -> String {
    if model.contains('/') {
        model.to_string()
    } else {
        format!("{MODEL_ALIAS_PREFIX}{model}")
    }
}

/// One stream-json line. Assistant text becomes a text delta; meta, tool and
/// non-JSON lines carry nothing renderable.
pub fn parse_kimi_line(line: &str) -> Option<ChatDelta> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    if value.get("role").and_then(|role| role.as_str()) != Some("assistant") {
        return None;
    }
    let text = match value.get("content")? {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
            .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
            .collect(),
        _ => return None,
    };
    (!text.is_empty()).then_some(ChatDelta::TextDelta(text))
}

#[cfg(test)]
#[path = "chat_subprocess_kimi_tests.rs"]
mod tests;
