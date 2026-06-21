//! TS-compatible `get_design_prompt` MCP tool.

use std::collections::BTreeMap;

use op_editor_core::{DesignMdSpec, EditorState};

use super::{McpTool, ToolErrorCode, ToolOutcome};

const PROMPT_SECTIONS: &[&str] = &[
    "all",
    "schema",
    "layout",
    "roles",
    "text",
    "style",
    "icons",
    "examples",
    "guidelines",
    "quality",
    "mobile",
    "planning",
    "elements",
    "elements-cookbook",
    "design-md",
    "copywriting",
    "overflow",
    "cjk",
    "variables",
    "codegen-planning",
    "codegen-chunk",
    "codegen-assembly",
    "codegen-react",
    "codegen-vue",
    "codegen-svelte",
    "codegen-html",
    "codegen-flutter",
    "codegen-swiftui",
    "codegen-compose",
    "codegen-react-native",
];

const INTRO: &str = r#"You are working with OpenPencil, a vector design tool.

TOOL SELECTION - match the user's intent:
- READ/INSPECT the canvas: read_nodes, snapshot_layout, get_selection, get_node
- CREATE new designs: batch_design, design_skeleton, design_content, design_refine
- MODIFY existing nodes: update_node, replace_node, set_node_* tools
- DELETE/REMOVE elements: delete_node after inspecting the target id
- MOVE/COPY: move_node, copy_node, copy_selected

When the user asks to read or inspect existing content, use read tools first.
Each node must follow the PenNode schema and stay under the root frame."#;

const PLANNING_GUIDE: &str = r#"DESIGN PLANNING:
- Classify by purpose: marketing/informational pages use desktop 1200px wide scrollable roots; single-task screens use mobile 375x812; dashboards/admin workspaces use desktop layouts.
- Create a skeleton first, fill each section with content, then refine.
- Keep forms together with their primary action. Split only when one section would be too large.
- Default to light neutral styling unless the request explicitly asks for dark, cyber, neon, terminal, noir, night, or similar themes."#;

const AESTHETIC_QUALITY_GUIDE: &str = r#"AESTHETIC QUALITY BAR:
- Avoid crowded output. Prefer fewer, stronger modules with visible negative space over filling every pixel.
- Mobile screens should use one App Content wrapper for the main body. The wrapper owns horizontal padding (16-20px) and vertical gap (20-24px); inner sections should not each add competing gutters.
- Mobile top rhythm: keep the header/title group and first primary module close. On 375-430px screens, the gap from a greeting/title/header cluster to search, primary action, chart, or first card should usually be 20-32px. Do not leave a blank hero-sized band above the first useful control unless the request explicitly asks for an editorial hero.
- Keep one primary job per screen. Above the fold, show the title/context, the primary action or search, and at most 2-3 supporting modules.
- Use a clear type rhythm: one display/title size, one section heading size, one body/caption size. Avoid many near-identical bold text sizes.
- Reuse one card radius, one card padding, and one shadow treatment within a screen.
- Use at most two saturated colors. Let hierarchy come from spacing, contrast, and content scale, not decoration.
- Product card favorite/heart controls must stay fully inside its card or image with an 8-12px inset. Treat favorite/heart as a functional icon-button, not a floating badge; never use negative x/y, straddle the card edge, or let it overlap the section heading row.
- Mobile tab bars are part of the page flow; do not add fake bottom spacers or overlays.
- Pick a distinct visual concept for each new design and build around one signature moment. Do not repeat the same predictable mobile stack of search + categories + orange promo + two cards unless the prompt specifically calls for that pattern.
- Use the user's domain to vary composition, imagery, palette, rhythm, and interaction affordances; polish should feel intentional rather than template-derived."#;

const RUST_ELEMENT_TOOL_GUIDE: &str = r##"RUST MCP ELEMENT TOOL COMPATIBILITY:
- This Rust MCP server does not expose the TS `add_*_v1` element-tool family unless those exact tools appear in tools/list. Do not call `add_*` tools just because older prompt text or examples mention them.
- For custom UI trees, use `batch_design` with the TS operations DSL in the `operations` argument. Supported writes: `binding=I(parent, nodeJson)` for inserts, plus one-at-a-time `U(nodeId, patchJson)`, `D(nodeId)`, `M(nodeId, parent, index?)`, `binding=C(sourceId, parent, overrides?)`, `binding=R(nodeId, nodeJson)`, and `binding=G(parent, "search"|"generate", prompt)` refine operations.
- Parent can be `null` for the active page root, a previous binding name, or one real existing parent id. The node JSON is canonical PenNode JSON; omit `id` if you do not care, because Rust remaps inserted ids.
- `U` currently patches geometry/name/fill fields; use dedicated `set_node_*` tools for text, rotation, stroke, font, effects, and other specialized fields.

Example:
root=I(null, {"type":"frame","name":"Page","width":1200,"height":800,"layout":"vertical","gap":24,"fill":"#ffffff"})
hero=I(root, {"type":"frame","name":"Hero","width":"fill_container","height":360,"layout":"vertical","gap":16})
title=I(hero, {"type":"text","name":"Headline","content":"Welcome Back","width":"fill_container","height":64,"fontSize":48,"fontWeight":700})

Light/dark handling:
- Inspect `get_variables` / `get_active_theme` first when the document already defines theme axes.
- Use variable refs such as `$color-bg`, `$color-text`, and `$color-surface` when the document provides them.
- If the user explicitly asks for a one-off dark or light design and no variables exist, use concrete high-contrast fills and text colors directly."##;

const DEFAULT_DESIGN_MD: &str = "No design.md loaded in the current document.";

pub struct GetDesignPrompt {
    design_md_policy: Option<String>,
}

impl McpTool for GetDesignPrompt {
    fn name(&self) -> &str {
        "get_design_prompt"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let section = args.get("section").map(String::as_str).unwrap_or("all");
        let available = match serde_json::to_string(PROMPT_SECTIONS) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize prompt sections failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("section".into(), section.into());
        out.insert("availableSections".into(), available);
        out.insert(
            "designPrompt".into(),
            build_design_prompt(Some(section), self.design_md_policy.as_deref()),
        );
        ToolOutcome::Ok(out)
    }
}

pub fn get_design_prompt_snapshot(state: &EditorState) -> GetDesignPrompt {
    GetDesignPrompt {
        design_md_policy: state
            .doc
            .design_md
            .as_ref()
            .map(build_design_md_style_policy)
            .filter(|policy| !policy.is_empty()),
    }
}

fn build_design_prompt(section: Option<&str>, design_md_policy: Option<&str>) -> String {
    if let Some(section) = section {
        if let Some(policy) = design_md_policy {
            if section == "style" {
                return format!("DESIGN SYSTEM (from design.md):\n{policy}");
            }
            if section == "design-md" {
                return policy.to_string();
            }
        }
        if let Some(content) = section_content(section) {
            return content;
        }
    }
    build_full_prompt()
}

fn section_content(section: &str) -> Option<String> {
    match section {
        "all" => Some(build_full_prompt()),
        "schema" => Some(skill_content("schema")),
        "layout" => Some(skill_content("layout")),
        "roles" => Some(skill_content("role-definitions")),
        "text" => Some(skill_content("text-rules")),
        "style" => Some(skill_content("style-defaults")),
        "icons" => Some(skill_content("icon-catalog")),
        "examples" => Some(skill_content("examples")),
        "guidelines" => Some(format!(
            "{}\n\n{}",
            skill_content("design-principles"),
            skill_content("product-principles")
        )),
        "quality" => Some(AESTHETIC_QUALITY_GUIDE.into()),
        "mobile" => Some(skill_content("mobile-app")),
        "planning" => Some(PLANNING_GUIDE.into()),
        "elements" => Some(RUST_ELEMENT_TOOL_GUIDE.into()),
        "elements-cookbook" => Some(RUST_ELEMENT_TOOL_GUIDE.into()),
        "design-md" => Some(DEFAULT_DESIGN_MD.into()),
        "copywriting" => Some(skill_content("copywriting")),
        "overflow" => Some(skill_content("overflow")),
        "cjk" => Some(skill_content("cjk-typography")),
        "variables" => Some(skill_content("variables")),
        "codegen-planning" => Some(skill_content("codegen-planning")),
        "codegen-chunk" => Some(skill_content("codegen-chunk")),
        "codegen-assembly" => Some(skill_content("codegen-assembly")),
        "codegen-react" => Some(skill_content("codegen-react")),
        "codegen-vue" => Some(skill_content("codegen-vue")),
        "codegen-svelte" => Some(skill_content("codegen-svelte")),
        "codegen-html" => Some(skill_content("codegen-html")),
        "codegen-flutter" => Some(skill_content("codegen-flutter")),
        "codegen-swiftui" => Some(skill_content("codegen-swiftui")),
        "codegen-compose" => Some(skill_content("codegen-compose")),
        "codegen-react-native" => Some(skill_content("codegen-react-native")),
        _ => None,
    }
}

fn build_full_prompt() -> String {
    [
        INTRO,
        &skill_content("schema"),
        &skill_content("style-defaults"),
        &skill_content("examples"),
        PLANNING_GUIDE,
        AESTHETIC_QUALITY_GUIDE,
        &skill_content("mobile-app"),
        &skill_content("role-definitions"),
        &skill_content("layout"),
        &skill_content("text-rules"),
        &skill_content("design-principles"),
        &skill_content("variables"),
        RUST_ELEMENT_TOOL_GUIDE,
    ]
    .join("\n\n")
}

fn skill_content(name: &str) -> String {
    op_ai_skills::get_skill_by_name(name)
        .map(|skill| skill.content.clone())
        .unwrap_or_default()
}

fn build_design_md_style_policy(spec: &DesignMdSpec) -> String {
    let mut parts = Vec::new();

    if let Some(theme) = spec.visual_theme.as_ref().filter(|s| !s.is_empty()) {
        parts.push(format!("VISUAL THEME: {}", truncate_chars(theme, 200)));
    }

    if let Some(palette) = spec
        .color_palette
        .as_ref()
        .filter(|colors| !colors.is_empty())
    {
        let colors = palette
            .iter()
            .take(10)
            .map(|c| format!("{} ({}) — {}", c.name, c.hex, c.role))
            .collect::<Vec<_>>()
            .join("\n- ");
        parts.push(format!("COLOR PALETTE:\n- {colors}"));

        let surfaces = palette
            .iter()
            .filter(|c| {
                let role = c.role.to_lowercase();
                ["surface", "card", "panel", "sidebar", "tile", "chip"]
                    .iter()
                    .any(|needle| role.contains(needle))
            })
            .take(6)
            .map(|c| format!("{} ({}) — {}", c.name, c.hex, c.role))
            .collect::<Vec<_>>();
        if !surfaces.is_empty() {
            parts.push(format!(
                "SURFACE COLORS (use ONLY as `fill` on visually distinct components placed on top \
                 of the page background — cards, sidebars, floating panels, chips, badges. DO NOT \
                 fill section root frames or generic wrapper frames with these; section containers \
                 must stay transparent and inherit the page background. NEVER use these as the \
                 page/rootFrame fill):\n- {}",
                surfaces.join("\n- ")
            ));
        }
    }

    if let Some(typography) = &spec.typography {
        if let Some(font) = typography.font_family.as_ref().filter(|s| !s.is_empty()) {
            parts.push(format!("FONT: {font}"));
        }
        if let Some(headings) = typography.headings.as_ref().filter(|s| !s.is_empty()) {
            parts.push(format!("Headings: {headings}"));
        }
        if let Some(body) = typography.body.as_ref().filter(|s| !s.is_empty()) {
            parts.push(format!("Body: {body}"));
        }
    }

    if let Some(styles) = spec.component_styles.as_ref().filter(|s| !s.is_empty()) {
        parts.push(format!(
            "COMPONENT STYLES:\n{}",
            truncate_chars(styles, 300)
        ));
    }
    if let Some(layout) = spec.layout_principles.as_ref().filter(|s| !s.is_empty()) {
        parts.push(format!(
            "LAYOUT PRINCIPLES:\n{}",
            truncate_chars(layout, 400)
        ));
    }
    if let Some(notes) = spec.generation_notes.as_ref().filter(|s| !s.is_empty()) {
        parts.push(format!("GENERATION NOTES:\n{}", truncate_chars(notes, 400)));
    }

    parts.join("\n\n")
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}...", s.chars().take(max).collect::<String>())
    } else {
        s.to_string()
    }
}
