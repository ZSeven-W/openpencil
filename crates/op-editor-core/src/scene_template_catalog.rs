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

const SCENE_TEMPLATES_TOML: &str = concat!(
    include_str!("../assets/scene_templates_prefix.toml"),
    include_str!("../assets/scene_templates.toml"),
    include_str!("../assets/scene_templates_app.toml"),
    include_str!("../assets/scene_templates_examples.toml")
);

#[path = "scene_template_documents.rs"]
mod documents;
pub use documents::{scene_template_document, scene_template_document_route};

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
    /// Single-board social cards — one finished image, not a sequence.
    ///
    /// Distinct from [`Self::Carousel`]: a carousel's unit of meaning is the
    /// swipe across several boards, so its templates ship a whole sequence. A
    /// card is delivered on its own, which is what makes "title + points +
    /// byline" a complete layout rather than a page of one.
    Card,
    /// Full-width scrolling web pages — marketing sites, product landings.
    ///
    /// Distinct from [`Self::Slides`] in the way that matters here: a deck is
    /// a fixed projector board, a page is one tall surface the reader
    /// scrolls, so the two crop, preview, and generate differently.
    Web,
    /// Long-form vertical graphics — one image read by scrolling.
    ///
    /// Distinct from [`Self::Card`] by reading mode, not by aspect: a card is
    /// taken in at a glance, so its whole argument has to fit one screenful.
    /// An infographic is 2.5-3x its own width and is read top to bottom, which
    /// is what lets it carry a chart, a breakdown and a conclusion in sequence.
    /// Filing both under "卡片" would put a 4:5 social board and a 1:3 scroll
    /// behind the same chip.
    Infographic,
    /// App interface screens — phone screen sets and desktop app windows.
    ///
    /// Distinct from [`Self::Web`]: a web page is one tall marketing surface
    /// read by scrolling, an app template is a set of fixed-size screens
    /// (375×812 phones, a 1440×900 window) a user taps through, each tagged
    /// with its own `screen` route so preview mode can navigate between them.
    App,
}

impl TemplateScene {
    /// The filter row's left-to-right order.
    ///
    /// Two positions are pinned by product decision and must not drift:
    /// **Web is last** and **Slides is second from the right**. Those are the
    /// two heavyweight, least-frequently-picked scenes, so they sit where the
    /// eye lands last. Everything else keeps the order it had — the rule is
    /// "pin those two, leave the rest alone", not "sort by anything".
    /// App joined later and takes the last unpinned slot, just left of the
    /// pinned pair.
    pub const ALL: [Self; 8] = [
        Self::Tutorial,
        Self::Comparison,
        Self::Carousel,
        Self::Card,
        Self::Infographic,
        Self::App,
        Self::Slides,
        Self::Web,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tutorial => "tutorial",
            Self::Comparison => "comparison",
            Self::Carousel => "carousel",
            Self::Slides => "slides",
            Self::Card => "card",
            Self::Web => "web",
            Self::Infographic => "infographic",
            Self::App => "app",
        }
    }

    /// i18n key for the filter chip label.
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::Tutorial => "sceneTemplate.scene.tutorial",
            Self::Comparison => "sceneTemplate.scene.comparison",
            Self::Carousel => "sceneTemplate.scene.carousel",
            Self::Slides => "sceneTemplate.scene.slides",
            Self::Card => "sceneTemplate.scene.card",
            Self::Web => "sceneTemplate.scene.web",
            Self::Infographic => "sceneTemplate.scene.infographic",
            Self::App => "sceneTemplate.scene.app",
        }
    }

    /// Whether the generate row can produce a document of this scene.
    ///
    /// The gate is upstream of the panel: the pipeline picks a canvas from
    /// the request text (`op_orchestrator::detect_design_type`), so a scene
    /// may only offer generation when that classifier has a design type
    /// behind it. Slides qualify because "PPT" triggers one; web pages
    /// qualify because a multi-section landing page is what the classifier
    /// *defaults* to. A card request, by contrast, classifies as a 400 px
    /// Component — offering generation there would hand back a widget where
    /// the card promised a poster. An infographic request triggers no branch
    /// at all, so it would come back as that same default landing page at
    /// 1200 px wide, which is not the 1080-wide scroll the card is showing.
    /// App templates are opened and edited, not generated from: the Studio
    /// Home App task owns that entry point with its own device option.
    pub const fn supports_generation(self) -> bool {
        matches!(self, Self::Slides | Self::Web)
    }

    /// Label shown when the catalogue key is not yet translated.
    ///
    /// Chip-sized nouns, not descriptions: the filter row is a single
    /// non-wrapping line. `Carousel` reads "轮播" rather than "知识卡片"
    /// because the latter is what a [`Self::Card`] chip means — two chips
    /// labelled the same thing is worse than a slightly drier word.
    pub const fn title_fallback(self) -> &'static str {
        match self {
            Self::Tutorial => "教程图",
            Self::Comparison => "对比图",
            Self::Carousel => "轮播",
            Self::Slides => "PPT",
            Self::Card => "卡片",
            Self::Web => "网页",
            Self::Infographic => "信息图",
            Self::App => "App 界面",
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
            "card" => Ok(Self::Card),
            "web" => Ok(Self::Web),
            "infographic" => Ok(Self::Infographic),
            "app" => Ok(Self::App),
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
    /// The style-guide the template's own palette and typography embody, as
    /// an id in `op_ai_skills::style_guide::style_guide_registry`.
    ///
    /// `None` is a real answer, not a gap: a template only carries a guide
    /// when generating from it would actually reproduce it, so this doubles
    /// as the gate on the card's "generate from this" action. Nothing here
    /// validates the name against the registry — op-editor-core does not
    /// depend on the skills corpus — so the cross-check lives beside the
    /// panel that reads both (`asset_center_style_cards`).
    pub style_guide: Option<String>,
}

impl SceneTemplateDefinition {
    /// The embedded document for this template.
    ///
    /// Native only, and deliberately so: it is infallible exactly because the
    /// bytes are in the binary. On `wasm32` the document may not have been
    /// fetched yet, which is not a failure this accessor could express without
    /// becoming an `Option` at every call site for the benefit of a platform
    /// that must go through the async path anyway.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn document(&self) -> &'static str {
        scene_template_document(&self.id)
            .expect("catalogue load verified every template has a document")
    }

    /// The guide to pin when generating from this template, or `None` when
    /// the card must not offer generation at all.
    ///
    /// Both halves have to hold: a guide to pin, and a scene the pipeline
    /// can build. Reading them through one accessor is what keeps paint and
    /// hit-testing from disagreeing about whether the button is there.
    pub fn generate_style_guide(&self) -> Option<&str> {
        if !self.scene.supports_generation() {
            return None;
        }
        self.style_guide.as_deref()
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
    style_guide: Option<String>,
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
        // without a document would compile fine and fail only when a user
        // clicked the card.
        //
        // Checked through the ROUTE, not the bytes: on wasm the bytes are not
        // in the binary at all, so asking for them here would reject the whole
        // catalogue at start-up. The route answers the question actually being
        // asked — "does this id ship a document?" — on both platforms.
        if scene_template_document_route(&id).is_none() {
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
        // An empty `style_guide = ""` is the one shape that would read as
        // "has a guide" while naming none, which is exactly the state that
        // paints a generate button with nothing to pin.
        let style_guide = match self.style_guide {
            Some(name) => Some(non_empty(name, "style_guide", line)?),
            None => None,
        };
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
            style_guide,
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
            "style_guide" => set_once(
                &mut builder.style_guide,
                parse_text(raw_value, line_number)?,
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
