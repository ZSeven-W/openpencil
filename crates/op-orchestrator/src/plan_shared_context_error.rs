//! Validation errors for the immutable shared content payload.

#[derive(Debug)]
pub(super) enum SharedContextError {
    TooManyEntries { sources: usize, entities: usize },
    InvalidEntity { index: usize },
    TooLarge { bytes: usize, limit: usize },
    Serialization(serde_json::Error),
}

impl std::fmt::Display for SharedContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyEntries { sources, entities } => write!(f,
                "sharedContext supports at most 12 source excerpts and 8 entities; received {sources} and {entities}"),
            Self::InvalidEntity { index } => write!(f,
                "sharedContext entity {index} needs a unique id, a name and compact attributes"),
            Self::TooLarge { bytes, limit } => write!(f,
                "sharedContext is {bytes} bytes, exceeding {limit}; shorten the plan data, never truncate identities or values"),
            Self::Serialization(error) => write!(f, "sharedContext serialization: {error}"),
        }
    }
}

impl std::error::Error for SharedContextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
            _ => None,
        }
    }
}
