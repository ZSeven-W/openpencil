//! Batch frame export — the render + IO loop behind File ▸ "Export all
//! frames".
//!
//! This is a loop wrapped around the EXISTING single-frame exporter, not
//! a second renderer: the scene is built by the same
//! `op_pen_loader::editor_state_to_active_page_layout_scene` the File ▸
//! Export image path uses, and each frame goes through the untouched
//! [`export::export_node_raster`]. Exporting a deck therefore produces
//! byte-identical images to selecting each frame and exporting it by
//! hand — only the file naming and the loop live here.
//!
//! Which frames get exported (and under which names) is decided
//! upstream by `op_editor_core::export_batch`; this module only renders
//! what it is handed and aggregates the outcome, so one bad frame never
//! aborts the rest of the run.

use op_editor_core::export_batch::FrameExportTarget;
use op_editor_core::EditorState;
use std::path::{Path, PathBuf};

/// Batch export always writes PNG. The export dialog's format applies
/// to the single-frame path only — a deck of screenshots wants lossless
/// alpha, and PDF/SVG have page-level (not per-frame) semantics here.
pub const BATCH_EXTENSION: &str = "png";

/// Outcome of one batch run: which files landed and which frames
/// failed, so the host can report both in a single dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatchExportReport {
    /// File names written, in export order.
    pub written: Vec<String>,
    /// `(file name, error detail)` for frames that failed to render.
    pub failed: Vec<(String, String)>,
}

impl BatchExportReport {
    /// Frames attempted (written + failed).
    pub fn attempted(&self) -> usize {
        self.written.len() + self.failed.len()
    }

    /// True when every attempted frame was written.
    pub fn all_written(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Render each planned frame into `directory` at the editor's current
/// export scale. Never short-circuits: a frame that fails is recorded
/// and the loop moves on to the next one.
pub fn export_frames_to_dir(
    state: &EditorState,
    directory: &Path,
    targets: &[FrameExportTarget],
) -> BatchExportReport {
    let mut report = BatchExportReport::default();
    if targets.is_empty() {
        return report;
    }
    // One scene for the whole batch — the layout pass is the expensive
    // part and it is identical for every frame on the page.
    let scene = op_pen_loader::editor_state_to_active_page_export_layout_scene(state);
    let scale = state.editor_ui.export_scale;
    for target in targets {
        let path: PathBuf = directory.join(&target.file_name);
        match crate::export::export_node_raster(
            &scene,
            target.node_id.as_str(),
            &path,
            crate::export::RasterFormat::Png,
            scale,
        ) {
            Ok(()) => report.written.push(target.file_name.clone()),
            Err(e) => report
                .failed
                .push((target.file_name.clone(), e.to_string())),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::export_batch::plan_frame_exports;
    use op_editor_core::node_id::NodeId;

    fn temp_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!(
            "openpencil-batch-export-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).expect("temp dir");
        p
    }

    fn deck_state() -> EditorState {
        let doc = jian_ops_schema::load_str(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"封面","x":0,"y":0,"width":40,"height":40,
                 "fill":[{"type":"solid","color":"#ff0000"}]},
                {"type":"frame","id":"f2","name":"步骤 1","x":100,"y":0,"width":40,"height":40,
                 "fill":[{"type":"solid","color":"#00ff00"}]}
            ]}"##,
        )
        .expect("fixture JSON parses")
        .value;
        EditorState::from_document(doc)
    }

    #[test]
    fn every_planned_frame_lands_as_its_own_file() {
        let state = deck_state();
        let dir = temp_dir("all-frames");
        let targets = plan_frame_exports(&state, BATCH_EXTENSION);

        let report = export_frames_to_dir(&state, &dir, &targets);

        assert!(report.all_written(), "unexpected failures: {report:?}");
        assert_eq!(report.written, vec!["01-封面.png", "02-步骤 1.png"]);
        for name in &report.written {
            let bytes = std::fs::read(dir.join(name)).expect("export file exists");
            assert!(bytes.starts_with(b"\x89PNG"), "{name} is not a PNG");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failing_frame_is_reported_without_stopping_the_batch() {
        let state = deck_state();
        let dir = temp_dir("partial-failure");
        let mut targets = plan_frame_exports(&state, BATCH_EXTENSION);
        // Splice a target the scene cannot resolve between two good
        // ones — the run must still write both neighbours.
        targets.insert(
            1,
            FrameExportTarget {
                node_id: NodeId::new("does-not-exist"),
                file_name: "02-ghost.png".to_string(),
            },
        );

        let report = export_frames_to_dir(&state, &dir, &targets);

        assert_eq!(report.attempted(), 3);
        assert_eq!(report.written.len(), 2);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "02-ghost.png");
        assert!(!report.all_written());
        assert!(!dir.join("02-ghost.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_plan_writes_nothing() {
        let state = deck_state();
        let dir = temp_dir("empty-plan");
        let report = export_frames_to_dir(&state, &dir, &[]);
        assert_eq!(report.attempted(), 0);
        assert!(report.all_written());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
