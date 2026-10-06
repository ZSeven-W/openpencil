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

use op_editor_ui::layout_scene::{stable_image_source_id, SceneNode};
use op_editor_ui::widgets::canvas_viewport_image::{
    has_cached_image_bytes, has_pending_remote_image_requests, note_remote_image_miss,
};
use op_host_services::export::{export_node_raster_with_margin, RasterFormat};
use std::time::{Duration, Instant};

use crate::render_cli_error::RenderCliError;

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
    // Transparent border (doc px) around each node. Defaults to a tight
    // crop, the same as the editor's own raster export (its frame was
    // removed; `op_render_export::MARGIN` is 0): a 375-wide phone screen
    // must come out 375 px wide, not 407. `OPENPENCIL_RENDER_MARGIN=<px>`
    // opts back into a frame for eyeballing.
    let margin: f32 = std::env::var("OPENPENCIL_RENDER_MARGIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let out_dir = PathBuf::from(out_dir);
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("render-shots: mkdir {}: {e}", out_dir.display());
        std::process::exit(1);
    }

    let state = match load_render_state(file) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    // Register bundled design fonts before the layout/measure + paint
    // pass so `fit_content` heights and glyphs match Pencil even when the
    // family isn't installed system-wide.
    crate::bundled_fonts::register();
    // Imported fonts too, so headless render-shots / export match the editor
    // canvas for any family the user imported. Best-effort.
    match crate::fonts::FontStore::user() {
        Ok(store) => store.rescan_and_register(),
        Err(err) => eprintln!("render-shots: skipping imported-font rescan: {err}"),
    }
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&state);

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

    prefetch_remote_images(&page.children);
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

fn load_render_state(file: &str) -> Result<op_editor_core::EditorState, RenderCliError> {
    // `doc_io` belongs to `op-host-services`, a crate this pass does not own;
    // carry its message inside the `render-shots:` envelope.
    op_host_services::doc_io::load_editor_state(
        std::path::Path::new(file),
        op_editor_core::Locale::EnUs,
    )
    .map_err(|e| RenderCliError::LoadDocument {
        file: file.to_string(),
        message: e.to_string(),
    })
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

/// Longest a headless render waits for remote image bytes before it
/// exports with whatever landed (the rest paint as placeholders).
const REMOTE_IMAGE_WARMUP: Duration = Duration::from_secs(45);

/// Warm the painter's byte cache with every remote (`http(s)`) image
/// source in the page before exporting.
///
/// The platform-free painter only RECORDS a remote miss and paints the
/// dashed placeholder; the desktop app drains those misses through
/// `RemoteImageSession` on its event loop. A headless render has no
/// loop, so without this pass every image an enrichment step just
/// resolved came out as a grey box in the shot. Records the misses the
/// way paint would (same stable id), pumps the same fetcher until the
/// queue drains or the deadline passes, then lets the export proceed.
/// `OPENPENCIL_RENDER_NO_FETCH=1` skips it for offline renders.
fn prefetch_remote_images(nodes: &[SceneNode]) {
    if std::env::var("OPENPENCIL_RENDER_NO_FETCH").is_ok() {
        return;
    }
    fn collect(node: &SceneNode, out: &mut Vec<(u64, String)>) {
        if let Some(src) = node.image_src.as_deref() {
            let lower = src.get(..8).unwrap_or(src).to_ascii_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                let id = if node.image_src_id == 0 {
                    stable_image_source_id(src)
                } else {
                    node.image_src_id
                };
                out.push((id, src.to_owned()));
            }
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    let mut sources = Vec::new();
    for node in nodes {
        collect(node, &mut sources);
    }
    sources.retain(|(id, _)| !has_cached_image_bytes(*id));
    if sources.is_empty() {
        return;
    }
    let wanted = sources.len();
    for (id, url) in &sources {
        note_remote_image_miss(*id, url);
    }
    let mut session = crate::remote_image_host::RemoteImageSession::new();
    let deadline = Instant::now() + REMOTE_IMAGE_WARMUP;
    while session.is_pending() || has_pending_remote_image_requests() {
        session.pump();
        if Instant::now() >= deadline {
            eprintln!("render-shots: remote image warm-up timed out; exporting with placeholders");
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let landed = sources
        .iter()
        .filter(|(id, _)| has_cached_image_bytes(*id))
        .count();
    eprintln!("render-shots: remote images {landed}/{wanted} fetched");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_render_state_accepts_future_version_and_restores_editor_meta() {
        let path = std::env::temp_dir().join(format!(
            "openpencil-render-shots-future-version-{}.op",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r##"{
  "version": "2.8",
  "editorMeta": {
    "activePageIndex": 0,
    "preserveAuthoredGeometry": true
  },
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
        )
        .expect("write future-version fixture");
        let state = load_render_state(path.to_str().expect("UTF-8 temp path"))
            .expect("render-shots loader should accept TS future-version files");

        match &state.doc.children[0] {
            jian_ops_schema::node::PenNode::Frame(frame) => {
                assert_eq!(frame.base.id, "future-root");
            }
            other => panic!("expected frame root, got {other:?}"),
        }
        assert!(
            state.editor_ui.preserve_authored_geometry,
            "render-shots must use the same reopen geometry mode as the editor"
        );
        let _ = std::fs::remove_file(path);
    }
}
