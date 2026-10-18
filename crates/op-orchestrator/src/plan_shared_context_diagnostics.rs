//! Locate type errors only after the authoritative shared-context parser rejects.
//!
//! This diagnostic walker never accepts, edits or repairs a plan. Semantic
//! validation and unknown-field errors remain with the existing deserializer.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonKind {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
    Missing,
}

impl JsonKind {
    fn of(value: Option<&Value>) -> Self {
        match value {
            Some(Value::Object(_)) => Self::Object,
            Some(Value::Array(_)) => Self::Array,
            Some(Value::String(_)) => Self::String,
            Some(Value::Number(_)) => Self::Number,
            Some(Value::Bool(_)) => Self::Boolean,
            Some(Value::Null) => Self::Null,
            None => Self::Missing,
        }
    }
}

impl std::fmt::Display for JsonKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Object => "object",
            Self::Array => "array",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
            Self::Missing => "missing",
        })
    }
}

#[derive(Debug)]
pub(crate) struct SharedContextShapeError {
    pub path: String,
    pub expected: JsonKind,
    pub actual: JsonKind,
}

impl SharedContextShapeError {
    pub(crate) fn new(path: String, expected: JsonKind, value: Option<&Value>) -> Self {
        Self {
            path,
            expected,
            actual: JsonKind::of(value),
        }
    }
}

impl std::fmt::Display for SharedContextShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} expected {}; received {}",
            self.path, self.expected, self.actual
        )
    }
}

fn expect(
    path: &str,
    expected: JsonKind,
    value: Option<&Value>,
) -> Option<SharedContextShapeError> {
    (JsonKind::of(value) != expected)
        .then(|| SharedContextShapeError::new(path.to_owned(), expected, value))
}

fn strings(path: &str, value: &Value) -> Option<SharedContextShapeError> {
    if let Some(error) = expect(path, JsonKind::Array, Some(value)) {
        return Some(error);
    }
    value
        .as_array()?
        .iter()
        .enumerate()
        .find_map(|(index, value)| {
            expect(&format!("{path}[{index}]"), JsonKind::String, Some(value))
        })
}

fn property_path(parent: &str, key: &str) -> String {
    let mut chars = key.chars();
    if chars
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        format!("{parent}.{key}")
    } else {
        format!(
            "{parent}[{}]",
            serde_json::to_string(key).expect("a string serializes")
        )
    }
}

pub(crate) fn invalid_shape(context: &Value) -> Option<SharedContextShapeError> {
    invalid_shapes(context, 1).into_iter().next()
}

/// Bounded diagnostics for a rejected plan, never a substitute for deserialization.
pub(crate) fn invalid_shapes(context: &Value, limit: usize) -> Vec<SharedContextShapeError> {
    let mut errors = Vec::new();
    collect_shapes(context, &mut |error| {
        if errors.len() < limit {
            errors.push(error);
        }
        errors.len() >= limit
    });
    errors
}

// A visitor returning true stops the walk. Invalid parents are not traversed.
fn collect_shapes(context: &Value, emit: &mut impl FnMut(SharedContextShapeError) -> bool) {
    const ROOT: &str = "$.sharedContext";
    if let Some(error) = expect(ROOT, JsonKind::Object, Some(context)) {
        emit(error);
        return;
    }
    if let Some(facts) = context.get("sourceFacts") {
        if let Some(error) = strings(&format!("{ROOT}.sourceFacts"), facts) {
            if emit(error) {
                return;
            }
        }
    }
    let Some(entities) = context.get("exampleEntities") else {
        return;
    };
    let entities_path = format!("{ROOT}.exampleEntities");
    if let Some(error) = expect(&entities_path, JsonKind::Array, Some(entities)) {
        emit(error);
        return;
    }
    for (index, entity) in entities
        .as_array()
        .expect("checked array")
        .iter()
        .enumerate()
    {
        let path = format!("{entities_path}[{index}]");
        if let Some(error) = expect(&path, JsonKind::Object, Some(entity)) {
            if emit(error) {
                return;
            }
            continue;
        }
        for key in ["id", "name"] {
            if let Some(error) = expect(&format!("{path}.{key}"), JsonKind::String, entity.get(key))
            {
                if emit(error) {
                    return;
                }
            }
        }
        // Option<String> deliberately permits null; optional/defaulted fields
        // may be absent. The walker must not invent a stricter accepted schema.
        if let Some(subject) = entity.get("imageSubject").filter(|value| !value.is_null()) {
            if let Some(error) = expect(
                &format!("{path}.imageSubject"),
                JsonKind::String,
                Some(subject),
            ) {
                if emit(error) {
                    return;
                }
            }
        }
        if let Some(attributes) = entity.get("attributes") {
            let attributes_path = format!("{path}.attributes");
            if let Some(error) = expect(&attributes_path, JsonKind::Object, Some(attributes)) {
                if emit(error) {
                    return;
                }
            } else {
                for (key, value) in attributes.as_object().expect("checked object") {
                    if let Some(error) = expect(
                        &property_path(&attributes_path, key),
                        JsonKind::String,
                        Some(value),
                    ) {
                        if emit(error) {
                            return;
                        }
                    }
                }
            }
        }
        if let Some(used_by) = entity.get("usedBy") {
            if let Some(error) = strings(&format!("{path}.usedBy"), used_by) {
                if emit(error) {
                    return;
                }
            }
        }
    }
}
