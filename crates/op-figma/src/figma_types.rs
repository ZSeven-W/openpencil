//! Figma `.fig` decoded-file model — ports `figma-types.ts` +
//! `fig-parser.ts::parseFigFile`. The decoded data is kept as
//! [`FigValue`] trees (the TS code likewise works on untyped `any`);
//! this module adds the geometric primitive structs worth typing and
//! the `parseFigFile` pipeline that ties the container + Kiwi layers.

use crate::container::{fig_to_binary_parts, ContainerError};
use crate::kiwi::{decode_binary_schema, decode_message, FigValue, KiwiError};
use std::collections::HashMap;

/// Figma node identity — `sessionID:localID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FigGuid {
    pub session_id: u32,
    pub local_id: u32,
}

impl FigGuid {
    /// Extract from a decoded `{ sessionID, localID }` object.
    pub fn from_value(v: &FigValue) -> Option<FigGuid> {
        Some(FigGuid {
            session_id: v.get_f64("sessionID")? as u32,
            local_id: v.get_f64("localID")? as u32,
        })
    }

    /// Canonical map key — matches `guidToString`.
    pub fn to_key(self) -> String {
        format!("{}:{}", self.session_id, self.local_id)
    }
}

/// Figma 2×3 affine transform `[m00 m01 m02 / m10 m11 m12]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FigMatrix {
    pub m00: f64,
    pub m01: f64,
    pub m02: f64,
    pub m10: f64,
    pub m11: f64,
    pub m12: f64,
}

impl FigMatrix {
    pub fn from_value(v: &FigValue) -> Option<FigMatrix> {
        Some(FigMatrix {
            m00: v.get_f64("m00").unwrap_or(0.0),
            m01: v.get_f64("m01").unwrap_or(0.0),
            m02: v.get_f64("m02").unwrap_or(0.0),
            m10: v.get_f64("m10").unwrap_or(0.0),
            m11: v.get_f64("m11").unwrap_or(0.0),
            m12: v.get_f64("m12").unwrap_or(0.0),
        })
        .filter(|_| v.is_object())
    }
}

/// Figma colour — channels 0.0–1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FigColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: Option<f64>,
}

impl FigColor {
    pub fn from_value(v: &FigValue) -> Option<FigColor> {
        if !v.is_object() {
            return None;
        }
        Some(FigColor {
            r: v.get_f64("r").unwrap_or(0.0),
            g: v.get_f64("g").unwrap_or(0.0),
            b: v.get_f64("b").unwrap_or(0.0),
            a: v.get_f64("a"),
        })
    }
}

/// Figma 2D vector / size pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FigVec2 {
    pub x: f64,
    pub y: f64,
}

impl FigVec2 {
    pub fn from_value(v: &FigValue) -> Option<FigVec2> {
        if !v.is_object() {
            return None;
        }
        Some(FigVec2 {
            x: v.get_f64("x").unwrap_or(0.0),
            y: v.get_f64("y").unwrap_or(0.0),
        })
    }
}

/// A decoded blob — Figma's `blobs` array holds either raw bytes
/// (vector/path geometry) or strings.
#[derive(Debug, Clone, PartialEq)]
pub enum BlobOrString {
    Bytes(Vec<u8>),
    Str(String),
}

impl BlobOrString {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            BlobOrString::Bytes(b) => Some(b),
            BlobOrString::Str(_) => None,
        }
    }
}

/// The fully decoded `.fig` file — the Rust [`FigmaDecodedFile`].
pub struct FigmaDecodedFile {
    /// Flat list of node changes (each a [`FigValue::Object`]).
    pub node_changes: Vec<FigValue>,
    /// Geometry / image blob pool, indexed by `commandsBlob` etc.
    pub blobs: Vec<BlobOrString>,
    /// ZIP-embedded image files keyed by hex hash.
    pub image_files: HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub enum FigFileError {
    Container(ContainerError),
    Kiwi(KiwiError),
    /// Fewer than the required schema + data parts.
    TooFewParts(usize),
    /// Decoded data carried no node changes.
    NoNodeChanges,
}

impl std::fmt::Display for FigFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FigFileError::Container(e) => write!(f, "{e}"),
            FigFileError::Kiwi(e) => write!(f, "{e}"),
            FigFileError::TooFewParts(n) => {
                write!(f, "invalid .fig file: expected ≥2 binary parts, got {n}")
            }
            FigFileError::NoNodeChanges => write!(f, "decoded .fig data carried no node changes"),
        }
    }
}

/// Pull the `blobs` array off the decoded root. Mirrors `extractBlobs`:
/// an element with a `bytes` byte-array → raw bytes; a string element
/// → string; anything else → empty bytes.
fn extract_blobs(raw: &FigValue) -> Vec<BlobOrString> {
    let Some(arr) = raw.get_array("blobs") else {
        return Vec::new();
    };
    arr.iter()
        .map(|blob| {
            if let Some(bytes) = blob.get("bytes").and_then(|b| b.as_bytes()) {
                BlobOrString::Bytes(bytes.to_vec())
            } else if let Some(s) = blob.as_str() {
                BlobOrString::Str(s.to_string())
            } else {
                BlobOrString::Bytes(Vec::new())
            }
        })
        .collect()
}

/// Locate the node-change array — `nodeChanges` if present + non-empty,
/// else the first array of objects whose elements carry a `guid`.
fn find_node_changes(raw: &FigValue) -> Option<Vec<FigValue>> {
    if let Some(arr) = raw.get_array("nodeChanges") {
        if !arr.is_empty() {
            return Some(arr.to_vec());
        }
    }
    if let FigValue::Object(pairs) = raw {
        for (_, v) in pairs {
            if let Some(arr) = v.as_array() {
                if arr
                    .first()
                    .map(|n| n.get("guid").is_some())
                    .unwrap_or(false)
                {
                    return Some(arr.to_vec());
                }
            }
        }
    }
    None
}

/// Parse a `.fig` file buffer into a [`FigmaDecodedFile`] — the Rust
/// `parseFigFile`. Splits the container, decodes the Kiwi schema +
/// data, and extracts node changes + blobs + embedded images.
pub fn parse_fig_file(bytes: &[u8]) -> Result<FigmaDecodedFile, FigFileError> {
    let container = fig_to_binary_parts(bytes).map_err(FigFileError::Container)?;
    if container.parts.len() < 2 {
        return Err(FigFileError::TooFewParts(container.parts.len()));
    }
    let schema = decode_binary_schema(&container.parts[0]).map_err(FigFileError::Kiwi)?;
    let raw = decode_message(&schema, &container.parts[1]).map_err(FigFileError::Kiwi)?;
    let node_changes = find_node_changes(&raw).ok_or(FigFileError::NoNodeChanges)?;
    Ok(FigmaDecodedFile {
        node_changes,
        blobs: extract_blobs(&raw),
        image_files: container.image_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
        FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn guid_to_key_formats_session_local() {
        let g = obj(vec![
            ("sessionID", FigValue::Uint(7)),
            ("localID", FigValue::Uint(42)),
        ]);
        assert_eq!(FigGuid::from_value(&g).unwrap().to_key(), "7:42");
    }

    #[test]
    fn matrix_extracts_six_fields() {
        let m = obj(vec![
            ("m00", FigValue::Float(1.0)),
            ("m11", FigValue::Float(1.0)),
            ("m02", FigValue::Float(12.0)),
            ("m12", FigValue::Float(34.0)),
        ]);
        let mat = FigMatrix::from_value(&m).unwrap();
        assert_eq!(mat.m02, 12.0);
        assert_eq!(mat.m12, 34.0);
        assert_eq!(mat.m01, 0.0);
    }

    #[test]
    fn color_alpha_optional() {
        let opaque = obj(vec![
            ("r", FigValue::Float(1.0)),
            ("g", FigValue::Float(0.5)),
            ("b", FigValue::Float(0.0)),
        ]);
        let c = FigColor::from_value(&opaque).unwrap();
        assert_eq!(c.a, None);
        assert_eq!(c.g, 0.5);
    }

    #[test]
    fn extract_blobs_handles_bytes_and_strings() {
        let raw = obj(vec![(
            "blobs",
            FigValue::Array(vec![
                obj(vec![("bytes", FigValue::Bytes(vec![1, 2, 3]))]),
                FigValue::Str("inline".into()),
                FigValue::Int(0),
            ]),
        )]);
        let blobs = extract_blobs(&raw);
        assert_eq!(blobs.len(), 3);
        assert_eq!(blobs[0].as_bytes(), Some(&[1u8, 2, 3][..]));
        assert_eq!(blobs[1], BlobOrString::Str("inline".into()));
        assert_eq!(blobs[2], BlobOrString::Bytes(vec![]));
    }

    #[test]
    fn find_node_changes_falls_back_to_guid_array() {
        let raw = obj(vec![(
            "someOtherKey",
            FigValue::Array(vec![obj(vec![(
                "guid",
                obj(vec![("sessionID", FigValue::Uint(0))]),
            )])]),
        )]);
        let nc = find_node_changes(&raw).expect("fallback finds the guid array");
        assert_eq!(nc.len(), 1);
    }
}
