//! Compile-time embedded Prompt Center catalogue.
//!
//! The checked-in asset uses a deliberately small TOML subset. Parsing it
//! here keeps the catalogue dependency-free, deterministic, and wasm-safe.

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use op_i18n::Locale;

use crate::catalog_toml::{
    non_empty as toml_non_empty, parse_string_array as toml_string_array, parse_text as toml_text,
    required as toml_required, set_once as toml_set_once, ValueError,
};
use serde::{Deserialize, Serialize};

const PROMPT_CENTER_TOML: &str = include_str!("../assets/prompt_center.toml");

/// A content category shared by built-in and user-saved prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PromptCategory {
    Starter,
    MobileApp,
    WebPage,
    Dashboard,
    Component,
    Modify,
}

impl PromptCategory {
    pub const ALL: [Self; 6] = [
        Self::Starter,
        Self::MobileApp,
        Self::WebPage,
        Self::Dashboard,
        Self::Component,
        Self::Modify,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starter => "starter",
            Self::MobileApp => "mobile-app",
            Self::WebPage => "web-page",
            Self::Dashboard => "dashboard",
            Self::Component => "component",
            Self::Modify => "modify",
        }
    }
}

impl FromStr for PromptCategory {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "starter" => Ok(Self::Starter),
            "mobile-app" => Ok(Self::MobileApp),
            "web-page" => Ok(Self::WebPage),
            "dashboard" => Ok(Self::Dashboard),
            "component" => Ok(Self::Component),
            "modify" => Ok(Self::Modify),
            _ => Err(()),
        }
    }
}

/// One immutable entry from the built-in prompt catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptDefinition {
    pub id: String,
    pub category: PromptCategory,
    pub tags: Vec<String>,
    pub title_key: String,
    pub title_fallback: String,
    pub body_zh: String,
    pub body_en: String,
    pub screens: Option<u8>,
}

impl PromptDefinition {
    /// Select Chinese content for the four CJK UI locales required by the
    /// Prompt Center contract; all other locales use the English body.
    pub fn body_for_locale(&self, locale: Locale) -> &str {
        if matches!(
            locale,
            Locale::ZhCn | Locale::ZhTw | Locale::Ja | Locale::Ko
        ) {
            &self.body_zh
        } else {
            &self.body_en
        }
    }

    /// Resolve the translated title, falling back to the asset title if the
    /// catalogue key is absent.
    pub fn title_for_locale(&'static self, locale: Locale) -> &'static str {
        let key = self.title_key.as_str();
        let translated = op_i18n::translate(locale, key);
        if translated == key {
            &self.title_fallback
        } else {
            translated
        }
    }

    /// Case-insensitive Latin search plus direct CJK substring matching.
    ///
    /// Both bodies participate regardless of the current locale so users can
    /// find the same card with either a Chinese or English query.
    pub fn matches_query(&'static self, locale: Locale, query: &str) -> bool {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        [
            self.title_for_locale(locale),
            self.title_fallback.as_str(),
            self.body_zh.as_str(),
            self.body_en.as_str(),
        ]
        .into_iter()
        .chain(self.tags.iter().map(String::as_str))
        .any(|candidate| candidate.to_lowercase().contains(&needle))
    }
}

/// A checked-in catalogue parse failure. The line is one-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCatalogError {
    pub line: usize,
    pub message: String,
}

impl PromptCatalogError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for PromptCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "prompt_center.toml:{}: {}",
            self.line, self.message
        )
    }
}

impl std::error::Error for PromptCatalogError {}

static PROMPT_CATALOGUE: OnceLock<Vec<PromptDefinition>> = OnceLock::new();

/// Return the built-in catalogue, parsed once from its compile-time asset.
pub fn prompt_catalogue() -> &'static [PromptDefinition] {
    PROMPT_CATALOGUE
        .get_or_init(|| {
            parse_prompt_catalogue(PROMPT_CENTER_TOML)
                .unwrap_or_else(|error| panic!("embedded {error}"))
        })
        .as_slice()
}

#[derive(Default)]
struct PromptBuilder {
    line: usize,
    id: Option<String>,
    category: Option<PromptCategory>,
    tags: Option<Vec<String>>,
    title_key: Option<String>,
    title_fallback: Option<String>,
    body_zh: Option<String>,
    body_en: Option<String>,
    screens: Option<u8>,
    screens_seen: bool,
}

impl PromptBuilder {
    fn new(line: usize) -> Self {
        Self {
            line,
            ..Self::default()
        }
    }

    fn finish(self, ids: &mut HashSet<String>) -> Result<PromptDefinition, PromptCatalogError> {
        let id = required(self.id, "id", self.line)?;
        if !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(PromptCatalogError::new(
                self.line,
                format!("id `{id}` must use lowercase ASCII kebab-case"),
            ));
        }
        if !ids.insert(id.clone()) {
            return Err(PromptCatalogError::new(
                self.line,
                format!("duplicate prompt id `{id}`"),
            ));
        }
        let tags = required(self.tags, "tags", self.line)?;
        if tags.is_empty() || tags.iter().any(|tag| tag.trim().is_empty()) {
            return Err(PromptCatalogError::new(
                self.line,
                "tags must contain only non-empty values",
            ));
        }
        let unique_tags: HashSet<_> = tags.iter().collect();
        if unique_tags.len() != tags.len() {
            return Err(PromptCatalogError::new(
                self.line,
                format!("prompt `{id}` contains duplicate tags"),
            ));
        }
        Ok(PromptDefinition {
            id,
            category: required(self.category, "category", self.line)?,
            tags,
            title_key: non_empty(
                required(self.title_key, "title_key", self.line)?,
                "title_key",
                self.line,
            )?,
            title_fallback: non_empty(
                required(self.title_fallback, "title_fallback", self.line)?,
                "title_fallback",
                self.line,
            )?,
            body_zh: non_empty(
                required(self.body_zh, "body_zh", self.line)?,
                "body_zh",
                self.line,
            )?,
            body_en: non_empty(
                required(self.body_en, "body_en", self.line)?,
                "body_en",
                self.line,
            )?,
            screens: self.screens,
        })
    }
}

impl From<ValueError> for PromptCatalogError {
    fn from((line, message): ValueError) -> Self {
        Self { line, message }
    }
}

fn required<T>(value: Option<T>, field: &str, line: usize) -> Result<T, PromptCatalogError> {
    toml_required(value, field, line).map_err(PromptCatalogError::from)
}

fn non_empty(value: String, field: &str, line: usize) -> Result<String, PromptCatalogError> {
    toml_non_empty(value, field, line).map_err(PromptCatalogError::from)
}

fn set_once<T>(
    slot: &mut Option<T>,
    value: T,
    field: &str,
    line: usize,
) -> Result<(), PromptCatalogError> {
    toml_set_once(slot, value, field, line).map_err(PromptCatalogError::from)
}

fn parse_text(value: &str, line: usize) -> Result<String, PromptCatalogError> {
    toml_text(value, line).map_err(PromptCatalogError::from)
}

fn parse_string_array(value: &str, line: usize) -> Result<Vec<String>, PromptCatalogError> {
    toml_string_array(value, line).map_err(PromptCatalogError::from)
}

/// Parse the strict TOML subset used by the embedded catalogue.
pub(crate) fn parse_prompt_catalogue(
    source: &str,
) -> Result<Vec<PromptDefinition>, PromptCatalogError> {
    let mut prompts = Vec::new();
    let mut current: Option<PromptBuilder> = None;
    let mut ids = HashSet::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[prompt]]" {
            if let Some(builder) = current.take() {
                prompts.push(builder.finish(&mut ids)?);
            }
            current = Some(PromptBuilder::new(line_number));
            continue;
        }
        let builder = current.as_mut().ok_or_else(|| {
            PromptCatalogError::new(line_number, "field appears before `[[prompt]]`")
        })?;
        let (field, raw_value) = line
            .split_once('=')
            .ok_or_else(|| PromptCatalogError::new(line_number, "expected `field = value`"))?;
        let field = field.trim();
        let raw_value = raw_value.trim();
        match field {
            "id" => set_once(
                &mut builder.id,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "category" => {
                let category_text = parse_text(raw_value, line_number)?;
                let category = category_text.parse().map_err(|()| {
                    PromptCatalogError::new(
                        line_number,
                        format!("unknown category `{category_text}`"),
                    )
                })?;
                set_once(&mut builder.category, category, field, line_number)?;
            }
            "tags" => set_once(
                &mut builder.tags,
                parse_string_array(raw_value, line_number)?,
                field,
                line_number,
            )?,
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
            "body_zh" => set_once(
                &mut builder.body_zh,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "body_en" => set_once(
                &mut builder.body_en,
                parse_text(raw_value, line_number)?,
                field,
                line_number,
            )?,
            "screens" => {
                if builder.screens_seen {
                    return Err(PromptCatalogError::new(line_number, "duplicate `screens`"));
                }
                let screens = raw_value.parse::<u8>().map_err(|_| {
                    PromptCatalogError::new(line_number, "`screens` must be an integer")
                })?;
                if screens == 0 {
                    return Err(PromptCatalogError::new(
                        line_number,
                        "`screens` must be greater than zero",
                    ));
                }
                builder.screens = Some(screens);
                builder.screens_seen = true;
            }
            _ => {
                return Err(PromptCatalogError::new(
                    line_number,
                    format!("unknown field `{field}`"),
                ))
            }
        }
    }

    if let Some(builder) = current {
        prompts.push(builder.finish(&mut ids)?);
    }
    if prompts.is_empty() {
        return Err(PromptCatalogError::new(
            1,
            "catalogue must contain at least one prompt",
        ));
    }
    Ok(prompts)
}
