//! Hidden `--render-shots` headless raster mode.
//!
//! `openpencil-desktop --render-shots <file.op> <out_dir> [scale]`
//! loads a canonical `.op`, runs the SAME jian layout pass the editor
//! canvas uses (`op_pen_loader::editor_state_to_layout_scene`), and
//! writes one cropped PNG per top-level node on the active page via
//! [`op_host_services::export::export_node_raster`] — node-only screenshots (no
//! editor chrome) for the model-design benchmark. No GUI / GL window,
//! so it runs headless in CI / scripts.

use std::path::PathBuf;

use op_host_services::export::{export_node_raster_with_margin, RasterFormat};

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
    // Transparent border (doc px) around each node. Defaults to the
    // editor export frame (16 px); `OPENPENCIL_RENDER_MARGIN=0` gives a
    // tight crop so the render-parity benchmark matches Pencil's
    // `export_nodes`, which adds no frame (otherwise every node reads as
    // +2*margin wider/taller than its baseline).
    let margin: f32 = std::env::var("OPENPENCIL_RENDER_MARGIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16.0);
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
    let loaded = match load_render_document(&src, file) {
        Ok(l) => l,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };
    for warning in &loaded.warnings {
        eprintln!("render-shots: schema warning: {warning:?}");
    }
    // Register bundled design fonts before the layout/measure + paint
    // pass so `fit_content` heights and glyphs match Pencil even when the
    // family isn't installed system-wide.
    crate::bundled_fonts::register();
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

    // Diagnostic: dump every node's computed rect as JSONL so we can diff our
    // layout against Pencil's `snapshot_layout` element-for-element (pins down
    // exactly which element heights / gaps drift, vs inferring from pixels).
    if std::env::var("OPENPENCIL_DUMP_LAYOUT").is_ok() {
        for node in &page.children {
            dump_layout_node(node);
        }
        return true;
    }

    let total = page.children.len();
    let mut ok = 0usize;
    for node in &page.children {
        let target = out_dir.join(format!("{}.png", sanitize(&node.id)));
        match export_node_raster_with_margin(
            &scene,
            &node.id,
            &target,
            RasterFormat::Png,
            scale,
            margin,
        ) {
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

fn load_render_document(
    src: &str,
    file: &str,
) -> Result<jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>, String> {
    op_pen_loader::load_canonical(src).map_err(|e| format!("render-shots: parse {file}: {e}"))
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

/// Recursively print a scene node's computed rect as one JSON line, for the
/// `OPENPENCIL_DUMP_LAYOUT` diagnostic (diff vs Pencil `snapshot_layout`).
fn dump_layout_node(node: &op_editor_ui::layout_scene::SceneNode) {
    let b = node.bounds;
    println!(
        "{{\"id\":\"{}\",\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1}}}",
        node.id, b.origin.x, b.origin.y, b.size.x, b.size.y
    );
    for c in &node.children {
        dump_layout_node(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_render_document_accepts_ts_future_version_files() {
        let loaded = load_render_document(
            r##"{
  "version": "2.8",
  "children": [
    {
      "id": "future-root",
      "type": "frame",
      "name": "Future Root",
      "x": 0,
      "y": 0,
      "width": 120,
      "height": 80,
      "fill": [{ "type": "solid", "color": "#FFFFFF" }],
      "children": []
    }
  ]
}"##,
            "future.op",
        )
        .expect("render-shots loader should accept TS future-version files");

        match &loaded.value.children[0] {
            jian_ops_schema::node::PenNode::Frame(frame) => {
                assert_eq!(frame.base.id, "future-root");
            }
            other => panic!("expected frame root, got {other:?}"),
        }
    }
}
