//! Multi-screen consistency brief for the design-agent loop.
//!
//! When a design turn starts on a NON-empty canvas (a profile page after a
//! home page), the new screen must read as the SAME product. Pencil leaves
//! that to the model's diligence (it must remember to `batch_get` the old
//! screen); this module front-loads the established facts deterministically
//! so consistency does not depend on the model choosing to look.

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

/// Deterministic multi-screen consistency brief for a design-loop turn.
///
/// When the document already holds screens, the new screen must read as the
/// SAME product: same palette/typography, same shared chrome (navbar / tab
/// bar / sidebar) with only its active state changed. Pencil leaves this to
/// the model's diligence (it must remember to `batch_get` the old screen);
/// this brief front-loads the established facts — screen inventory, typefaces,
/// accent colors, and the exact node ids of copyable chrome — so consistency
/// does not depend on the model choosing to look. Returns `None` on an empty
/// canvas (a first screen has nothing to be consistent with).
pub fn design_context_brief(state: &EditorState) -> Option<String> {
    let screens: Vec<&PenNode> = state
        .active_children()
        .iter()
        .filter(|n| matches!(n, PenNode::Frame(_)) && n.children().is_some_and(|c| !c.is_empty()))
        .collect();
    if screens.is_empty() {
        return None;
    }

    let mut fonts: Vec<String> = Vec::new();
    let mut accents: Vec<String> = Vec::new();
    let mut chrome: Vec<String> = Vec::new();
    let mut screen_lines: Vec<String> = Vec::new();
    for screen in screens.iter().take(4) {
        let name = screen.base().name.as_deref().unwrap_or("(unnamed)");
        let size = match (screen.width_px(), screen.height_px()) {
            (Some(w), Some(h)) => format!("{}x{}", w as i64, h as i64),
            _ => "auto".to_string(),
        };
        let bg = op_editor_core::first_solid_fill_hex(screen).unwrap_or("none");
        screen_lines.push(format!("\"{name}\" ({size}, bg {bg})"));
        if let Ok(value) = serde_json::to_value(screen) {
            collect_design_facts(&value, name, &mut fonts, &mut accents, &mut chrome);
        }
    }

    let mut brief = format!(
        "EXISTING CANVAS CONTEXT — this document already contains {} screen(s); the new screen \
         must read as the SAME product.\n- Screens: {}.",
        screens.len(),
        screen_lines.join("; ")
    );
    if !fonts.is_empty() {
        brief.push_str(&format!(
            "\n- Typefaces in use: {}. Reuse them - do not introduce new families.",
            fonts.join(", ")
        ));
    }
    if !accents.is_empty() {
        brief.push_str(&format!(
            "\n- Accent colors in use: {}. Reuse the established palette.",
            accents.join(", ")
        ));
    }
    if !chrome.is_empty() {
        brief.push_str(&format!(
            "\n- Shared chrome to COPY (never rebuild): {}. Copy with C(id, newScreenRoot) and \
             update only the active-state styling (active tab/nav item, page title).",
            chrome.join("; ")
        ));
    }
    brief.push_str(
        "\n- Read the newest screen with batch_get before designing; place the new screen to \
         the RIGHT with find_empty_space.",
    );
    Some(brief)
}

/// Walk one screen's JSON collecting typefaces, saturated accent fills, and
/// copyable shared-chrome nodes (bounded, dedup-in-order, first-N caps).
fn collect_design_facts(
    value: &serde_json::Value,
    screen_name: &str,
    fonts: &mut Vec<String>,
    accents: &mut Vec<String>,
    chrome: &mut Vec<String>,
) {
    const CHROME_ROLES: [&str; 4] = ["bottom-tab-bar", "navbar", "sidebar", "header"];
    if let Some(obj) = value.as_object() {
        if let Some(family) = obj.get("fontFamily").and_then(serde_json::Value::as_str) {
            if !family.is_empty() && !fonts.iter().any(|f| f == family) && fonts.len() < 3 {
                fonts.push(family.to_string());
            }
        }
        if let Some(fills) = obj.get("fill").and_then(serde_json::Value::as_array) {
            for fill in fills {
                let hex = fill
                    .get("color")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if is_saturated_accent(hex)
                    && !accents.iter().any(|a| a == hex)
                    && accents.len() < 3
                {
                    accents.push(hex.to_string());
                }
            }
        }
        let role = obj.get("role").and_then(serde_json::Value::as_str);
        let name = obj.get("name").and_then(serde_json::Value::as_str);
        let is_chrome = role.is_some_and(|r| CHROME_ROLES.contains(&r))
            || name.is_some_and(|n| {
                let n = n.to_ascii_lowercase();
                n.contains("tab bar") || n.contains("navbar") || n.contains("sidebar")
            });
        if is_chrome && chrome.len() < 3 {
            if let Some(id) = obj.get("id").and_then(serde_json::Value::as_str) {
                chrome.push(format!(
                    "\"{}\" (id {id}, in \"{screen_name}\")",
                    name.unwrap_or("chrome")
                ));
            }
        }
        if let Some(children) = obj.get("children").and_then(serde_json::Value::as_array) {
            for child in children {
                collect_design_facts(child, screen_name, fonts, accents, chrome);
            }
        }
    }
}

/// A fill hex that reads as a brand/accent color: parseable RGB that is
/// neither near-grayscale nor near-white/black.
fn is_saturated_accent(hex: &str) -> bool {
    let h = hex.trim_start_matches('#');
    let Some(rgb) = h.get(0..6) else {
        return false;
    };
    let (Ok(r), Ok(g), Ok(b)) = (
        u8::from_str_radix(&rgb[0..2], 16),
        u8::from_str_radix(&rgb[2..4], 16),
        u8::from_str_radix(&rgb[4..6], 16),
    ) else {
        return false;
    };
    let (min, max) = (r.min(g).min(b), r.max(g).max(b));
    // Chroma floor rejects grayscale; luminance band rejects near-white/black.
    let luminance = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
    max - min > 40 && (24.0..232.0).contains(&luminance)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_home_screen() -> EditorState {
        let home: PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame", "id": "home", "name": "Home",
            "width": 390, "height": 844, "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#0D0D0Dff" }],
            "children": [
                { "type": "text", "id": "t1", "name": "Greeting", "content": "Good morning",
                  "fontFamily": "Inter", "fontSize": 24, "width": 200, "height": 30 },
                { "type": "frame", "id": "cta", "name": "CTA", "width": 120, "height": 44,
                  "fill": [{ "type": "solid", "color": "#34D399ff" }] },
                { "type": "frame", "id": "nav1", "name": "Bottom Tab Bar", "role": "bottom-tab-bar",
                  "width": "fill_container", "height": 64 }
            ]
        }))
        .expect("home screen node");
        let mut state = EditorState::new();
        state
            .insert_subtree_returning_root_ids(vec![home], &op_editor_core::NodeId::NONE)
            .expect("insert home");
        state
    }

    #[test]
    fn empty_canvas_yields_no_brief() {
        assert!(design_context_brief(&EditorState::new()).is_none());
    }

    #[test]
    fn brief_reports_screens_fonts_accents_and_copyable_chrome() {
        let state = state_with_home_screen();
        let brief = design_context_brief(&state).expect("brief on populated canvas");
        assert!(brief.contains("\"Home\" (390x844"), "{brief}");
        assert!(brief.contains("Inter"), "typeface reported: {brief}");
        assert!(brief.contains("#34D399"), "accent reported: {brief}");
        assert!(
            brief.contains("Bottom Tab Bar") && brief.contains("C(id, newScreenRoot)"),
            "chrome copy instruction with node id: {brief}"
        );
        assert!(
            brief.contains("batch_get"),
            "read-first instruction: {brief}"
        );
    }

    #[test]
    fn grayscale_fills_are_not_accents() {
        assert!(
            !is_saturated_accent("#0D0D0Dff"),
            "near-black is not an accent"
        );
        assert!(
            !is_saturated_accent("#F5F5F5ff"),
            "near-white is not an accent"
        );
        assert!(!is_saturated_accent("#808080ff"), "gray is not an accent");
        assert!(is_saturated_accent("#34D399ff"), "brand green IS an accent");
    }
}
