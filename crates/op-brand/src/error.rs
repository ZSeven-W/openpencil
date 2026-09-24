//! Typed brand-extraction failures. Plain enum + hand-written `Display`,
//! matching the workspace's error-enum convention.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrandError {
    /// The page request failed; `detail` is the fetcher's own message.
    Fetch { url: String, detail: String },
    /// The response was not an HTML page.
    NotHtml { url: String },
    /// The image bytes could not be decoded (not PNG / JPEG, or corrupt).
    ImageDecode { detail: String },
    /// The image decoded to no opaque pixels.
    ImageEmpty,
    /// Neither colours nor fonts could be read from the source.
    NoBrandSignals,
}

impl fmt::Display for BrandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrandError::Fetch { url, detail } => write!(f, "could not fetch {url}: {detail}"),
            BrandError::NotHtml { url } => write!(f, "{url} did not return an HTML page"),
            BrandError::ImageDecode { detail } => write!(f, "could not decode the image: {detail}"),
            BrandError::ImageEmpty => write!(f, "the image has no opaque pixels"),
            BrandError::NoBrandSignals => {
                write!(f, "no brand colours or fonts could be read from the source")
            }
        }
    }
}

impl std::error::Error for BrandError {}
