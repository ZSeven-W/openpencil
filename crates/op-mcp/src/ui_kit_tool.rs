//! Read-only, bounded UIKit catalogue for script-first design generation.

use std::collections::BTreeMap;

use op_editor_core::uikit::{ComponentCategory, UIKit};
use op_editor_core::EditorState;
use serde::Serialize;

use super::{McpTool, ToolErrorCode, ToolOutcome};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentResult {
    id: String,
    name: String,
    category: &'static str,
    tags: Vec<String>,
    width: f32,
    height: f32,
    script_ref: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KitResult {
    id: String,
    name: String,
    version: String,
    built_in: bool,
    components: Vec<ComponentResult>,
}

pub struct ListUiKits {
    kits: Vec<UIKit>,
}

impl McpTool for ListUiKits {
    fn name(&self) -> &str {
        "list_ui_kits"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let limit = match parse_limit(args) {
            Ok(limit) => limit,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        let category = match parse_category(args) {
            Ok(category) => category,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        let kit_id = trimmed_arg(args, "kitId");
        let query = trimmed_arg(args, "query").map(str::to_ascii_lowercase);
        let mut returned = 0usize;
        let mut total_matches = 0usize;
        let mut kits = Vec::new();

        for kit in &self.kits {
            if kit_id.is_some_and(|wanted| kit.id != wanted) {
                continue;
            }
            let kit_query_match = query.as_deref().is_some_and(|needle| {
                contains_folded(&kit.id, needle) || contains_folded(&kit.name, needle)
            });
            let mut components = Vec::new();
            for component in &kit.components {
                if category.is_some_and(|wanted| component.category != wanted) {
                    continue;
                }
                if let Some(needle) = query.as_deref() {
                    let component_match = contains_folded(&component.id, needle)
                        || contains_folded(&component.name, needle)
                        || contains_folded(component.category.as_ts_str(), needle)
                        || component
                            .tags
                            .iter()
                            .any(|tag| contains_folded(tag, needle));
                    if !kit_query_match && !component_match {
                        continue;
                    }
                }
                total_matches += 1;
                if returned >= limit {
                    continue;
                }
                returned += 1;
                components.push(ComponentResult {
                    id: component.id.clone(),
                    name: component.name.clone(),
                    category: component.category.as_ts_str(),
                    tags: component.tags.clone(),
                    width: component.width,
                    height: component.height,
                    script_ref: script_ref(kit, &component.id),
                });
            }
            if !components.is_empty() || kit_id.is_some_and(|wanted| wanted == kit.id) {
                kits.push(KitResult {
                    id: kit.id.clone(),
                    name: kit.name.clone(),
                    version: kit.version.clone(),
                    built_in: kit.built_in,
                    components,
                });
            }
        }

        let kit_count = kits.len();
        let result = serde_json::json!({
            "kits": kits,
            "kitCount": kit_count,
            "componentCount": returned,
            "totalMatches": total_matches,
            "limit": limit,
            "truncated": total_matches > returned,
        });
        ToolOutcome::OkJson(result.to_string())
    }
}

pub fn list_ui_kits_snapshot(state: &EditorState) -> ListUiKits {
    ListUiKits {
        kits: state.ui_kits.clone(),
    }
}

fn trimmed_arg<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    args.get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_limit(args: &BTreeMap<String, String>) -> Result<usize, String> {
    let Some(raw) = trimmed_arg(args, "limit") else {
        return Ok(DEFAULT_LIMIT);
    };
    let limit = raw
        .parse::<usize>()
        .map_err(|_| format!("limit must be an integer from 1 to {MAX_LIMIT}"))?;
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(format!("limit must be an integer from 1 to {MAX_LIMIT}"));
    }
    Ok(limit)
}

fn parse_category(args: &BTreeMap<String, String>) -> Result<Option<ComponentCategory>, String> {
    let Some(raw) = trimmed_arg(args, "category") else {
        return Ok(None);
    };
    match raw {
        "buttons" => Ok(Some(ComponentCategory::Buttons)),
        "inputs" => Ok(Some(ComponentCategory::Inputs)),
        "cards" => Ok(Some(ComponentCategory::Cards)),
        "navigation" => Ok(Some(ComponentCategory::Navigation)),
        "layout" => Ok(Some(ComponentCategory::Layout)),
        "feedback" => Ok(Some(ComponentCategory::Feedback)),
        "data-display" => Ok(Some(ComponentCategory::DataDisplay)),
        "other" => Ok(Some(ComponentCategory::Other)),
        _ => Err(format!(
            "category must be buttons, inputs, cards, navigation, layout, feedback, data-display, or other, got {raw:?}"
        )),
    }
}

fn contains_folded(value: &str, needle: &str) -> bool {
    value.to_ascii_lowercase().contains(needle)
}

fn script_ref(kit: &UIKit, component_id: &str) -> String {
    match kit.id.as_str() {
        "openpencil-starter" => format!("starter/{component_id}"),
        "shadcn-ui" => format!(
            "shadcn/{}",
            component_id.strip_prefix("shadcn-").unwrap_or(component_id)
        ),
        _ => format!("{}/{component_id}", kit.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::uikit::{KitComponent, UIKit};

    fn call(args: BTreeMap<String, String>) -> serde_json::Value {
        let ToolOutcome::OkJson(json) = list_ui_kits_snapshot(&EditorState::new()).call(&args)
        else {
            panic!("expected nested JSON kit result");
        };
        serde_json::from_str(&json).expect("kit JSON")
    }

    #[test]
    fn exposes_canonical_script_refs_and_metadata() {
        let starter = call(BTreeMap::from([(
            "kitId".to_string(),
            "openpencil-starter".to_string(),
        )]));
        assert_eq!(starter["kits"][0]["builtIn"], true);
        assert_eq!(starter["kits"][0]["version"], "1.0.0");
        assert_eq!(
            starter["kits"][0]["components"][0]["scriptRef"],
            "starter/btn-primary"
        );
        assert!(starter["kits"][0]["components"][0]["width"].is_number());

        let shadcn = call(BTreeMap::from([
            ("kitId".to_string(), "shadcn-ui".to_string()),
            ("query".to_string(), "btn-primary".to_string()),
        ]));
        assert_eq!(
            shadcn["kits"][0]["components"][0]["scriptRef"],
            "shadcn/btn-primary"
        );
    }

    #[test]
    fn filters_and_applies_a_global_hard_limit() {
        let value = call(BTreeMap::from([
            ("category".to_string(), "cards".to_string()),
            ("limit".to_string(), "1".to_string()),
        ]));
        assert_eq!(value["componentCount"], 1);
        assert_eq!(value["limit"], 1);
        assert_eq!(value["truncated"], true);
        assert_eq!(
            value["kits"][0]["components"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn imported_kits_use_full_kit_component_reference() {
        let mut state = EditorState::new();
        let template = state.ui_kits[0].components[0].template.clone();
        state.ui_kits = vec![UIKit {
            id: "team-kit".into(),
            name: "Team Kit".into(),
            version: "2".into(),
            built_in: false,
            components: vec![KitComponent {
                id: "hero-card".into(),
                name: "Hero Card".into(),
                category: ComponentCategory::Cards,
                tags: vec!["hero".into()],
                width: 320.0,
                height: 180.0,
                template,
            }],
            variables: None,
        }];
        let ToolOutcome::OkJson(json) = list_ui_kits_snapshot(&state).call(&BTreeMap::new()) else {
            panic!("expected nested JSON kit result");
        };
        let value: serde_json::Value = serde_json::from_str(&json).expect("kit JSON");
        assert_eq!(value["kits"][0]["builtIn"], false);
        assert_eq!(
            value["kits"][0]["components"][0]["scriptRef"],
            "team-kit/hero-card"
        );
    }

    #[test]
    fn rejects_unbounded_limit_and_unknown_category() {
        let tool = list_ui_kits_snapshot(&EditorState::new());
        assert!(matches!(
            tool.call(&BTreeMap::from([("limit".into(), "101".into())])),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
        assert!(matches!(
            tool.call(&BTreeMap::from([("category".into(), "unknown".into())])),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
    }
}
