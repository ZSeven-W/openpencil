//! Allocation-bounded Save As for an unchanged, already-bound `.op` file.

use super::{commit_staged_document, create_sibling_temp, DocIoError};
use op_pen_loader::EditorMeta;
use std::path::Path;

/// Save As for a clean canonical document without cloning its in-memory
/// [`jian_ops_schema::PenDocument`].
///
/// The source file is mapped read-only and copied into the normal sibling
/// temporary file while only the top-level `editorMeta` value is replaced.
/// This preserves old/future schema fields, image tables, and all authored JSON
/// values byte-for-byte. Callers must only select this path when the live
/// document is still at its saved revision; dirty documents require a normal
/// typed snapshot.
pub fn copy_clean_document_with_editor_meta_to_path(
    source_path: &Path,
    target_path: &Path,
    meta: EditorMeta,
) -> Result<(), DocIoError> {
    let source_file = std::fs::File::open(source_path).map_err(|error| DocIoError::Open {
        path: source_path.display().to_string(),
        detail: error.to_string(),
    })?;
    if source_file
        .metadata()
        .map_err(|error| DocIoError::Stat {
            path: source_path.display().to_string(),
            detail: error.to_string(),
        })?
        .len()
        == 0
    {
        return Err(DocIoError::SourceEmpty {
            path: source_path.display().to_string(),
        });
    }
    // SAFETY: The source mapping is read-only and remains alive until the
    // sibling-temp write completes. OpenPencil replaces files atomically
    // instead of mutating mapped inodes in place.
    let source_bytes =
        unsafe { memmap2::MmapOptions::new().map(&source_file) }.map_err(|error| {
            DocIoError::Map {
                path: source_path.display().to_string(),
                detail: error.to_string(),
            }
        })?;
    let source =
        std::str::from_utf8(source_bytes.as_ref()).map_err(|error| DocIoError::SourceNotUtf8 {
            path: source_path.display().to_string(),
            detail: error.to_string(),
        })?;

    let (tmp, file) = create_sibling_temp(target_path)?;
    let write_result = (|| {
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
        // `op_pen_loader::EditorMetaWriteError` belongs to a crate this pass
        // does not own; render it with `to_string` so the adapter survives if
        // that crate reshapes its error.
        op_pen_loader::write_source_with_editor_meta(&mut writer, source, meta)
            .map_err(|error| DocIoError::Serialize(error.to_string()))?;
        std::io::Write::flush(&mut writer).map_err(|error| DocIoError::Io(error.to_string()))?;
        drop(writer);
        commit_staged_document(&tmp, target_path)
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

/// Rewrite a successfully loaded legacy source in current schema form without
/// cloning the live typed document. Known wire repairs use one raw JSON DOM so
/// nested unknown fields survive; metadata-only migrations stay streaming and
/// allocation-bounded.
pub fn copy_document_to_current_schema_path(
    source_path: &Path,
    target_path: &Path,
    meta: EditorMeta,
    normalize_legacy: bool,
) -> Result<(), DocIoError> {
    let source_file = std::fs::File::open(source_path).map_err(|error| DocIoError::Open {
        path: source_path.display().to_string(),
        detail: error.to_string(),
    })?;
    if source_file
        .metadata()
        .map_err(|error| DocIoError::Stat {
            path: source_path.display().to_string(),
            detail: error.to_string(),
        })?
        .len()
        == 0
    {
        return Err(DocIoError::SourceEmpty {
            path: source_path.display().to_string(),
        });
    }
    // SAFETY: The read-only map pins the source inode through the complete
    // sibling write; publication happens separately after fingerprint review.
    let source_bytes =
        unsafe { memmap2::MmapOptions::new().map(&source_file) }.map_err(|error| {
            DocIoError::Map {
                path: source_path.display().to_string(),
                detail: error.to_string(),
            }
        })?;
    let source =
        std::str::from_utf8(source_bytes.as_ref()).map_err(|error| DocIoError::SourceNotUtf8 {
            path: source_path.display().to_string(),
            detail: error.to_string(),
        })?;
    let (tmp, file) = create_sibling_temp(target_path)?;
    let write_result = (|| {
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
        // Same unowned-`op_pen_loader` seam as above.
        if normalize_legacy {
            op_pen_loader::write_normalized_source_with_current_schema(&mut writer, source, meta)
                .map_err(|error| DocIoError::Serialize(error.to_string()))?;
        } else {
            op_pen_loader::write_source_with_current_schema(&mut writer, source, meta)
                .map_err(|error| DocIoError::Serialize(error.to_string()))?;
        }
        std::io::Write::flush(&mut writer).map_err(|error| DocIoError::Io(error.to_string()))?;
        drop(writer);
        commit_staged_document(&tmp, target_path)
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc_io::sidecar_path;
    use std::path::PathBuf;

    fn temp_op_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "openpencil-clean-copy-{tag}-{}-{}.op",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn preserves_schema_and_updates_only_editor_meta() {
        let source = temp_op_path("source");
        let target = temp_op_path("target");
        let source_json = concat!(
            "{\n",
            "  \"version\":\"0.8.0\",\n",
            "  \"futureExtension\":{\"mustSurvive\":true},\n",
            "  \"children\":[{\"type\":\"rectangle\",\"id\":\"legacy\",\"width\":10}],\n",
            "  \"editorMeta\":{\"activePageIndex\":0,\"preserveAuthoredGeometry\":false}\n",
            "}\n"
        );
        std::fs::write(&source, source_json).expect("write clean bound source");
        std::fs::write(sidecar_path(&target), r#"{"active_page_index":0}"#)
            .expect("write stale target sidecar");

        copy_clean_document_with_editor_meta_to_path(
            &source,
            &target,
            EditorMeta {
                active_page_index: 4,
                preserve_authored_geometry: true,
                scenario: None,
                pinned_style_guide: None,
                share_recipe: None,
                imported_from: None,
            },
        )
        .expect("clean copy save");

        let target_json = std::fs::read_to_string(&target).expect("read target");
        let parsed: serde_json::Value =
            serde_json::from_str(&target_json).expect("target stays valid JSON");
        assert_eq!(parsed["version"], "0.8.0");
        assert_eq!(parsed["futureExtension"]["mustSurvive"], true);
        assert_eq!(parsed["children"][0]["id"], "legacy");
        assert_eq!(parsed["editorMeta"]["activePageIndex"], 4);
        assert_eq!(parsed["editorMeta"]["preserveAuthoredGeometry"], true);
        assert!(
            !sidecar_path(&target).exists(),
            "the normal commit path removes stale metadata sidecars"
        );

        let source_without_meta = source_json.replace(
            ",\n  \"editorMeta\":{\"activePageIndex\":0,\"preserveAuthoredGeometry\":false}",
            "",
        );
        std::fs::write(&source, source_without_meta).expect("rewrite source without metadata");
        copy_clean_document_with_editor_meta_to_path(
            &source,
            &target,
            EditorMeta {
                active_page_index: 2,
                preserve_authored_geometry: false,
                scenario: None,
                pinned_style_guide: None,
                share_recipe: None,
                imported_from: None,
            },
        )
        .expect("clean copy appends metadata");
        let appended: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).expect("read metadata-appended target"))
                .expect("metadata-appended target stays valid");
        assert_eq!(appended["editorMeta"]["activePageIndex"], 2);
        assert_eq!(appended["editorMeta"]["preserveAuthoredGeometry"], false);

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(target);
    }
}
