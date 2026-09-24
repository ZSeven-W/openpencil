//! Golden-output tests for the `react-tailwind` target.
//!
//! Each fixture under `fixtures/tailwind/<name>.op` has a checked-in
//! `<name>.tsx.golden` holding the full single-file output (component +
//! globals.css + tailwind.config.ts). Regenerate after an intentional
//! change with `UPDATE_GOLDEN=1 cargo test -p op-codegen tailwind`.

use super::*;
use std::path::PathBuf;

const FIXTURES: &[&str] = &["card_variables", "flex_row", "shadcn_refs", "dark_theme"];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/tailwind")
}

fn load(name: &str) -> PenDocument {
    let path = fixture_dir().join(format!("{name}.op"));
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    jian_ops_schema::load_str(&text)
        .unwrap_or_else(|e| panic!("parse {name}: {e:?}"))
        .value
}

fn check_golden(name: &str, actual: &str) {
    let path = fixture_dir().join(format!("{name}.tsx.golden"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (run with UPDATE_GOLDEN=1)", path.display()));
    assert!(
        expected == actual,
        "{name}: generated output drifted from {}\n--- actual ---\n{actual}",
        path.display()
    );
}

/// Structural JSX sanity: every opened tag closes in order, braces
/// balance, and no raw `{`/`}` leaks into text. Not a TypeScript
/// compiler, but it catches the malformed-markup class of bugs.
fn assert_balanced_jsx(name: &str, component: &str) {
    let start = component.find("return (").expect("return");
    let end = component.rfind("\n  )\n").unwrap_or(component.len());
    let body = &component[start..end];
    let mut stack: Vec<String> = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut braces = 0i32;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => braces += 1,
            b'}' => braces -= 1,
            b'<' => {
                let end = body[i..].find('>').expect("tag end") + i;
                let tag = &body[i + 1..end];
                if let Some(closing) = tag.strip_prefix('/') {
                    let open = stack.pop().unwrap_or_default();
                    assert_eq!(open, closing.trim(), "{name}: mismatched </{closing}>");
                } else if !tag.ends_with('/') {
                    let tag_name: String = tag
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric())
                        .collect();
                    stack.push(tag_name);
                }
                i = end;
            }
            _ => {}
        }
        i += 1;
    }
    assert!(stack.is_empty(), "{name}: unclosed tags {stack:?}");
    assert_eq!(braces, 0, "{name}: unbalanced braces");
}

#[test]
fn tailwind_fixtures_match_golden_output() {
    for name in FIXTURES {
        let doc = load(name);
        let output = ReactTailwind.generate(&doc);
        check_golden(name, &output);
        let files = ReactTailwind.generate_files(&doc);
        assert_balanced_jsx(name, &files.component);
        // Everything after the component is commented out, so the
        // single-file view stays a valid TSX module.
        let tail = &output[files.component.len()..];
        assert!(
            tail.lines().all(|l| l.is_empty() || l.starts_with("//")),
            "{name}: uncommented theme line"
        );
    }
}

#[test]
fn tailwind_output_is_byte_stable_across_runs() {
    for name in FIXTURES {
        let first = ReactTailwind.generate(&load(name));
        for _ in 0..5 {
            assert_eq!(first, ReactTailwind.generate(&load(name)), "{name}");
        }
    }
}

#[test]
fn card_maps_variables_to_token_classes_and_binds_both_themes() {
    let files = ReactTailwind.generate_files(&load("card_variables"));
    let tsx = &files.component;
    assert!(
        tsx.contains("export default function PricingCard()"),
        "{tsx}"
    );
    for class in [
        "bg-card",
        "border border-border",
        "rounded-lg",
        "text-card-foreground",
        "text-muted-foreground",
        "bg-primary/90",
        "flex flex-col items-start gap-3",
        "p-6",
        "w-[360px]",
        "text-3xl font-bold leading-tight",
        "self-stretch",
        "px-4",
        "shadow-[0px_1px_3px_0px_rgba(0,0,0,0.08)]",
    ] {
        assert!(tsx.contains(class), "missing {class:?} in\n{tsx}");
    }
    // JSX-unsafe characters in text are entity-encoded.
    assert!(tsx.contains("&#123;unlimited&#125; projects &amp; &lt;priority&gt;"));
    let css = &files.globals_css;
    assert!(css.contains(":root {\n  --background: #FFFFFF;"), "{css}");
    assert!(css.contains(".dark {\n  --background: #09090B;"), "{css}");
    assert!(css.contains("  --radius: 10px;"), "{css}");
    assert!(css.contains("  --color-card-foreground: var(--card-foreground);"));
    assert!(css.contains("  --radius-lg: var(--radius);"));
    // Non-themed tokens do not repeat in the dark block.
    let dark = &css[css.find(".dark {").unwrap()..];
    assert!(!dark[..dark.find('}').unwrap()].contains("--primary:"));
    let config = &files.tailwind_config;
    assert!(config.contains("\"card-foreground\": \"var(--card-foreground)\""));
    assert!(config.contains("lg: \"var(--radius)\""));
}

#[test]
fn flex_row_uses_spacing_scale_and_arbitrary_values_off_scale() {
    let tsx = ReactTailwind.generate_files(&load("flex_row")).component;
    for class in [
        "flex items-center justify-between gap-[13px]",
        "pt-2.5 pr-4 pb-2.5 pl-5",
        "border-b border-[#e2e8f0]",
        "w-[1200px] h-16",
        "flex-1 min-w-0",
        "w-10 h-10 shrink-0 bg-[#22c55e] rounded-full",
        "object-cover",
        "text-[15px]",
        "font-['Inter']",
        "relative grid",
        "col-start-1 row-start-1",
        "absolute left-[26px] top-1.5",
        "bg-[linear-gradient(180deg,#6366f1_0%,#ec4899_100%)]",
    ] {
        assert!(tsx.contains(class), "missing {class:?} in\n{tsx}");
    }
    assert!(tsx.contains("import { SearchIcon } from \"lucide-react\""));
    assert!(tsx.contains("<SearchIcon className=\"w-4 h-4 shrink-0 text-[#64748b]\" />"));
}

#[test]
fn shadcn_kit_instances_become_component_usages() {
    let files = ReactTailwind.generate_files(&load("shadcn_refs"));
    // shadcn components style themselves with the core tokens, so those
    // are bound even though the design never references them directly.
    for token in ["--primary:", "--primary-foreground:", "--input:", "--ring:"] {
        assert!(files.globals_css.contains(token), "{}", files.globals_css);
    }
    let tsx = files.component;
    for import in [
        "import { Badge } from \"@/components/ui/badge\"",
        "import { Button } from \"@/components/ui/button\"",
        "import { Card, CardContent, CardDescription, CardHeader, CardTitle } from \"@/components/ui/card\"",
        "import { Tabs, TabsList, TabsTrigger } from \"@/components/ui/tabs\"",
        "import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from \"@/components/ui/table\"",
    ] {
        assert!(tsx.contains(import), "missing {import:?} in\n{tsx}");
    }
    for usage in [
        "<Button variant=\"outline\">Cancel</Button>",
        "<Button>Save</Button>",
        "<Button>Choose</Button>",
        "<Badge>New</Badge>",
        "<CardTitle>Team</CardTitle>",
        "<CardDescription>Invite members and manage roles.</CardDescription>",
        "<Card className=\"w-[280px]\">",
        "<TabsTrigger value=\"tab-2\">Tab 2</TabsTrigger>",
        "<BreadcrumbPage>Current Page</BreadcrumbPage>",
        "<TableHead>Status</TableHead>",
        "<TableCell>Jane Smith</TableCell>",
        "<Input placeholder=\"you@example.com\" className=\"self-stretch h-9\" />",
        "<SelectItem value=\"editor\">Editor</SelectItem>",
        "<Checkbox id=\"terms\" defaultChecked />",
        "<Switch defaultChecked />",
        "<Progress value={50} className=\"w-60\" />",
    ] {
        assert!(tsx.contains(usage), "missing {usage:?} in\n{tsx}");
    }
    // A frame named like a kit component but holding an image keeps
    // its content (structural confirmation failed → generic markup).
    assert!(tsx.contains("<img src=\"./assets/photo.png\""), "{tsx}");
}

#[test]
fn dark_default_theme_and_extra_axes_emit_their_own_blocks() {
    let files = ReactTailwind.generate_files(&load("dark_theme"));
    let css = &files.globals_css;
    // Default theme is Dark (first axis value) → :root holds dark values.
    assert!(css.contains(":root {\n  --accent:"), "{css}");
    assert!(css.contains("  --background: #0B0B0F;"), "{css}");
    // With dark as the default there is no `.dark` block; Light and the
    // Density axis get data-attribute blocks with differing values only
    // (per-axis entries resolve independently in a two-axis document).
    assert!(!css.contains(".dark {"), "{css}");
    assert!(
        css.contains("[data-density=\"compact\"] {\n  --space-section: 20px;\n}"),
        "{css}"
    );
    let light = &css[css.find("[data-mode=\"light\"] {").expect("light block")..];
    let light = &light[..light.find('}').unwrap()];
    assert!(light.contains("  --background: #FFFFFF;"), "{css}");
    // Undefined tokens follow the palette's light/dark pair too.
    assert!(light.contains("  --muted-foreground:"), "{css}");
    assert!(!light.contains("--space-section"), "{css}");
    let tsx = &files.component;
    for class in [
        "gap-[var(--space-section)]",
        "font-[family-name:var(--font-heading)]",
        "text-7xl font-extrabold leading-none text-center text-foreground",
        "text-muted-foreground",
        "rounded-tl-[4px] rounded-tr-[4px]",
        "opacity-60",
        "blur-[12px]",
        "bg-accent",
        "overflow-hidden",
        "Themes follow the document.<br />Dark first.",
    ] {
        assert!(tsx.contains(class), "missing {class:?} in\n{tsx}");
    }
    // Palette fallbacks resolve referenced-but-undefined tokens.
    assert!(css.contains("--muted-foreground:"), "{css}");
}

#[test]
fn kit_table_matches_the_embedded_shadcn_kit() {
    let kit = op_editor_core::uikit_shadcn::shadcn_kit();
    for (id, name) in crate::codegen_tailwind::kit_component_names() {
        let component = kit
            .components
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("kit component {id} missing"));
        assert_eq!(component.name, name, "{id}");
    }
}

#[test]
fn referenced_components_collects_masters_transitively() {
    let doc = load("shadcn_refs");
    let pages = doc.pages.as_ref().unwrap();
    let roots: Vec<&PenNode> = pages[0].children.iter().collect();
    let components = referenced_components(&roots, &doc);
    let ids: Vec<&str> = components.iter().map(|c| c.base().id.as_str()).collect();
    assert_eq!(ids, vec!["comp-card", "plan-tile"]);
    // The selection alone (no masters) still resolves them when the
    // components are passed alongside.
    let mut selection_doc = doc.clone();
    selection_doc.pages = None;
    selection_doc.children = pages[0].children.clone();
    let with = ReactTailwind.generate_files_with_components(&selection_doc, &components);
    assert!(
        with.component.contains("<CardTitle>Team</CardTitle>"),
        "{}",
        with.component
    );
    assert!(with.component.contains("<Button>Choose</Button>"));
}
