//! Public extraction helper for modify-script replay.

use serde_json::Value;

/// Extract raw `I(parent, object)` payloads from a recorded batch-design program.
///
/// Unlike the normal `batch_design` apply path, this keeps author-provided ids
/// inside the object tree untouched. Modify uses that to diff against the live
/// document before deciding which subtrees are truly new.
pub fn parse_program_objects(program: &str) -> Vec<(String, Value)> {
    crate::batch_design::split_operations(program)
        .into_iter()
        .filter_map(|line| parse_program_object(&line))
        .collect()
}

fn parse_program_object(line: &str) -> Option<(String, Value)> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    let call = match crate::batch_design::find_top_level_char(trimmed, '=') {
        Some(eq) => trimmed[eq + 1..].trim(),
        None => trimmed,
    };
    if !call.starts_with("I(") || !call.ends_with(')') {
        return None;
    }
    let body = &call[2..call.len() - 1];
    let comma = crate::batch_design::find_top_level_char(body, ',')?;
    let parent = parent_label(body[..comma].trim())?;
    let object = serde_json::from_str::<Value>(body[comma + 1..].trim()).ok()?;
    if object.is_object() {
        Some((parent, object))
    } else {
        None
    }
}

fn parent_label(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || matches!(
            raw,
            "null" | "undefined" | "0" | "\"\"" | "''" | "\"null\"" | "'null'"
        )
    {
        return Some("null".to_string());
    }
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw).ok();
    }
    if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2 {
        return Some(raw[1..raw.len() - 1].to_string());
    }
    Some(raw.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn program_object_extraction_preserves_real_ids() {
        let program = r#"
            I(null, {"id":"n217","type":"frame","name":"Mini Player","children":[{"id":"n217-row","type":"frame","name":"Controls","children":[]},{"type":"frame","name":"Progress Bar","children":[]}]});
        "#;

        let objects = super::parse_program_objects(program);

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].0, "null");
        assert_eq!(objects[0].1["id"], serde_json::json!("n217"));
        assert_eq!(
            objects[0].1.pointer("/children/0/id"),
            Some(&serde_json::json!("n217-row"))
        );
        assert_eq!(
            objects[0].1.pointer("/children/1/name"),
            Some(&serde_json::json!("Progress Bar"))
        );
        assert!(objects[0].1.pointer("/children/1/id").is_none());
    }

    #[test]
    #[cfg(feature = "script")]
    fn script_program_object_extraction_preserves_real_ids() {
        let script = r#"
            I(null, {
                id: "n217",
                type: "frame",
                name: "Mini Player",
                children: [
                    { id: "n217-row", type: "frame", name: "Controls", children: [] },
                    { type: "frame", name: "Progress Bar", children: [] }
                ]
            });
        "#;

        let program = crate::script_runner::run_script_to_program(script).expect("script runs");
        let objects = super::parse_program_objects(&program);

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].0, "null");
        assert_eq!(objects[0].1["id"], serde_json::json!("n217"));
        assert_eq!(
            objects[0].1.pointer("/children/0/id"),
            Some(&serde_json::json!("n217-row"))
        );
        assert_eq!(
            objects[0].1.pointer("/children/1/name"),
            Some(&serde_json::json!("Progress Bar"))
        );
        assert!(
            objects[0].1.pointer("/children/1/id").is_none(),
            "new child must remain id-less for modify diffing"
        );
    }
}
