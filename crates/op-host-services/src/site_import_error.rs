//! Typed failures of the one-click website import.

use std::fmt;

use op_brand::BrandError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteImportError {
    /// The page could not be fetched (policy refusal, network, HTTP status).
    Fetch(BrandError),
    /// The response was not an HTML page.
    NotHtml { url: String },
    /// The page parsed to nothing importable.
    NoContent { detail: String },
    /// Another import / extraction is already using every fetch slot.
    Busy,
}

impl fmt::Display for SiteImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SiteImportError::Fetch(error) => write!(f, "{error}"),
            SiteImportError::NotHtml { url } => write!(f, "{url} did not return an HTML page"),
            SiteImportError::NoContent { detail } => {
                write!(f, "the page has no importable content: {detail}")
            }
            SiteImportError::Busy => write!(f, "too many concurrent fetches; try again"),
        }
    }
}

impl std::error::Error for SiteImportError {}

impl From<BrandError> for SiteImportError {
    fn from(error: BrandError) -> Self {
        match error {
            BrandError::NotHtml { url } => SiteImportError::NotHtml { url },
            other => SiteImportError::Fetch(other),
        }
    }
}
