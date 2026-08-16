//! MCP tools `list_scene_templates` and `use_scene_template` — the scene
//! template catalogue, shipped half and user-saved half.
//!
//! The shipped catalogue has been in the editor since `v0.8.3` behind
//! File ▸ New from template, so an agent could only ever start from a blank
//! frame. Listing it is what lets one start from a real layout, and the deck
//! templates are the entry point to the whole presentation workflow. The
//! user's own saved templates are the material most worth starting from, so
//! they are part of the same list.
//!
//! `use_scene_template` returns a command rather than mutating: the boards and
//! the palette they resolve against have to land in one transaction, which is
//! exactly what `EditorCommand::AdoptSceneTemplate` gives. On an untouched
//! starter page the template takes the page over; anywhere else it appends to
//! the right of what is already there.

use std::collections::BTreeMap;

use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_catalogue};
use op_editor_core::user_scene_templates::user_scene_templates;
use op_editor_core::EditorCommand;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

pub struct ListSceneTemplates {
    include_user: bool,
}

impl McpTool for ListSceneTemplates {
    fn name(&self) -> &str {
        "list_scene_templates"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let scene_filter = args
            .get("scene")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let tag_filter = args
            .get("tag")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());

        let mut templates: Vec<serde_json::Value> = Vec::new();

        if self.include_user {
            // Saved templates come first, as they do in the panel: a list where
            // your own material sits below sixty shipped entries is a list you
            // scroll past your own work in. Ids keep the two apart, so a save
            // that shares a shipped template's name cannot quietly take its
            // place. A saved template has no scene or tags to filter by, so the
            // filters below apply to the shipped half only.
            for user in user_scene_templates() {
                templates.push(serde_json::json!({
                    "id": user.id,
                    "name": user.name,
                    "frames": user.frames,
                    "frameWidth": user.frame_width,
                    "frameHeight": user.frame_height,
                    "isUser": true,
                }));
            }
        }

        for template in scene_template_catalogue()
            .iter()
            .filter(|template| {
                scene_filter.is_none_or(|scene| scene.eq_ignore_ascii_case(template.scene.as_str()))
            })
            .filter(|template| {
                tag_filter.is_none_or(|tag| {
                    template
                        .tags
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
                })
            })
        {
            templates.push(serde_json::json!({
                "id": template.id,
                "scene": template.scene.as_str(),
                "title": template.title_fallback,
                "summary": template.summary_fallback,
                "tags": template.tags,
                "frames": template.frames,
                "frameWidth": template.frame_width,
                "frameHeight": template.frame_height,
                // Absent rather than null when a template carries no
                // guide: that is the gate on generating from it, not a
                // missing field.
                "styleGuide": template.style_guide,
            }));
        }

        ToolOutcome::OkJson(serde_json::json!({ "templates": templates }).to_string())
    }
}

pub struct UseSceneTemplate {
    include_user: bool,
}

impl McpTool for UseSceneTemplate {
    fn name(&self) -> &str {
        "use_scene_template"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let template_id = match args.get("templateId").map(|value| value.trim()) {
            Some(id) if !id.is_empty() => id,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "templateId is required — call list_scene_templates for the catalogue".into(),
                );
            }
        };
        // Validate here so an unknown id is a named argument error rather
        // than a command the host silently applies as a no-op — checked
        // against both halves of the catalogue.
        let shipped = scene_template_by_id(template_id);
        let user_template = if self.include_user && shipped.is_none() {
            user_scene_templates()
                .into_iter()
                .find(|template| template.id == template_id)
        } else {
            None
        };
        let (title, frames, frame_width, frame_height) = match (shipped, &user_template) {
            (Some(template), _) => (
                template.title_fallback.to_string(),
                template.frames,
                template.frame_width,
                template.frame_height,
            ),
            (None, Some(user)) => (
                user.name.clone(),
                user.frames,
                user.frame_width,
                user.frame_height,
            ),
            (None, None) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("unknown template id {template_id:?}"),
                );
            }
        };
        ToolOutcome::OkJsonWithCommand(
            serde_json::json!({
                "id": template_id,
                "title": title,
                "frames": frames,
                "frameWidth": frame_width,
                "frameHeight": frame_height,
            })
            .to_string(),
            EditorCommand::AdoptSceneTemplate {
                template_id: template_id.to_string(),
            },
        )
    }
}

pub fn list_scene_templates_snapshot(include_user: bool) -> ListSceneTemplates {
    ListSceneTemplates { include_user }
}

pub fn use_scene_template_snapshot(include_user: bool) -> UseSceneTemplate {
    UseSceneTemplate { include_user }
}

#[cfg(test)]
pub(super) fn exclusive_user_template_registry_for_tests() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    op_editor_core::user_scene_templates::set_user_scene_templates(Vec::new());
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save(name: &str) {
        let id = op_editor_core::user_scene_templates::allocate_template_id(name);
        op_editor_core::user_scene_templates::load_user_scene_template(
            op_editor_core::user_scene_templates::UserSceneTemplate {
                id,
                name: name.to_string(),
                frames: 2,
                frame_width: 1920,
                frame_height: 1080,
                document: "{\"version\":\"1.0.0\",\"children\":[]}".to_string(),
                preview_jpeg: Vec::new(),
            },
        )
        .expect("fixture fits the quota");
    }

    fn call_json(tool: &ListSceneTemplates, args: &BTreeMap<String, String>) -> serde_json::Value {
        match tool.call(args) {
            ToolOutcome::OkJson(json) => serde_json::from_str(&json).expect("tool json parses"),
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// The list is two-source, saved first, with a discriminating `isUser`
    /// flag on saved entries and no scene/tags/styleGuide keys — the same
    /// shape the styles list uses.
    #[test]
    fn listing_puts_saved_templates_first_with_is_user() {
        let _guard = exclusive_user_template_registry_for_tests();
        save("my-deck");
        let json = call_json(&list_scene_templates_snapshot(true), &BTreeMap::new());
        let templates = json["templates"].as_array().expect("a templates array");

        assert!(templates.len() > 60, "the shipped catalogue plus one save");
        assert_eq!(templates[0]["id"], "user:my-deck");
        assert_eq!(templates[0]["name"], "my-deck");
        assert_eq!(templates[0]["isUser"], true);
        assert_eq!(templates[0]["frames"], 2);
        assert_eq!(templates[0]["frameWidth"], 1920);
        assert_eq!(templates[0]["frameHeight"], 1080);
        assert!(
            templates[0].get("scene").is_none() && templates[0].get("tags").is_none(),
            "saved entries carry no scene or tags"
        );
        // The shipped half keeps its historical shape.
        assert_eq!(templates[1]["isUser"], serde_json::Value::Null);
        assert!(templates[1]["scene"].is_string());
    }

    /// Saved templates carry no scene, so the scene filter narrows the
    /// shipped half only.
    #[test]
    fn scene_filter_leaves_saved_templates_alone() {
        let _guard = exclusive_user_template_registry_for_tests();
        save("my-deck");
        let mut args = BTreeMap::new();
        args.insert("scene".into(), "slides".into());
        let json = call_json(&list_scene_templates_snapshot(true), &args);
        let templates = json["templates"].as_array().expect("a templates array");
        assert_eq!(templates[0]["id"], "user:my-deck");
        assert!(
            templates[1..].iter().all(|t| t["scene"] == "slides"),
            "the shipped half is filtered, the saved half is not"
        );
    }

    #[test]
    fn shipped_only_listing_never_reads_the_user_registry() {
        let _guard = exclusive_user_template_registry_for_tests();
        save("private-deck");
        let json = call_json(&list_scene_templates_snapshot(false), &BTreeMap::new());
        let templates = json["templates"].as_array().expect("a templates array");

        assert!(templates
            .iter()
            .any(|template| template["id"] == "slide-deck"));
        assert!(
            templates
                .iter()
                .all(|template| template["id"] != "user:private-deck"),
            "a shared deployment only receives immutable shipped entries"
        );
    }

    /// A saved id is usable and answers with its own name; an id neither
    /// half knows is an argument error.
    #[test]
    fn use_scene_template_reaches_both_halves() {
        let _guard = exclusive_user_template_registry_for_tests();
        save("my-deck");
        let tool = use_scene_template_snapshot(true);

        let mut args = BTreeMap::new();
        args.insert("templateId".into(), "user:my-deck".into());
        match tool.call(&args) {
            ToolOutcome::OkJsonWithCommand(json, command) => {
                let parsed: serde_json::Value = serde_json::from_str(&json).expect("parses");
                assert_eq!(parsed["id"], "user:my-deck");
                assert_eq!(parsed["title"], "my-deck");
                assert_eq!(parsed["frames"], 2);
                assert!(matches!(
                    command,
                    EditorCommand::AdoptSceneTemplate { template_id } if template_id == "user:my-deck"
                ));
            }
            other => panic!("expected OkJsonWithCommand, got {other:?}"),
        }

        let mut unknown = BTreeMap::new();
        unknown.insert("templateId".into(), "user:never-saved".into());
        assert!(matches!(
            tool.call(&unknown),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
        let mut unknown_shipped = BTreeMap::new();
        unknown_shipped.insert("templateId".into(), "no-such-template".into());
        assert!(matches!(
            tool.call(&unknown_shipped),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
    }

    #[test]
    fn shipped_only_use_snapshot_does_not_resolve_user_registry_ids() {
        let _guard = exclusive_user_template_registry_for_tests();
        save("private-deck");
        let tool = use_scene_template_snapshot(false);

        let user = BTreeMap::from([("templateId".to_string(), "user:private-deck".to_string())]);
        assert!(matches!(
            tool.call(&user),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));

        let shipped = BTreeMap::from([("templateId".to_string(), "slide-deck".to_string())]);
        assert!(matches!(
            tool.call(&shipped),
            ToolOutcome::OkJsonWithCommand(_, EditorCommand::AdoptSceneTemplate { template_id })
                if template_id == "slide-deck"
        ));
    }
}
