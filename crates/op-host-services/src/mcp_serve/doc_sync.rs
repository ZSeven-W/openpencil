//! Whole-document REST sync wire helpers (TS `/api/mcp/document` parity).
//! Shared by the desktop live server (`mcp_live`) and the headless web-canvas
//! daemon (`web_canvas_server`) so both speak the exact same shape as the TS
//! web app's `apps/web/server/api/mcp/document.post.ts`. Re-exported from
//! `mcp_serve` so callers keep using `crate::mcp_serve::*`.

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::fmt;

use super::McpServeError;

/// True for the TS live-canvas whole-document sync route
/// (`POST /api/mcp/document`). Any other method/path falls through to the
/// JSON-RPC `/mcp` handling.
pub fn is_document_sync_route(method: &str, path: &str) -> bool {
    method == "POST" && path == "/api/mcp/document"
}

/// Parsed whole-document sync request metadata.
///
/// Editor view state lives beside `document` so older clients and daemons can
/// ignore it without changing the canonical `PenDocument` schema. Each `None`
/// distinguishes an older sender from an explicit top-level override.
pub struct DocumentSyncRequest<'a> {
    /// Exact `document` JSON borrowed from the HTTP request body.
    pub document_json: &'a str,
    pub base_version: Option<u64>,
    pub active_page_index: Option<usize>,
    pub preserve_authored_geometry: Option<bool>,
    pub metadata_only: bool,
    pub embedded_editor_meta: Option<op_pen_loader::EditorMeta>,
}

impl DocumentSyncRequest<'_> {
    /// Merge wrapper metadata over the document's embedded metadata one field
    /// at a time. Supplying only one top-level override must not erase the
    /// other embedded field.
    pub fn resolved_editor_meta(
        &self,
        embedded: Option<op_pen_loader::EditorMeta>,
    ) -> op_pen_loader::EditorMeta {
        let embedded = embedded.unwrap_or_default();
        op_pen_loader::EditorMeta {
            active_page_index: self.active_page_index.unwrap_or(embedded.active_page_index),
            preserve_authored_geometry: self
                .preserve_authored_geometry
                .unwrap_or(embedded.preserve_authored_geometry),
            // No wrapper override exists for the scenario tag, the pinned
            // style guide or the share recipe, so the document's own values
            // always survive the merge.
            scenario: embedded.scenario,
            pinned_style_guide: embedded.pinned_style_guide,
            share_recipe: embedded.share_recipe,
            imported_from: embedded.imported_from,
        }
    }
}

/// Scalar metadata and the document slice borrowed from a Web request body.
///
/// Large values stay as [`RawValue`] slices while the wrapper is decoded. This
/// keeps request parsing proportional to the wrapper fields instead of
/// materializing the complete document as `serde_json::Value`.
pub(crate) struct BorrowedDocumentEnvelope<'a> {
    pub document_json: Option<&'a str>,
    pub base_version: Option<u64>,
    pub active_page_index: Option<u64>,
    pub preserve_authored_geometry: Option<bool>,
    pub metadata_only: bool,
}

#[derive(Default)]
struct BorrowedJsonFields<'a> {
    is_object: bool,
    document: Option<&'a RawValue>,
    version: Option<&'a RawValue>,
    children: Option<&'a RawValue>,
    pages: Option<&'a RawValue>,
    editor_meta: Option<&'a RawValue>,
    base_version: Option<&'a RawValue>,
    active_page_index: Option<&'a RawValue>,
    preserve_authored_geometry: Option<&'a RawValue>,
    metadata_only: Option<&'a RawValue>,
}

struct BorrowedJsonFieldsVisitor;

impl<'de> Visitor<'de> for BorrowedJsonFieldsVisitor {
    type Value = BorrowedJsonFields<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = BorrowedJsonFields {
            is_object: true,
            ..Default::default()
        };
        while let Some(key) = map.next_key::<String>()? {
            let value: &'de RawValue = map.next_value()?;
            match key.as_str() {
                "document" => fields.document = Some(value),
                "version" => fields.version = Some(value),
                "children" => fields.children = Some(value),
                "pages" => fields.pages = Some(value),
                "editorMeta" => fields.editor_meta = Some(value),
                "baseVersion" => fields.base_version = Some(value),
                "activePageIndex" => fields.active_page_index = Some(value),
                "preserveAuthoredGeometry" => fields.preserve_authored_geometry = Some(value),
                "metadataOnly" => fields.metadata_only = Some(value),
                _ => {}
            }
        }
        Ok(fields)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<&'de RawValue>()?.is_some() {}
        Ok(BorrowedJsonFields::default())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(BorrowedJsonFields::default())
    }
}

impl<'de> Deserialize<'de> for BorrowedJsonFields<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BorrowedJsonFieldsVisitor)
    }
}

fn borrowed_json_fields(src: &str) -> Result<BorrowedJsonFields<'_>, serde_json::Error> {
    serde_json::from_str(src)
}

fn raw_u64(value: Option<&RawValue>) -> Option<u64> {
    value.and_then(|value| serde_json::from_str(value.get()).ok())
}

fn raw_bool(value: Option<&RawValue>) -> Option<bool> {
    value.and_then(|value| serde_json::from_str(value.get()).ok())
}

fn raw_is_nonempty_string(value: Option<&RawValue>) -> bool {
    value.is_some_and(|value| {
        let raw = value.get().trim();
        raw.starts_with('"') && raw.ends_with('"') && raw.len() > 2
    })
}

fn raw_is_array(value: Option<&RawValue>) -> bool {
    value.is_some_and(|value| value.get().trim().starts_with('['))
}

/// Decode only the wrapper around a potentially huge document.
pub(crate) fn parse_borrowed_document_envelope(
    body: &str,
) -> Result<BorrowedDocumentEnvelope<'_>, serde_json::Error> {
    let fields = borrowed_json_fields(body)?;
    Ok(BorrowedDocumentEnvelope {
        document_json: fields.document.map(|value| value.get()),
        base_version: raw_u64(fields.base_version),
        active_page_index: raw_u64(fields.active_page_index),
        preserve_authored_geometry: raw_bool(fields.preserve_authored_geometry),
        metadata_only: raw_bool(fields.metadata_only).unwrap_or(false),
    })
}

/// Validate a `/api/mcp/document` body and return the inner `document` JSON
/// (ready for `load_canonical`). Mirrors `document.post.ts`: `document` must be
/// present (else "Missing document in request body"), carry a non-empty
/// `version`, and have an array `children` OR `pages` (else "Invalid document
/// format").
pub fn parse_document_sync_request(body: &str) -> Result<DocumentSyncRequest<'_>, McpServeError> {
    let invalid = || McpServeError::Validation("Invalid document format".to_string());
    let envelope = parse_borrowed_document_envelope(body).map_err(|_| invalid())?;
    let document_json = envelope
        .document_json
        .ok_or_else(|| McpServeError::Validation("Missing document in request body".to_string()))?;
    let document = borrowed_json_fields(document_json).map_err(|_| invalid())?;
    let has_version = raw_is_nonempty_string(document.version);
    let has_children = raw_is_array(document.children);
    let has_pages = raw_is_array(document.pages);
    if !document.is_object || !has_version || (!has_children && !has_pages) {
        return Err(invalid());
    }
    Ok(DocumentSyncRequest {
        document_json,
        base_version: envelope.base_version,
        active_page_index: envelope
            .active_page_index
            .and_then(|index| usize::try_from(index).ok()),
        preserve_authored_geometry: envelope.preserve_authored_geometry,
        metadata_only: envelope.metadata_only,
        embedded_editor_meta: document
            .editor_meta
            .and_then(|value| serde_json::from_str(value.get()).ok()),
    })
}

/// Compatibility helper for callers that only need the canonical document.
pub fn parse_document_sync_body(body: &str) -> Result<&str, McpServeError> {
    parse_document_sync_request(body).map(|request| request.document_json)
}

/// Success body for a whole-document sync — matches `document.post.ts`'s
/// `{ ok: true, version }`.
pub fn document_sync_ok(version: u64) -> String {
    format!(r#"{{"ok":true,"version":{version}}}"#)
}

/// Error body for a rejected whole-document sync (HTTP 400).
pub fn rest_error_body(message: &str) -> String {
    format!(r#"{{"ok":false,"error":"{}"}}"#, json_escape(message))
}

/// JSON string escaping for embedding a message in a JSON reply body.
/// Delegates to the canonical op-util escaper. (The old local copy lossily
/// replaced control characters with spaces; they now get proper `\uXXXX`
/// escapes, so the message round-trips instead of being mangled.)
pub fn json_escape(s: &str) -> String {
    op_util::json_escape::escape_json(s)
}
