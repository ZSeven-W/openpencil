//! The source-independent evidence a brand kit is resolved from.
//!
//! Both extractors (website CSS, screenshot pixels) reduce their input to
//! these structs; `tokens` then maps them onto the shadcn vocabulary with
//! the same contrast rules, so a kit reads the same whichever way it came.

use crate::color::Rgb;

/// Evidence for one colour scheme. Every field is optional: a missing one
/// is derived from the others (never invented from a fixed palette).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeSignals {
    pub background: Option<Rgb>,
    pub foreground: Option<Rgb>,
    pub primary: Option<Rgb>,
    pub primary_foreground: Option<Rgb>,
    pub card: Option<Rgb>,
    pub secondary: Option<Rgb>,
    pub muted: Option<Rgb>,
    pub muted_foreground: Option<Rgb>,
    /// shadcn semantics: a SUBTLE hover surface, never the brand colour.
    pub accent: Option<Rgb>,
    pub border: Option<Rgb>,
    pub destructive: Option<Rgb>,
    /// Further brand colours after the primary, most prominent first.
    pub extra_brand: Vec<Rgb>,
    /// Where each resolved value came from, e.g. `primary: .btn background`.
    pub provenance: Vec<String>,
}

impl ModeSignals {
    /// Whether this scheme carries enough to stand on its own (a page
    /// ground plus either ink or a brand colour).
    pub fn is_usable(&self) -> bool {
        self.background.is_some() && (self.foreground.is_some() || self.primary.is_some())
    }

    pub(crate) fn note(&mut self, line: impl Into<String>) {
        self.provenance.push(line.into());
    }
}

/// Everything extracted from one source.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrandSignals {
    /// The scheme the source is served in (website default / screenshot).
    pub base: ModeSignals,
    /// An explicit alternate scheme the source declares (a dark theme in
    /// `.dark` / `prefers-color-scheme: dark`, or a light one for a site
    /// that ships dark by default). `None` = derive it.
    pub alternate: Option<ModeSignals>,
    /// Body copy family (first real family in the stack).
    pub font_body: Option<String>,
    /// Heading family, when it differs from the body.
    pub font_heading: Option<String>,
    /// Base corner radius in px (controls / cards).
    pub radius: Option<f64>,
    /// Buttons are pill-shaped (radius ≥ half their height).
    pub pill_buttons: bool,
    /// Human name hint (page title / site name / file stem).
    pub name_hint: Option<String>,
    /// Source used shadcn-style semantic tokens (`--primary-foreground`,
    /// `--card-foreground` …), so `--accent` / `--secondary` already mean
    /// the subtle shadcn surfaces rather than extra brand colours.
    pub shadcn_like: bool,
    /// Extraction notes that are not tied to one colour scheme.
    pub notes: Vec<String>,
}
