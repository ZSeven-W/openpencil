//! Allocation-bounded reader for OpenPencil's top-level `editorMeta` extension.
//!
//! The canonical schema intentionally ignores this editor-only object, so
//! hosts must read it before constructing an [`op_editor_core::EditorState`].
//! This scanner finds only the top-level value and deserializes that small
//! slice; it never materializes or rewrites the document-sized JSON tree.

use crate::editor_meta_error::EditorMetaWriteError;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::ShareRecipe;

/// Editor state that affects how a canonical document is reopened.
///
/// Every field defaults to its legacy behavior, so files written before a
/// field existed remain compatible. Snake-case aliases accept the former
/// sidecar spelling as well as the canonical camel-case wire spelling.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorMeta {
    /// Zero-based active page index at save time.
    #[serde(default, alias = "active_page_index")]
    pub active_page_index: usize,
    /// Use the numeric parent-local geometry authored by a Preserve-mode
    /// Figma import instead of resolving the tree through flex layout.
    #[serde(default, alias = "preserve_authored_geometry")]
    pub preserve_authored_geometry: bool,
    /// What the document is for — see `EditorUiState::scenario`. Written as
    /// the kebab-case scene name; anything unrecognized reads back as `None`
    /// so a stale or hand-edited tag can never fail a load.
    #[serde(
        default,
        with = "scenario_serde",
        skip_serializing_if = "Option::is_none"
    )]
    pub scenario: Option<TemplateScene>,
    /// Style guide pinned in the Asset Center — see
    /// `EditorUiState::pinned_style_guide`. Written as the guide's `name`;
    /// anything that is not a non-empty string reads back as `None` for the
    /// same reason [`EditorMeta::scenario`] does.
    #[serde(
        default,
        with = "pinned_style_guide_serde",
        skip_serializing_if = "Option::is_none"
    )]
    pub pinned_style_guide: Option<String>,
    /// How the document was made (brief, task + options, style guide) —
    /// see `EditorUiState::share_recipe`. Written by the share export and
    /// kept on resave of a received file; sanitized on read, and anything
    /// malformed reads back as `None`.
    #[serde(
        default,
        with = "crate::editor_meta_share",
        skip_serializing_if = "Option::is_none"
    )]
    pub share_recipe: Option<ShareRecipe>,
    /// The website the document was imported from — see
    /// `HomeState::imported_from`. Scheme + host + path only, sanitized on
    /// write and again on read; anything else reads back as `None`.
    #[serde(
        default,
        with = "crate::editor_meta_import",
        skip_serializing_if = "Option::is_none"
    )]
    pub imported_from: Option<String>,
}

impl EditorMeta {
    /// Capture the editor state a save must carry into the file.
    ///
    /// The inverse of [`apply_editor_meta`]. Every writer goes through this
    /// so a field added to the metadata is persisted by all of them at once,
    /// instead of surviving only the save paths someone remembered.
    pub fn from_state(state: &op_editor_core::EditorState) -> Self {
        Self {
            active_page_index: state.ui.active_page_index,
            preserve_authored_geometry: state.editor_ui.preserve_authored_geometry,
            scenario: state.editor_ui.scenario,
            pinned_style_guide: state.editor_ui.pinned_style_guide.clone(),
            // Only a recipe the file ARRIVED with is carried through a
            // save. A live run's brief is written solely by the share
            // export (which sanitizes it): an ordinary save must neither
            // publish the brief nor reopen the author's own work as a
            // shared one.
            share_recipe: state.editor_ui.home.recipe.clone(),
            // Provenance, not a preference: an imported site stays an
            // imported site through every save, so a later AI edit keeps
            // deferring to its authored design.
            imported_from: state.editor_ui.home.imported_from.clone(),
        }
    }
}

/// Wire adapter for [`EditorMeta::scenario`].
///
/// A scenario is an editor-only UI hint: refusing to open someone's document
/// because that hint is a number, a future scene name, or `null` would trade
/// a nicety for their file, so every value this does not recognize decodes to
/// `None`. Absent stays absent — the field is omitted when unset rather than
/// written as `null`, keeping old readers and byte-comparison tests unchanged.
mod scenario_serde {
    use super::TemplateScene;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub(super) fn serialize<S: Serializer>(
        value: &Option<TemplateScene>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(scene) => serializer.serialize_str(scene.as_str()),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<TemplateScene>, D::Error> {
        // Any JSON at all parses into `Value`, so the only error this can
        // propagate is a malformed document the caller must fail on anyway.
        Ok(match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::String(name) => TemplateScene::from_str(&name).ok(),
            _ => None,
        })
    }
}

/// Wire adapter for [`EditorMeta::pinned_style_guide`].
///
/// The pin is a hint about future generations, so the same rule the scenario
/// tag follows applies: a number, an object, `null`, or a blank string is
/// dropped rather than failing the load. An unrecognized *name* is kept —
/// only the generation path can say whether the registry still carries it,
/// and it falls back to automatic ranking when it does not.
mod pinned_style_guide_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        value: &Option<String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(name) => serializer.serialize_str(name),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<String>, D::Error> {
        Ok(match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::String(name) if !name.trim().is_empty() => Some(name),
            _ => None,
        })
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEditorMeta {
    #[serde(default, alias = "active_page_index")]
    active_page_index: usize,
    #[serde(default, alias = "preserve_authored_geometry")]
    preserve_authored_geometry: Option<bool>,
    #[serde(default, with = "scenario_serde")]
    scenario: Option<TemplateScene>,
    #[serde(default, with = "pinned_style_guide_serde")]
    pinned_style_guide: Option<String>,
    #[serde(default, with = "crate::editor_meta_share")]
    share_recipe: Option<ShareRecipe>,
    #[serde(default, with = "crate::editor_meta_import")]
    imported_from: Option<String>,
}

/// Parsed metadata plus compatibility inference used for migration decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMetaExtraction {
    pub meta: EditorMeta,
    /// The short-lived Figma writer omitted the geometry bit and it was
    /// recovered from the generated first-page id.
    pub inferred_preserve_authored_geometry: bool,
}

/// Extract the last top-level `editorMeta` object from canonical JSON.
///
/// Invalid, nested, or absent metadata returns `None`; the normal canonical
/// loader remains responsible for validating the complete document. Matching
/// the last duplicate key follows `serde_json` object semantics.
pub fn extract_editor_meta(src: &str) -> Option<EditorMeta> {
    extract_editor_meta_with_report(src).map(|extraction| extraction.meta)
}

/// Like [`extract_editor_meta`], but preserves whether a known legacy Figma
/// metadata omission had to be inferred.
pub fn extract_editor_meta_with_report(src: &str) -> Option<EditorMetaExtraction> {
    let scan = scan_top_level(src, "editorMeta")?;
    let wire = serde_json::from_str::<WireEditorMeta>(scan.value?).ok()?;
    let inferred_preserve_authored_geometry =
        wire.preserve_authored_geometry.is_none() && scan.first_page_has_figma_id;
    Some(EditorMetaExtraction {
        meta: EditorMeta {
            active_page_index: wire.active_page_index,
            // A short-lived writer version emitted editorMeta without this
            // field for Preserve-mode Figma imports. Recover only when the
            // canonical first page carries op-figma's unambiguous generated
            // ID. Explicit true/false always wins.
            preserve_authored_geometry: wire
                .preserve_authored_geometry
                .unwrap_or(scan.first_page_has_figma_id),
            scenario: wire.scenario,
            pinned_style_guide: wire.pinned_style_guide,
            share_recipe: wire.share_recipe,
            imported_from: wire.imported_from,
        },
        inferred_preserve_authored_geometry,
    })
}

/// Copy canonical JSON to `writer`, replacing or appending only its top-level
/// `editorMeta` value.
///
/// This is the clean-document Save-As path: the canonical document bytes stay
/// untouched (including legacy/future schema fields and image tables), while
/// the current active page and authored-geometry mode are persisted. The
/// scanner retains only byte offsets, so a large source document is never
/// materialized as a `String`, `Value`, or `PenDocument` clone.
pub fn write_source_with_editor_meta<W: std::io::Write>(
    writer: &mut W,
    src: &str,
    meta: EditorMeta,
) -> Result<(), EditorMetaWriteError> {
    let scan = scan_top_level(src, "editorMeta").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let src_bytes = src.as_bytes();
    let write = |result: std::io::Result<()>| {
        result.map_err(|error| EditorMetaWriteError::Write(error.to_string()))
    };
    let serialize = |error: serde_json::Error| EditorMetaWriteError::Serialize(error.to_string());

    if let Some((value_start, value_end)) = scan.value_range {
        write(writer.write_all(&src_bytes[..value_start]))?;
        serde_json::to_writer(&mut *writer, &meta).map_err(serialize)?;
        write(writer.write_all(&src_bytes[value_end..]))?;
        return Ok(());
    }

    write(writer.write_all(&src_bytes[..scan.root_close]))?;
    if scan.has_members {
        write(writer.write_all(b",\"editorMeta\":"))?;
    } else {
        write(writer.write_all(b"\"editorMeta\":"))?;
    }
    serde_json::to_writer(&mut *writer, &meta).map_err(serialize)?;
    write(writer.write_all(&src_bytes[scan.root_close..]))
}

/// Copy canonical JSON while upgrading only the top-level format marker and
/// editor metadata. Every nested byte, including unknown future fields, is
/// preserved verbatim.
pub fn write_source_with_current_schema<W: std::io::Write>(
    writer: &mut W,
    src: &str,
    meta: EditorMeta,
) -> Result<(), EditorMetaWriteError> {
    #[derive(Clone, Copy)]
    enum Replacement {
        FormatVersion,
        EditorMeta,
    }

    let meta_scan =
        scan_top_level(src, "editorMeta").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let format_scan =
        scan_top_level(src, "formatVersion").ok_or(EditorMetaWriteError::NotTopLevelObject)?;
    let write_err = |error: std::io::Error| EditorMetaWriteError::Write(error.to_string());
    let serialize_err =
        |error: serde_json::Error| EditorMetaWriteError::Serialize(error.to_string());
    let src_bytes = src.as_bytes();
    let mut replacements = Vec::with_capacity(2);
    if let Some((start, end)) = format_scan.value_range {
        replacements.push((start, end, Replacement::FormatVersion));
    }
    if let Some((start, end)) = meta_scan.value_range {
        replacements.push((start, end, Replacement::EditorMeta));
    }
    replacements.sort_unstable_by_key(|replacement| replacement.0);

    let mut cursor = 0usize;
    for (start, end, replacement) in replacements {
        writer
            .write_all(&src_bytes[cursor..start])
            .map_err(write_err)?;
        match replacement {
            Replacement::FormatVersion => {
                serde_json::to_writer(
                    &mut *writer,
                    jian_ops_schema::version::FORMAT_VERSION_CURRENT,
                )
                .map_err(serialize_err)?;
            }
            Replacement::EditorMeta => {
                serde_json::to_writer(&mut *writer, &meta).map_err(serialize_err)?;
            }
        }
        cursor = end;
    }
    writer
        .write_all(&src_bytes[cursor..meta_scan.root_close])
        .map_err(write_err)?;

    let missing_format = format_scan.value_range.is_none();
    let missing_meta = meta_scan.value_range.is_none();
    let mut needs_comma = meta_scan.has_members;
    if missing_format {
        if needs_comma {
            writer.write_all(b",").map_err(write_err)?;
        }
        writer.write_all(b"\"formatVersion\":").map_err(write_err)?;
        serde_json::to_writer(
            &mut *writer,
            jian_ops_schema::version::FORMAT_VERSION_CURRENT,
        )
        .map_err(serialize_err)?;
        needs_comma = true;
    }
    if missing_meta {
        if needs_comma {
            writer.write_all(b",").map_err(write_err)?;
        }
        writer.write_all(b"\"editorMeta\":").map_err(write_err)?;
        serde_json::to_writer(&mut *writer, &meta).map_err(serialize_err)?;
    }
    writer
        .write_all(&src_bytes[meta_scan.root_close..])
        .map_err(write_err)
}

/// Apply embedded editor metadata to a freshly loaded editor state.
///
/// The page index is clamped to the document's current page count. The
/// authored-geometry bit is copied verbatim; callers should invoke this only
/// when metadata was actually present so an absent extension retains whatever
/// host-specific default was already installed.
pub fn apply_editor_meta(state: &mut op_editor_core::EditorState, meta: EditorMeta) {
    let page_count = state
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.len())
        .unwrap_or(1)
        .max(1);
    state.ui.active_page_index = meta.active_page_index.min(page_count - 1);
    state.editor_ui.preserve_authored_geometry = meta.preserve_authored_geometry;
    state.editor_ui.scenario = meta.scenario;
    state.editor_ui.pinned_style_guide = meta.pinned_style_guide;
    state.editor_ui.home.recipe = meta.share_recipe;
    state.editor_ui.home.imported_from = meta.imported_from;
}

/// Apply saved metadata, or use the legacy reopen policy when it is absent.
///
/// Old files predate Preserve-mode geometry and therefore always reopen in
/// layout mode. If their first page is empty, land on the first page with
/// content so a valid multi-page document does not appear blank.
pub fn apply_editor_meta_or_legacy_fallback(
    state: &mut op_editor_core::EditorState,
    meta: Option<EditorMeta>,
) {
    if let Some(meta) = meta {
        apply_editor_meta(state, meta);
        return;
    }
    state.editor_ui.preserve_authored_geometry = false;
    // No metadata means nothing is KNOWN about what the document is for, and
    // an unknown scenario must read as `None` rather than inherit whatever
    // the caller's state happened to carry in.
    state.editor_ui.scenario = None;
    state.editor_ui.pinned_style_guide = None;
    state.editor_ui.home.recipe = None;
    state.editor_ui.home.imported_from = None;
    state.ui.active_page_index = state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.iter().position(|page| !page.children.is_empty()))
        .unwrap_or(0);
}

struct TopLevelScan<'a> {
    value: Option<&'a str>,
    value_range: Option<(usize, usize)>,
    first_page_has_figma_id: bool,
    root_close: usize,
    has_members: bool,
}

fn scan_top_level<'a>(src: &'a str, wanted: &str) -> Option<TopLevelScan<'a>> {
    let bytes = src.as_bytes();
    let mut cursor = skip_ws(bytes, 0);
    let mut found = None;
    let mut found_range = None;
    let mut first_page_has_figma_id = false;
    let mut has_members = false;
    if bytes.get(cursor) != Some(&b'{') {
        return None;
    }
    cursor += 1;

    loop {
        cursor = skip_ws(bytes, cursor);
        if bytes.get(cursor) == Some(&b'}') {
            return Some(TopLevelScan {
                value: found,
                value_range: found_range,
                first_page_has_figma_id,
                root_close: cursor,
                has_members,
            });
        }
        has_members = true;
        let key_start = cursor;
        let key_end = string_end(bytes, key_start)?;
        let matches = key_matches(&src[key_start..key_end], wanted);
        let is_pages = key_matches(&src[key_start..key_end], "pages");

        cursor = skip_ws(bytes, key_end);
        if bytes.get(cursor) != Some(&b':') {
            return None;
        }
        let value_start = skip_ws(bytes, cursor + 1);
        if is_pages {
            // PenPage's canonical field order begins with `id`. Inspect only a
            // bounded prefix while the main scanner is already at `pages`, so
            // a 200+ MB document is not traversed a second time for migration.
            const PAGE_HEAD_LIMIT: usize = 512;
            let mut head_end = value_start.saturating_add(PAGE_HEAD_LIMIT).min(src.len());
            while !src.is_char_boundary(head_end) {
                head_end -= 1;
            }
            first_page_has_figma_id = first_page_has_reliable_figma_id(&src[value_start..head_end]);
        }
        let value_end = value_end(bytes, value_start)?;
        if matches {
            found = Some(&src[value_start..value_end]);
            found_range = Some((value_start, value_end));
        }

        cursor = skip_ws(bytes, value_end);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => {
                return Some(TopLevelScan {
                    value: found,
                    value_range: found_range,
                    first_page_has_figma_id,
                    root_close: cursor,
                    has_members,
                });
            }
            _ => return None,
        }
    }
}

fn first_page_has_reliable_figma_id(pages_head: &str) -> bool {
    let bytes = pages_head.as_bytes();
    let mut cursor = skip_ws(bytes, 0);
    if bytes.get(cursor) != Some(&b'[') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    if bytes.get(cursor) != Some(&b'{') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    let Some(key_end) = string_end(bytes, cursor) else {
        return false;
    };
    if !key_matches(&pages_head[cursor..key_end], "id") {
        return false;
    }
    cursor = skip_ws(bytes, key_end);
    if bytes.get(cursor) != Some(&b':') {
        return false;
    }
    cursor = skip_ws(bytes, cursor + 1);
    let Some(id_end) = string_end(bytes, cursor) else {
        return false;
    };
    let Ok(id) = serde_json::from_str::<String>(&pages_head[cursor..id_end]) else {
        return false;
    };
    id.strip_prefix("figma-page-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn skip_ws(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
    {
        cursor += 1;
    }
    cursor
}

fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => return Some(cursor + 1),
            b'\\' => cursor = cursor.checked_add(2)?,
            0x00..=0x1f => return None,
            _ => cursor += 1,
        }
    }
    None
}

fn key_matches(raw_json_string: &str, wanted: &str) -> bool {
    let inner = &raw_json_string[1..raw_json_string.len() - 1];
    if !inner.as_bytes().contains(&b'\\') {
        return inner == wanted;
    }
    serde_json::from_str::<String>(raw_json_string).is_ok_and(|key| key == wanted)
}

fn value_end(bytes: &[u8], start: usize) -> Option<usize> {
    match bytes.get(start)? {
        b'"' => string_end(bytes, start),
        b'{' | b'[' => compound_value_end(bytes, start),
        _ => {
            let mut end = start;
            while !matches!(bytes.get(end), None | Some(b',') | Some(b'}')) {
                end += 1;
            }
            while end > start && matches!(bytes[end - 1], b' ' | b'\n' | b'\r' | b'\t') {
                end -= 1;
            }
            (end > start).then_some(end)
        }
    }
}

fn compound_value_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut stack = vec![*bytes.get(start)?];
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => cursor = string_end(bytes, cursor)?,
            b'{' | b'[' => {
                stack.push(byte);
                cursor += 1;
            }
            b'}' | b']' => {
                let expected = if byte == b'}' { b'{' } else { b'[' };
                if stack.pop() != Some(expected) {
                    return None;
                }
                cursor += 1;
                if stack.is_empty() {
                    return Some(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

#[cfg(test)]
#[path = "editor_meta_tests.rs"]
mod tests;
