//! Vector-decoder tests.

use super::*;

fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn push_f32(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[test]
fn decodes_command_blob() {
    let mut blob = Vec::new();
    blob.push(0x01); // M
    push_f32(&mut blob, 1.0);
    push_f32(&mut blob, 2.0);
    blob.push(0x02); // L
    push_f32(&mut blob, 3.0);
    push_f32(&mut blob, 4.0);
    blob.push(0x00); // Z
    assert_eq!(
        decode_figma_path_blob(&blob).as_deref(),
        Some("M1 2 L3 4 Z")
    );
}

#[test]
fn decodes_cubic_command() {
    let mut blob = Vec::new();
    blob.push(0x01);
    push_f32(&mut blob, 0.0);
    push_f32(&mut blob, 0.0);
    blob.push(0x04); // C
    for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        push_f32(&mut blob, v);
    }
    assert_eq!(
        decode_figma_path_blob(&blob).as_deref(),
        Some("M0 0 C1 2 3 4 5 6")
    );
}

#[test]
fn short_blob_is_none() {
    assert!(decode_figma_path_blob(&[0x01, 0, 0]).is_none());
}

#[test]
fn path_bounds_from_coordinates() {
    let b = compute_svg_path_bounds("M1 2 L3 4 L-5 10 Z").expect("bounds");
    assert_eq!(b.min_x, -5.0);
    assert_eq!(b.min_y, 2.0);
    assert_eq!(b.max_x, 3.0);
    assert_eq!(b.max_y, 10.0);
}

#[test]
fn path_bounds_empty_string_none() {
    assert!(compute_svg_path_bounds("").is_none());
}

#[test]
fn vector_path_from_fill_geometry() {
    let mut blob = Vec::new();
    blob.push(0x01);
    push_f32(&mut blob, 0.0);
    push_f32(&mut blob, 0.0);
    blob.push(0x02);
    push_f32(&mut blob, 8.0);
    push_f32(&mut blob, 0.0);
    let node = obj(vec![
        (
            "fillPaints",
            FigValue::Array(vec![obj(vec![("type", FigValue::Str("SOLID".into()))])]),
        ),
        (
            "fillGeometry",
            FigValue::Array(vec![obj(vec![("commandsBlob", FigValue::Uint(0))])]),
        ),
    ]);
    let blobs = [BlobOrString::Bytes(blob)];
    assert_eq!(
        decode_figma_vector_path(&node, &blobs).as_deref(),
        Some("M0 0 L8 0")
    );
}

#[test]
fn vector_network_straight_segment() {
    let mut blob = Vec::new();
    push_u32(&mut blob, 2); // vertexCount
    push_f32(&mut blob, 0.0);
    push_f32(&mut blob, 0.0); // v0
    push_f32(&mut blob, 10.0);
    push_f32(&mut blob, 0.0); // v1
    push_u32(&mut blob, 1); // segmentCount
    push_u32(&mut blob, 0); // start
    push_u32(&mut blob, 1); // end
    for _ in 0..4 {
        push_f32(&mut blob, 0.0); // zero tangents → straight
    }
    let node = obj(vec![(
        "vectorData",
        obj(vec![("vectorNetworkBlob", FigValue::Uint(0))]),
    )]);
    let blobs = [BlobOrString::Bytes(blob)];
    assert_eq!(
        decode_vector_network_blob(&node, &blobs).as_deref(),
        Some("M0 0 L10 0")
    );
}

#[test]
fn r_strips_trailing_zeros() {
    assert_eq!(r(1.5), "1.5");
    assert_eq!(r(2.0), "2");
    assert_eq!(r(0.00001), "0");
    assert_eq!(r(-0.0), "0");
}
