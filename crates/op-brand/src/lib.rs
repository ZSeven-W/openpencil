//! Brand extraction → design variables.
//!
//! A website URL (served HTML + CSS, no script) or a screenshot becomes a
//! [`BrandKit`]: light + dark colour tokens, fonts, and a radius scale in
//! the shadcn vocabulary the bundled semantic palette uses (`--primary`,
//! `--background`, `--font-primary`, `--radius-m` …). Applied to a
//! document, the kit's variables are what generated nodes bind to
//! (`$--primary` refs + the orchestrator's existing-wins palette seed), so
//! swapping a brand later means swapping the variable set.
//!
//! Everything here is deterministic and offline: the network is a trait
//! the host implements ([`BrandFetcher`]), and images arrive as bytes.

mod color;
mod css_scan;
mod css_select;
mod css_signals;
mod css_tokens;
mod error;
mod image_palette;
mod kit;
mod signals;
mod tokens;
mod url_source;

pub use color::{parse_css_color, Rgb, AA_TEXT};
pub use css_signals::{analyze_site, inline_sheets};
pub use error::BrandError;
pub use image_palette::{palette_from_image, signals_from_swatches, Swatch};
pub use kit::{
    BrandKit, BrandSource, ThemeOrigin, BRAND_KIT_MARKER, THEME_AXIS, THEME_DARK, THEME_LIGHT,
};
pub use signals::{BrandSignals, ModeSignals};
pub use tokens::{ModeTokens, COLOR_TOKENS, CONTRAST_PAIRS};
pub use url_source::{
    extract_from_html, extract_from_url, host_of, BrandFetcher, FetchKind, Fetched, MAX_CSS_BYTES,
    MAX_STYLESHEETS,
};

/// Kit from a screenshot's bytes (PNG / JPEG). `label` names the source
/// (usually the file name) and is the fallback kit name.
pub fn extract_from_image(bytes: &[u8], label: &str) -> Result<BrandKit, BrandError> {
    let swatches = palette_from_image(bytes)?;
    let signals = signals_from_swatches(&swatches);
    if !signals.base.is_usable() {
        return Err(BrandError::NoBrandSignals);
    }
    let file = label.rsplit(['/', '\\']).next().unwrap_or(label);
    let name = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    Ok(BrandKit::from_signals(
        signals,
        BrandSource::Image(label.to_string()),
        name,
    ))
}

#[cfg(test)]
mod lib_tests;
