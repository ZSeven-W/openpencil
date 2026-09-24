//! The share page's chrome text, in every UI locale.
//!
//! A share page is written once by the author and read by whoever it is
//! sent to, so its chrome ("Make one like this", the footer, the
//! `task · style` line, the arrow labels) follows the RECIPIENT's browser
//! language, not the author's. The page is server-rendered in the
//! author's locale — the no-script and unmatched-language fallback — and
//! carries the chrome for all 15 locales as a small JSON table
//! ([`CHROME_ELEMENT_ID`]); [`CHROME_JS`] swaps the text in by
//! `navigator.languages` before the first paint of the viewer.
//!
//! Only chrome is translated. The design itself, its title and the
//! style's name are the author's and stay as they are.

use op_editor_core::{HomeFamily, Locale, ShareRecipe};
use serde_json::{json, Map, Value};

/// Element id of the embedded chrome table.
pub const CHROME_ELEMENT_ID: &str = "op-share-chrome";

/// Every piece of page chrome text for one locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareLabels {
    pub make_same: String,
    pub make_same_hint: String,
    pub made_with: String,
    pub prev: String,
    pub next: String,
    /// `任务 · 风格` summary under the title; empty without a recipe.
    pub recipe_line: String,
    pub html_lang: &'static str,
}

impl ShareLabels {
    /// The chrome in `locale`. `style_name` is the pinned style guide's
    /// display name (see [`style_display_name`]); `None` reads as the
    /// locale's "automatic".
    pub fn for_locale(
        locale: Locale,
        recipe: Option<&ShareRecipe>,
        style_name: Option<&str>,
    ) -> Self {
        let t = |key: &'static str| op_i18n::translate(locale, key).to_string();
        let recipe_line = recipe
            .map(|recipe| {
                let task = op_i18n::translate(locale, task_name_key(recipe.family));
                let style = style_name
                    .map(str::to_string)
                    .unwrap_or_else(|| t("share.page.autoStyle"));
                format!("{task} · {} {style}", t("share.page.style"))
            })
            .unwrap_or_default();
        Self {
            make_same: t("share.page.makeSame"),
            make_same_hint: t("share.page.makeSameHint"),
            made_with: t("share.page.madeWith"),
            prev: t("share.page.prev"),
            next: t("share.page.next"),
            recipe_line,
            html_lang: locale.code(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "make": self.make_same,
            "hint": self.make_same_hint,
            "made": self.made_with,
            "prev": self.prev,
            "next": self.next,
            "recipe": self.recipe_line,
        })
    }
}

/// The page chrome: the author's labels (server-rendered) plus the same
/// labels in every locale for the recipient-side swap.
#[derive(Debug, Clone)]
pub struct ShareChrome {
    /// What the markup is rendered with — the author's locale.
    pub author: ShareLabels,
    /// `{"author": "<code>", "locales": {"<code>": {...}}}`.
    pub table_json: String,
}

impl ShareChrome {
    pub fn new(author: Locale, recipe: Option<&ShareRecipe>, style_name: Option<&str>) -> Self {
        let mut locales = Map::new();
        for locale in Locale::ALL {
            let labels = ShareLabels::for_locale(locale, recipe, style_name);
            locales.insert(locale.code().to_string(), labels.to_json());
        }
        let table = json!({ "author": author.code(), "locales": Value::Object(locales) });
        Self {
            author: ShareLabels::for_locale(author, recipe, style_name),
            table_json: table.to_string(),
        }
    }
}

/// The name a person reads for a style guide id.
///
/// An imported guide carries its own name (`user:<slug>` → the name in
/// its DESIGN.md); a corpus guide's name IS its registry id, so it is
/// humanized the way the variants strip labels it
/// (`editorial-dark` → `Editorial Dark`).
pub fn style_display_name(id: &str) -> String {
    let id = id.trim();
    op_ai_skills::style_guide::style_guide_card(id)
        .filter(|card| card.is_user && !card.name.trim().is_empty())
        .map(|card| card.name.trim().to_string())
        .unwrap_or_else(|| op_orchestrator::variants::humanize_style_guide_name(id))
}

/// The i18n key of a Home task's display name.
fn task_name_key(family: HomeFamily) -> &'static str {
    match family {
        HomeFamily::AppUi => "home.task.app.name",
        HomeFamily::Web => "home.task.web.name",
        HomeFamily::Presentation => "home.task.presentation.name",
        HomeFamily::KnowledgeCards => "home.task.knowledge.name",
        HomeFamily::ScreenshotTutorial => "home.task.tutorial.name",
        HomeFamily::Infographic => "home.task.infographic.name",
        HomeFamily::EventPoster => "home.task.poster.name",
    }
}

/// Picks the recipient's locale from `navigator.languages` and swaps
/// the chrome text in. Matching, per preferred language in order: the
/// exact tag; Chinese by script/region (`zh-Hant`, `-TW`, `-HK`, `-MO` →
/// `zh-TW`, any other `zh` → `zh-CN`); then the primary subtag (`en-GB`
/// → `en-US`, `pt-BR` → `pt`). Nothing matching keeps the author's text.
pub const CHROME_JS: &str = r#"
(function () {
  var node = document.getElementById('op-share-chrome');
  if (!node) { return; }
  var table;
  try { table = JSON.parse(node.textContent); } catch (e) { return; }
  var locales = table.locales || {};
  var codes = Object.keys(locales);
  function match(tag) {
    var lower = String(tag || '').toLowerCase();
    if (!lower) { return null; }
    for (var i = 0; i < codes.length; i++) {
      if (codes[i].toLowerCase() === lower) { return codes[i]; }
    }
    var primary = lower.split('-')[0];
    if (primary === 'zh') {
      return /-(hant|tw|hk|mo)\b/.test(lower) ? 'zh-TW' : 'zh-CN';
    }
    for (var j = 0; j < codes.length; j++) {
      if (codes[j].toLowerCase().split('-')[0] === primary) { return codes[j]; }
    }
    return null;
  }
  var wanted = navigator.languages && navigator.languages.length
    ? navigator.languages : [navigator.language];
  var code = null;
  for (var k = 0; k < wanted.length && !code; k++) { code = match(wanted[k]); }
  if (!code || code === table.author || !locales[code]) { return; }
  var t = locales[code];
  document.documentElement.lang = code;
  var make = document.getElementById('make');
  if (make) { make.textContent = t.make; make.title = t.hint; }
  var prev = document.getElementById('prev');
  if (prev) { prev.setAttribute('aria-label', t.prev); }
  var next = document.getElementById('next');
  if (next) { next.setAttribute('aria-label', t.next); }
  var made = document.querySelector('#foot .made');
  if (made) { made.textContent = t.made; }
  var hint = document.querySelector('#foot .hint');
  if (hint) { hint.textContent = t.hint; }
  var recipe = document.querySelector('#bar .recipe');
  if (recipe) { recipe.textContent = t.recipe; }
})();
"#;
