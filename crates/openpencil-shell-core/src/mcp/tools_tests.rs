//! Tests for `mcp::tools`. Sibling file so the impl stays under
//! the 800-line cap.

use super::tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

fn doc_with_variables() -> crate::document::Document {
    use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    doc.var_table.variables.push(Variable {
        name: "color-1".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
    });
    doc.var_table.variables.push(Variable {
        name: "spacing".into(),
        kind: VariableKind::Number,
        value: VariableValue::Scalar(VariableScalar::Num(16.0)),
    });
    doc.var_table.variables.push(Variable {
        name: "compact".into(),
        kind: VariableKind::Boolean,
        value: VariableValue::Scalar(VariableScalar::Bool(true)),
    });
    doc
}

#[test]
fn list_variables_reports_count_and_records() {
    let doc = doc_with_variables();
    let tool = list_variables_snapshot(&doc);
    assert_eq!(tool.variables.len(), 3);
    assert_eq!(tool.variables[0].name, "color-1");
    assert_eq!(tool.variables[0].kind, "color");
    assert_eq!(tool.variables[0].value, "#ff0000");
    assert_eq!(tool.variables[1].kind, "number");
    assert_eq!(tool.variables[1].value, "16");
    assert_eq!(tool.variables[2].kind, "boolean");
    assert_eq!(tool.variables[2].value, "true");
}

#[test]
fn list_variables_encodes_for_wire() {
    let doc = doc_with_variables();
    let tool = list_variables_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"3".to_string()));
            let encoded = out.get("variables").expect("variables field");
            assert!(encoded.contains("color-1|color|#ff0000"));
            assert!(encoded.contains("spacing|number|16"));
            assert!(encoded.contains("compact|boolean|true"));
            // Three records separated by `;`.
            assert_eq!(encoded.matches(';').count(), 2);
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_active_theme_reports_active_axes_and_options() {
    use crate::document::{Document, ThemeAxis};
    let mut doc = Document::empty();
    doc.var_table.themes.push(ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into(), "sepia".into()],
    });
    doc.var_table.themes.push(ThemeAxis {
        name: "density".into(),
        values: vec!["compact".into(), "comfortable".into()],
    });
    doc.var_table.set_active_theme("mode", "dark");
    // Only `mode` is in active_theme; `density` is defined but
    // not yet selected.
    let tool = get_active_theme_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("axis_count"), Some(&"2".to_string()));
            // Active selection: only mode|dark.
            assert_eq!(out.get("axes"), Some(&"mode|dark".to_string()));
            // Options carry both axes + full value lists.
            let opts = out.get("options").expect("options field");
            assert!(opts.contains("density|compact,comfortable"));
            assert!(opts.contains("mode|light,dark,sepia"));
            // Two records → one separator.
            assert_eq!(opts.matches(';').count(), 1);
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_active_theme_empty_document_is_zero() {
    let doc = crate::document::Document::empty();
    let tool = get_active_theme_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("axis_count"), Some(&"0".to_string()));
            assert_eq!(out.get("axes"), Some(&String::new()));
            assert_eq!(out.get("options"), Some(&String::new()));
        }
        _ => panic!("expected Ok"),
    }
}

#[test]
fn get_active_theme_axes_field_does_not_escape_commas() {
    // Codex stop-gate (third pass): `axes` is a 2-level format
    // (`;`-records of `|`-pairs) identical in shape to
    // list_variables, so it must keep the legacy 3-char escape
    // set (`\;|`) — clients decode with standard
    // unescape_record_field which doesn't strip `\,`.
    //
    // The previous commit had me using escape_layered_field for
    // BOTH `axes` and `options`. Splitting them: `axes` uses
    // `escape_record_field` (no comma escape); `options` uses
    // `escape_layered_field` (comma escape required for inner
    // value list).
    use crate::document::Document;
    let mut doc = Document::empty();
    // Active selection where the value carries a comma (rare
    // but valid — schema doesn't forbid).
    doc.var_table
        .set_active_theme("axis,with,commas", "value,with,commas");
    let tool = get_active_theme_snapshot(&doc);
    let axes = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o.get("axes").unwrap().clone(),
        _ => panic!(),
    };
    // axis name + value commas must pass through verbatim —
    // the standard list_variables-compatible decoder would
    // otherwise see `\,` as a literal 2-char sequence.
    assert!(
        axes.contains("axis,with,commas|value,with,commas"),
        "axes must not escape commas; got {axes}"
    );
    assert!(
        !axes.contains("\\,"),
        "axes must not contain backslash-comma; got {axes}"
    );
}

#[test]
fn list_variables_does_not_escape_commas_backward_compat() {
    // Codex stop-gate (second pass): an earlier fix added `,`
    // to the shared escape set, which changed the wire output
    // of `list_variables` for values containing commas. That's
    // a backward-incompatible change for anyone decoding the
    // schema with the original two-delimiter unescape rules.
    // The split now keeps `list_variables` on the legacy
    // 3-char escape set (`\;|`); only `get_active_theme` uses
    // the extended set that includes `,`.
    use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    doc.var_table.variables.push(Variable {
        name: "msg".into(),
        kind: VariableKind::String,
        value: VariableValue::Scalar(VariableScalar::Str("red, white, blue".into())),
    });
    let tool = list_variables_snapshot(&doc);
    let encoded = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o.get("variables").unwrap().clone(),
        _ => panic!(),
    };
    // The literal commas must pass through unescaped — pre-
    // this-commit clients depend on that.
    assert!(
        encoded.contains("red, white, blue"),
        "commas must NOT be escaped in list_variables output; got {encoded}"
    );
    assert!(
        !encoded.contains("\\,"),
        "no backslash-comma should appear in list_variables: {encoded}"
    );
}

#[test]
fn get_active_theme_round_trips_comma_in_value() {
    // Codex stop-gate: theme values can legitimately contain
    // commas (e.g. a description-style value like "red, white,
    // and blue"). The comma joiner inside `options` would
    // otherwise mis-split into multiple fake values.
    //
    // Decoding strategy is LAYERED: each split level only
    // unescapes the delimiter for THAT level, leaving other
    // escapes intact for inner splits. A walker that unescapes
    // every delimiter at once over-decodes the comma escapes
    // before the inner split sees them.
    use crate::document::{Document, ThemeAxis};
    let mut doc = Document::empty();
    doc.var_table.themes.push(ThemeAxis {
        name: "palette".into(),
        values: vec!["a,b,c".into(), "plain".into()],
    });
    let tool = get_active_theme_snapshot(&doc);
    let opts = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o.get("options").unwrap().clone(),
        _ => panic!(),
    };
    // Split on `|` — only `\|` is unescaped at this level;
    // `\,` and `\;` pass through verbatim into the inner blob.
    let fields = layered_split(&opts, '|');
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0], "palette");
    // Split fields[1] on `,` — only `\,` is unescaped here.
    let values = layered_split(&fields[1], ',');
    assert_eq!(
        values,
        vec!["a,b,c".to_string(), "plain".to_string()],
        "comma must round-trip through escape"
    );
}

/// Split `s` on unescaped `delim`. Only `\<delim>` is unescaped
/// to a literal `<delim>` byte; every other backslash sequence
/// passes through verbatim so inner splits can decode their own
/// delimiters. Decoder-side counterpart to `escape_record_field`
/// for hierarchical wire formats like `get_active_theme.options`
/// (`axis|v1,v2,v3;axis2|v1,v2`).
fn layered_split(s: &str, delim: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(&n) if n == delim => {
                    chars.next();
                    cur.push(delim);
                }
                _ => cur.push(c),
            }
        } else if c == delim {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

#[test]
fn get_active_theme_escapes_pipe_and_semicolon_in_values() {
    // Theme axis values with weird payloads (unusual but valid
    // per the canonical schema — values are strings). The
    // backslash-escape rules from list_variables apply here too.
    use crate::document::{Document, ThemeAxis};
    let mut doc = Document::empty();
    doc.var_table.themes.push(ThemeAxis {
        name: "weird".into(),
        values: vec!["a|b".into(), "c;d".into(), "e\\f".into()],
    });
    let tool = get_active_theme_snapshot(&doc);
    let opts = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o.get("options").unwrap().clone(),
        _ => panic!(),
    };
    // Pipe inside values is escaped → split on `|` yields exactly
    // 2 fields (axis | comma-joined-values).
    let mut depth_safe_split: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = opts.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&n) = chars.peek() {
                if matches!(n, '\\' | ';' | '|') {
                    cur.push(chars.next().unwrap());
                    continue;
                }
            }
            cur.push(c);
        } else if c == '|' {
            depth_safe_split.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    depth_safe_split.push(cur);
    assert_eq!(depth_safe_split.len(), 2, "expected axis|values, got {depth_safe_split:?}");
    assert_eq!(depth_safe_split[0], "weird");
    // Values comma-joined; each value individually escaped.
    // Decode the comma-separated values and verify each survived.
    let decoded: Vec<String> = depth_safe_split[1]
        .split(',')
        .map(unescape_record_field)
        .collect();
    assert_eq!(decoded, vec!["a|b", "c;d", "e\\f"]);
}

#[test]
fn list_variables_empty_document_returns_zero_count() {
    let doc = crate::document::Document::empty();
    let tool = list_variables_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"0".to_string()));
            assert_eq!(out.get("variables"), Some(&String::new()));
        }
        _ => panic!("expected Ok"),
    }
}

#[test]
fn escape_record_field_round_trips_pipe_semicolon_backslash() {
    // Codex stop-gate caught: the canonical `.op` schema does
    // NOT forbid `;`, `|`, or `\` in variable names or string
    // values. The encoding must round-trip every such payload.
    for raw in &[
        "plain",
        "a|b",
        "a;b",
        "a\\b",
        "a|b;c\\d",
        "\\",
        ";;|||",
        "",
        "color/primary",
        "label with space",
    ] {
        let escaped = escape_record_field(raw);
        let back = unescape_record_field(&escaped);
        assert_eq!(&back, raw, "round-trip failed for {raw:?}");
    }
}

#[test]
fn list_variables_encodes_string_value_with_special_chars() {
    // String variable whose value contains every delimiter.
    // The wire format must keep the record boundary clear AND
    // recover the original payload on decode.
    use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    doc.var_table.variables.push(Variable {
        name: "msg".into(),
        kind: VariableKind::String,
        value: VariableValue::Scalar(VariableScalar::Str("a|b;c\\d".into())),
    });
    let tool = list_variables_snapshot(&doc);
    let out = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o,
        other => panic!("expected Ok, got {other:?}"),
    };
    assert_eq!(out.get("count"), Some(&"1".to_string()));
    let encoded = out.get("variables").expect("variables field");
    // Pipe inside the value is escaped, so splitting on `|`
    // still yields exactly 3 fields (name | kind | value).
    // Decode each field + verify the original payload.
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&n) = chars.peek() {
                if matches!(n, '\\' | ';' | '|') {
                    cur.push(chars.next().unwrap());
                    continue;
                }
            }
            cur.push(c);
        } else if c == '|' {
            fields.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    assert_eq!(fields.len(), 3, "expected name|kind|value, got {fields:?}");
    assert_eq!(fields[0], "msg");
    assert_eq!(fields[1], "string");
    assert_eq!(fields[2], "a|b;c\\d");
}

fn doc_with_color_var(name: &str, hex: &str) -> crate::document::Document {
    use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    doc.var_table.variables.push(Variable {
        name: name.into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str(hex.into())),
    });
    doc
}

#[test]
fn set_variable_color_validates_args_and_returns_command() {
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    args.insert("hex".into(), "#00ff00".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, cmd) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            match cmd {
                crate::mcp::McpCommand::SetVariableColor { name, hex } => {
                    assert_eq!(name, "brand");
                    assert_eq!(hex, "#00ff00");
                }
                other => panic!("expected SetVariableColor, got {other:?}"),
            }
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

#[test]
fn set_variable_color_errors_on_missing_args() {
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
        }
        _ => panic!("expected MissingArgument"),
    }
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("hex"));
        }
        _ => panic!("expected MissingArgument"),
    }
}

#[test]
fn set_variable_color_errors_on_unknown_variable() {
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "no-such-var".into());
    args.insert("hex".into(), "#000000".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("no-such-var"));
        }
        _ => panic!("expected ToolFailed"),
    }
}

#[test]
fn set_variable_color_errors_on_invalid_hex() {
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    for bad in &["not-hex", "ff00ff", "#12", "#fffffg"] {
        args.insert("hex".into(), (*bad).into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "hex={bad}");
            }
            _ => panic!("expected InvalidArgument for {bad}"),
        }
    }
}

#[test]
fn apply_mcp_command_routes_set_variable_color_to_var_table() {
    // End-to-end: tool validates → registry returns
    // OkWithCommand → host's `var_table.apply_mcp_command`
    // writes through to the storage. resolve_color reads the
    // new value back.
    use crate::mcp::McpCommand;
    let mut doc = doc_with_color_var("brand", "#ff8800");
    let cmd = McpCommand::SetVariableColor {
        name: "brand".into(),
        hex: "#11ccaa".into(),
    };
    assert!(doc.var_table.apply_mcp_command(&cmd));
    let c = doc.var_table.resolve_color("brand").unwrap();
    assert!((c.r - 0x11 as f32 / 255.0).abs() < 0.01);
    assert!((c.g - 0xcc as f32 / 255.0).abs() < 0.01);
    assert!((c.b - 0xaa as f32 / 255.0).abs() < 0.01);
}
