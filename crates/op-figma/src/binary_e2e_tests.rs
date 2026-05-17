//! End-to-end binary `.fig` test — assembles a real Kiwi-encoded
//! `.fig` container from scratch (schema chunk + data chunk, both
//! deflate-compressed) and drives it through the full
//! [`parse_fig_binary`] pipeline.

use crate::{parse_fig_binary, FigLayoutMode};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use std::io::Write;

/// Minimal Kiwi wire writer (mirrors the in-crate decoder).
#[derive(Default)]
struct W {
    out: Vec<u8>,
}

impl W {
    fn byte(&mut self, b: u8) {
        self.out.push(b);
    }
    fn var_uint(&mut self, mut v: u32) {
        loop {
            let mut byte = (v & 127) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 128;
            }
            self.out.push(byte);
            if v == 0 {
                break;
            }
        }
    }
    fn var_int(&mut self, v: i32) {
        self.var_uint(((v << 1) ^ (v >> 31)) as u32);
    }
    fn var_float(&mut self, v: f32) {
        let stored = v.to_bits().rotate_left(9);
        if stored & 0xff == 0 {
            self.out.push(0);
        } else {
            self.out.extend_from_slice(&stored.to_le_bytes());
        }
    }
    fn string(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
        self.out.push(0);
    }
    /// Schema field row: name, type code, array flag, value.
    fn field(&mut self, name: &str, type_code: i32, is_array: bool, value: u32) {
        self.string(name);
        self.var_int(type_code);
        self.byte(if is_array { 1 } else { 0 });
        self.var_uint(value);
    }
}

/// Build the fixture schema chunk. Definition indices: GUID=0, Vec=1,
/// Matrix=2, ParentIndex=3, NodeType=4, NodeChange=5, Message=6.
fn build_schema() -> Vec<u8> {
    let mut w = W::default();
    w.var_uint(7); // definition count

    // 0: struct GUID { sessionID: uint, localID: uint }
    w.string("GUID");
    w.byte(1);
    w.var_uint(2);
    w.field("sessionID", -4, false, 0);
    w.field("localID", -4, false, 0);

    // 1: struct Vec { x: float, y: float }
    w.string("Vec");
    w.byte(1);
    w.var_uint(2);
    w.field("x", -5, false, 0);
    w.field("y", -5, false, 0);

    // 2: struct Matrix { m00..m12: float }
    w.string("Matrix");
    w.byte(1);
    w.var_uint(6);
    for m in ["m00", "m01", "m02", "m10", "m11", "m12"] {
        w.field(m, -5, false, 0);
    }

    // 3: struct ParentIndex { guid: GUID, position: string }
    w.string("ParentIndex");
    w.byte(1);
    w.var_uint(2);
    w.field("guid", 0, false, 0);
    w.field("position", -6, false, 0);

    // 4: enum NodeType { DOCUMENT=1, CANVAS=2, FRAME=3, RECTANGLE=4 }
    w.string("NodeType");
    w.byte(0);
    w.var_uint(4);
    w.field("DOCUMENT", 0, false, 1);
    w.field("CANVAS", 0, false, 2);
    w.field("FRAME", 0, false, 3);
    w.field("RECTANGLE", 0, false, 4);

    // 5: message NodeChange
    w.string("NodeChange");
    w.byte(2);
    w.var_uint(6);
    w.field("guid", 0, false, 1);
    w.field("parentIndex", 3, false, 2);
    w.field("type", 4, false, 3);
    w.field("name", -6, false, 4);
    w.field("size", 1, false, 5);
    w.field("transform", 2, false, 6);

    // 6: message Message { nodeChanges: NodeChange[] }
    w.string("Message");
    w.byte(2);
    w.var_uint(1);
    w.field("nodeChanges", 5, true, 1);

    w.out
}

/// One node-change descriptor for the data chunk.
struct NodeSpec {
    local_id: u32,
    parent: Option<(u32, u32, &'static str)>, // (session, local, position)
    type_value: u32,
    name: &'static str,
    size: Option<(f32, f32)>,
    transform: Option<[f32; 6]>,
}

fn encode_guid(w: &mut W, session: u32, local: u32) {
    w.var_uint(session);
    w.var_uint(local);
}

fn encode_node(w: &mut W, n: &NodeSpec) {
    // guid — field id 1.
    w.var_uint(1);
    encode_guid(w, 0, n.local_id);
    // parentIndex — field id 2.
    if let Some((s, l, pos)) = n.parent {
        w.var_uint(2);
        encode_guid(w, s, l);
        w.string(pos);
    }
    // type — field id 3.
    w.var_uint(3);
    w.var_uint(n.type_value);
    // name — field id 4.
    w.var_uint(4);
    w.string(n.name);
    // size — field id 5.
    if let Some((x, y)) = n.size {
        w.var_uint(5);
        w.var_float(x);
        w.var_float(y);
    }
    // transform — field id 6.
    if let Some(m) = n.transform {
        w.var_uint(6);
        for v in m {
            w.var_float(v);
        }
    }
    w.var_uint(0); // end NodeChange
}

/// Build the data chunk: DOCUMENT → CANVAS → RECTANGLE.
fn build_data() -> Vec<u8> {
    let nodes = [
        NodeSpec {
            local_id: 1,
            parent: None,
            type_value: 1, // DOCUMENT
            name: "Doc",
            size: None,
            transform: None,
        },
        NodeSpec {
            local_id: 2,
            parent: Some((0, 1, "a")),
            type_value: 2, // CANVAS
            name: "Page 1",
            size: None,
            transform: None,
        },
        NodeSpec {
            local_id: 3,
            parent: Some((0, 2, "a")),
            type_value: 4, // RECTANGLE
            name: "Box",
            size: Some((100.0, 50.0)),
            transform: Some([1.0, 0.0, 10.0, 0.0, 1.0, 20.0]),
        },
    ];
    let mut w = W::default();
    w.var_uint(1); // Message field id 1 = nodeChanges
    w.var_uint(nodes.len() as u32);
    for n in &nodes {
        encode_node(&mut w, n);
    }
    w.var_uint(0); // end Message
    w.out
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

/// Assemble a bare `fig-kiwi` container around the two chunks.
fn build_fig(schema: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"fig-kiwi");
    buf.extend_from_slice(&[0, 0, 0, 0]); // delimiter
    for chunk in [schema, data] {
        let comp = deflate(chunk);
        buf.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        buf.extend_from_slice(&comp);
    }
    buf
}

#[test]
fn parses_a_full_binary_fig_into_a_document() {
    let fig = build_fig(&build_schema(), &build_data());
    let import = parse_fig_binary(&fig, "Test", FigLayoutMode::OpenPencil)
        .expect("binary .fig parses end-to-end");

    let pages = import.document.pages.expect("document has pages");
    assert_eq!(pages.len(), 1, "one user page");
    assert_eq!(pages[0].name, "Page 1");
    assert_eq!(pages[0].children.len(), 1, "one rectangle on the page");

    match &pages[0].children[0] {
        PenNode::Rectangle(rect) => {
            assert_eq!(rect.base.name.as_deref(), Some("Box"));
            // transform translation (m02, m12) → top-left.
            assert_eq!(rect.base.x, Some(10.0));
            assert_eq!(rect.base.y, Some(20.0));
            assert!(matches!(
                rect.container.width,
                Some(SizingBehavior::Number(w)) if w == 100.0
            ));
            assert!(matches!(
                rect.container.height,
                Some(SizingBehavior::Number(h)) if h == 50.0
            ));
        }
        other => panic!("expected a Rectangle, got {other:?}"),
    }
}

#[test]
fn document_serializes_to_canonical_json() {
    let fig = build_fig(&build_schema(), &build_data());
    let import = parse_fig_binary(&fig, "Test", FigLayoutMode::OpenPencil).unwrap();
    // The converted document must round-trip through the canonical
    // schema's serde representation.
    let json = serde_json::to_string(&import.document).expect("serializes");
    assert!(json.contains("\"Page 1\""));
    assert!(json.contains("\"Box\""));
}
