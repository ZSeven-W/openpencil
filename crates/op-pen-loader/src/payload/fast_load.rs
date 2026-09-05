//! Streaming canonical loader for current Preserve-mode documents.
//!
//! The compatibility loader intentionally materializes one complete JSON
//! `Value` so it can repair legacy wire shapes. OpenPencil's current Figma
//! Preserve output is already canonical, so that DOM is pure peak-memory
//! overhead. This module probes only the top-level header and image tables,
//! then deserializes the typed document directly under the image-ref scope.
//! Any ambiguity or parse failure returns `None` and leaves the established
//! compatibility/normalization path in charge.

use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Default)]
struct CapturedStringTable(Option<BTreeMap<String, String>>);

impl<'de> Deserialize<'de> for CapturedStringTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        BTreeMap::<String, String>::deserialize(deserializer).map(|table| Self(Some(table)))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreserveEditorMeta {
    #[serde(default, alias = "preserve_authored_geometry")]
    preserve_authored_geometry: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CanonicalHeader {
    #[serde(default)]
    format_version: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    responsive: Option<bool>,
    #[serde(default)]
    id: Option<IgnoredAny>,
    #[serde(default)]
    name: Option<IgnoredAny>,
    #[serde(default)]
    themes: Option<IgnoredAny>,
    #[serde(default)]
    variables: Option<IgnoredAny>,
    #[serde(default)]
    pages: Option<IgnoredAny>,
    #[serde(default)]
    children: Option<IgnoredAny>,
    #[serde(default)]
    app: Option<IgnoredAny>,
    #[serde(default)]
    routes: Option<IgnoredAny>,
    #[serde(default)]
    state: Option<IgnoredAny>,
    #[serde(default)]
    lifecycle: Option<IgnoredAny>,
    #[serde(default)]
    logic_modules: Option<IgnoredAny>,
    #[serde(default)]
    design_md: Option<IgnoredAny>,
    #[serde(default)]
    conversion: Option<IgnoredAny>,
    #[serde(default)]
    editor_meta: Option<PreserveEditorMeta>,
    #[serde(default)]
    images: CapturedStringTable,
    #[serde(default)]
    image_thumbs: CapturedStringTable,
    #[serde(flatten)]
    extra: BTreeMap<String, IgnoredAny>,
}

pub(super) fn try_load_preserve(
    src: &str,
) -> Option<jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>> {
    let header: CanonicalHeader = super::deserialize_deep(src).ok()?;
    let declared_version = header
        .format_version
        .as_deref()
        .or(header.version.as_deref());
    let (major, _) = jian_ops_schema::version::parse(declared_version);
    if major != 1
        || !jian_ops_schema::version::supports(declared_version)
        || header.responsive == Some(true)
        || header
            .editor_meta
            .as_ref()
            .and_then(|meta| meta.preserve_authored_geometry)
            != Some(true)
        || contains_legacy_normalization_marker(src)
    {
        return None;
    }

    let warnings = header_warnings(&header);
    let image_table: HashMap<String, Arc<str>> = header
        .images
        .0
        .unwrap_or_default()
        .into_iter()
        .map(|(id, source)| (id, Arc::from(source)))
        .collect();
    let pending_thumbs = pending_thumbs(header.image_thumbs.0);
    let mut document =
        jian_ops_schema::node::image_src::intern::with_load_scope(image_table, || {
            super::deserialize_deep::<jian_ops_schema::PenDocument>(src)
        })
        .ok()?;
    jian_ops_schema::image_thumbs::attach_to_document(&mut document, pending_thumbs);

    Some(jian_ops_schema::LoadResult {
        value: document,
        warnings,
    })
}

/// Two legacy fields can deserialize successfully while still requiring the
/// old DOM repair for correct meaning: disabled fill entries must be removed,
/// and Pencil's path `geometry` must be copied to canonical `d`. A quoted-key
/// search is intentionally conservative (a rare same-named non-node field
/// only loses the fast path) and keeps the common current-Figma probe
/// allocation-free. Any JSON unicode escape also falls back: an escaped key
/// such as `en\u0061bled` is semantically identical after parsing and must not
/// bypass normalization.
fn contains_legacy_normalization_marker(src: &str) -> bool {
    src.contains("\\u")
        || src.contains("\"enabled\"")
        || src.contains("\"geometry\"")
        // The fast path deserializes directly and therefore cannot rewrite
        // the authoring-only video alias before PenNode sees its tag.
        || (src.contains("\"type\"") && src.contains("\"video\""))
}

fn pending_thumbs(
    table: Option<BTreeMap<String, String>>,
) -> jian_ops_schema::image_thumbs::PendingThumbSeed {
    let mut root = serde_json::Map::new();
    if let Some(table) = table {
        root.insert(
            "imageThumbs".to_owned(),
            serde_json::Value::Object(
                table
                    .into_iter()
                    .map(|(id, encoded)| (id, serde_json::Value::String(encoded)))
                    .collect(),
            ),
        );
    }
    jian_ops_schema::image_thumbs::take_pending_from_document(&mut serde_json::Value::Object(root))
}

fn header_warnings(header: &CanonicalHeader) -> Vec<jian_ops_schema::LoadWarning> {
    let mut warnings = header
        .extra
        .keys()
        .map(|field| jian_ops_schema::LoadWarning::UnknownField {
            path: "$".to_owned(),
            field: field.to_owned(),
        })
        .collect::<Vec<_>>();

    if let Some(format_version) = header.format_version.as_deref() {
        let (current_major, current_minor) =
            jian_ops_schema::version::parse(Some(jian_ops_schema::version::FORMAT_VERSION_CURRENT));
        let (major, minor) = jian_ops_schema::version::parse(Some(format_version));
        if major > current_major || (major == current_major && minor > current_minor) {
            warnings.push(jian_ops_schema::LoadWarning::FutureFormatVersion {
                found: format_version.to_owned(),
                supported_max: jian_ops_schema::version::FORMAT_VERSION_CURRENT,
            });
        }
    }
    if header.logic_modules.is_some() {
        warnings.push(jian_ops_schema::LoadWarning::LogicModulesSkipped {
            reason: "Tier 3 WASM is not implemented in this build",
        });
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::PenNode;

    #[test]
    fn fast_preserve_load_resolves_shared_images_and_attaches_thumbnails() {
        let _guard = super::super::lock_thumbnail_registry_for_test();
        let source = "data:image/png;base64,c2hhcmVkLWltYWdl";
        let paint_id = jian_ops_schema::node::image_src::paint_image_id(source);
        let src = format!(
            r#"{{
              "version":"1",
              "children":[
                {{"type":"image","id":"a","src":"op-image:shared"}},
                {{"type":"image","id":"b","src":"op-image:shared"}}
              ],
              "images":{{"shared":"{source}"}},
              "imageThumbs":{{"{paint_id}":"/9j/2Q=="}},
              "editorMeta":{{"activePageIndex":0,"preserveAuthoredGeometry":true}},
              "futureTopLevel":true
            }}"#
        );

        let loaded = try_load_preserve(&src).expect("current Preserve document uses fast load");
        assert_eq!(
            loaded.warnings,
            vec![jian_ops_schema::LoadWarning::UnknownField {
                path: "$".to_owned(),
                field: "futureTopLevel".to_owned(),
            }]
        );
        let (PenNode::Image(first), PenNode::Image(second)) =
            (&loaded.value.children[0], &loaded.value.children[1])
        else {
            panic!("two image nodes expected");
        };
        assert_eq!(first.src.as_str(), source);
        assert_eq!(second.src.as_str(), source);
        assert!(Arc::ptr_eq(&first.src.as_arc(), &second.src.as_arc()));

        assert!(
            jian_ops_schema::image_thumbs::activate_for_document(&loaded.value),
            "the decoded table must stay attached to the typed document"
        );
        assert_eq!(
            &*jian_ops_schema::image_thumbs::thumb_for(paint_id)
                .expect("attached thumbnail activates"),
            &[0xff, 0xd8, 0xff, 0xd9]
        );
    }

    #[test]
    fn legacy_document_bypasses_fast_load_and_keeps_normalization() {
        let src = r##"{
          "version":"2.8",
          "children":[
            {"type":"frame","id":"legacy","fill":"#123456","children":[]}
          ],
          "editorMeta":{"preserveAuthoredGeometry":false}
        }"##;

        assert!(try_load_preserve(src).is_none());
        let loaded = super::super::load_canonical(src).expect("legacy compatibility load");
        assert_eq!(loaded.value.version, "1.0");
        let PenNode::Frame(frame) = &loaded.value.children[0] else {
            panic!("legacy frame expected");
        };
        assert_eq!(frame.container.fill.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn preserve_legacy_markers_and_typed_failure_fall_back_to_repair() {
        let src = r##"{
          "version":"1",
          "children":[
            {"type":"frame","id":"legacy","fill":"#abcdef","children":[]},
            {"type":"rectangle","id":"disabled",
             "fill":[{"type":"solid","color":"#ff0000","enabled":false}]},
            {"type":"path","id":"path","geometry":"M0 0L9 9"}
          ],
          "editorMeta":{"preserveAuthoredGeometry":true}
        }"##;

        assert!(try_load_preserve(src).is_none());
        let loaded = super::super::load_canonical(src).expect("fallback repairs legacy fill");
        let PenNode::Frame(frame) = &loaded.value.children[0] else {
            panic!("legacy frame expected");
        };
        assert_eq!(frame.container.fill.as_ref().map(Vec::len), Some(1));
        let PenNode::Rectangle(disabled) = &loaded.value.children[1] else {
            panic!("disabled rectangle expected");
        };
        assert!(
            disabled.container.fill.as_ref().is_some_and(Vec::is_empty),
            "disabled legacy fill must remain unpainted"
        );
        let PenNode::Path(path) = &loaded.value.children[2] else {
            panic!("legacy path expected");
        };
        assert_eq!(path.d.as_deref(), Some("M0 0L9 9"));
    }

    #[test]
    fn escaped_legacy_keys_cannot_bypass_normalization() {
        let src = r##"{
          "version":"1",
          "children":[
            {"type":"rectangle","id":"disabled",
             "fill":[{"type":"solid","color":"#ff0000","en\u0061bled":false}]},
            {"type":"path","id":"path","geo\u006detry":"M0 0L9 9"}
          ],
          "editorMeta":{"preserveAuthoredGeometry":true}
        }"##;

        assert!(try_load_preserve(src).is_none());
        let loaded = super::super::load_canonical(src).expect("escaped keys use legacy repair");
        let PenNode::Rectangle(disabled) = &loaded.value.children[0] else {
            panic!("disabled rectangle expected");
        };
        assert!(disabled.container.fill.as_ref().is_some_and(Vec::is_empty));
        let PenNode::Path(path) = &loaded.value.children[1] else {
            panic!("legacy path expected");
        };
        assert_eq!(path.d.as_deref(), Some("M0 0L9 9"));
    }

    #[test]
    fn preserve_fast_load_rejects_trailing_json_garbage() {
        let src = r#"{
          "version":"1",
          "children":[],
          "editorMeta":{"preserveAuthoredGeometry":true}
        } trailing"#;

        assert!(try_load_preserve(src).is_none());
        assert!(super::super::load_canonical(src).is_err());
    }
}
