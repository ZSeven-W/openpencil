//! Hidden `--render-shots` headless raster mode.
//!
//! `openpencil-desktop --render-shots <file.op> <out_dir> [scale]`
//! loads a canonical `.op`, runs the SAME jian layout pass the editor
//! canvas uses (`op_pen_loader::editor_state_to_layout_scene`), and
//! writes one cropped PNG per top-level node on the active page via
//! [`crate::export::export_node_raster`] — node-only screenshots (no
//! editor chrome) for the model-design benchmark. No GUI / GL window,
//! so it runs headless in CI / scripts.

use std::path::PathBuf;

use crate::export::{export_node_raster, RasterFormat};

/// When `--render-shots` is present on argv, render node PNGs and return
/// `true` so `main` exits 0; otherwise return `false` and let the normal
/// GUI path continue. Any failure (bad args, unreadable / unparsable
/// `.op`, no nodes, or a node that fails to render) exits non-zero — a
/// benchmark driver must not read success when no PNG was produced.
pub fn run_cli_if_requested() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let Some(pos) = args.iter().position(|a| a == "--render-shots") else {
        return false;
    };
    let (Some(file), Some(out_dir)) = (args.get(pos + 1), args.get(pos + 2)) else {
        eprintln!("usage: openpencil-desktop --render-shots <file.op> <out_dir> [scale]");
        std::process::exit(2);
    };
    let scale: f32 = args
        .get(pos + 3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2.0);
    let out_dir = PathBuf::from(out_dir);
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("render-shots: mkdir {}: {e}", out_dir.display());
        std::process::exit(1);
    }

    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("render-shots: read {file}: {e}");
            std::process::exit(1);
        }
    };
    let loaded = match jian_ops_schema::load_str(&src) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("render-shots: parse {file}: {e}");
            std::process::exit(1);
        }
    };
    let state = op_editor_core::EditorState::from_document(loaded.value);
    let scene = op_pen_loader::editor_state_to_layout_scene(&state);

    let Some(page) = scene.active_page() else {
        eprintln!("render-shots: no active page");
        std::process::exit(1);
    };
    if page.children.is_empty() {
        eprintln!("render-shots: active page has no nodes");
        std::process::exit(1);
    }

    let total = page.children.len();
    let mut ok = 0usize;
    for node in &page.children {
        let target = out_dir.join(format!("{}.png", sanitize(&node.id)));
        match export_node_raster(&scene, &node.id, &target, RasterFormat::Png, scale) {
            Ok(()) => {
                ok += 1;
                eprintln!("render-shots: wrote {}", target.display());
            }
            Err(e) => eprintln!("render-shots: node {} skipped: {e}", node.id),
        }
    }
    eprintln!("render-shots: {ok}/{total} node(s) → {}", out_dir.display());
    if ok < total {
        // A benchmark driver must see a non-zero exit when any node failed
        // to render, not a silent success.
        std::process::exit(1);
    }
    true
}

/// Filesystem-safe filename from a node id (ids are short slugs like
/// `n1`, but guard against odd characters just in case).
fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
