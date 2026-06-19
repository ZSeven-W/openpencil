//! Bundled Iconify catalog plus lightweight SVG-body parsing.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconRenderStyle {
    Stroke,
    Fill,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    icons: Vec<IconCatalogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct IconCatalogEntry {
    pub collection: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub style: IconRenderStyle,
    pub d: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedIconBody {
    pub width: f32,
    pub height: f32,
    pub style: IconRenderStyle,
    pub d: String,
}

// Only the general-purpose UI sets (lucide + feather, ~0.46 MB) are embedded.
// The ~3700 simple-icons brand logos (~4.83 MB) are loaded at runtime via
// `set_brand_catalog` — embedded at startup on desktop, fetched from the daemon
// on web — so they never bloat the wasm first-load. Both stores yield `'static`
// references (each backed by a `OnceLock`).
const CORE_CATALOG_JSON: &str = include_str!("../../assets/iconify-catalog-core.json");

/// Brand-logo catalog (simple-icons): set once at runtime. `None` until loaded;
/// lookups / searches simply skip it while it is absent.
static BRAND_CATALOG: OnceLock<BrandCatalog> = OnceLock::new();

struct BrandCatalog {
    icons: Vec<IconCatalogEntry>,
    index: HashMap<String, usize>,
}

/// Install the brand-logo catalog from JSON (same shape as the core asset).
/// Returns `true` when newly installed, `false` if the JSON is invalid or the
/// catalog was already set (set-once; later calls are ignored).
pub fn set_brand_catalog(json: &str) -> bool {
    if BRAND_CATALOG.get().is_some() {
        return false;
    }
    let Ok(catalog) = serde_json::from_str::<Catalog>(json) else {
        return false;
    };
    let index = catalog
        .icons
        .iter()
        .enumerate()
        .map(|(idx, icon)| (format!("{}:{}", icon.collection, icon.name), idx))
        .collect();
    BRAND_CATALOG
        .set(BrandCatalog {
            icons: catalog.icons,
            index,
        })
        .is_ok()
}

/// Whether the brand-logo catalog has been loaded yet.
pub fn brand_catalog_loaded() -> bool {
    BRAND_CATALOG.get().is_some()
}

pub fn lookup_icon(collection: &str, name: &str) -> Option<&'static IconCatalogEntry> {
    let key = format!("{}:{}", collection.trim(), name.trim());
    if let Some(idx) = core_index().get(&key) {
        return core_catalog().get(*idx);
    }
    let brands = BRAND_CATALOG.get()?;
    brands
        .index
        .get(&key)
        .and_then(|idx| brands.icons.get(*idx))
}

pub fn search_icons(query: &str, limit: usize) -> Vec<&'static IconCatalogEntry> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return all_icons().take(limit).collect();
    }
    let mut out = Vec::new();
    for icon in all_icons() {
        if icon.name == query || format!("{}:{}", icon.collection, icon.name) == query {
            out.push(icon);
            if out.len() >= limit {
                return out;
            }
        }
    }
    for icon in all_icons() {
        if (icon.name != query && matches_query(icon, &query))
            || format!("{}:{}", icon.collection, icon.name) == query
        {
            out.push(icon);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// Core (embedded) icons first, then brand logos if loaded. Both halves are
/// `'static`, so the chained iterator yields `&'static IconCatalogEntry`.
fn all_icons() -> impl Iterator<Item = &'static IconCatalogEntry> {
    core_catalog().iter().chain(
        BRAND_CATALOG
            .get()
            .into_iter()
            .flat_map(|brands| brands.icons.iter()),
    )
}

pub fn parse_iconify_body(body: &str, width: f32, height: f32) -> Option<ParsedIconBody> {
    let mut paths = paths_from_body(body);
    if paths.is_empty() {
        return None;
    }
    for d in paths.iter_mut().skip(1) {
        if d.starts_with('m') {
            d.replace_range(0..1, "M");
        }
    }
    Some(ParsedIconBody {
        width,
        height,
        style: style_from_body(body),
        d: paths.join(" "),
    })
}

fn core_catalog() -> &'static [IconCatalogEntry] {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    &CATALOG
        .get_or_init(|| {
            serde_json::from_str(CORE_CATALOG_JSON).expect("bundled core icon catalog is valid")
        })
        .icons
}

fn core_index() -> &'static HashMap<String, usize> {
    static INDEX: OnceLock<HashMap<String, usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        core_catalog()
            .iter()
            .enumerate()
            .map(|(idx, icon)| (format!("{}:{}", icon.collection, icon.name), idx))
            .collect()
    })
}

fn matches_query(icon: &IconCatalogEntry, query: &str) -> bool {
    icon.name.contains(query)
        || icon.collection.contains(query)
        || format!("{}:{}", icon.collection, icon.name).contains(query)
}

fn paths_from_body(body: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for tag in tags(body, "path") {
        if invisible(tag) {
            continue;
        }
        if let Some(d) = attr(tag, "d") {
            paths.push(d);
        }
    }
    for tag in tags(body, "line") {
        paths.push(format!(
            "M{} {}L{} {}",
            num(attr(tag, "x1").as_deref(), 0.0),
            num(attr(tag, "y1").as_deref(), 0.0),
            num(attr(tag, "x2").as_deref(), 0.0),
            num(attr(tag, "y2").as_deref(), 0.0)
        ));
    }
    for tag in tags(body, "circle") {
        let cx = num(attr(tag, "cx").as_deref(), 0.0);
        let cy = num(attr(tag, "cy").as_deref(), 0.0);
        let r = num(attr(tag, "r").as_deref(), 0.0);
        paths.push(format!(
            "M{} {}A{} {} 0 1 0 {} {}A{} {} 0 1 0 {} {}Z",
            cx - r,
            cy,
            r,
            r,
            cx + r,
            cy,
            r,
            r,
            cx - r,
            cy
        ));
    }
    for tag in tags(body, "ellipse") {
        let cx = num(attr(tag, "cx").as_deref(), 0.0);
        let cy = num(attr(tag, "cy").as_deref(), 0.0);
        let rx = num(attr(tag, "rx").as_deref(), 0.0);
        let ry = num(attr(tag, "ry").as_deref(), 0.0);
        paths.push(format!(
            "M{} {}A{} {} 0 1 0 {} {}A{} {} 0 1 0 {} {}Z",
            cx - rx,
            cy,
            rx,
            ry,
            cx + rx,
            cy,
            rx,
            ry,
            cx - rx,
            cy
        ));
    }
    for tag in tags(body, "rect") {
        paths.push(rect_path(
            num(attr(tag, "x").as_deref(), 0.0),
            num(attr(tag, "y").as_deref(), 0.0),
            num(attr(tag, "width").as_deref(), 0.0),
            num(attr(tag, "height").as_deref(), 0.0),
            num(attr(tag, "rx").or_else(|| attr(tag, "ry")).as_deref(), 0.0),
        ));
    }
    for tag in tags(body, "polyline") {
        if let Some(points) = attr(tag, "points").and_then(|p| points_path(&p, false)) {
            paths.push(points);
        }
    }
    for tag in tags(body, "polygon") {
        if let Some(points) = attr(tag, "points").and_then(|p| points_path(&p, true)) {
            paths.push(points);
        }
    }
    paths
}

fn tags<'a>(body: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!("<{name}");
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(&needle) {
        rest = &rest[start..];
        let Some(end) = rest.find('>') else {
            break;
        };
        out.push(&rest[..=end]);
        rest = &rest[end + 1..];
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{name}=""#);
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

fn num(value: Option<&str>, fallback: f32) -> f32 {
    value
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(fallback)
}

fn invisible(tag: &str) -> bool {
    attr(tag, "fill").as_deref() == Some("none") && attr(tag, "stroke").as_deref() == Some("none")
}

fn style_from_body(body: &str) -> IconRenderStyle {
    let has_stroke = body.contains("stroke=")
        || body.contains("stroke-width=")
        || body.contains("stroke-linecap=")
        || body.contains(r#"fill="none""#);
    let has_fill =
        body.contains(r#"fill="currentColor""#) || body.contains(r#"fill='currentColor'"#);
    if has_stroke && !has_fill {
        IconRenderStyle::Stroke
    } else {
        IconRenderStyle::Fill
    }
}

fn rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> String {
    if r <= 0.0 {
        return format!("M{x} {y}H{}V{}H{x}Z", x + w, y + h);
    }
    let rr = r.min(w / 2.0).min(h / 2.0);
    format!(
        "M{} {y}H{}A{rr} {rr} 0 0 1 {} {}V{}A{rr} {rr} 0 0 1 {} {}H{}A{rr} {rr} 0 0 1 {x} {}V{}A{rr} {rr} 0 0 1 {} {y}Z",
        x + rr,
        x + w - rr,
        x + w,
        y + rr,
        y + h - rr,
        x + w - rr,
        y + h,
        x + rr,
        y + h - rr,
        y + rr,
        x + rr
    )
}

fn points_path(points: &str, close: bool) -> Option<String> {
    let nums: Vec<f32> = points
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|p| p.parse::<f32>().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let mut out = String::from("M");
    for (idx, pair) in nums.chunks_exact(2).enumerate() {
        if idx > 0 {
            out.push('L');
        }
        out.push_str(&format!("{} {}", pair[0], pair[1]));
    }
    if close {
        out.push('Z');
    }
    Some(out)
}
