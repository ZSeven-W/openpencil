//! Native loading for the process-wide saved scene-template registry.
//!
//! Saved templates live under `~/.openpencil/templates/<slug>/` as a
//! canonical `document.op`, a `meta.json`, and an optional `preview.jpg`.
//! The registry itself belongs to `op-editor-core` so every host can resolve
//! a `user:` id, but reading the native user's directory belongs here: this
//! crate is shared by the desktop GUI, stdio/HTTP MCP, and serve-web without
//! pulling a windowing dependency into either headless binary.

use std::path::{Path, PathBuf};

use op_editor_core::user_scene_templates::{
    set_user_scene_templates, UserSceneTemplate, USER_TEMPLATE_ID_PREFIX,
};

const TEMPLATES_DIR: &str = "templates";

#[cfg(test)]
fn guard_user_config() {
    let root = std::env::temp_dir().join(format!(
        "op-host-services-test-config-{}",
        std::process::id()
    ));
    op_config_store::redirect_user_root_for_tests(root);
}

#[cfg(not(test))]
#[inline]
fn guard_user_config() {}

/// Resolve the standard saved-template directory, creating it on first use.
pub fn user_scene_templates_dir() -> std::io::Result<PathBuf> {
    // A unit test that reaches this shared native entry point must never scan
    // or create the developer's real collection. Desktop tests install their
    // own redirect before calling across the crate boundary; service tests use
    // this equivalent process-level guard.
    guard_user_config();
    let dir = op_config_store::openpencil_dir()?.join(TEMPLATES_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Fill the shared registry from disk once for this native host process.
///
/// A failed scan is non-fatal startup work: MCP and the editor can still use
/// the shipped catalogue, while the diagnostic says why saved entries are
/// absent. Process-global once semantics also prevent constructing a second
/// GUI state from discarding templates saved since the first scan.
pub fn initialize_user_scene_templates_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Err(error) = reload_user_scene_templates() {
            eprintln!("[templates] could not scan saved templates: {error}");
        }
    });
}

/// Replace the shared registry with the current standard-directory snapshot.
///
/// This is public for native persistence tests and explicit recovery flows;
/// ordinary host startup should use [`initialize_user_scene_templates_once`].
pub fn reload_user_scene_templates() -> std::io::Result<usize> {
    let templates = read_user_scene_templates(&user_scene_templates_dir()?)?;
    let count = templates.len();
    set_user_scene_templates(templates);
    Ok(count)
}

fn read_user_scene_templates(dir: &Path) -> std::io::Result<Vec<UserSceneTemplate>> {
    let entries = std::fs::read_dir(dir)?;
    let mut templates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(slug) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(meta) = read_meta(&path.join("meta.json")) else {
            eprintln!(
                "[templates] {}: no readable meta.json — skipped",
                path.display()
            );
            continue;
        };
        let document = match std::fs::read_to_string(path.join("document.op")) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("[templates] {}: {error}", path.display());
                continue;
            }
        };
        templates.push(UserSceneTemplate {
            id: format!("{USER_TEMPLATE_ID_PREFIX}{slug}"),
            name: meta.name,
            frames: meta.frames,
            frame_width: meta.frame_width,
            frame_height: meta.frame_height,
            document,
            // A missing or unreadable preview is not a broken template; the
            // card paints its normal empty-preview block instead.
            preview_jpeg: std::fs::read(path.join("preview.jpg")).unwrap_or_default(),
        });
    }
    // Directory iteration order is platform-specific. Stable ids produce a
    // stable MCP list and card order after every process restart.
    templates.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(templates)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateMeta {
    name: String,
    frames: u16,
    frame_width: u32,
    frame_height: u32,
}

fn read_meta(path: &Path) -> Option<TemplateMeta> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Encode the metadata sidecar in the same format the shared scanner reads.
pub fn encode_user_scene_template_meta(template: &UserSceneTemplate) -> std::io::Result<Vec<u8>> {
    serde_json::to_vec_pretty(&TemplateMeta {
        name: template.name.clone(),
        frames: template.frames,
        frame_width: template.frame_width,
        frame_height: template.frame_height,
    })
    .map_err(std::io::Error::other)
}
