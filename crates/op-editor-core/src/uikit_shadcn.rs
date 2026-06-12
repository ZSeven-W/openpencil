//! Embedded shadcn UIKit — the Rust port of the TS built-in kit
//! (GAP #23).
//!
//! The TS kit (`apps/web/src/uikit/kits/shadcn-kit{,-extra,-meta}.ts`)
//! is 31 shadcn-styled `PenNode` JSON literals. Rather than hand-port
//! ~1,600 lines of builders, the literals are exported to JSON by
//! `tools/export-shadcn-kit.ts` (bun), embedded here via
//! `include_str!`, and parsed through the canonical schema on first
//! access. Components whose JSON the schema rejects are skipped
//! best-effort — `shadcn_kit_skipped()` reports them so tests can
//! assert the survival floor.
//!
//! Re-run the export after editing the TS kit:
//! `bun tools/export-shadcn-kit.ts`.

use std::sync::LazyLock;

use jian_ops_schema::node::PenNode;
use serde_json::Value;

use crate::pen_node_ext::PenNodeExt;
use crate::uikit::{ComponentCategory, KitComponent, UIKit};

static SHADCN_KIT_JSON: &str = include_str!("../assets/shadcn-kit.json");

/// Kit id — matches the TS built-in registry (`shadcn-ui`).
pub const SHADCN_KIT_ID: &str = "shadcn-ui";

struct ParsedKit {
    kit: UIKit,
    skipped: Vec<String>,
}

static SHADCN_KIT: LazyLock<ParsedKit> = LazyLock::new(build_shadcn_kit);

/// The embedded shadcn kit (cloned — `UIKit` is a value type the
/// editor state owns per instance).
pub fn shadcn_kit() -> UIKit {
    SHADCN_KIT.kit.clone()
}

/// Component ids from the embedded JSON the canonical schema refused
/// to parse (best-effort load contract).
pub fn shadcn_kit_skipped() -> &'static [String] {
    &SHADCN_KIT.skipped
}

fn build_shadcn_kit() -> ParsedKit {
    let mut components = Vec::new();
    let mut skipped = Vec::new();
    let root: Value = match serde_json::from_str(SHADCN_KIT_JSON) {
        Ok(v) => v,
        Err(_) => {
            return ParsedKit {
                kit: empty_kit(components),
                skipped: vec!["<embedded kit JSON failed to parse>".to_string()],
            }
        }
    };
    let meta = root.get("meta").and_then(Value::as_object);
    let children = root
        .get("document")
        .and_then(|d| d.get("children"))
        .and_then(Value::as_array);
    let Some(children) = children else {
        return ParsedKit {
            kit: empty_kit(components),
            skipped: vec!["<embedded kit document has no children>".to_string()],
        };
    };
    for raw in children {
        let id = raw
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<missing id>")
            .to_string();
        let template: PenNode = match serde_json::from_value(raw.clone()) {
            Ok(node) => node,
            Err(_) => {
                skipped.push(id);
                continue;
            }
        };
        let name = template.base().name.clone().unwrap_or_else(|| id.clone());
        // Width / height hints mirror TS `extractComponentsFromDocument`
        // (literal number or the 100-px fallback).
        let width = raw.get("width").and_then(Value::as_f64).unwrap_or(100.0) as f32;
        let height = raw.get("height").and_then(Value::as_f64).unwrap_or(100.0) as f32;
        let (category, tags) = meta
            .and_then(|m| m.get(&id))
            .map(|entry| {
                let category = entry
                    .get("category")
                    .and_then(Value::as_str)
                    .map(category_from)
                    .unwrap_or(ComponentCategory::Other);
                let tags = entry
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|t| {
                        t.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                (category, tags)
            })
            .unwrap_or((ComponentCategory::Other, Vec::new()));
        components.push(KitComponent {
            id,
            name,
            category,
            tags,
            width,
            height,
            template,
        });
    }
    ParsedKit {
        kit: UIKit {
            id: SHADCN_KIT_ID.to_string(),
            name: "shadcn UI".to_string(),
            version: "1.0.0".to_string(),
            built_in: true,
            components,
            variables: None,
        },
        skipped,
    }
}

fn empty_kit(components: Vec<KitComponent>) -> UIKit {
    UIKit {
        id: SHADCN_KIT_ID.to_string(),
        name: "shadcn UI".to_string(),
        version: "1.0.0".to_string(),
        built_in: true,
        components,
        variables: None,
    }
}

/// Map the TS `ComponentCategory` string literals onto the Rust enum.
fn category_from(s: &str) -> ComponentCategory {
    match s {
        "buttons" => ComponentCategory::Buttons,
        "inputs" => ComponentCategory::Inputs,
        "cards" => ComponentCategory::Cards,
        "navigation" => ComponentCategory::Navigation,
        "layout" => ComponentCategory::Layout,
        "feedback" => ComponentCategory::Feedback,
        "data-display" | "data_display" => ComponentCategory::DataDisplay,
        _ => ComponentCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorState;

    #[test]
    fn shadcn_kit_parses_at_least_25_components() {
        let kit = shadcn_kit();
        assert_eq!(kit.id, SHADCN_KIT_ID);
        assert!(kit.built_in);
        // Surfaced under `--nocapture` so the survival count is
        // inspectable without a failure.
        eprintln!(
            "shadcn kit: {} components parsed, skipped: {:?}",
            kit.components.len(),
            shadcn_kit_skipped()
        );
        assert!(
            kit.components.len() >= 25,
            "expected >= 25 shadcn components to survive the schema parse, got {} (skipped: {:?})",
            kit.components.len(),
            shadcn_kit_skipped(),
        );
    }

    #[test]
    fn shadcn_components_carry_meta_categories_and_tags() {
        let kit = shadcn_kit();
        let btn = kit
            .components
            .iter()
            .find(|c| c.id == "shadcn-btn-primary")
            .expect("primary button survives");
        assert_eq!(btn.category, ComponentCategory::Buttons);
        assert!(btn.tags.iter().any(|t| t == "button"));
        assert_eq!(btn.name, "Primary Button");
        assert!(btn.width > 0.0 && btn.height > 0.0);
    }

    #[test]
    fn shadcn_component_instantiates_onto_the_page() {
        let mut state = EditorState::new();
        let before = state.active_children().len();
        let id = state
            .instantiate_kit_component(SHADCN_KIT_ID, "shadcn-btn-primary", 50.0, 60.0)
            .expect("shadcn kit is registered as a built-in");
        let after = state.active_children();
        assert_eq!(after.len(), before + 1);
        let inserted = after.last().unwrap();
        assert_eq!(inserted.base().id, id.as_str());
        assert_eq!(inserted.base().x, Some(50.0));
        assert_eq!(inserted.base().y, Some(60.0));
        // The shadcn templates are container frames with children —
        // the clone must carry the subtree.
        assert!(inserted.children().map(|c| !c.is_empty()).unwrap_or(false));
    }
}
