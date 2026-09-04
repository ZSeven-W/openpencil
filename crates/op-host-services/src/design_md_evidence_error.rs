use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignMdEvidenceError {
    EmptyBody,
    BodyTooLarge,
    InvalidJson,
    NotObject,
    MissingField(&'static str),
    ForbiddenField(String),
    FieldNameTooLong,
    OverlongString,
    ExternalReference,
    Schema(String),
    Serialization,
    SanitizedTooLarge,
    Field { field: String, reason: String },
}

impl DesignMdEvidenceError {
    pub(crate) fn field(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Field {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for DesignMdEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBody => f.write_str("evidence body is empty"),
            Self::BodyTooLarge => f.write_str("evidence body exceeds 256 KiB"),
            Self::InvalidJson => f.write_str("evidence must be valid JSON"),
            Self::NotObject => f.write_str("evidence must be a JSON object"),
            Self::MissingField(field) => write!(f, "evidence is missing required field `{field}`"),
            Self::ForbiddenField(field) => {
                write!(f, "evidence contains forbidden field `{field}`")
            }
            Self::FieldNameTooLong => f.write_str("evidence field name is too long"),
            Self::OverlongString => f.write_str("evidence contains an overlong string"),
            Self::ExternalReference => {
                f.write_str("evidence strings must not contain URLs or embedded data")
            }
            Self::Schema(error) => write!(f, "evidence does not match schema v1: {error}"),
            Self::Serialization => f.write_str("failed to serialize validated evidence"),
            Self::SanitizedTooLarge => f.write_str("sanitized evidence exceeds 256 KiB"),
            Self::Field { field, reason } => write!(f, "{field} {reason}"),
        }
    }
}

impl std::error::Error for DesignMdEvidenceError {}
