//! Website HTML + CSS → [`BrandSignals`].
//!
//! Two kinds of evidence, strongest first:
//! 1. **Design tokens the site already names** — custom properties on
//!    `:root` / theme hooks (`--primary`, `--brand`, `--bs-primary`,
//!    `--radius`, `--font-sans` …), with `.dark` / `prefers-color-scheme:
//!    dark` scopes read as the dark theme.
//! 2. **What the key elements actually paint** — colours, fonts, and radii
//!    on `body`, buttons, links, headers, headings, cards, weighted by role
//!    and by how often the rule's classes occur in the page's own DOM (so a
//!    CSS framework's unused `.btn-danger` does not outvote the site's CTA).

use std::collections::{BTreeMap, HashMap};

use op_html::css::declarations::parse_declarations;
use op_html::dom::{parse_dom, DomNode, ParsedDom};

use crate::color::{parse_css_color, Rgb};
use crate::css_scan::{scan_stylesheet, ScanOutput, Scheme};
use crate::css_select::{
    first_brand_family, is_state_selector, length_px, lookup_in, resolve_vars, role_of,
    split_selector_list, subject_of, var_scope, Role, Subject, VarScope,
};
use crate::css_tokens::{mode_from_evidence, RoleStats};
use crate::signals::BrandSignals;

/// Elements walked for DOM class counts / inline styles.
const MAX_DOM_ELEMENTS: usize = 20_000;

/// Analyse one page. `sheets` are the page's stylesheets in document order
/// (external sheets already fetched, inline `<style>` blocks included);
/// use [`inline_sheets`] when only the HTML is available.
pub fn analyze_site(html: &str, sheets: &[String]) -> BrandSignals {
    let dom = parse_dom(html);
    let mut scan = ScanOutput::default();
    for sheet in sheets {
        scan_stylesheet(sheet, &mut scan);
    }
    let usage = DomUsage::collect(&dom);

    // ── pass 1: custom properties by theme scope ──────────────────────
    let mut root_vars = BTreeMap::new();
    let mut dark_vars = BTreeMap::new();
    let mut light_vars = BTreeMap::new();
    let mut other_vars = BTreeMap::new();
    for rule in &scan.rules {
        let decls: Vec<_> = parse_declarations(&rule.block)
            .into_iter()
            .filter(|d| d.name.starts_with("--"))
            .collect();
        if decls.is_empty() {
            continue;
        }
        for selector in split_selector_list(&rule.selectors) {
            let scope = match (rule.scheme, var_scope(&selector)) {
                (_, VarScope::Other) => VarScope::Other,
                (Scheme::Dark, _) | (_, VarScope::Dark) => VarScope::Dark,
                (Scheme::Light, _) | (_, VarScope::Light) => VarScope::Light,
                _ => VarScope::Root,
            };
            let target = match scope {
                VarScope::Root => &mut root_vars,
                VarScope::Dark => &mut dark_vars,
                VarScope::Light => &mut light_vars,
                VarScope::Other => &mut other_vars,
            };
            for decl in &decls {
                target.insert(decl.name.clone(), decl.value.clone());
            }
        }
    }
    // An explicit light hook refines the root theme.
    let mut base_vars = root_vars.clone();
    base_vars.extend(light_vars);

    // ── pass 2: what key elements paint ───────────────────────────────
    let mut stats: HashMap<Role, RoleStats> = HashMap::new();
    let mut dark_stats: HashMap<Role, RoleStats> = HashMap::new();
    let mut global = HashMap::new();
    let lookup_light = |name: &str| lookup_in(&[&base_vars, &other_vars], name);
    let lookup_dark_rule = |name: &str| lookup_in(&[&dark_vars, &base_vars, &other_vars], name);
    for rule in &scan.rules {
        for selector in split_selector_list(&rule.selectors) {
            if is_state_selector(&selector) {
                continue;
            }
            let dark = rule.scheme == Scheme::Dark || var_scope(&selector) == VarScope::Dark;
            let subject = subject_of(&selector);
            let role = role_of(&subject);
            let weight = role.weight() * usage.factor(&subject, role);
            if dark {
                let target = dark_stats.entry(role).or_default();
                let mut unused = HashMap::new();
                record_block(&rule.block, &lookup_dark_rule, weight, target, &mut unused);
            } else {
                let target = stats.entry(role).or_default();
                record_block(&rule.block, &lookup_light, weight, target, &mut global);
            }
        }
    }
    for (role, style) in &usage.inline_styles {
        let lookup = |name: &str| lookup_in(&[&base_vars], name);
        let target = stats.entry(*role).or_default();
        record_block(style, &lookup, role.weight(), target, &mut global);
    }

    let lookup_base = |name: &str| lookup_in(&[&base_vars, &other_vars], name);
    let lookup_dark = |name: &str| lookup_in(&[&dark_vars, &base_vars], name);
    let shadcn_like = [
        "--primary-foreground",
        "--card-foreground",
        "--muted-foreground",
    ]
    .iter()
    .any(|n| base_vars.contains_key(*n));
    let theme_color = meta_content(html, "theme-color").and_then(|v| parse_css_color(&v));

    let mut signals = BrandSignals {
        shadcn_like,
        ..BrandSignals::default()
    };
    signals.base = mode_from_evidence(
        &lookup_base,
        &stats,
        &global,
        theme_color,
        shadcn_like,
        "site",
    );
    let has_dark_evidence = !dark_vars.is_empty()
        || dark_stats
            .get(&Role::Page)
            .is_some_and(|s| !s.bg.is_empty());
    if has_dark_evidence {
        let dark = mode_from_evidence(
            &lookup_dark,
            &dark_stats,
            &HashMap::new(),
            None,
            shadcn_like,
            "site dark theme",
        );
        if dark.is_usable() {
            signals.alternate = Some(dark);
        }
    }

    fonts_and_radius(&mut signals, &lookup_base, &stats, &scan);
    signals.name_hint = site_name(html, &dom);
    signals
}

/// Only the page's inline `<style>` blocks, in document order — for a
/// saved HTML file whose linked sheets are not available.
pub fn inline_sheets(html: &str) -> Vec<String> {
    parse_dom(html).style_blocks
}

/// Record one declaration block's colours / fonts / radii.
fn record_block(
    block: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    weight: f64,
    stats: &mut RoleStats,
    global: &mut HashMap<Rgb, f64>,
) {
    for decl in parse_declarations(block) {
        if decl.name.starts_with("--") {
            continue;
        }
        let value = resolve_vars(&decl.value, lookup);
        // A shorthand that carried `var()` was deferred by the parser;
        // re-expand it now that the reference is resolved.
        let expanded = if decl.value.contains("var(") {
            parse_declarations(&format!("{}: {}", decl.name, value))
        } else {
            vec![decl]
        };
        for d in expanded {
            let value = if d.value.contains("var(") {
                resolve_vars(&d.value, lookup)
            } else {
                d.value.clone()
            };
            let name = d.name.as_str();
            let bucket = match name {
                "background-color" => Some(&mut stats.bg),
                "color" | "fill" => Some(&mut stats.text),
                n if n.starts_with("border-") && n.ends_with("-color") => Some(&mut stats.border),
                "outline-color" => Some(&mut stats.border),
                _ => None,
            };
            if let Some(bucket) = bucket {
                if let Some(rgb) = parse_css_color(&value) {
                    *bucket.entry(rgb).or_default() += weight;
                    *global.entry(rgb).or_default() += weight;
                }
                continue;
            }
            match name {
                "font-family" => {
                    if let Some(family) = first_brand_family(&value) {
                        *stats.fonts.entry(family).or_default() += weight;
                    }
                }
                "border-top-left-radius" => {
                    if let Some(px) = length_px(&value) {
                        stats.radii.push((px, weight));
                    } else if value.trim().ends_with('%') {
                        stats.radii.push((9999.0, weight));
                    }
                }
                _ => {}
            }
        }
    }
}

fn fonts_and_radius(
    signals: &mut BrandSignals,
    lookup: &dyn Fn(&str) -> Option<String>,
    stats: &HashMap<Role, RoleStats>,
    scan: &ScanOutput,
) {
    let prop_font = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| lookup(n).and_then(|v| first_brand_family(&resolve_vars(&v, lookup))))
    };
    let top_font = |roles: &[Role]| {
        let mut merged: HashMap<String, f64> = HashMap::new();
        for role in roles {
            if let Some(s) = stats.get(role) {
                for (family, w) in &s.fonts {
                    *merged.entry(family.clone()).or_default() += w;
                }
            }
        }
        top_key(&merged)
    };
    let body = prop_font(&[
        "--font-sans",
        "--font-body",
        "--font-family",
        "--font-primary",
        "--bs-body-font-family",
        "--body-font",
        "--font-base",
        "--default-font-family",
    ])
    .or_else(|| top_font(&[Role::Page]))
    .or_else(|| top_font(&[Role::Other, Role::Card, Role::Link, Role::Button]))
    .or_else(|| font_face_family(scan));
    let heading = prop_font(&[
        "--font-heading",
        "--font-display",
        "--heading-font",
        "--font-title",
        "--bs-heading-font-family",
    ])
    .or_else(|| top_font(&[Role::Heading]));
    if body.is_none() {
        signals.notes.push("fonts: system stack only".into());
    }
    signals.font_heading = heading.filter(|h| Some(h) != body.as_ref());
    signals.font_body = body;

    let prop_radius = [
        "--radius",
        "--border-radius",
        "--bs-border-radius",
        "--radius-md",
        "--radius-base",
        "--rounded",
    ]
    .iter()
    .find_map(|n| lookup(n).and_then(|v| length_px(&resolve_vars(&v, lookup))));
    let button_radius = stats
        .get(&Role::Button)
        .and_then(|s| weighted_median(&s.radii));
    signals.pill_buttons = button_radius.is_some_and(|r| r >= 99.0);
    let control = [Role::Card, Role::Input, Role::Button]
        .iter()
        .filter_map(|r| stats.get(r).and_then(|s| weighted_median(&s.radii)))
        .find(|r| *r < 99.0);
    signals.radius = prop_radius
        .or(button_radius.filter(|r| *r < 99.0))
        .or(control)
        .map(|r| r.clamp(0.0, 32.0));
}

fn font_face_family(scan: &ScanOutput) -> Option<String> {
    scan.font_faces.iter().find_map(|block| {
        parse_declarations(block)
            .into_iter()
            .find(|d| d.name == "font-family")
            .and_then(|d| first_brand_family(&d.value))
    })
}

pub(crate) fn top_key<K: Clone + Ord>(map: &HashMap<K, f64>) -> Option<K> {
    let mut entries: Vec<_> = map.iter().collect();
    // Deterministic: weight desc, then key asc.
    entries.sort_by(|a, b| b.1.total_cmp(a.1).then_with(|| a.0.cmp(b.0)));
    entries.first().map(|(k, _)| (*k).clone())
}

fn weighted_median(values: &[(f64, f64)]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f64 = sorted.iter().map(|(_, w)| w).sum();
    let mut acc = 0.0;
    for (v, w) in &sorted {
        acc += w;
        if acc >= total / 2.0 {
            return Some(*v);
        }
    }
    sorted.last().map(|(v, _)| *v)
}

/// Class / tag occurrence counts from the page's own DOM, plus inline
/// `style` attributes by role.
#[derive(Default)]
struct DomUsage {
    classes: HashMap<String, usize>,
    tags: HashMap<String, usize>,
    inline_styles: Vec<(Role, String)>,
}

impl DomUsage {
    fn collect(dom: &ParsedDom) -> Self {
        let mut usage = DomUsage::default();
        let mut seen = 0usize;
        let mut stack: Vec<&DomNode> = dom.body.iter().rev().collect();
        if let Some(style) = dom.body_attrs.iter().find(|(k, _)| k == "style") {
            usage.inline_styles.push((Role::Page, style.1.clone()));
        }
        while let Some(node) = stack.pop() {
            let DomNode::Element(element) = node else {
                continue;
            };
            seen += 1;
            if seen > MAX_DOM_ELEMENTS {
                break;
            }
            *usage
                .tags
                .entry(element.tag.to_ascii_lowercase())
                .or_default() += 1;
            for class in element.classes() {
                *usage.classes.entry(class.to_ascii_lowercase()).or_default() += 1;
            }
            if let Some(style) = element.attr("style") {
                let subject = Subject {
                    tag: Some(element.tag.to_ascii_lowercase()),
                    classes: element
                        .classes()
                        .iter()
                        .map(|c| c.to_ascii_lowercase())
                        .collect(),
                    raw: element.tag.to_ascii_lowercase(),
                };
                usage
                    .inline_styles
                    .push((role_of(&subject), style.to_string()));
            }
            stack.extend(element.children.iter().rev());
        }
        usage
    }

    /// Usage multiplier for a rule's subject: rules for classes the page
    /// never uses are framework noise; frequently used ones count more.
    fn factor(&self, subject: &Subject, role: Role) -> f64 {
        if role == Role::Page || (self.classes.is_empty() && self.tags.is_empty()) {
            return 1.0;
        }
        if !subject.classes.is_empty() {
            let uses = subject
                .classes
                .iter()
                .map(|c| self.classes.get(c).copied().unwrap_or(0))
                .min()
                .unwrap_or(0);
            return if uses == 0 {
                0.1
            } else {
                1.0 + (1.0 + uses as f64).ln().min(3.0)
            };
        }
        match subject.tag.as_deref() {
            Some(tag) => {
                let uses = self.tags.get(tag).copied().unwrap_or(0);
                if uses == 0 {
                    0.5
                } else {
                    1.0 + 0.5 * (1.0 + uses as f64).ln().min(3.0)
                }
            }
            None => 1.0,
        }
    }
}

/// `<meta name|property="…" content="…">` — `parse_dom` drops `<meta>`, so
/// this reads the raw markup.
pub(crate) fn meta_content(html: &str, key: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(pos) = lower[from..].find("<meta") {
        let start = from + pos;
        let end = lower[start..].find('>').map_or(lower.len(), |e| start + e);
        let tag = &html[start..end];
        let tag_lower = &lower[start..end];
        let names = [
            attr_value(tag, tag_lower, "name"),
            attr_value(tag, tag_lower, "property"),
        ];
        if names.iter().flatten().any(|n| n.eq_ignore_ascii_case(key)) {
            if let Some(content) = attr_value(tag, tag_lower, "content") {
                return Some(content);
            }
        }
        from = end;
    }
    None
}

fn attr_value(tag: &str, tag_lower: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let mut search = 0;
    while let Some(pos) = tag_lower[search..].find(&needle) {
        let at = search + pos;
        let boundary = at == 0 || tag_lower.as_bytes()[at - 1].is_ascii_whitespace();
        let value_start = at + needle.len();
        if boundary {
            let rest = &tag[value_start..];
            let quote = rest.chars().next()?;
            return Some(if quote == '"' || quote == '\'' {
                rest[1..].split(quote).next().unwrap_or("").to_string()
            } else {
                rest.split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("")
                    .to_string()
            });
        }
        search = value_start;
    }
    None
}

/// A short brand name: the site-name meta, else the title's leading part.
fn site_name(html: &str, dom: &ParsedDom) -> Option<String> {
    let from_meta =
        meta_content(html, "og:site_name").or_else(|| meta_content(html, "application-name"));
    let from_title = dom.title.as_deref().map(|title| {
        title
            .split(['|', '–', '—', ':', '·'])
            .next()
            .unwrap_or(title)
            .split(" - ")
            .next()
            .unwrap_or(title)
            .to_string()
    });
    from_meta
        .or(from_title)
        .map(|n| n.trim().chars().take(40).collect::<String>())
        .filter(|n| !n.is_empty())
}
