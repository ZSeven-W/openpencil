//! Wire adapter for [`crate::EditorMeta::imported_from`].
//!
//! Written as a plain string, `"importedFrom": "https://acme.example/pricing"`,
//! by any save of a document that Studio Home imported from a website. The
//! value is reduced to scheme + host + path
//! ([`op_editor_core::sanitize_import_origin`]) on BOTH sides: a save never
//! publishes a query string, fragment or credential, and a file arriving from
//! someone else is sanitized again rather than trusted. Like the other
//! editor-only hints, anything that does not sanitize to an origin — a
//! number, `null`, a `file:` URL — reads back as `None` instead of failing the
//! load.

use serde::{Deserialize, Deserializer, Serializer};

pub(crate) fn serialize<S: Serializer>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value
        .as_deref()
        .and_then(op_editor_core::sanitize_import_origin)
    {
        Some(origin) => serializer.serialize_str(&origin),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(url) => op_editor_core::sanitize_import_origin(&url),
        _ => None,
    })
}

#[cfg(test)]
#[path = "editor_meta_import_tests.rs"]
mod tests;
