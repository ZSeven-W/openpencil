//! End-to-end fixtures. Every brand here is made up; no network is used —
//! the fetcher is an in-memory map.

use std::cell::RefCell;
use std::collections::BTreeMap;

use jian_ops_schema::variable::{VariableScalar, VariableValue};

use super::*;

/// In-memory fetcher that records what was asked for.
struct MapFetcher {
    pages: BTreeMap<String, (String, &'static str)>,
    requested: RefCell<Vec<String>>,
}

impl MapFetcher {
    fn new(entries: &[(&str, &str, &'static str)]) -> Self {
        Self {
            pages: entries
                .iter()
                .map(|(url, body, ct)| (url.to_string(), (body.to_string(), *ct)))
                .collect(),
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl BrandFetcher for MapFetcher {
    fn fetch(&self, url: &str, _kind: FetchKind) -> Result<Fetched, BrandError> {
        self.requested.borrow_mut().push(url.to_string());
        let (body, ct) = self.pages.get(url).ok_or_else(|| BrandError::Fetch {
            url: url.to_string(),
            detail: "404".into(),
        })?;
        Ok(Fetched {
            final_url: url.to_string(),
            body: body.as_bytes().to_vec(),
            content_type: Some((*ct).to_string()),
        })
    }
}

const VERDANT_HTML: &str = r##"<!doctype html><html><head>
<title>Verdant Loop — plant care, simplified</title>
<link rel="stylesheet" href="/assets/app.css">
</head><body><header class="site-header"><a class="logo" href="/">Verdant Loop</a></header>
<main><h1>Healthy plants</h1><button class="btn">Start</button></main></body></html>"##;

/// A shadcn-vocabulary site: tokens named on `:root`, a `.dark` theme.
const VERDANT_CSS: &str = r##"
:root {
  --background: 0 0% 100%;
  --foreground: 160 30% 8%;
  --primary: 162 80% 26%;
  --primary-foreground: 0 0% 100%;
  --muted-foreground: 160 8% 42%;
  --radius: 0.75rem;
  --font-sans: "Grotwell", system-ui, sans-serif;
}
.dark {
  --background: 160 30% 6%;
  --foreground: 150 20% 96%;
  --primary: 160 70% 45%;
  --primary-foreground: 160 30% 8%;
}
body { background: hsl(var(--background)); color: hsl(var(--foreground)); font-family: var(--font-sans); }
.btn { background: hsl(var(--primary)); border-radius: var(--radius); }
"##;

const HARBOR_HTML: &str = r##"<html><head>
<meta name="theme-color" content="#C2410C">
<meta property="og:site_name" content="Cobalt Harbor">
<title>Home | Cobalt Harbor</title>
<style>@import "theme.css";</style>
<link rel="stylesheet" href="https://cdn.example.test/site.css">
<link rel="stylesheet" href="missing.css">
</head><body>
<nav class="topnav"><a href="/">Tours</a><a href="/about">About</a></nav>
<h1>Sail the bay</h1><p>Guided tours.</p>
<a class="btn btn-primary" href="/book">Book</a>
<span class="badge">New</span><span class="badge">Sunset</span>
</body></html>"##;

/// A classic site: no tokens, colours only on elements; a framework rule
/// for a class the page never uses; a dark scheme via media query.
const HARBOR_CSS: &str = r##"
body { background: #FBFAF7; color: #1F2430; font-family: 'Harbor Serif', Georgia, serif; }
h1 { font-family: "Tidewater Display", serif; color: #1F2430; }
a { color: #C2410C; }
.btn-primary { background-color: #C2410C; color: #fff; border-radius: 6px; }
.btn-primary:hover { background-color: #9A3412; }
.btn-danger { background-color: #DC3545; color: #fff; }
.alert-danger { background-color: #DC3545; }
.alert-warning { background-color: #FFC107; }
@media (prefers-color-scheme: dark) { body { background: #101418; color: #E8E6E1; } }
"##;

const HARBOR_THEME_CSS: &str = ".badge { background: #0F766E; color: #fff; border-radius: 999px; }";

fn hex(kit: &BrandKit, dark: bool, name: &str) -> String {
    let tokens = if dark { &kit.dark } else { &kit.light };
    tokens.get(name).unwrap().to_hex()
}

fn assert_contrast_pairs(kit: &BrandKit) {
    for tokens in [&kit.light, &kit.dark] {
        for (surface, text) in CONTRAST_PAIRS {
            let (s, t) = (tokens.get(surface).unwrap(), tokens.get(text).unwrap());
            assert!(t.contrast(s) >= AA_TEXT, "{text} on {surface}");
        }
    }
}

#[test]
fn shadcn_site_maps_tokens_light_and_dark() {
    let fetcher = MapFetcher::new(&[
        (
            "https://verdant.example/",
            VERDANT_HTML,
            "text/html; charset=utf-8",
        ),
        (
            "https://verdant.example/assets/app.css",
            VERDANT_CSS,
            "text/css",
        ),
    ]);
    let kit = extract_from_url("https://verdant.example/", &fetcher).unwrap();
    assert_eq!(kit.name, "Verdant Loop");
    // hsl(162 80% 26%) ≈ #0D775A
    let primary = kit.light.get("--primary").unwrap();
    assert!(
        primary.distance(Rgb::parse_hex("#0D775A").unwrap()) < 8.0,
        "{}",
        primary.to_hex()
    );
    assert_eq!(hex(&kit, false, "--background"), "#FFFFFF");
    assert_eq!(kit.dark_origin, ThemeOrigin::Extracted);
    assert!(kit.dark.get("--background").unwrap().is_dark());
    assert_eq!(kit.font_primary, "Grotwell");
    assert_eq!(kit.radius, 12.0);
    assert!(!kit.pill_buttons);
    assert_contrast_pairs(&kit);
}

#[test]
fn element_site_uses_buttons_links_dom_usage_and_imports() {
    let fetcher = MapFetcher::new(&[
        ("https://harbor.example/", HARBOR_HTML, "text/html"),
        ("https://cdn.example.test/site.css", HARBOR_CSS, "text/css"),
        (
            "https://harbor.example/theme.css",
            HARBOR_THEME_CSS,
            "text/css",
        ),
    ]);
    let kit = extract_from_url("https://harbor.example/", &fetcher).unwrap();
    assert_eq!(kit.name, "Cobalt Harbor");
    assert_eq!(hex(&kit, false, "--primary"), "#C2410C");
    assert_eq!(hex(&kit, false, "--background"), "#FBFAF7");
    assert_eq!(hex(&kit, false, "--foreground"), "#1F2430");
    // The imported badge colour is a second brand colour; the unused
    // framework danger/warning rules are not.
    assert_eq!(kit.charts[0], Rgb::parse_hex("#0F766E").unwrap());
    assert!(
        !kit.charts.contains(&Rgb::parse_hex("#C2410C").unwrap()),
        "primary is not a chart"
    );
    assert_eq!(kit.font_primary, "Harbor Serif");
    assert_eq!(kit.font_secondary, "Tidewater Display");
    assert_eq!(kit.radius, 6.0);
    // The media-query dark theme restyled only ground + ink; the brand
    // carries over and still stands out.
    assert_eq!(kit.dark_origin, ThemeOrigin::Extracted);
    assert_eq!(hex(&kit, true, "--background"), "#101418");
    let dark_primary = kit.dark.get("--primary").unwrap();
    assert!(dark_primary.contrast(kit.dark.get("--background").unwrap()) >= 3.0);
    // A missing stylesheet is skipped, not fatal.
    assert!(fetcher
        .requested
        .borrow()
        .iter()
        .any(|u| u == "https://harbor.example/missing.css"));
    assert_contrast_pairs(&kit);
}

#[test]
fn derives_dark_when_the_site_has_none() {
    let html = "<html><head><style>body{background:#fff;color:#222} .cta{background:#7C3AED;color:#fff}</style></head>\
                <body><a class=cta>Go</a></body></html>";
    let kit = extract_from_html(html, &[], "violet-test");
    assert_eq!(hex(&kit, false, "--primary"), "#7C3AED");
    assert_eq!(kit.dark_origin, ThemeOrigin::Derived);
    assert_eq!(kit.font_primary, "Inter", "system stack falls back");
    assert_contrast_pairs(&kit);
}

#[test]
fn page_fetch_failure_and_non_html_are_errors() {
    let fetcher = MapFetcher::new(&[("https://json.example/", "{\"a\":1}", "application/json")]);
    assert!(matches!(
        extract_from_url("https://nowhere.example/", &fetcher),
        Err(BrandError::Fetch { .. })
    ));
    assert!(matches!(
        extract_from_url("https://json.example/", &fetcher),
        Err(BrandError::NotHtml { .. })
    ));
}

#[test]
fn screenshot_kit_end_to_end() {
    let png = crate::image_palette::tests::landing_png(
        [250, 248, 244],
        [24, 24, 32],
        [230, 80, 20],
        [20, 120, 200],
    );
    let kit = extract_from_image(&png, "/tmp/shots/Ember Bakery.png").unwrap();
    assert_eq!(kit.name, "Ember Bakery");
    assert!(
        kit.light
            .get("--primary")
            .unwrap()
            .distance(Rgb::new(230, 80, 20))
            < 30.0
    );
    assert_eq!(kit.dark_origin, ThemeOrigin::Derived);
    assert_contrast_pairs(&kit);
}

#[test]
fn variables_use_the_palette_vocabulary_on_the_mode_axis() {
    let kit = extract_from_html(
        "<style>body{background:#fff;color:#111} button{background:#0E7C66;border-radius:10px}</style><button>x</button>",
        &[],
        "t",
    );
    let vars = kit.variables();
    assert_eq!(vars.len(), 27 + 6 + 2 + 5);
    let VariableValue::Themed(pair) = &vars["--primary"].value else {
        panic!("themed primary");
    };
    assert_eq!(pair[0].value, VariableScalar::Str("#0E7C66".into()));
    assert_eq!(
        pair[1]
            .theme
            .as_ref()
            .unwrap()
            .get(THEME_AXIS)
            .map(String::as_str),
        Some(THEME_DARK)
    );
    assert_eq!(
        vars["--radius-m"].value,
        VariableValue::Scalar(VariableScalar::Num(10.0))
    );
    assert_eq!(kit.themes()[THEME_AXIS], vec!["Light", "Dark"]);
    let md = kit.design_md();
    assert!(md.raw.starts_with(BRAND_KIT_MARKER));
    assert!(md.generation_notes.unwrap().contains("$--primary"));
    let json = kit.to_json();
    assert_eq!(json["light"]["--primary"], "#0E7C66");
    assert_eq!(json["radius"]["--radius-m"], 10.0);
}

#[test]
fn host_of_strips_scheme_www_port_and_path() {
    assert_eq!(host_of("https://www.acme.example:8443/a?b"), "acme.example");
    assert_eq!(host_of("http://user@shop.example"), "shop.example");
}
