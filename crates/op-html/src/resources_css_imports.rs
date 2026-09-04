use crate::import_warning::ImportWarning;
use std::collections::HashSet;

use crate::css_encoding::decode_css_bytes;

use super::css_urls::{
    function_end, parse_url_argument, rebase_stylesheet_urls, string_end, unescape_css,
};
use super::{resolve_resource_url, ResourceBudget, ResourceFetcher};

pub(crate) const MAX_STYLESHEET_IMPORT_DEPTH: usize = 16;

/// Expands leading CSS `@import` rules in source order.
///
/// The returned stylesheet has every local `url(...)` rebased to the sheet that
/// declared it. Feeding this single stream to one `StylesheetParser` preserves
/// the CSS rule order: imported rules precede the importing sheet's own rules.
pub(crate) fn expand_stylesheet_imports(
    source: &str,
    source_url: Option<&str>,
    source_identity: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
) -> String {
    let mut active = HashSet::new();
    if let Some(url) = source_identity {
        active.insert(canonical_url(url));
    }
    expand_inner(
        source,
        source_url,
        fetcher,
        budget,
        warnings,
        &mut active,
        0,
    )
}

fn expand_inner(
    source: &str,
    source_url: Option<&str>,
    fetcher: Option<&ResourceFetcher<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<ImportWarning>,
    active: &mut HashSet<String>,
    depth: usize,
) -> String {
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    loop {
        let item_start = skip_trivia(source, cursor);
        let Some((delimiter, byte)) = top_level_delimiter(source, item_start) else {
            append_rebased(&mut output, &source[cursor..], source_url);
            break;
        };
        let prelude = source[item_start..delimiter].trim();
        if byte != b';' {
            append_rebased(&mut output, &source[cursor..], source_url);
            break;
        }

        if at_keyword(prelude, "charset") || at_keyword(prelude, "layer") {
            append_rebased(&mut output, &source[cursor..=delimiter], source_url);
            cursor = delimiter + 1;
            continue;
        }
        if !at_keyword(prelude, "import") {
            append_rebased(&mut output, &source[cursor..], source_url);
            break;
        }

        append_rebased(&mut output, &source[cursor..item_start], source_url);
        let import_prelude = prelude["@import".len()..].trim();
        let Some(parsed) = parse_import(import_prelude) else {
            warnings.push(ImportWarning::CssImportInvalid {
                prelude: prelude.to_string(),
            });
            cursor = delimiter + 1;
            continue;
        };
        let Some(resolved) = resolve_resource_url(source_url, &parsed.reference) else {
            warnings.push(ImportWarning::CssImportUnresolvable {
                reference: parsed.reference.to_string(),
            });
            cursor = delimiter + 1;
            continue;
        };
        let canonical = canonical_url(&resolved);
        if active.contains(&canonical) {
            warnings.push(ImportWarning::CssImportCycle {
                url: resolved.to_string(),
            });
            cursor = delimiter + 1;
            continue;
        }
        if depth >= MAX_STYLESHEET_IMPORT_DEPTH {
            warnings.push(ImportWarning::CssImportDepthLimit {
                max_depth: MAX_STYLESHEET_IMPORT_DEPTH,
                url: resolved.to_string(),
            });
            cursor = delimiter + 1;
            continue;
        }
        if !budget.take(warnings) {
            cursor = delimiter + 1;
            continue;
        }
        let Some(bytes) = fetcher.and_then(|fetch| fetch(&resolved)) else {
            warnings.push(ImportWarning::CssImportUnavailable {
                url: resolved.to_string(),
            });
            cursor = delimiter + 1;
            continue;
        };
        let imported = decode_css_bytes(&bytes);
        active.insert(canonical.clone());
        let expanded = expand_inner(
            imported.as_ref(),
            Some(&resolved),
            fetcher,
            budget,
            warnings,
            active,
            depth + 1,
        );
        active.remove(&canonical);
        output.push_str(&wrap_imported(expanded, parsed.modifiers));
        cursor = delimiter + 1;
    }
    output
}

fn append_rebased(output: &mut String, source: &str, source_url: Option<&str>) {
    if let Some(url) = source_url {
        output.push_str(&rebase_stylesheet_urls(source, url));
    } else {
        output.push_str(source);
    }
}

struct ParsedImport<'a> {
    reference: String,
    modifiers: &'a str,
}

fn parse_import(prelude: &str) -> Option<ParsedImport<'_>> {
    let source = prelude.trim_start();
    let first = source.chars().next()?;
    if matches!(first, '\'' | '"') {
        let end = string_end(source, 0, first)?;
        let value = unescape_css(&source[first.len_utf8()..end - first.len_utf8()], true)?;
        return Some(ParsedImport {
            reference: value,
            modifiers: source[end..].trim(),
        });
    }
    if source.len() >= 4
        && source.as_bytes()[..3].eq_ignore_ascii_case(b"url")
        && source.as_bytes()[3] == b'('
    {
        let close = function_end(source, 3)?;
        let parsed = parse_url_argument(&source[4..close])?;
        return Some(ParsedImport {
            reference: parsed.value,
            modifiers: source[close + 1..].trim(),
        });
    }
    None
}

fn wrap_imported(mut source: String, modifiers: &str) -> String {
    let mut rest = modifiers.trim();
    let mut layer = None;
    let mut supports = None;
    if let Some((value, remaining)) = take_function(rest, "layer") {
        layer = Some(value);
        rest = remaining.trim_start();
    } else if let Some(remaining) = take_keyword(rest, "layer") {
        layer = Some("");
        rest = remaining.trim_start();
    }
    if let Some((value, remaining)) = take_function(rest, "supports") {
        supports = Some(value);
        rest = remaining.trim_start();
    }
    if !rest.is_empty() {
        source = format!("@media {rest}{{{source}}}");
    }
    if let Some(condition) = supports {
        source = format!("@supports ({condition}){{{source}}}");
    }
    if let Some(name) = layer {
        source = if name.trim().is_empty() {
            format!("@layer{{{source}}}")
        } else {
            format!("@layer {name}{{{source}}}")
        };
    }
    source
}

fn take_function<'a>(source: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let prefix = source.get(..name.len())?;
    if !prefix.eq_ignore_ascii_case(name) || source.as_bytes().get(name.len()) != Some(&b'(') {
        return None;
    }
    let close = function_end(source, name.len())?;
    Some((&source[name.len() + 1..close], &source[close + 1..]))
}

fn take_keyword<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let prefix = source.get(..name.len())?;
    if !prefix.eq_ignore_ascii_case(name) {
        return None;
    }
    source[name.len()..]
        .chars()
        .next()
        .is_none_or(|next| next.is_ascii_whitespace())
        .then(|| &source[name.len()..])
}

fn at_keyword(prelude: &str, name: &str) -> bool {
    let expected_len = name.len() + 1;
    prelude
        .get(..expected_len)
        .is_some_and(|prefix| prefix[0..1].eq("@") && prefix[1..].eq_ignore_ascii_case(name))
        && prelude[expected_len..]
            .chars()
            .next()
            .is_none_or(|next| !is_identifier_char(next))
}

fn is_identifier_char(value: char) -> bool {
    value.is_alphanumeric() || matches!(value, '-' | '_' | '\\') || !value.is_ascii()
}

fn skip_trivia(source: &str, mut cursor: usize) -> usize {
    loop {
        while source
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        if source[cursor..].starts_with("/*") {
            cursor = source[cursor + 2..]
                .find("*/")
                .map_or(source.len(), |offset| cursor + 2 + offset + 2);
        } else {
            return cursor;
        }
    }
}

fn top_level_delimiter(source: &str, start: usize) -> Option<(usize, u8)> {
    let mut cursor = start;
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    while cursor < source.len() {
        if source[cursor..].starts_with("/*") {
            cursor = source[cursor + 2..]
                .find("*/")
                .map_or(source.len(), |offset| cursor + 2 + offset + 2);
            continue;
        }
        let current = source[cursor..]
            .chars()
            .next()
            .expect("cursor remains on a character boundary");
        match current {
            '\'' | '"' => cursor = string_end(source, cursor, current)?,
            '(' => {
                parentheses += 1;
                cursor += 1;
            }
            ')' => {
                parentheses = parentheses.saturating_sub(1);
                cursor += 1;
            }
            '[' => {
                brackets += 1;
                cursor += 1;
            }
            ']' => {
                brackets = brackets.saturating_sub(1);
                cursor += 1;
            }
            delimiter @ (';' | '{') if parentheses == 0 && brackets == 0 => {
                return Some((cursor, delimiter as u8));
            }
            '\\' => {
                cursor += 1;
                if let Some(escaped) = source[cursor..].chars().next() {
                    cursor += escaped.len_utf8();
                }
            }
            _ => cursor += current.len_utf8(),
        }
    }
    None
}

fn canonical_url(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };
    parsed.set_fragment(None);
    parsed.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(source: &str, fetcher: &ResourceFetcher<'_>) -> (String, Vec<String>) {
        let mut warnings = Vec::new();
        let mut budget = ResourceBudget::default();
        let output = expand_stylesheet_imports(
            source,
            Some("https://example.test/css/site.css"),
            Some("https://example.test/css/site.css"),
            Some(fetcher),
            &mut budget,
            &mut warnings,
        );
        (output, crate::render_warnings(&warnings))
    }

    #[test]
    fn recursively_expands_imports_and_rebases_each_sheet() {
        let fetcher = |url: &str| match url {
            "https://example.test/css/a.css" => {
                Some(b"@import '../shared/b.css';.a{background:url(a.png)}".to_vec())
            }
            "https://example.test/shared/b.css" => Some(b".b{background:url(img/b.png)}".to_vec()),
            _ => None,
        };
        let (output, warnings) = expand("@import 'a.css';.root{color:red}", &fetcher);
        assert!(warnings.is_empty(), "{warnings:?}");
        let b = output.find(".b{").expect("nested import");
        let a = output.find(".a{").expect("direct import");
        let root = output.find(".root{").expect("importing sheet");
        assert!(b < a && a < root);
        assert!(output.contains("https://example.test/css/a.png"));
        assert!(output.contains("https://example.test/shared/img/b.png"));
    }

    #[test]
    fn detects_cycles_without_suppressing_the_importing_rules() {
        let fetcher = |url: &str| match url {
            "https://example.test/css/a.css" => Some(b"@import 'site.css';.a{color:red}".to_vec()),
            _ => None,
        };
        let (output, warnings) = expand("@import 'a.css';.root{color:blue}", &fetcher);
        assert!(output.contains(".a{color:red}"));
        assert!(output.contains(".root{color:blue}"));
        assert!(warnings.iter().any(|warning| warning.contains("cycle")));
    }

    #[test]
    fn wraps_import_conditions_for_the_existing_at_rule_parser() {
        let fetcher = |_: &str| Some(b".wide{color:red}".to_vec());
        let (output, warnings) = expand(
            "@import url(a.css) layer(theme) supports(display: grid) screen;",
            &fetcher,
        );
        assert!(warnings.is_empty());
        assert_eq!(
            output,
            "@layer theme{@supports (display: grid){@media screen{.wide{color:red}}}}"
        );
    }

    #[test]
    fn stops_pathological_import_depth_with_a_warning() {
        let fetcher = |url: &str| {
            let index = url
                .rsplit('/')
                .next()?
                .strip_suffix(".css")?
                .parse::<usize>()
                .ok()?;
            Some(format!("@import '{}.css';.level{index}{{color:red}}", index + 1).into_bytes())
        };
        let (output, warnings) = expand("@import '0.css';.root{color:blue}", &fetcher);
        assert!(output.contains(".root{color:blue}"));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("depth limit")));
    }

    #[test]
    fn top_level_scan_accepts_non_ascii_selectors() {
        let source = ".按钮 {}";
        assert_eq!(
            top_level_delimiter(source, 0),
            Some((source.find('{').unwrap(), b'{'))
        );
    }

    #[test]
    fn top_level_scan_ignores_delimiters_in_non_ascii_strings_comments_and_urls() {
        for (source, delimiter) in [
            (r#"@unknown "你好;{";"#, b';'),
            (r#"/* 注释;{ */@import url("主题/样式.css");"#, b';'),
            (r#".按\钮[data-label="中文;{"] /* 注释;{ */ {"#, b'{'),
        ] {
            assert_eq!(
                top_level_delimiter(source, skip_trivia(source, 0)),
                Some((source.len() - 1, delimiter)),
                "source: {source}"
            );
        }
    }

    #[test]
    fn expands_import_with_non_ascii_url_and_rules() {
        let fetcher = |url: &str| {
            assert!(url.ends_with("/%E4%B8%BB%E9%A2%98.css"), "URL: {url}");
            Some(".主题{color:red}".as_bytes().to_vec())
        };
        let (output, warnings) = expand(
            r#"@import url("主题.css");/* 中文注释 */.按钮{content:"你好"}"#,
            &fetcher,
        );

        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(output.contains(".主题{color:red}"));
        assert!(output.contains(r#".按钮{content:"你好"}"#));
    }
}
