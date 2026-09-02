//! The handful of colours that identify a scene template at a glance.
//!
//! A gallery card is a picture with a caption, and two decks rendered at
//! thumbnail size are often indistinguishable — the thing a user is actually
//! choosing between is the palette. So every card paints a band of the
//! template's own colours under its preview, and this module is where those
//! colours come from.
//!
//! They are read out of the document rather than hand-listed beside the
//! catalogue for the same reason the catalogue refuses an entry without a
//! document: a hand-kept table drifts, and a swatch that no longer matches
//! the file is a worse promise than no swatch at all.
//!
//! Reading them is not free — the documents are `include_str!`ed JSON of tens
//! of kilobytes each — and the gallery repaints every frame, so the result is
//! **memoized by template id**. That is safe here in the strongest possible
//! way: the documents are baked into the binary and cannot change at runtime,
//! so there is no invalidation path to get wrong (contrast
//! `op_ai_skills::style_guide::summary`, whose imports do change and whose
//! mutators therefore drop the memo).

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;

use crate::scene_template_catalog::scene_template_document;

/// Widest band a card paints. Six reads as a palette; more reads as a chart,
/// and at card width each stripe would be too thin to name a colour.
pub const TEMPLATE_PALETTE_MAX: usize = 6;

/// Fewer than this many colours out of the document's variables means the
/// file does not declare a palette in the usual shape, and the fill-frequency
/// fallback takes over.
const PALETTE_MIN_FROM_VARIABLES: usize = 3;

/// The roles a band shows, in the order it shows them.
///
/// Templates name their variables in two conventions — `c-bg` / `c-ink` in
/// the hand-authored decks, `--background` / `--primary` in the generated
/// ones — and both put the same ideas in different orders. Hoisting the roles
/// this table names, then filling the rest in declaration order, gives one
/// reading (ground, surface, accent, ink, …) across both, without a
/// per-template list to maintain.
const ROLE_KEYWORDS: [&[&str]; 6] = [
    &["bg", "background", "paper", "canvas", "base"],
    &["surface", "card", "panel", "tile", "band"],
    &["accent", "primary", "brand", "signal", "highlight"],
    &["ink", "text", "foreground", "fg"],
    &["muted", "secondary", "subtle"],
    &["border", "line", "rule", "hair", "grid"],
];

/// A template's palette as canonical `#RRGGBB` strings, memoized by id.
///
/// An unknown id — or a document that declares nothing readable — returns an
/// empty slice rather than an invented palette: a card with no band says "no
/// palette was declared", which is true, where a fabricated one would say
/// something false about the file the user is about to open.
pub fn scene_template_palette(template_id: &str) -> Arc<Vec<String>> {
    if let Ok(memo) = memo().read() {
        if let Some(hit) = memo.get(template_id) {
            return hit.clone();
        }
    }
    let palette = Arc::new(extract_palette(
        scene_template_document(template_id).unwrap_or_default(),
    ));
    if let Ok(mut memo) = memo().write() {
        memo.insert(template_id.to_string(), palette.clone());
    }
    palette
}

type Memo = HashMap<String, Arc<Vec<String>>>;

fn memo() -> &'static RwLock<Memo> {
    static MEMO: LazyLock<RwLock<Memo>> = LazyLock::new(|| RwLock::new(HashMap::new()));
    &MEMO
}

/// How many documents have been parsed since the process started.
///
/// The memo is the whole reason this module exists, and a memo that silently
/// stops working costs a JSON parse per card per frame while every assertion
/// about the *colours* still passes. This counter is what lets a test see the
/// difference.
#[cfg(test)]
pub(crate) fn palette_parse_count() -> u64 {
    PARSE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

static PARSE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Read a palette out of one document's JSON.
///
/// Public to the crate so the tests can drive it with a fixture instead of
/// only through the memo, which would let a fixture leak into a shipped id's
/// cached answer.
pub(crate) fn extract_palette(document: &str) -> Vec<String> {
    PARSE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let Ok(parsed) = serde_json::from_str::<PaletteDocument>(document) else {
        return Vec::new();
    };
    let from_variables = palette_from_variables(&parsed.variables);
    if from_variables.len() >= PALETTE_MIN_FROM_VARIABLES {
        return from_variables;
    }
    // The fallback is not a merge: a document with one or two declared
    // variables is one whose palette lives in its literal fills, and mixing
    // the two orders would put a stray variable at the head of a band that is
    // otherwise ranked by how much of the design each colour paints.
    let by_frequency = palette_from_fills(&parsed.children);
    if by_frequency.len() > from_variables.len() {
        by_frequency
    } else {
        from_variables
    }
}

/// Colour variables, role-ranked then in declaration order.
fn palette_from_variables(variables: &OrderedMap<VariableDefinition>) -> Vec<String> {
    let declared: Vec<(&str, String)> = variables
        .0
        .iter()
        .filter(|(_, definition)| definition.kind.as_deref() == Some("color"))
        .filter_map(|(name, definition)| {
            Some((name.as_str(), declared_hex(definition.value.as_ref()?)?))
        })
        .collect();

    // Roles first, each satisfied by the earliest variable that names it,
    // then every remaining variable in declaration order. Duplicate indices
    // are harmless — `push_unique` dedupes on the colour, which is also what
    // keeps a template that spells one hex under two names from spending two
    // stripes on it.
    let mut ranked: Vec<usize> = Vec::new();
    for keywords in ROLE_KEYWORDS {
        if let Some(index) = declared
            .iter()
            .position(|(name, _)| matches(name, keywords))
        {
            if !ranked.contains(&index) {
                ranked.push(index);
            }
        }
    }
    ranked.extend(0..declared.len());

    let mut palette = Vec::new();
    for index in ranked {
        push_unique(&mut palette, declared[index].1.clone());
        if palette.len() == TEMPLATE_PALETTE_MAX {
            break;
        }
    }
    palette
}

/// Literal solid fills, most-painted first.
///
/// The count is over occurrences in the tree rather than over painted area:
/// area needs a layout pass, and the question a band answers — "which colours
/// is this design made of" — is served well enough by how often the file
/// reaches for each one.
fn palette_from_fills(children: &[serde_json::Value]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for child in children {
        collect_fill_colors(child, &mut counts);
    }
    // Stable by construction: `counts` is in first-appearance order, and the
    // sort is stable, so equally-used colours keep the document's order
    // instead of flipping between builds.
    counts.sort_by(|left, right| right.1.cmp(&left.1));
    counts
        .into_iter()
        .map(|(hex, _)| hex)
        .take(TEMPLATE_PALETTE_MAX)
        .collect()
}

fn collect_fill_colors(value: &serde_json::Value, counts: &mut Vec<(String, usize)>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, field) in fields {
                if key == "color" {
                    if let Some(hex) = field.as_str().and_then(canonical_hex) {
                        match counts.iter_mut().find(|(seen, _)| *seen == hex) {
                            Some((_, count)) => *count += 1,
                            None => counts.push((hex, 1)),
                        }
                    }
                }
                collect_fill_colors(field, counts);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_fill_colors(item, counts);
            }
        }
        _ => {}
    }
}

/// The hex a colour variable declares, across both shapes a `.op` uses.
///
/// A themed variable states one value per theme-axis value
/// (`[{value, theme: {Mode: "Light"}}, …]`) and the document does not record
/// which one is active, so the band takes the **first** — which is the first
/// value the file's own `themes` axis declares, and therefore what the
/// template opens in.
fn declared_hex(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(raw) => canonical_hex(raw),
        serde_json::Value::Array(entries) => entries
            .iter()
            .find_map(|entry| canonical_hex(entry.get("value")?.as_str()?)),
        _ => None,
    }
}

/// `#RRGGBB` for anything the shared parser accepts, or `None`.
///
/// A `$c-accent` reference resolves to nothing here on purpose: a band that
/// painted the *reference* would need the whole variable resolver, and every
/// document that uses references also declares them, so the variables path
/// has already answered.
fn canonical_hex(raw: &str) -> Option<String> {
    let [r, g, b, _] =
        op_util::hex_color::parse_hex_rgba8(raw, op_util::hex_color::HexOptions::LENIENT)?;
    Some(format!("#{r:02X}{g:02X}{b:02X}"))
}

fn matches(name: &str, keywords: &[&str]) -> bool {
    let lowered = name.to_lowercase();
    keywords.iter().any(|keyword| {
        lowered
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|segment| segment == *keyword)
    })
}

fn push_unique(palette: &mut Vec<String>, hex: String) {
    if !palette.contains(&hex) {
        palette.push(hex);
    }
}

/// Only the two fields a palette needs. Everything else in a `.op` document —
/// the whole node tree's geometry and typography — is skipped by serde rather
/// than materialized, which is most of why one parse per template is cheap
/// enough to do at all.
#[derive(Deserialize)]
struct PaletteDocument {
    #[serde(default)]
    variables: OrderedMap<VariableDefinition>,
    #[serde(default)]
    children: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct VariableDefinition {
    #[serde(rename = "type")]
    kind: Option<String>,
    /// A hex string, or the per-theme array a themed variable declares —
    /// see [`declared_hex`].
    value: Option<serde_json::Value>,
}

/// A JSON object kept in **declaration order**.
///
/// `serde_json::Map` is a `BTreeMap` in this build, so deserializing the
/// variables through it would sort them alphabetically and hand the band
/// `c-accent` before `c-bg` for reasons that have nothing to do with the
/// design. Authors write their palettes ground-first; this preserves that.
struct OrderedMap<T>(Vec<(String, T)>);

impl<T> Default for OrderedMap<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for OrderedMap<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for OrderedVisitor<T> {
            type Value = OrderedMap<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                mut access: M,
            ) -> Result<OrderedMap<T>, M::Error> {
                let mut entries = Vec::with_capacity(access.size_hint().unwrap_or(0));
                while let Some((key, value)) = access.next_entry()? {
                    entries.push((key, value));
                }
                Ok(OrderedMap(entries))
            }
        }

        deserializer.deserialize_map(OrderedVisitor(std::marker::PhantomData))
    }
}

#[cfg(test)]
#[path = "scene_template_palette_tests.rs"]
mod scene_template_palette_tests;
