//! Typed failures for reference-page enrichment.

use std::fmt;

use crate::design_md_llm::DesignMdError;
use crate::import_html_url_error::ImportHtmlUrlError;

/// Why a reference page could not become planning context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceContextError {
    /// The existing policy-screened HTML importer rejected or could not fetch
    /// the requested page.
    Import(ImportHtmlUrlError),
    /// The imported page had no usable root structure.
    NoStructure,
    /// The attachment supplied for screenshot reference extraction is not a
    /// non-empty image payload.
    NotAnImage,
    /// The existing design.md LLM enrichment path failed.
    DesignMd(DesignMdError),
}

impl fmt::Display for ReferenceContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Import(error) => error.fmt(f),
            Self::NoStructure => f.write_str("reference page produced no usable structure"),
            Self::NotAnImage => f.write_str("reference attachment is not a non-empty image"),
            Self::DesignMd(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ReferenceContextError {}

impl From<ImportHtmlUrlError> for ReferenceContextError {
    fn from(error: ImportHtmlUrlError) -> Self {
        Self::Import(error)
    }
}

impl From<DesignMdError> for ReferenceContextError {
    fn from(error: DesignMdError) -> Self {
        Self::DesignMd(error)
    }
}
