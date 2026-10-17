//! Replay only production geometry repairs on an existing canonical document.
//! Usage: replay_geometry [--] input.op new-output.op

use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use op_editor_core::{EditorCommand, EditorState, Locale, PenNodeExt};
use op_host_services::doc_io;
use op_orchestrator::types::DocSink;
use serde_json::json;
use sha2::{Digest, Sha256};

struct ReplaySink<'a> {
    state: &'a mut EditorState,
}

impl DocSink for ReplaySink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }

    fn apply(&mut self, command: EditorCommand) -> bool {
        self.state.apply(command)
    }

    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--") {
        args.remove(0);
    }
    if args.len() != 2 {
        return Err(
            std::io::Error::other("usage: replay_geometry [--] input.op new-output.op").into(),
        );
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    if std::fs::symlink_metadata(&output).is_ok() {
        return Err(std::io::Error::other("refusing to overwrite an existing output").into());
    }
    let input_bytes = std::fs::read(&input)?;
    let loaded = doc_io::load_editor_state_with_report(&input, Locale::EnUs)?;
    if loaded.report.rewrite_blocked_by_schema_warning {
        return Err(std::io::Error::other("document schema cannot safely be rewritten").into());
    }
    if std::fs::read(&input)? != input_bytes {
        return Err(std::io::Error::other("input changed while loading").into());
    }
    let mut state = loaded.state;
    state.validate()?;
    op_host_native::bundled_design_fonts::register();
    let active_page = state.ui.active_page_index;
    let mut roots = Vec::new();
    for page_index in 0..state.page_count() {
        if !state.set_active_page(page_index) && state.ui.active_page_index != page_index {
            return Err(std::io::Error::other("cannot activate document page").into());
        }
        let ids: Vec<_> = state
            .active_children()
            .iter()
            .map(|node| node.id_str().to_owned())
            .collect();
        let mut sink = ReplaySink { state: &mut state };
        for id in ids {
            let rounds =
                op_orchestrator::geometry_validation::geometry_validate_and_fix(&mut sink, &id);
            roots.push(json!({"page_index": page_index, "root_id": id, "repair_rounds": rounds}));
        }
    }
    if roots.is_empty() {
        return Err(std::io::Error::other("input contains no roots to replay").into());
    }
    let _ = state.set_active_page(active_page);
    state.validate()?;
    let output_bytes = doc_io::canonical_document_bytes(
        &state.doc,
        op_pen_loader::EditorMeta::from_state(&state),
    )?;
    // The canonical serializer retains images, thumbnails and editor metadata.
    // create_new also refuses a pre-existing dangling symlink at the destination.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(&output_bytes)?;
    file.sync_all()?;
    println!(
        "[REPLAY] {}",
        json!({
            "kind": "production-geometry-replay",
            "input": input,
            "output": output,
            "input_sha256": format!("{:x}", Sha256::digest(&input_bytes)),
            "output_sha256": format!("{:x}", Sha256::digest(&output_bytes)),
            "roots": roots,
            "active_page_index": active_page,
            "model_invocations": 0,
            "image_search_invocations": 0,
            "manual_design_edits": false,
            "visual_acceptance": "pending-inspection",
            "note": "A successful geometry replay or zero repair rounds is not visual acceptance."
        })
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("replay_geometry: {error}");
            ExitCode::FAILURE
        }
    }
}
