//! Kiwi decoder tests. A small in-test Kiwi *writer* builds schema +
//! data fixtures so the decoder is exercised on real round-trips
//! rather than hand-counted byte arrays.

use super::*;

/// Minimal Kiwi wire writer — inverse of [`ByteBuffer`]'s reads.
#[derive(Default)]
struct ByteWriter {
    out: Vec<u8>,
}

impl ByteWriter {
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
        // Zigzag: (v << 1) ^ (v >> 31).
        let zz = ((v << 1) ^ (v >> 31)) as u32;
        self.var_uint(zz);
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

    fn byte_array(&mut self, bytes: &[u8]) {
        self.var_uint(bytes.len() as u32);
        self.out.extend_from_slice(bytes);
    }
}

/// Build the fixture schema: enum `Kind`, struct `Vec2`, message
/// `Message`. Definition indices: Kind=0, Vec2=1, Message=2.
fn fixture_schema_bytes() -> Vec<u8> {
    let mut w = ByteWriter::default();
    w.var_uint(3); // definition count

    // -- def 0: enum Kind { NONE=0, RECT=1, TEXT=2 } --
    w.string("Kind");
    w.byte(0); // ENUM
    w.var_uint(3);
    for (name, val) in [("NONE", 0u32), ("RECT", 1), ("TEXT", 2)] {
        w.string(name);
        w.var_int(0); // enum field type is unused; kiwi writes 0
        w.byte(0); // not array
        w.var_uint(val);
    }

    // -- def 1: struct Vec2 { x: float, y: float } --
    w.string("Vec2");
    w.byte(1); // STRUCT
    w.var_uint(2);
    for name in ["x", "y"] {
        w.string(name);
        w.var_int(-5); // native "float" → !4 = -5
        w.byte(0);
        w.var_uint(0); // struct fields ignore value
    }

    // -- def 2: message Message { name:1, kind:2, pos:3, tags:4, blob:5, count:6 } --
    w.string("Message");
    w.byte(2); // MESSAGE
    w.var_uint(6);
    // name: string (native idx5 → -6)
    w.string("name");
    w.var_int(-6);
    w.byte(0);
    w.var_uint(1);
    // kind: Kind (def index 0)
    w.string("kind");
    w.var_int(0);
    w.byte(0);
    w.var_uint(2);
    // pos: Vec2 (def index 1)
    w.string("pos");
    w.var_int(1);
    w.byte(0);
    w.var_uint(3);
    // tags: string[] (native idx5 → -6, array)
    w.string("tags");
    w.var_int(-6);
    w.byte(1);
    w.var_uint(4);
    // blob: byte[] (native idx1 → -2, array)
    w.string("blob");
    w.var_int(-2);
    w.byte(1);
    w.var_uint(5);
    // count: int (native idx2 → -3)
    w.string("count");
    w.var_int(-3);
    w.byte(0);
    w.var_uint(6);

    w.out
}

#[test]
fn decodes_binary_schema() {
    let schema = decode_binary_schema(&fixture_schema_bytes()).expect("schema decodes");
    assert_eq!(schema.definitions.len(), 3);
    assert_eq!(schema.definitions[0].kind, DefKind::Enum);
    assert_eq!(schema.definitions[1].kind, DefKind::Struct);
    assert_eq!(schema.definitions[2].kind, DefKind::Message);
    // Native + cross-definition type names resolved.
    assert_eq!(schema.definitions[1].fields[0].type_name, "float");
    assert_eq!(schema.definitions[2].fields[0].type_name, "string");
    assert_eq!(schema.definitions[2].fields[1].type_name, "Kind");
    assert_eq!(schema.definitions[2].fields[2].type_name, "Vec2");
    assert!(schema.definitions[2].fields[3].is_array);
}

#[test]
fn decodes_message_data() {
    let schema = decode_binary_schema(&fixture_schema_bytes()).unwrap();
    let mut w = ByteWriter::default();
    // name = "hello"
    w.var_uint(1);
    w.string("hello");
    // kind = RECT (enum value 1)
    w.var_uint(2);
    w.var_uint(1);
    // pos = Vec2 { x: 1.5, y: -2.0 }
    w.var_uint(3);
    w.var_float(1.5);
    w.var_float(-2.0);
    // tags = ["a", "bb"]
    w.var_uint(4);
    w.var_uint(2);
    w.string("a");
    w.string("bb");
    // blob = [1, 2, 3]
    w.var_uint(5);
    w.byte_array(&[1, 2, 3]);
    // count = -7
    w.var_uint(6);
    w.var_int(-7);
    // terminator
    w.var_uint(0);

    let value = decode_message(&schema, &w.out).expect("message decodes");
    assert_eq!(value.get("name").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(value.get("kind").and_then(|v| v.as_str()), Some("RECT"));
    let pos = value.get("pos").expect("pos present");
    assert_eq!(pos.get("x").and_then(|v| v.as_f64()), Some(1.5));
    assert_eq!(pos.get("y").and_then(|v| v.as_f64()), Some(-2.0));
    let tags = value.get("tags").and_then(|v| v.as_array()).unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[1].as_str(), Some("bb"));
    assert_eq!(
        value.get("blob").and_then(|v| v.as_bytes()),
        Some(&[1u8, 2, 3][..])
    );
    assert_eq!(value.get("count").and_then(|v| v.as_f64()), Some(-7.0));
}

#[test]
fn omitted_message_fields_are_absent() {
    let schema = decode_binary_schema(&fixture_schema_bytes()).unwrap();
    let mut w = ByteWriter::default();
    w.var_uint(1);
    w.string("only-name");
    w.var_uint(0); // terminator — every other field omitted
    let value = decode_message(&schema, &w.out).unwrap();
    assert_eq!(
        value.get("name").and_then(|v| v.as_str()),
        Some("only-name")
    );
    assert!(value.get("pos").is_none());
    assert!(value.get("count").is_none());
}

#[test]
fn var_float_round_trips() {
    let mut w = ByteWriter::default();
    let cases = [0.0f32, 1.0, -1.0, 3.14159, 1e-6, -42.5, 65536.0];
    for v in cases {
        w.var_float(v);
    }
    let mut bb = ByteBuffer::new(&w.out);
    for v in cases {
        assert_eq!(bb.read_var_float().unwrap(), v);
    }
}

#[test]
fn var_int_round_trips_signed() {
    let mut w = ByteWriter::default();
    let cases = [
        0i32,
        1,
        -1,
        127,
        -128,
        100_000,
        -100_000,
        i32::MAX,
        i32::MIN,
    ];
    for v in cases {
        w.var_int(v);
    }
    let mut bb = ByteBuffer::new(&w.out);
    for v in cases {
        assert_eq!(bb.read_var_int().unwrap(), v);
    }
}

#[test]
fn unknown_message_field_id_errors() {
    let schema = decode_binary_schema(&fixture_schema_bytes()).unwrap();
    let mut w = ByteWriter::default();
    w.var_uint(99); // no field has id 99
    let err = decode_message(&schema, &w.out);
    assert!(matches!(err, Err(KiwiError::UnknownField(_))));
}

#[test]
fn truncated_buffer_errors() {
    let schema = decode_binary_schema(&fixture_schema_bytes()).unwrap();
    let mut w = ByteWriter::default();
    w.var_uint(1); // field id 1 = name (string) — but no string follows
    let err = decode_message(&schema, &w.out);
    assert!(matches!(err, Err(KiwiError::OutOfBounds)));
}
