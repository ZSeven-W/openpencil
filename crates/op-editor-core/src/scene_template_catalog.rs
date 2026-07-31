//! Compile-time embedded Scene Template Center catalogue.
//!
//! A scene template is a finished document a user opens and edits — unlike a
//! Prompt Center entry, which is text handed to the model. That difference
//! drives the shape here: every entry carries an embedded `.op` document, and
//! the catalogue refuses to load if an entry's document or preview is
//! missing. A prompt with a missing preview still works; a template with a
//! missing document is a dead card.

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use op_i18n::Locale;

use crate::catalog_toml::{
    non_empty, parse_string_array, parse_text, required, set_once, ValueError,
};

const SCENE_TEMPLATES_TOML: &str = include_str!("../assets/scene_templates.toml");

/// The shipped documents, embedded so opening a template needs no filesystem
/// and works identically on the web host.
const SCREENSHOT_TUTORIAL_OP: &str =
    include_str!("../assets/scene_templates/screenshot-tutorial.op");
const KNOWLEDGE_CAROUSEL_OP: &str = include_str!("../assets/scene_templates/knowledge-carousel.op");
const BEFORE_AFTER_OP: &str = include_str!("../assets/scene_templates/before-after.op");
const SLIDE_DECK_OP: &str = include_str!("../assets/scene_templates/slide-deck.op");

/// Return the embedded document JSON for a template id.
///
/// The match is exhaustive over what ships; [`scene_template_catalogue`]
/// verifies every catalogue entry resolves here, so a template added to the
/// TOML without its document fails at first use rather than presenting a card
/// that does nothing when clicked.
pub fn scene_template_document(template_id: &str) -> Option<&'static str> {
    match template_id {
        "screenshot-tutorial" => Some(SCREENSHOT_TUTORIAL_OP),
        "knowledge-carousel" => Some(KNOWLEDGE_CAROUSEL_OP),
        "before-after" => Some(BEFORE_AFTER_OP),
        "slide-deck" => Some(SLIDE_DECK_OP),
        _ => None,
    }
}

/// The scene a template belongs to — the filter row in the template center.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateScene {
    /// Step-by-step instructional cards.
    Tutorial,
    /// Side-by-side before/after comparisons.
    Comparison,
    /// Swipeable multi-card narratives.
    Carousel,
    /// Presentation decks.
    Slides,
}

impl TemplateScene {
    pub const ALL: [Self; 4] = [
        Self::Tutorial,
        Self::Comparison,
        Self::Carousel,
        Self::Slides,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tutorial => "tutorial",
            Self::Comparison => "comparison",
            Self::Carousel => "carousel",
            Self::Slides => "slides",
        }
    }

    /// i18n key for the filter chip label.
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::Tutorial => "sceneTemplate.scene.tutorial",
            Self::Comparison => "sceneTemplate.scene.comparison",
            Self::Carousel => "sceneTemplate.scene.carousel",
            Self::Slides => "sceneTemplate.scene.slides",
        }
    }

    /// Label shown when the catalogue key is not yet translated.
    pub const fn title_fallback(self) -> &'static str {
        match self {
            Self::Tutorial => "教程图",
            Self::Comparison => "对比图",
            Self::Carousel => "知识卡片",
            Self::Slides => "PPT",
        }
    }
}

impl FromStr for TemplateScene {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tutorial" => Ok(Self::Tutorial),
            "comparison" => Ok(Self::Comparison),
            "carousel" => Ok(Self::Carousel),
            "slides" => Ok(Self::Slides),
            _ => Err(()),
        }
    }
}

/// One immutable entry from the built-in scene template catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTemplateDefinition {
    pub id: String,
    pub scene: TemplateScene,
    pub title_key: String,
    pub title_fallback: String,
    pub summary_key: String,
    pub summary_fallback: String,
    pub tags: Vec<String>,
    /// Number of top-level frames — shown on the card so a user can tell a
    /// five-page carousel from a single poster before opening it.
    pub frames: u16,
    pub frame_width: u32,
    pub frame_height: u32,
}

impl SceneTemplateDefinition {
    /// The embedded document for this template.
    pub fn document(&self) -> &'static str {
        scene_template_document(&self.id)
            .expect("catalogue load verified every template has a document")
    }

    pub fn title_for_locale(&'static self, locale: Locale) -> &'static str {
        translate_or(locale, &self.title_key, &self.title_fallback)
    }

    pub fn summary_for_locale(&'static self, locale: Locale) -> &'static str {
        translate_or(locale, &self.summary_key, &self.summary_fallback)
    }

    /// Case-insensitive Latin search plus direct CJK substring matching, over
    /// the same fields in every locale so a Chinese or English query finds the
    /// same card.
    pub fn matches_query(&'static self, locale: Locale, query: &str) -> bool {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        [
            self.title_for_locale(locale),
            self.title_fallback.as_str(),
            self.summary_for_locale(locale),
            self.summary_fallback.as_str(),
        ]
        .into_iter()
        .chain(self.tags.iter().map(String::as_str))
        .any(|candidate| candidate.to_lowercase().contains(&needle))
    }
}

fn translate_or(locale: Locale, key: &'static str, fallback: &'static str) -> &'static str {
    let translated = op_i18n::translate(locale, key);
    if translated == key {
        fallback
    } else {
        translated
    }
}

/// A checked-in catalogue parse failure. The line is one-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTemplateCatalogError {
    pub line: usize,
    pub message: String,
}

impl SceneTemplateCatalogError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl From<ValueError> for SceneTemplateCatalogError {
    fn from((line, message): ValueError) -> Self {
        Self { line, message }
    }
}

impl fmt::Display for SceneTemplateCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "scene_templates.toml:{}: {}",
            self.line, self.message
        )
    }
}

impl std::error::Error for SceneTemplateCatalogError {}

static SCENE_TEMPLATE_CATALOGUE: OnceLock<Vec<SceneTemplateDefinition>> = OnceLock::new();

/// Return the built-in catalogue, parsed once from its compile-time asset.
pub fn scene_template_catalogue() -> &'static [SceneTemplateDefinition] {
    SCENE_TEMPLATE_CATALOGUE
        .get_or_init(|| {
            parse_scene_template_catalogue(SCENE_TEMPLATES_TOML)
                .unwrap_or_else(|error| panic!("embedded {error}"))
        })
        .as_slice()
}

/// Templates belonging to `scene`, in catalogue order.
pub fn scene_templates_for(
    scene: TemplateScene,
) -> impl Iterator<Item = &'static SceneTemplateDefinition> {
    scene_template_catalogue()
        .iter()
        .filter(move |template| template.scene == scene)
}

/// Look up one template by id.
pub fn scene_template_by_id(id: &str) -> Option<&'static SceneTemplateDefinition> {
    scene_template_catalogue()
        .iter()
        .find(|template| template.id == id)
}

#[derive(Default)]
struct TemplateBuilder {
    line: usize,
    id: Option<String>,
    scene: Option<TemplateScene>,
    title_key: Option<String>,
    title_fallback: Option<String>,
    summary_key: Option<String>,
    summary_fallback: Option<String>,
    tags: Option<Vec<String>>,
    frames: Option<u16>,
    frame_width: Option<u32>,
    frame_height: Option<u32>,
}

impl TemplateBuilder {
    fn new(line: usize) -> Self {
        Self {
            line,
            ..Self::default()
        }
    }

    fn finish(
        self,
        ids: &mut HashSet<String>,
    ) -> Result<SceneTemplateDefinition, SceneTemplateCatalogError> {
        let line = self.line;
        let id = required(self.id, "id", line)?;
        if !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(SceneTemplateCatalogError::new(
                line,
                format!("id `{id}` must use lowercase ASCII kebab-case"),
            ));
        }
        if !ids.insert(id.clone()) {
            return Err(SceneTemplateCatalogError::new(
                line,
                format!("duplicate template id `{id}`"),
            ));
        }
        // The load-time guarantee `document()` relies on: a catalogue entry
        // without an embedded document would compile fine and fail only when
        // a user clicked the card.
        if scene_template_document(&id).is_none() {
            return Err(SceneTemplateCatalogError::new(
                line,
                format!("template `{id}` has no embedded document"),
            ));
        }
        let tags = required(self.tags, "tags", line)?;
        if tags.is_empty() || tags.iter().any(|tag| tag.trim().is_empty()) {
            return Err(SceneTemplateCatalogError::new(
                line,
                "tags must contain only non-empty values",
            ));
        }
        let frames = required(self.frames, "frames", line)?;
        if frames == 0 {
            return Err(SceneTemplateCatalogError::new(
                line,
                "`frames` must be greater than zero",
            ));
        }
        Ok(SceneTemplateDefinition {
            id,
            scene: required(self.scene, "scene", line)?,
            title_key: non_empty(
                required(self.title_key, "title_key", line)?,
                "title_key",
                line,
            )?,
            title_fallback: non_empty(
                required(self.title_fallback, "title_fallback", line)?,
                "title_fallback",
                line,
            )?,
            summary_key: non_empty(
                required(self.summary_key, "summary_key", line)?,
                "summary_key",
                line,
            )?,
            summary_fallback: non_empty(
                required(self.summary_fallback, "summary_fallback", line)?,
                "summary_fallback",
                line,
            )?,
            tags,
            frames,
            frame_width: required(self.frame_width, "frame_width", line)?,
            frame_height: required(self.frame_height, "frame_height", line)?,
        })
    }
}

fn parse_integer<T: FromStr>(
    raw: &str,
    field: &str,
    line: usize,
) -> Result<T, SceneTemplateCatalogError> {
    raw.parse::<T>().map_err(|_| {
        SceneTemplateCatalogError::new(line, format!("`{field}` must be a positive integer"))
    })
}

/// Parse the strict TOML subset used by the embedded catalogue.
pub(crate) fn parse_scene_template_catalogue(
    source: &str,
) -> Result<Vec<SceneTemplateDefinition>, SceneTemplateCatalogError> {
    let mut templates = Vec::new();
    let mut current: Option<TemplateBuilder> = None;
    let mut ids = HashSet::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[template]]" {
            if let Some(builder) = current.take() {
                templates.push(builder.finish(&mut ids)?);
            }
            current = Some(TemplateBuilder::new(line_number));
            continue;
        }
        let builder = current.as_mut().ok_or_else(|| {
            SceneTemplateCatalogError::new(line_number, "field appears before `[[template]]`")
        })?;
        let (field, raw_value) = line.split_once('=').ok_or_else(|| {
            SceneTemplateCatalogError::new(line_number, "expected `field = value`")
        })?;
        let field = field.trim();
        let raw_value = raw_value.trim();
        match field {
            "id" => set_once(
                &mut builder.id,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "scene" => {
                let scene_text = parse_text(raw_value, line_number)?;
                let scene = scene_text.parse().map_err(|()| {
                    SceneTemplateCatalogError::new(
                        line_number,
                        format!("unknown scene `{scene_text}`"),
                    )
                })?;
                set_once(&mut builder.scene, scene, field, line_number)?;
            }
            "title_key" => set_once(
                &mut builder.title_key,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "title_fallback" => set_once(
                &mut builder.title_fallback,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "summary_key" => set_once(
                &mut builder.summary_key,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "summary_fallback" => set_once(
                &mut builder.summary_fallback,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "tags" => set_once(
                &mut builder.tags,
                parse_string_array(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "frames" => set_once(
                &mut builder.frames,
                parse_integer(raw_value, field, line_number)?,
                field,
                line_number,
            )?,
            "frame_width" => set_once(
                &mut builder.frame_width,
                parse_integer(raw_value, field, line_number)?,
                field,
                line_number,
            )?,
            "frame_height" => set_once(
                &mut builder.frame_height,
                parse_integer(raw_value, field, line_number)?,
                field,
                line_number,
            )?,
            unknown => {
                return Err(SceneTemplateCatalogError::new(
                    line_number,
                    format!("unknown field `{unknown}`"),
                ))
            }
        }
    }

    if let Some(builder) = current {
        templates.push(builder.finish(&mut ids)?);
    }
    if templates.is_empty() {
        return Err(SceneTemplateCatalogError::new(
            1,
            "catalogue must define at least one template",
        ));
    }
    Ok(templates)
}

#[cfg(test)]
#[path = "scene_template_catalog_tests.rs"]
mod scene_template_catalog_tests;
