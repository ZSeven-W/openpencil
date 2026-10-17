//! Explicit compatibility work performed while loading a canonical document.
//!
//! Ordinary schema warnings are intentionally kept separate. The migration
//! writer preserves unknown raw fields; a future declared schema still blocks
//! rewriting because its semantics are not known to this build.

/// Compatibility repairs that make a loaded document eligible for an
/// explicit, user-approved rewrite in the current canonical shape.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompatibilityReport {
    /// At least one known legacy wire shape was repaired in the parsed DOM.
    pub normalized_legacy: bool,
    /// A legacy top-level `version` (with no `formatVersion`) used an
    /// unsupported product-version major and was interpreted as schema v1.
    pub patched_legacy_version: Option<String>,
}

impl CompatibilityReport {
    /// Whether the loader performed an explicit, understood compatibility
    /// repair worth persisting through the raw-preserving migration writer.
    pub fn needs_schema_upgrade(&self) -> bool {
        self.normalized_legacy || self.patched_legacy_version.is_some()
    }
}

/// Canonical load result plus the exact compatibility work that produced it.
pub struct CanonicalLoad {
    pub loaded: jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    pub compatibility: CompatibilityReport,
}

#[cfg(test)]
#[path = "compatibility_roundtrip_tests.rs"]
mod roundtrip_tests;

/// Why the raw-preserving legacy migration writer could not produce a
/// current-schema rewrite.
///
/// Structured per failure stage; `Display` reproduces the exact strings this
/// writer used to return as `String` so host-side save dialogs keep their
/// wording. Note the "not a top-level JSON object" sentence deliberately omits
/// the word "valid" — that is what this path has always emitted, unlike
/// [`crate::EditorMetaWriteError::NotTopLevelObject`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedWriteError {
    /// The stack-protected `serde_json` parse of the migration DOM failed.
    Parse(String),
    /// The parsed source root is not a JSON object, so no top-level field can
    /// be inserted.
    NotTopLevelObject,
    /// Serializing the replacement `editorMeta` value failed.
    SerializeMeta(String),
    /// Streaming the migrated DOM into the destination writer failed.
    Write(String),
}

impl std::fmt::Display for NormalizedWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizedWriteError::NotTopLevelObject => {
                f.write_str("source document is not a top-level JSON object")
            }
            NormalizedWriteError::Parse(error)
            | NormalizedWriteError::SerializeMeta(error)
            | NormalizedWriteError::Write(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for NormalizedWriteError {}

/// `op-host-services`' migration Save-As `?`s this writer inside a closure
/// that still collects failures as `String`; this keeps that call site
/// compiling unchanged.
impl From<NormalizedWriteError> for String {
    fn from(error: NormalizedWriteError) -> String {
        error.to_string()
    }
}

/// Persist known legacy repairs while retaining unknown fields in the raw JSON
/// tree. This is intentionally reserved for an explicit one-time migration:
/// the compatibility DOM costs more memory than the typed streaming writer,
/// but it prevents a typed save from dropping nested extension fields.
pub fn write_normalized_source_with_current_schema<W: std::io::Write>(
    writer: &mut W,
    src: &str,
    meta: crate::EditorMeta,
) -> Result<(), NormalizedWriteError> {
    use serde::Serialize as _;

    let mut raw: serde_json::Value = super::deserialize_deep(src)
        .map_err(|error| NormalizedWriteError::Parse(error.to_string()))?;
    super::normalize_legacy_value(&mut raw);
    let root = raw
        .as_object_mut()
        .ok_or(NormalizedWriteError::NotTopLevelObject)?;
    root.insert(
        "formatVersion".to_string(),
        serde_json::Value::String(jian_ops_schema::version::FORMAT_VERSION_CURRENT.to_string()),
    );
    root.insert(
        "editorMeta".to_string(),
        serde_json::to_value(meta)
            .map_err(|error| NormalizedWriteError::SerializeMeta(error.to_string()))?,
    );
    let mut serializer = serde_json::Serializer::new(writer);
    let result = raw
        .serialize(serde_stacker::Serializer::new(&mut serializer))
        .map_err(|error| NormalizedWriteError::Write(error.to_string()));
    drop_json_value_iteratively(raw);
    result
}

/// `serde_json::Value` drops recursively. Drain containers iteratively after
/// stack-protected serialization so a valid deeply nested legacy document
/// cannot overflow the process stack while its migration DOM is released.
fn drop_json_value_iteratively(root: serde_json::Value) {
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        match value {
            serde_json::Value::Array(items) => pending.extend(items),
            serde_json::Value::Object(fields) => {
                pending.extend(fields.into_iter().map(|(_, value)| value));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::load_canonical_with_compatibility, write_normalized_source_with_current_schema,
    };

    #[test]
    fn explicit_legacy_normalization_is_reported() {
        let loaded = load_canonical_with_compatibility(
            r#"{"version":"1.0","children":[{"type":"path","id":"p","geometry":"M0 0L1 1"}]}"#,
        )
        .expect("legacy document loads");
        assert!(loaded.compatibility.normalized_legacy);
        assert!(loaded.compatibility.needs_schema_upgrade());
    }

    #[test]
    fn unsupported_legacy_product_version_without_format_version_is_reported() {
        let loaded = load_canonical_with_compatibility(
            r#"{"version":"2.8","children":[{"type":"rectangle","id":"r"}]}"#,
        )
        .expect("legacy product-version document loads");
        assert_eq!(
            loaded.compatibility.patched_legacy_version.as_deref(),
            Some("2.8")
        );
    }

    #[test]
    fn shallow_future_format_major_stays_unsupported() {
        let error = load_canonical_with_compatibility(
            r#"{"version":"2.8","formatVersion":"2.0","children":[]}"#,
        )
        .err()
        .expect("future schema major must fail");
        assert!(matches!(
            error,
            jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion { .. }
        ));
    }

    #[test]
    fn deep_future_format_major_stays_unsupported() {
        let mut source = String::from(r#"{"version":"2.8","formatVersion":"2.0","children":["#);
        for depth in 0..=121 {
            source.push_str(&format!(
                r#"{{"type":"frame","id":"f-{depth}","children":["#
            ));
        }
        source.push_str(r#"{"type":"rectangle","id":"leaf"}"#);
        for _ in 0..=121 {
            source.push_str("]}");
        }
        source.push_str("]}");

        let error = load_canonical_with_compatibility(&source)
            .err()
            .expect("deep future schema major must fail");
        assert!(matches!(
            error,
            jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion { .. }
        ));
    }

    #[test]
    fn current_document_has_no_upgrade_signal() {
        let loaded = load_canonical_with_compatibility(
            r#"{"version":"1.0.0","formatVersion":"1.2","children":[]}"#,
        )
        .expect("current document loads");
        assert_eq!(loaded.compatibility, Default::default());
        assert!(!loaded.compatibility.needs_schema_upgrade());
    }

    #[test]
    fn normalized_rewrite_preserves_nested_unknown_fields() {
        let source = r#"{"version":"1.0","children":[{"type":"path","id":"p","geometry":"M0 0L1 1","futureNodeField":{"mustSurvive":true}}]}"#;
        let mut output = Vec::new();

        write_normalized_source_with_current_schema(
            &mut output,
            source,
            crate::EditorMeta::default(),
        )
        .expect("normalize raw source");

        let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
        assert_eq!(parsed["children"][0]["d"], "M0 0L1 1");
        assert!(parsed["children"][0].get("geometry").is_none());
        assert_eq!(
            parsed["children"][0]["futureNodeField"]["mustSurvive"],
            true
        );
        assert_eq!(
            parsed["formatVersion"],
            jian_ops_schema::version::FORMAT_VERSION_CURRENT
        );
    }

    #[test]
    fn deeply_nested_normalized_rewrite_is_stack_safe() {
        let mut source = String::from(r#"{"version":"1.0","children":["#);
        for depth in 0..2_000 {
            source.push_str(&format!(
                r#"{{"type":"frame","id":"f-{depth}","children":["#
            ));
        }
        source.push_str(r#"{"type":"path","id":"leaf","geometry":"M0 0L1 1","futureLeaf":true}"#);
        for _ in 0..2_000 {
            source.push_str("]}");
        }
        source.push_str("]}");
        let mut output = Vec::new();

        write_normalized_source_with_current_schema(
            &mut output,
            &source,
            crate::EditorMeta::default(),
        )
        .expect("deep migration is stack protected");

        assert!(output
            .windows(b"futureLeaf".len())
            .any(|w| w == b"futureLeaf"));
        assert!(output
            .windows(b"\"geometry\"".len())
            .all(|w| w != b"\"geometry\""));
        assert!(output.windows(b"\"d\"".len()).any(|w| w == b"\"d\""));
    }
}
