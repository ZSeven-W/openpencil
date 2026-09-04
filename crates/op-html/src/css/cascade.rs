use crate::css::cascade_display::canonical_display_serialization;
use crate::css::cascade_shared::{is_ident, matching, top_level_delimiter};
use crate::css::declarations::{parse_declarations, resolve_deferred_shorthand, Declaration};
use crate::css::selectors::{matches, matches_for_pseudo, specificity, PseudoElement, Selector};
use crate::dom::DomElement;
use crate::length::{parse_length, LengthCtx};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[path = "cascade_initial.rs"]
mod cascade_initial;
use cascade_initial::initial_value;

#[path = "cascade_logical.rs"]
mod cascade_logical;

pub use super::cascade_parser::{
    parse_stylesheet, parse_stylesheet_for_viewport, parse_stylesheet_for_viewport_with_origin,
    parse_stylesheet_with_origin, StylesheetParser,
};

pub const UA_STYLESHEET: &str = concat!(
    "[hidden]{display:none}body{margin:8px}h1{font-size:32px;font-weight:700;margin:21px 0}",
    "h2{font-size:24px;font-weight:700;margin:20px 0}h3{font-size:19px;font-weight:700;margin:18px 0}",
    "h4{font-size:16px;font-weight:700;margin:21px 0}h5{font-size:13px;font-weight:700;margin:22px 0}",
    "h6{font-size:11px;font-weight:700;margin:24px 0}p{margin:16px 0}ul,ol{margin:16px 0;padding:0 0 0 40px}",
    "b,strong{font-weight:700}i,em{font-style:italic}u{text-decoration:underline}",
    "s,del,strike{text-decoration:line-through}a{color:#0066cc;text-decoration:underline}",
    "code,pre{font-family:monospace}hr{margin:8px 0}",
    "button{font-family:Arial;font-size:13.3333px;font-style:normal;font-weight:400;font-stretch:100%;line-height:normal;text-align:center}",
    // Table box tree. `thead`/`tbody`/`tfoot` use `display:contents` so their
    // rows flatten into the table in document order (html5ever also inserts an
    // implicit `tbody`), and `col`/`colgroup` are read as attributes by the
    // table post-pass rather than mapped to nodes.
    "table{display:table;border-collapse:separate;border-spacing:2px}",
    "thead,tbody,tfoot{display:contents}tr{display:table-row}td,th{display:table-cell}",
    "th{font-weight:700;text-align:center}caption{display:block;text-align:center}",
    "colgroup,col{display:none}",
    // Replaced-element metadata children never paint on their own.
    "source,track,param{display:none}",
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StyleOrigin {
    UserAgent,
    User,
    Author,
}

#[derive(Clone, Debug)]
pub struct CascadeLayer {
    /// Source-order index at each nesting level, outermost first.
    pub path: Vec<u32>,
}

impl PartialEq for CascadeLayer {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for CascadeLayer {}

impl PartialOrd for CascadeLayer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CascadeLayer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

#[derive(Clone, Debug)]
pub struct StyleRule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
    pub order: usize,
    pub origin: StyleOrigin,
    /// Global order of a named/anonymous layer within this origin.
    pub layer: Option<CascadeLayer>,
}

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    pub props: BTreeMap<String, String>,
    pub font_size: f64,
}

impl ComputedStyle {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.props.get(name).map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CascadeScope {
    Unlayered,
    Layer(CascadeLayer),
    Inline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Priority {
    important: bool,
    origin: u8,
    scope: CascadeScope,
    specificity: (u32, u32, u32),
    order: usize,
    declaration_order: usize,
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.important
            .cmp(&other.important)
            .then_with(|| self.origin.cmp(&other.origin))
            .then_with(|| compare_scopes(&self.scope, &other.scope, self.important))
            .then_with(|| self.specificity.cmp(&other.specificity))
            .then_with(|| self.order.cmp(&other.order))
            .then_with(|| self.declaration_order.cmp(&other.declaration_order))
    }
}

#[derive(Clone, Debug)]
struct Candidate {
    target: String,
    priority: Priority,
    value: String,
    deferred_shorthand: Option<String>,
    origin: StyleOrigin,
    scope: CascadeScope,
}

#[derive(Clone, Debug)]
struct CascadedValue {
    target: String,
    value: String,
    deferred_shorthand: Option<String>,
}

fn cascaded<F>(
    path: &[&DomElement],
    rules: &[StyleRule],
    mut selector_matches: F,
    inline: bool,
    logical_rtl: Option<bool>,
) -> BTreeMap<String, CascadedValue>
where
    F: FnMut(&Selector, &[&DomElement]) -> bool,
{
    let mut candidates: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for rule in rules
        .iter()
        .filter(|rule| selector_matches(&rule.selector, path))
    {
        let specificity = specificity(&rule.selector);
        let scope = rule
            .layer
            .clone()
            .map_or(CascadeScope::Unlayered, CascadeScope::Layer);
        for (declaration_order, declaration) in rule.declarations.iter().enumerate() {
            push_candidate(
                &mut candidates,
                declaration,
                rule.origin,
                scope.clone(),
                specificity,
                rule.order,
                declaration_order,
            );
        }
    }
    if inline {
        let declarations = path
            .last()
            .and_then(|element| element.attr("style"))
            .map(parse_declarations)
            .unwrap_or_default();
        for (order, declaration) in declarations.iter().enumerate() {
            push_candidate(
                &mut candidates,
                declaration,
                StyleOrigin::Author,
                CascadeScope::Inline,
                (1, 0, 0),
                usize::MAX,
                order,
            );
        }
        cascade_logical::push_direction_hint(&mut candidates, path);
    }
    if let Some(rtl) = logical_rtl {
        cascade_logical::remap_margin_candidates(&mut candidates, rtl);
    }
    candidates
        .into_iter()
        .filter_map(|(name, candidates)| resolve_candidate(candidates).map(|value| (name, value)))
        .collect()
}

fn push_candidate(
    candidates: &mut BTreeMap<String, Vec<Candidate>>,
    declaration: &Declaration,
    origin: StyleOrigin,
    scope: CascadeScope,
    specificity: (u32, u32, u32),
    order: usize,
    declaration_order: usize,
) {
    let important = declaration.important;
    let priority = Priority {
        important,
        origin: origin_rank(origin, important),
        scope: scope.clone(),
        specificity,
        order,
        declaration_order,
    };
    candidates
        .entry(declaration.name.clone())
        .or_default()
        .push(Candidate {
            target: declaration.name.clone(),
            priority,
            value: declaration.value.clone(),
            deferred_shorthand: declaration.deferred_shorthand.clone(),
            origin,
            scope,
        });
}

fn origin_rank(origin: StyleOrigin, important: bool) -> u8 {
    match (important, origin) {
        (false, StyleOrigin::UserAgent) | (true, StyleOrigin::Author) => 0,
        (false, StyleOrigin::User) | (true, StyleOrigin::User) => 1,
        (false, StyleOrigin::Author) | (true, StyleOrigin::UserAgent) => 2,
    }
}

fn compare_scopes(left: &CascadeScope, right: &CascadeScope, important: bool) -> Ordering {
    match (left, right) {
        (CascadeScope::Inline, CascadeScope::Inline)
        | (CascadeScope::Unlayered, CascadeScope::Unlayered) => Ordering::Equal,
        (CascadeScope::Inline, _) => Ordering::Greater,
        (_, CascadeScope::Inline) => Ordering::Less,
        (CascadeScope::Unlayered, CascadeScope::Layer(_)) => {
            if important {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (CascadeScope::Layer(_), CascadeScope::Unlayered) => {
            if important {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (CascadeScope::Layer(left), CascadeScope::Layer(right)) => {
            let normal = compare_layer_paths(&left.path, &right.path);
            if important {
                normal.reverse()
            } else {
                normal
            }
        }
    }
}

fn compare_layer_paths(left: &[u32], right: &[u32]) -> Ordering {
    for index in 0..=left.len().max(right.len()) {
        let left = left.get(index).copied().unwrap_or(u32::MAX);
        let right = right.get(index).copied().unwrap_or(u32::MAX);
        match left.cmp(&right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

fn resolve_candidate(mut candidates: Vec<Candidate>) -> Option<CascadedValue> {
    candidates.sort_unstable_by(|left, right| right.priority.cmp(&left.priority));
    let mut reverted_origins = BTreeSet::new();
    let mut reverted_scopes = BTreeSet::new();
    for candidate in candidates {
        if reverted_origins.contains(&candidate.origin)
            || reverted_scopes.contains(&(candidate.origin, candidate.scope.clone()))
        {
            continue;
        }
        match candidate.value.trim().to_ascii_lowercase().as_str() {
            "revert" => {
                reverted_origins.insert(candidate.origin);
            }
            "revert-layer" => {
                if matches!(&candidate.scope, CascadeScope::Layer(_)) {
                    reverted_scopes.insert((candidate.origin, candidate.scope));
                } else {
                    reverted_origins.insert(candidate.origin);
                }
            }
            _ => {
                return Some(CascadedValue {
                    target: candidate.target,
                    value: candidate.value,
                    deferred_shorthand: candidate.deferred_shorthand,
                });
            }
        }
    }
    None
}

pub fn compute_style(
    path: &[&DomElement],
    rules: &[StyleRule],
    parent: Option<&ComputedStyle>,
    root_font_size: f64,
) -> ComputedStyle {
    compute_style_for_viewport(path, rules, parent, root_font_size, 0.0, 0.0)
}

/// Viewport-aware variant used for viewport font units and CSS math.
pub fn compute_style_for_viewport(
    path: &[&DomElement],
    rules: &[StyleRule],
    parent: Option<&ComputedStyle>,
    root_font_size: f64,
    viewport_w: f64,
    viewport_h: f64,
) -> ComputedStyle {
    let preliminary = cascaded(path, rules, matches, true, None);
    let direction = compute_raw(preliminary, parent, root_font_size, viewport_w, viewport_h);
    let raw = cascaded(
        path,
        rules,
        matches,
        true,
        Some(direction.get("direction") == Some("rtl")),
    );
    compute_raw(raw, parent, root_font_size, viewport_w, viewport_h)
}

pub fn compute_pseudo_style(
    path: &[&DomElement],
    rules: &[StyleRule],
    parent: Option<&ComputedStyle>,
    root_font_size: f64,
    pseudo: PseudoElement,
) -> ComputedStyle {
    compute_pseudo_style_for_viewport(path, rules, parent, root_font_size, pseudo, 0.0, 0.0)
}

pub fn compute_pseudo_style_for_viewport(
    path: &[&DomElement],
    rules: &[StyleRule],
    parent: Option<&ComputedStyle>,
    root_font_size: f64,
    pseudo: PseudoElement,
    viewport_w: f64,
    viewport_h: f64,
) -> ComputedStyle {
    let preliminary = cascaded(
        path,
        rules,
        |selector, path| matches_for_pseudo(selector, path, pseudo),
        false,
        None,
    );
    let direction = compute_raw(preliminary, parent, root_font_size, viewport_w, viewport_h);
    let raw = cascaded(
        path,
        rules,
        |selector, path| matches_for_pseudo(selector, path, pseudo),
        false,
        Some(direction.get("direction") == Some("rtl")),
    );
    compute_raw(raw, parent, root_font_size, viewport_w, viewport_h)
}

fn compute_raw(
    raw: BTreeMap<String, CascadedValue>,
    parent: Option<&ComputedStyle>,
    root_size: f64,
    viewport_w: f64,
    viewport_h: f64,
) -> ComputedStyle {
    let parent_custom: BTreeMap<_, _> = parent
        .into_iter()
        .flat_map(|style| style.props.iter())
        .filter(|(name, _)| name.starts_with("--"))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let mut custom_source = parent_custom.clone();
    for (name, cascaded) in raw.iter().filter(|(name, _)| name.starts_with("--")) {
        match cascaded.value.trim().to_ascii_lowercase().as_str() {
            "initial" => {
                custom_source.remove(name);
            }
            "inherit" | "unset" => {
                if let Some(value) = parent_custom.get(name) {
                    custom_source.insert(name.clone(), value.clone());
                } else {
                    custom_source.remove(name);
                }
            }
            _ => {
                custom_source.insert(name.clone(), cascaded.value.clone());
            }
        }
    }
    let mut props = BTreeMap::new();
    let mut invalid = BTreeSet::new();
    for name in custom_source.keys() {
        if let Some(value) = resolve_custom(name, &custom_source, &mut Vec::new(), &mut invalid, 0)
        {
            props.insert(name.clone(), value);
        }
    }
    if let Some(cascaded) = raw.get("direction") {
        let resolved = resolve_vars(
            &cascaded.value,
            &custom_source,
            &mut Vec::new(),
            &mut invalid,
            0,
        );
        apply_property(
            &mut props,
            "direction",
            resolved.as_deref().unwrap_or("unset"),
            parent,
        );
    }
    for (name, cascaded) in raw
        .iter()
        .filter(|(name, _)| !name.starts_with("--") && name.as_str() != "direction")
    {
        let resolved = resolve_vars(
            &cascaded.value,
            &custom_source,
            &mut Vec::new(),
            &mut invalid,
            0,
        )
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| match cascaded.deferred_shorthand.as_deref() {
            Some(shorthand) => resolve_deferred_shorthand(shorthand, &cascaded.target, &value),
            None => Some(value),
        });
        apply_property(
            &mut props,
            name,
            resolved.as_deref().unwrap_or("unset"),
            parent,
        );
    }
    if let Some(parent) = parent {
        for &name in INHERITED_PROPERTIES {
            if !props.contains_key(name) {
                if let Some(value) = parent.props.get(name) {
                    props.insert(name.to_string(), value.clone());
                }
            }
        }
    }
    let parent_size = parent.map_or(root_size, |style| style.font_size);
    let font_size = props
        .get("font-size")
        .and_then(|value| {
            resolve_font_size(
                value,
                parent_size,
                root_size,
                LengthCtx {
                    font_size: parent_size,
                    root_font_size: root_size,
                    viewport_w,
                    viewport_h,
                },
            )
        })
        .unwrap_or(parent_size);
    props.insert("font-size".into(), format_px(font_size));
    let parent_weight = parent
        .and_then(|style| style.get("font-weight"))
        .and_then(parse_numeric_weight)
        .unwrap_or(400);
    if let Some(weight) = props
        .get("font-weight")
        .and_then(|value| resolve_font_weight(value, parent_weight))
    {
        props.insert("font-weight".into(), weight.to_string());
    }
    ComputedStyle { props, font_size }
}

fn apply_property(
    props: &mut BTreeMap<String, String>,
    name: &str,
    value: &str,
    parent: Option<&ComputedStyle>,
) {
    let keyword = value.trim().to_ascii_lowercase();
    let inherit = || parent.and_then(|style| style.props.get(name)).cloned();
    if name == "direction" {
        let direction = match keyword.as_str() {
            "ltr" | "rtl" => keyword,
            "initial" => "ltr".to_string(),
            "inherit" | "unset" | "revert" | "revert-layer" => {
                inherit().unwrap_or_else(|| "ltr".to_string())
            }
            // Invalid computed values are discarded. Since `direction` is
            // inherited, the parent's value (or the initial LTR) resurfaces.
            _ => inherit().unwrap_or_else(|| "ltr".to_string()),
        };
        props.insert(name.to_string(), direction);
        return;
    }
    let selected = match keyword.as_str() {
        "inherit" => inherit().or_else(|| initial_value(name).map(str::to_string)),
        "initial" => initial_value(name).map(str::to_string),
        "unset" | "revert" | "revert-layer" if is_inherited(name) => {
            inherit().or_else(|| initial_value(name).map(str::to_string))
        }
        "unset" | "revert" | "revert-layer" => initial_value(name).map(str::to_string),
        _ => Some(value.trim().to_string()),
    };
    if let Some(mut value) = selected {
        if name == "display" {
            value = canonical_display_serialization(&value)
                .or_else(|| initial_value("display"))
                .unwrap_or("inline")
                .to_string();
        }
        props.insert(name.to_string(), value);
    } else {
        props.remove(name);
    }
}

const MAX_VAR_DEPTH: usize = 64;
const MAX_VAR_OUTPUT: usize = 1_048_576;

fn resolve_custom(
    name: &str,
    values: &BTreeMap<String, String>,
    stack: &mut Vec<String>,
    invalid: &mut BTreeSet<String>,
    depth: usize,
) -> Option<String> {
    if invalid.contains(name) {
        return None;
    }
    if depth >= MAX_VAR_DEPTH {
        invalid.extend(stack.iter().cloned());
        invalid.insert(name.to_string());
        return None;
    }
    if let Some(start) = stack.iter().position(|item| item == name) {
        invalid.extend(stack[start..].iter().cloned());
        invalid.insert(name.to_string());
        return None;
    }
    let value = values.get(name)?;
    stack.push(name.to_string());
    let result = resolve_vars(value, values, stack, invalid, depth + 1);
    stack.pop();
    (!invalid.contains(name))
        .then_some(result?)
        .filter(|value| !value.is_empty())
}

fn resolve_vars(
    value: &str,
    custom: &BTreeMap<String, String>,
    stack: &mut Vec<String>,
    invalid: &mut BTreeSet<String>,
    depth: usize,
) -> Option<String> {
    if depth >= MAX_VAR_DEPTH || value.len() > MAX_VAR_OUTPUT {
        return None;
    }
    let mut output = String::new();
    let mut at = 0;
    while let Some(relative) = find_var(&value[at..]) {
        let start = at + relative;
        output.push_str(&value[at..start]);
        let open = start + 3;
        let close = matching(value, open, b'(', b')', value.len())?;
        let (name, fallback) = split_var_body(&value[open + 1..close]);
        let name = name.trim();
        if !valid_custom_name(name) {
            return None;
        }
        let replacement =
            resolve_custom(name, custom, stack, invalid, depth + 1).or_else(|| {
                fallback
                    .and_then(|fallback| resolve_vars(fallback, custom, stack, invalid, depth + 1))
            })?;
        if output.len().saturating_add(replacement.len()) > MAX_VAR_OUTPUT {
            return None;
        }
        output.push_str(&replacement);
        at = close + 1;
    }
    if output.len().saturating_add(value.len() - at) > MAX_VAR_OUTPUT {
        return None;
    }
    output.push_str(&value[at..]);
    Some(output.trim().to_string())
}

fn find_var(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let (mut quote, mut escaped) = (None, false);
    let mut at = 0;
    while at + 4 <= bytes.len() {
        let byte = bytes[at];
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if bytes[at..at + 4].eq_ignore_ascii_case(b"var(")
            && value[..at]
                .chars()
                .next_back()
                .is_none_or(|ch| !is_ident(ch) && ch != '\\')
        {
            return Some(at);
        }
        at += 1;
    }
    None
}

fn split_var_body(body: &str) -> (&str, Option<&str>) {
    match top_level_delimiter(body, ',') {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    }
}

fn valid_custom_name(name: &str) -> bool {
    name.strip_prefix("--")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(is_ident))
}

fn resolve_font_size(
    value: &str,
    parent_size: f64,
    root_size: f64,
    context: LengthCtx,
) -> Option<f64> {
    let keyword = value.trim().to_ascii_lowercase();
    let ratio = match keyword.as_str() {
        "xx-small" => Some(0.6),
        "x-small" => Some(0.75),
        "small" => Some(0.8),
        "medium" | "normal" => Some(1.0),
        "large" => Some(1.2),
        "x-large" => Some(1.5),
        "xx-large" => Some(2.0),
        "xxx-large" => Some(3.0),
        _ => None,
    };
    if let Some(ratio) = ratio {
        return Some(root_size * ratio);
    }
    match keyword.as_str() {
        "larger" => return Some(parent_size * 1.2),
        "smaller" => return Some(parent_size / 1.2),
        _ => {}
    }
    Some(parse_length(&keyword, &context)?.resolve(parent_size))
}

fn resolve_font_weight(value: &str, parent: u32) -> Option<u32> {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" | "regular" => Some(400),
        "bold" => Some(700),
        "bolder" => Some(match parent {
            0..=349 => 400,
            350..=549 => 700,
            550..=899 => 900,
            _ => parent,
        }),
        "lighter" => Some(match parent {
            0..=349 => 100,
            350..=549 => 100,
            550..=749 => 400,
            _ => 700,
        }),
        value => parse_numeric_weight(value),
    }
}

fn parse_numeric_weight(value: &str) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .map(|value| value.clamp(1, 1000))
}

fn format_px(value: f64) -> String {
    if value.fract().abs() < 1e-9 {
        format!("{value:.0}px")
    } else {
        format!("{value}px")
    }
}

fn is_inherited(name: &str) -> bool {
    INHERITED_PROPERTIES.contains(&name)
}

const INHERITED_PROPERTIES: &[&str] = &[
    "border-collapse",
    "border-spacing",
    "caption-side",
    "color",
    "cursor",
    "direction",
    "empty-cells",
    "font-family",
    "font-feature-settings",
    "font-kerning",
    "font-optical-sizing",
    "font-size",
    "font-size-adjust",
    "font-stretch",
    "font-style",
    "font-synthesis",
    "font-variant",
    "font-variant-caps",
    "font-variant-ligatures",
    "font-weight",
    "hyphens",
    "letter-spacing",
    "line-height",
    "list-style",
    "list-style-image",
    "list-style-position",
    "list-style-type",
    "orphans",
    "overflow-wrap",
    "quotes",
    "tab-size",
    "text-align",
    "text-align-last",
    "text-fill-color",
    "text-indent",
    "text-justify",
    "text-orientation",
    "text-rendering",
    "text-shadow",
    "text-transform",
    "text-underline-position",
    "visibility",
    "white-space",
    "widows",
    "word-break",
    "word-spacing",
    "word-wrap",
    "writing-mode",
    "fill",
    "stroke",
    "stroke-width",
];

#[cfg(test)]
#[path = "cascade_deferred_tests.rs"]
mod deferred_tests;
