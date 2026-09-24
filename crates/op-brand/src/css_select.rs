//! Selector reading for brand extraction: what a rule styles (its role)
//! and which theme scope its custom properties belong to. Plus the small
//! value helpers (`var()` resolution, font stacks, lengths) the CSS
//! analysis shares.
//!
//! Selectors are read from their raw text rather than through the
//! cascade's selector parser: brand evidence only needs the subject
//! compound's tag and classes, and a raw reading never drops a rule over
//! an unsupported pseudo-element (`::placeholder`) the way a strict parse
//! would.

use std::collections::BTreeMap;

/// What a rule's subject element is, for weighting its colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Role {
    Page,
    Button,
    Link,
    Header,
    Heading,
    Card,
    Input,
    Footer,
    Other,
}

impl Role {
    /// How strongly a colour on this role speaks for the brand.
    pub(crate) fn weight(self) -> f64 {
        match self {
            Role::Page => 5.0,
            Role::Button => 4.0,
            Role::Link => 3.0,
            Role::Header => 2.5,
            Role::Heading => 2.0,
            Role::Card => 1.5,
            Role::Input | Role::Footer => 1.0,
            Role::Other => 0.5,
        }
    }
}

/// Which theme a custom-property rule defines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VarScope {
    /// `:root`, `html`, `body`, `:host`, `*`, or a light theme hook.
    Root,
    /// `.dark`, `[data-theme="dark"]`, …
    Dark,
    /// Explicit light hooks (`.light`, `[data-theme=light]`).
    Light,
    /// Scoped to a component.
    Other,
}

/// One complex selector's subject compound, read loosely.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Subject {
    pub tag: Option<String>,
    pub classes: Vec<String>,
    pub raw: String,
}

/// Split a selector list on top-level commas.
pub(crate) fn split_selector_list(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in list.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth <= 0 => {
                if !current.trim().is_empty() {
                    out.push(current.trim().to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

const STATE_PSEUDOS: &[&str] = &[
    ":hover",
    ":focus",
    ":active",
    ":visited",
    ":disabled",
    "::placeholder",
    "::selection",
    "::before",
    "::after",
    ":before",
    ":after",
    "::-webkit",
    "::-moz",
];

/// Whether the selector only styles a transient state or pseudo content —
/// hover tints and placeholder greys are not brand colours.
pub(crate) fn is_state_selector(selector: &str) -> bool {
    let lower = selector.to_ascii_lowercase();
    STATE_PSEUDOS.iter().any(|p| lower.contains(p))
}

/// The last compound of a complex selector.
pub(crate) fn subject_of(selector: &str) -> Subject {
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in selector.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ' ' | '>' | '+' | '~' if depth <= 0 => start = i + c.len_utf8(),
            _ => {}
        }
    }
    let raw = selector[start..].trim().to_ascii_lowercase();
    let tag: String = raw
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    Subject {
        tag: (!tag.is_empty()).then_some(tag),
        classes: classes_in(&raw),
        raw,
    }
}

/// Every `.class` token in `text` (lower-cased by the caller).
pub(crate) fn classes_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut depth = 0i32;
    while let Some((_, c)) = chars.next() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            '.' if depth <= 0 => {
                let mut name = String::new();
                while let Some((_, n)) = chars.peek() {
                    if n.is_alphanumeric() || *n == '-' || *n == '_' || *n == '\\' {
                        name.push(*n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !name.is_empty() {
                    out.push(name);
                }
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn role_of(subject: &Subject) -> Role {
    let tag = subject.tag.as_deref().unwrap_or("");
    let class_has = |needles: &[&str]| {
        subject
            .classes
            .iter()
            .any(|c| needles.iter().any(|n| c.contains(n)))
    };
    if subject.raw.contains(":root") || matches!(tag, "html" | "body") {
        return Role::Page;
    }
    if tag == "button"
        || subject.raw.contains("type=submit")
        || subject.raw.contains("type=\"submit\"")
        || subject.raw.contains("role=\"button\"")
        || class_has(&["btn", "button", "cta"])
    {
        return Role::Button;
    }
    if tag == "a" || class_has(&["link"]) {
        return Role::Link;
    }
    if matches!(tag, "header" | "nav") || class_has(&["header", "navbar", "topbar", "nav"]) {
        return Role::Header;
    }
    if matches!(tag, "h1" | "h2" | "h3") || class_has(&["title", "heading", "headline", "hero"]) {
        return Role::Heading;
    }
    if matches!(tag, "input" | "textarea" | "select") || class_has(&["input", "form-control"]) {
        return Role::Input;
    }
    if class_has(&["card", "panel", "tile"]) {
        return Role::Card;
    }
    if tag == "footer" || class_has(&["footer"]) {
        return Role::Footer;
    }
    Role::Other
}

const DARK_CLASSES: &[&str] = &["dark", "dark-mode", "dark-theme", "theme-dark", "is-dark"];
const LIGHT_CLASSES: &[&str] = &["light", "light-mode", "light-theme", "theme-light"];

/// Theme scope of a custom-property rule's selector.
pub(crate) fn var_scope(selector: &str) -> VarScope {
    let lower = selector.to_ascii_lowercase();
    let classes = classes_in(&lower);
    let attr_values = attribute_values(&lower);
    let has = |set: &[&str], word: &str| {
        classes.iter().any(|c| set.contains(&c.as_str()))
            || attr_values.iter().any(|v| v == word || v.contains(set[0]))
    };
    if has(DARK_CLASSES, "dark") {
        return VarScope::Dark;
    }
    let light = has(LIGHT_CLASSES, "light");
    let subject = subject_of(&lower);
    let root_like = lower.contains(":root")
        || lower.contains(":host")
        || matches!(subject.tag.as_deref(), Some("html" | "body"))
        || lower.trim() == "*";
    if light {
        return VarScope::Light;
    }
    if root_like {
        VarScope::Root
    } else {
        VarScope::Other
    }
}

/// Unquoted values of every `[attr=value]` in the selector.
fn attribute_values(lower: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = lower;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            break;
        };
        let inside = &rest[open + 1..open + close];
        if let Some(eq) = inside.find('=') {
            let value = inside[eq + 1..]
                .trim()
                .trim_end_matches(" i")
                .trim_matches(|c| c == '"' || c == '\'');
            out.push(value.to_string());
        }
        rest = &rest[open + close + 1..];
    }
    out
}

/// Substitute `var(--name, fallback)` references using `lookup`, depth
/// capped so a self-referencing property cannot loop.
pub(crate) fn resolve_vars(value: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    resolve_depth(value, lookup, 0)
}

fn resolve_depth(value: &str, lookup: &dyn Fn(&str) -> Option<String>, depth: usize) -> String {
    if depth > 8 || !value.contains("var(") {
        return value.to_string();
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("var(") {
        out.push_str(&rest[..start]);
        let inner_start = start + 4;
        let Some(len) = matching_paren(&rest[inner_start..]) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &rest[inner_start..inner_start + len];
        let (name, fallback) = match top_level_comma(inner) {
            Some(i) => (inner[..i].trim(), Some(inner[i + 1..].trim())),
            None => (inner.trim(), None),
        };
        let resolved = lookup(name)
            .map(|v| resolve_depth(&v, lookup, depth + 1))
            .or_else(|| fallback.map(|f| resolve_depth(f, lookup, depth + 1)))
            .unwrap_or_default();
        out.push_str(&resolved);
        rest = &rest[inner_start + len + 1..];
    }
    out.push_str(rest);
    out
}

/// Length of the text before the `)` that closes an already-open paren.
fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 1i32;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Families that say nothing about a brand: generic keywords, platform
/// UI stacks, and emoji fallbacks.
const NON_BRAND_FAMILIES: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "ui-rounded",
    "-apple-system",
    "blinkmacsystemfont",
    "segoe ui",
    "helvetica neue",
    "helvetica",
    "arial",
    "apple color emoji",
    "segoe ui emoji",
    "segoe ui symbol",
    "noto color emoji",
    "emoji",
    "math",
    "inherit",
    "initial",
    "unset",
    "revert",
];

/// The first family in a `font-family` stack that a brand chose, or
/// `None` when the stack is only platform defaults.
pub(crate) fn first_brand_family(stack: &str) -> Option<String> {
    stack
        .split(',')
        .map(|f| f.trim().trim_matches(|c| c == '"' || c == '\'').trim())
        .filter(|f| !f.is_empty() && !f.contains('(') && !f.starts_with("--"))
        .find(|f| !NON_BRAND_FAMILIES.contains(&f.to_ascii_lowercase().as_str()))
        .map(str::to_string)
}

/// A CSS length in px (`px`, `rem`/`em` at 16 px, unitless `0`). Percent
/// and calc() are not a fixed radius and return `None`.
pub(crate) fn length_px(value: &str) -> Option<f64> {
    let first = value.split_whitespace().next()?.trim();
    if first == "0" {
        return Some(0.0);
    }
    let (number, scale) = if let Some(n) = first.strip_suffix("px") {
        (n, 1.0)
    } else if let Some(n) = first.strip_suffix("rem") {
        (n, 16.0)
    } else if let Some(n) = first.strip_suffix("em") {
        (n, 16.0)
    } else {
        return None;
    };
    let px = number.parse::<f64>().ok()? * scale;
    (px.is_finite() && px >= 0.0).then_some(px)
}

/// Case-sensitive custom-property lookup across maps, first hit wins.
pub(crate) fn lookup_in(maps: &[&BTreeMap<String, String>], name: &str) -> Option<String> {
    maps.iter().find_map(|m| m.get(name).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_follow_the_subject_compound() {
        let role = |s: &str| role_of(&subject_of(s));
        assert_eq!(role("body"), Role::Page);
        assert_eq!(role(".site-header .btn-primary"), Role::Button);
        assert_eq!(role("nav a"), Role::Link);
        assert_eq!(role("header.site"), Role::Header);
        assert_eq!(role("main > h1"), Role::Heading);
        assert_eq!(role(".pricing-card"), Role::Card);
        assert_eq!(role("ul li"), Role::Other);
    }

    #[test]
    fn state_selectors_are_skipped() {
        assert!(is_state_selector(".btn:hover"));
        assert!(is_state_selector("input::placeholder"));
        assert!(!is_state_selector(".btn-primary"));
    }

    #[test]
    fn var_scopes_recognise_theme_hooks() {
        assert_eq!(var_scope(":root"), VarScope::Root);
        assert_eq!(var_scope(".dark"), VarScope::Dark);
        assert_eq!(var_scope("html[data-theme=\"dark\"]"), VarScope::Dark);
        assert_eq!(var_scope("[data-bs-theme=dark]"), VarScope::Dark);
        assert_eq!(var_scope(":root.light"), VarScope::Light);
        assert_eq!(var_scope(".darken"), VarScope::Other);
        assert_eq!(var_scope(".card"), VarScope::Other);
    }

    #[test]
    fn var_resolution_handles_fallbacks_and_cycles() {
        let mut map = BTreeMap::new();
        map.insert("--brand".to_string(), "#6D28D9".to_string());
        map.insert("--loop".to_string(), "var(--loop)".to_string());
        let lookup = |n: &str| lookup_in(&[&map], n);
        assert_eq!(resolve_vars("var(--brand)", &lookup), "#6D28D9");
        assert_eq!(resolve_vars("var(--missing, #111)", &lookup), "#111");
        assert_eq!(
            resolve_vars("1px solid var(--missing, var(--brand))", &lookup),
            "1px solid #6D28D9"
        );
        let _ = resolve_vars("var(--loop)", &lookup);
    }

    #[test]
    fn font_stacks_skip_platform_defaults() {
        assert_eq!(
            first_brand_family("\"Nimbus Grotesk\", -apple-system, sans-serif").as_deref(),
            Some("Nimbus Grotesk")
        );
        assert_eq!(
            first_brand_family("-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"),
            None
        );
    }

    #[test]
    fn lengths_convert_to_px() {
        assert_eq!(length_px("0.5rem"), Some(8.0));
        assert_eq!(length_px("6px 6px"), Some(6.0));
        assert_eq!(length_px("50%"), None);
    }
}
