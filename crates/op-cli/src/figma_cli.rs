use std::path::Path;

use serde_json::json;

use super::{flag_value, required_pos, Command, Flags};

pub(super) fn map_import_figma(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let fig_path = required_pos(
        positionals,
        1,
        "Usage: op import:figma <file.fig> [--out output.op]",
    )?;
    let out_path = flag_value(flags, "out").unwrap_or_else(|| figma_default_out_path(&fig_path));
    Ok(Command::ImportFigma { fig_path, out_path })
}

pub(super) fn figma_default_out_path(fig_path: &str) -> String {
    fig_path
        .strip_suffix(".fig")
        .map(|base| format!("{base}.op"))
        .unwrap_or_else(|| fig_path.to_string())
}

pub(super) fn run_import_figma(fig_path: &str, out_path: &str) -> Result<String, String> {
    let bytes = std::fs::read(fig_path).map_err(|e| format!("read {fig_path:?}: {e}"))?;
    let file_name = Path::new(fig_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Figma Import");
    let import = op_figma::parse_fig_binary(&bytes, file_name, op_figma::FigLayoutMode::OpenPencil)
        .map_err(|e| format!("import {fig_path:?}: {e}"))?;
    let raw = serde_json::to_string(&import.document)
        .map_err(|e| format!("serialize {out_path:?}: {e}"))?;
    std::fs::write(out_path, raw).map_err(|e| format!("write {out_path:?}: {e}"))?;
    let page_count = import.document.pages.as_ref().map_or(1, Vec::len);
    let node_count = import
        .document
        .pages
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.children.len()).sum())
        .unwrap_or_else(|| import.document.children.len());
    Ok(import_figma_success_json(
        out_path,
        page_count,
        node_count,
        &import.warnings,
    ))
}

fn import_figma_success_json(
    file_path: &str,
    page_count: usize,
    node_count: usize,
    warnings: &[String],
) -> String {
    json!({
        "ok": true,
        "filePath": file_path,
        "pageCount": page_count,
        "nodeCount": node_count,
        "warnings": warnings,
    })
    .to_string()
}
