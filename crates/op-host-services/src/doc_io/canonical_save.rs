//! Allocation-bounded canonical `.op` writer.
//!
//! The typed document streams directly into the destination. A one-byte tail
//! adapter holds back the root `}` so the image tables collected by
//! `ImageSrc::serialize` and editor metadata can be appended without a
//! document-sized `Value` or final `String` allocation.

use super::DocIoError;
use jian_ops_schema::image_thumbs::ImageThumbSnapshot;
use jian_ops_schema::PenDocument;
use op_editor_core::{EditorState, SharedDoc};
use op_pen_loader::EditorMeta;
use serde::Serialize;
use std::io::Write;

/// Immutable document + view-state + thumbnail snapshot for a background save.
pub struct CanonicalSaveSnapshot {
    document: SharedDoc,
    editor_meta: EditorMeta,
    image_thumbnails: ImageThumbSnapshot,
}

impl CanonicalSaveSnapshot {
    /// Capture all save inputs while the live editor still names one document.
    pub fn capture(state: &EditorState) -> Self {
        Self::capture_reusing(state, None)
    }

    /// Capture a structurally-shared save snapshot.
    ///
    /// The most recent in-flight save is the best anchor when available. The
    /// adjacent undo snapshot is the fallback: normal edits record the state
    /// immediately before the edit, so all untouched top-level subtrees can
    /// be retained by `Arc` rather than deep-cloned on the UI thread.
    pub fn capture_reusing(state: &EditorState, previous: Option<&Self>) -> Self {
        let history_anchor = state
            .history
            .future
            .back()
            .or_else(|| state.history.past.back())
            .map(|snapshot| &snapshot.doc);
        let anchor = previous
            .map(|snapshot| &snapshot.document)
            .or(history_anchor);
        Self {
            document: SharedDoc::capture(&state.doc, anchor),
            editor_meta: EditorMeta::from_state(state),
            image_thumbnails: jian_ops_schema::image_thumbs::capture_snapshot(),
        }
    }

    pub(crate) fn document(&self) -> &SharedDoc {
        &self.document
    }

    /// The view state this snapshot will embed under `editorMeta`.
    pub fn editor_meta(&self) -> EditorMeta {
        self.editor_meta.clone()
    }

    pub(crate) fn image_thumbnails(&self) -> &ImageThumbSnapshot {
        &self.image_thumbnails
    }
}

/// Small diagnostics returned by the streaming writer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamingSaveStats {
    pub wrote_images_table: bool,
    pub wrote_image_thumbs: bool,
}

/// Stream a canonical document using a fresh snapshot of the active thumbnail
/// registry. Disk save jobs should prefer `CanonicalSaveSnapshot` so this
/// capture happens before the job leaves the UI thread.
pub fn write_canonical_document<W: Write>(
    writer: &mut W,
    document: &PenDocument,
    active_page_index: usize,
) -> Result<StreamingSaveStats, DocIoError> {
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();
    // Document-only entry point: with no `EditorState` in hand the active page
    // is the only truthful field, and every other one keeps its legacy default.
    write_canonical_document_with_thumbnails(
        writer,
        document,
        EditorMeta {
            active_page_index,
            ..EditorMeta::default()
        },
        &thumbnails,
    )
}

/// Serialize a canonical document with explicit editor metadata into
/// memory — the `.op` bytes a share page embeds. Same writer (and so the
/// same wire shape, image table included) as a save to disk.
pub fn canonical_document_bytes(
    document: &PenDocument,
    meta: EditorMeta,
) -> Result<Vec<u8>, DocIoError> {
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();
    let mut bytes = Vec::new();
    write_canonical_document_with_thumbnails(&mut bytes, document, meta, &thumbnails)?;
    Ok(bytes)
}

pub(super) fn write_canonical_document_with_thumbnails<W: Write>(
    writer: &mut W,
    document: &PenDocument,
    meta: EditorMeta,
    thumbnails: &ImageThumbSnapshot,
) -> Result<StreamingSaveStats, DocIoError> {
    write_serializable_document_with_thumbnails(writer, document, meta, thumbnails)
}

pub(super) fn write_serializable_document_with_thumbnails<
    W: Write,
    D: Serialize + jian_ops_schema::image_table::SaveImageOrder + ?Sized,
>(
    writer: &mut W,
    document: &D,
    meta: EditorMeta,
    thumbnails: &ImageThumbSnapshot,
) -> Result<StreamingSaveStats, DocIoError> {
    let stats = jian_ops_schema::image_table::write_document_with_extension(
        writer,
        document,
        thumbnails,
        "editorMeta",
        &meta,
    )
    // `jian_ops_schema` is not owned by this pass — carry its sentence
    // verbatim so the wire text is unchanged.
    .map_err(|e| DocIoError::Serialize(e.to_string()))?;
    Ok(StreamingSaveStats {
        wrote_images_table: stats.wrote_images_table,
        wrote_image_thumbs: stats.wrote_image_thumbs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn long_source(tag: &str) -> String {
        format!("data:image/png;base64,{tag}{}", "A".repeat(4_096))
    }

    fn loaded(source: &str) -> PenDocument {
        jian_ops_schema::load_str(source)
            .expect("canonical fixture")
            .value
    }

    fn stream(document: &PenDocument, page: usize) -> (String, StreamingSaveStats) {
        let mut bytes = Vec::new();
        let stats = write_canonical_document(&mut bytes, document, page).expect("stream save");
        (String::from_utf8(bytes).expect("UTF-8 JSON"), stats)
    }

    fn legacy_value(document: &PenDocument, page: usize) -> Value {
        let mut value = serde_json::to_value(document).expect("legacy Value");
        jian_ops_schema::image_table::externalize_images(&mut value);
        value.as_object_mut().expect("document object").insert(
            "editorMeta".into(),
            json!({
                "activePageIndex": page,
                "preserveAuthoredGeometry": false
            }),
        );
        value
    }

    #[test]
    fn shared_save_snapshot_has_the_same_wire_shape_as_the_owned_document() {
        let document = loaded(
            r##"{
              "version":"1.0.0",
              "name":"Shared",
              "pages":[{"id":"p1","name":"One","backgroundColor":"#fff",
                "children":[{"type":"frame","id":"page-frame","children":[
                  {"type":"text","id":"label","content":"hello"}
                ]}]
              }],
              "children":[{"type":"rectangle","id":"root"}],
              "formatVersion":"1.0",
              "responsive":true
            }"##,
        );
        let state = EditorState::from_document(document.clone());
        let snapshot = CanonicalSaveSnapshot::capture(&state);
        let mut bytes = Vec::new();
        write_serializable_document_with_thumbnails(
            &mut bytes,
            snapshot.document(),
            EditorMeta {
                active_page_index: 3,
                ..snapshot.editor_meta()
            },
            snapshot.image_thumbnails(),
        )
        .expect("stream shared snapshot");

        let shared: Value = serde_json::from_slice(&bytes).expect("shared JSON");
        let (owned, _) = stream(&document, 3);
        let owned: Value = serde_json::from_str(&owned).expect("owned JSON");
        assert_eq!(shared, owned);
    }

    #[test]
    fn repeated_save_capture_reuses_unchanged_node_subtrees() {
        let document = loaded(
            r#"{"version":"1.0.0","children":[
              {"type":"frame","id":"a","children":[{"type":"text","id":"t","content":"hello"}]},
              {"type":"rectangle","id":"b"}
            ]}"#,
        );
        let mut state = EditorState::from_document(document);
        let first = CanonicalSaveSnapshot::capture(&state);
        state.doc.name = Some("metadata edit".into());
        state.mark_document_changed();
        let second = CanonicalSaveSnapshot::capture_reusing(&state, Some(&first));

        assert_eq!(second.document.root_children().len(), 2);
        for (before, after) in first
            .document
            .root_children()
            .iter()
            .zip(second.document.root_children())
        {
            assert!(std::sync::Arc::ptr_eq(before, after));
        }
    }

    #[test]
    fn save_snapshot_captures_authored_geometry_mode() {
        let mut state = EditorState::new();
        state.editor_ui.preserve_authored_geometry = true;

        let snapshot = CanonicalSaveSnapshot::capture(&state);

        assert!(snapshot.editor_meta().preserve_authored_geometry);
    }

    #[test]
    fn streaming_output_matches_the_previous_save_wire_semantics() {
        let long = long_source("shared");
        let short = "data:image/png;base64,tiny";
        let source = format!(
            r#"{{
              "version":"1.0.0",
              "pages":[{{"id":"page-1","name":"Page 1","children":[
                {{"type":"image","id":"page-image","src":{long:?}}}
              ]}}],
              "children":[
                {{"type":"image","id":"direct","src":{long:?}}},
                {{"type":"rectangle","id":"fill","fill":[{{"type":"image","url":{long:?}}}]}},
                {{"type":"rectangle","id":"stroke","stroke":{{"thickness":1,"fill":[{{"type":"image","url":{long:?}}}]}}}},
                {{"type":"tabs","id":"states","states":{{"hover":{{"fill":[{{"type":"image","url":{long:?}}}]}}}}}},
                {{"type":"image","id":"small","src":{short:?}}}
              ]
            }}"#
        );
        let document = loaded(&source);
        let (streamed, stats) = stream(&document, 7);
        let parsed: Value = serde_json::from_str(&streamed).expect("streamed JSON");

        assert_eq!(parsed, legacy_value(&document, 7));
        assert!(stats.wrote_images_table);
        assert_eq!(
            parsed["images"].as_object().map(|table| table.len()),
            Some(1),
            "all typed references deduplicate"
        );
        assert_eq!(streamed.matches("op-image:").count(), 5);
        assert_eq!(
            streamed.matches(long.as_str()).count(),
            1,
            "payload is written once"
        );
        assert!(streamed.contains(short), "short data URLs remain inline");
        assert_eq!(parsed["editorMeta"]["activePageIndex"], 7);
        assert_eq!(parsed["editorMeta"]["preserveAuthoredGeometry"], false);
        assert!(
            !streamed.contains("\n,\n"),
            "the root closing newline is removed before extension fields"
        );
    }

    #[test]
    fn no_large_images_omits_the_images_table_and_round_trips() {
        let document = loaded(
            r#"{"version":"1.0.0","children":[{"type":"image","id":"i","src":"assets/icon.png"}]}"#,
        );
        let (streamed, stats) = stream(&document, 0);
        let parsed: Value = serde_json::from_str(&streamed).expect("streamed JSON");
        assert!(!stats.wrote_images_table);
        assert!(parsed.get("images").is_none());

        let path =
            std::env::temp_dir().join(format!("openpencil-stream-load-{}.op", std::process::id()));
        std::fs::write(&path, streamed).expect("write fixture");
        let reloaded = super::super::load_editor_state(&path, op_editor_core::Locale::EnUs)
            .expect("product loader roundtrip");
        assert_eq!(reloaded.doc, document);
        let _ = std::fs::remove_file(path);
    }
}
