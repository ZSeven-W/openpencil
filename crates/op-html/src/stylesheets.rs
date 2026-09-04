//! Author stylesheet activation and resource loading.

use crate::css::cascade::{StyleRule, StylesheetParser};
use crate::dom::StylesheetSource;
use crate::import_warning::ImportWarning;
use crate::resources::{self, ResourceBudget, ResourceFetcher};
use crate::HtmlImportOptions;

#[allow(clippy::too_many_arguments)]
pub(crate) fn extend_author_rules(
    sources: Vec<StylesheetSource>,
    parser: &mut StylesheetParser,
    rules: &mut Vec<StyleRule>,
    opts: &HtmlImportOptions,
    viewport_height: f64,
    resource_base: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
) {
    for source in sources {
        let stylesheet = match source {
            StylesheetSource::Inline(stylesheet) => {
                let (media, source) = crate::dom::decode_inline_media(&stylesheet)
                    .map_or((None, stylesheet.as_str()), |(media, source)| {
                        (Some(media), source)
                    });
                let expanded = resources::expand_stylesheet_imports(
                    source,
                    resource_base,
                    None,
                    fetcher,
                    budget,
                    warnings,
                );
                media
                    .map(|media| format!("@media {media}{{{expanded}}}"))
                    .unwrap_or(expanded)
            }
            StylesheetSource::Link(href) => {
                let resolved = resources::resolve_resource_url(resource_base, &href);
                let display_url = resolved.as_deref().unwrap_or(&href);
                if !budget.take(warnings) {
                    continue;
                }
                let Some(bytes) = resolved
                    .as_deref()
                    .and_then(|url| fetcher.and_then(|fetch| fetch(url)))
                else {
                    warnings.push(ImportWarning::ExternalStylesheetSkipped {
                        url: display_url.to_string(),
                    });
                    continue;
                };
                let decoded = crate::css_encoding::decode_css_bytes(&bytes);
                resources::expand_stylesheet_imports(
                    &decoded,
                    Some(display_url),
                    Some(display_url),
                    fetcher,
                    budget,
                    warnings,
                )
            }
        };
        let (author_rules, stylesheet_warnings) =
            parser.parse_for_viewport(&stylesheet, opts.viewport_width, viewport_height);
        rules.extend(author_rules);
        warnings.extend(stylesheet_warnings);
    }
}
