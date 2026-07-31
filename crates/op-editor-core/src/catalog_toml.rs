//! Shared value parsers for the compile-time embedded catalogue assets.
//!
//! Both the Prompt Center (`prompt_center.toml`) and the Scene Template
//! Center (`scene_templates.toml`) ship as checked-in TOML written in a
//! deliberately small subset, parsed here rather than through a TOML crate
//! so the catalogues stay dependency-free and wasm-safe.
//!
//! Every function reports failures as `(line, message)` instead of a
//! catalogue-specific error type. Each catalogue keeps its own error type —
//! their `Display` impls name their own asset file — and converts at the
//! call site. Sharing the *parsing* without sharing the error type is what
//! lets a second catalogue reuse this without either one bending its
//! public surface.

/// A parse failure: one-based line number and a human-readable reason.
pub(crate) type ValueError = (usize, String);

fn err(line: usize, message: impl Into<String>) -> ValueError {
    (line, message.into())
}

/// A quoted string, either a basic `"…"` string or a `'''…'''` literal.
pub(crate) fn parse_text(value: &str, line: usize) -> Result<String, ValueError> {
    if let Some(inner) = value
        .strip_prefix("'''")
        .and_then(|rest| rest.strip_suffix("'''"))
    {
        if inner.contains("'''") {
            return Err(err(
                line,
                "literal string contains an unsupported triple quote",
            ));
        }
        return Ok(inner.to_string());
    }
    parse_basic_string(value, line)
}

/// A `"…"` string with the escape set the catalogues actually use.
pub(crate) fn parse_basic_string(value: &str, line: usize) -> Result<String, ValueError> {
    let inner = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| err(line, "expected a quoted string"))?;
    let mut output = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        match character {
            '"' => return Err(err(line, "unescaped quote in basic string")),
            '\\' => {
                let escaped = chars
                    .next()
                    .ok_or_else(|| err(line, "unterminated string escape"))?;
                output.push(match escaped {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => {
                        return Err(err(
                            line,
                            format!("unsupported string escape `\\{escaped}`"),
                        ))
                    }
                });
            }
            _ => output.push(character),
        }
    }
    Ok(output)
}

/// A `["a", "b"]` array of basic strings. An empty array parses to an empty
/// vector; whether that is legal is the catalogue's own rule.
pub(crate) fn parse_string_array(value: &str, line: usize) -> Result<Vec<String>, ValueError> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| err(line, "expected a string array"))?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| parse_basic_string(item.trim(), line))
        .collect()
}

/// A required field that must have been seen before the entry closed.
pub(crate) fn required<T>(value: Option<T>, field: &str, line: usize) -> Result<T, ValueError> {
    value.ok_or_else(|| err(line, format!("missing `{field}`")))
}

/// A field whose value must not be blank.
pub(crate) fn non_empty(value: String, field: &str, line: usize) -> Result<String, ValueError> {
    if value.trim().is_empty() {
        Err(err(line, format!("`{field}` must not be empty")))
    } else {
        Ok(value)
    }
}

/// Assign a field exactly once — a repeated key is an authoring mistake, not
/// a last-one-wins override.
pub(crate) fn set_once<T>(
    slot: &mut Option<T>,
    value: T,
    field: &str,
    line: usize,
) -> Result<(), ValueError> {
    if slot.is_some() {
        return Err(err(line, format!("duplicate `{field}`")));
    }
    *slot = Some(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_strings_round_trip_the_supported_escapes() {
        assert_eq!(parse_text(r#""plain""#, 1).unwrap(), "plain");
        assert_eq!(parse_text(r#""a\nb""#, 1).unwrap(), "a\nb");
        assert_eq!(parse_text(r#""q\"q""#, 1).unwrap(), "q\"q");
        assert_eq!(
            parse_text("'''literal '' body'''", 1).unwrap(),
            "literal '' body"
        );
    }

    #[test]
    fn malformed_values_report_their_line() {
        assert_eq!(parse_text("bare", 7).unwrap_err().0, 7);
        assert_eq!(
            parse_text("bare", 7).unwrap_err().1,
            "expected a quoted string"
        );
        // A value ending mid-escape: the closing quote is consumed as the
        // escaped character, leaving the backslash dangling.
        assert_eq!(
            parse_text(r#""trailing\""#, 3).unwrap_err().1,
            "unterminated string escape"
        );
        assert_eq!(
            parse_text(r#""bad\q""#, 4).unwrap_err().1,
            "unsupported string escape `\\q`"
        );
    }

    #[test]
    fn string_arrays_accept_empty_and_reject_unquoted_items() {
        assert_eq!(parse_string_array("[]", 1).unwrap(), Vec::<String>::new());
        assert_eq!(parse_string_array(r#"["a", "b"]"#, 1).unwrap(), ["a", "b"]);
        assert!(parse_string_array("[a]", 1).is_err());
    }

    #[test]
    fn set_once_rejects_a_repeated_key() {
        let mut slot = None;
        assert!(set_once(&mut slot, 1, "id", 1).is_ok());
        assert_eq!(
            set_once(&mut slot, 2, "id", 9).unwrap_err().1,
            "duplicate `id`"
        );
        assert_eq!(slot, Some(1), "the first value wins");
    }
}
