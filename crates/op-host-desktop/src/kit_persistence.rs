//! Imported-UIKit persistence — `<config>/openpencil/uikits.json`.
//!
//! The desktop counterpart of the TS `uikit-store.ts` persist /
//! hydrate pair (localStorage key `openpencil-uikits`). The JSON
//! shape mirrors the TS `PersistedState` exactly — `importedKits`
//! carries full TS-shaped `UIKit` objects (`id` / `name` / `version`
//! / `builtIn` / `document` / `components`) so a payload moved
//! between the TS and Rust apps hydrates on either side:
//!
//! ```json
//! { "importedKits": [ { "id": "…", "name": "…", "version": "1.0.0",
//!     "builtIn": false, "document": { …canonical PenDocument… },
//!     "components": [ { "id": "…", "name": "…", "category":
//!         "buttons", "tags": [], "width": 120, "height": 40 } ] } ],
//!   "browserOpen": false }
//! ```
//!
//! Documented divergence: the Rust `UIKit` keeps per-component
//! templates + the kit's `$variable` definitions rather than the full
//! backing document, so Save reconstitutes the document from those
//! (themes a kit file carried are not round-tripped through a
//! restart — instantiate only ever copies variables, TS
//! `component-browser-card.tsx`).

use std::path::PathBuf;

use op_editor_core::uikit::{KitComponent, UIKit};
use op_editor_core::{ComponentCategory, EditorState};
use serde::{Deserialize, Serialize};

const APP_DIR: &str = "openpencil";
const FILE_NAME: &str = "uikits.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedState {
    #[serde(default)]
    imported_kits: Vec<PersistedKit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    browser_open: Option<bool>,
}

/// TS `UIKit` wire shape.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedKit {
    id: String,
    name: String,
    version: String,
    built_in: bool,
    document: jian_ops_schema::PenDocument,
    components: Vec<PersistedComponent>,
}

/// TS `KitComponent` wire shape (metadata only — the node lives in
/// `document`).
#[derive(Debug, Serialize, Deserialize)]
struct PersistedComponent {
    id: String,
    name: String,
    category: String,
    tags: Vec<String>,
    width: f32,
    height: f32,
}

/// Resolve the platform-specific store path. `None` when no usable
/// config base exists — load/save become silent no-ops.
fn store_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

// ---------------------------------------------------------------------------
// Save
// ---------------------------------------------------------------------------

/// Persist the imported kits + browser-open flag. Best-effort: IO /
/// encode failures log and move on (TS persist() swallows quota
/// errors the same way).
pub fn save(state: &EditorState) {
    let Some(path) = store_path() else {
        return;
    };
    if let Err(e) = save_to(state, &path) {
        eprintln!("openpencil-desktop: uikits.json save failed: {e}");
    }
}

fn save_to(state: &EditorState, path: &std::path::Path) -> Result<(), String> {
    let payload = PersistedState {
        imported_kits: state
            .ui_kits
            .iter()
            .filter(|k| !k.built_in)
            .map(kit_to_persisted)
            .collect(),
        browser_open: Some(state.editor_ui.component_browser_open),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn kit_to_persisted(kit: &UIKit) -> PersistedKit {
    PersistedKit {
        id: kit.id.clone(),
        name: kit.name.clone(),
        version: kit.version.clone(),
        built_in: false,
        document: jian_ops_schema::PenDocument {
            version: kit.version.clone(),
            name: Some(kit.name.clone()),
            themes: None,
            variables: kit.variables.clone(),
            pages: None,
            children: kit.components.iter().map(|c| c.template.clone()).collect(),
            format_version: None,
            id: None,
            app: None,
            routes: None,
            state: None,
            lifecycle: None,
            logic_modules: None,
            design_md: None,
            conversion: None,
        },
        components: kit
            .components
            .iter()
            .map(|c| PersistedComponent {
                id: c.id.clone(),
                name: c.name.clone(),
                category: c.category.as_ts_str().to_string(),
                tags: c.tags.clone(),
                width: c.width,
                height: c.height,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

/// Hydrate imported kits + the browser-open flag onto a fresh
/// `EditorState` (call once at startup, after `op_host_services::settings_io::load`).
/// Best-effort: a missing / unreadable / malformed file leaves the
/// built-in kits alone.
pub fn load(state: &mut EditorState) {
    let Some(path) = store_path() else {
        return;
    };
    load_from(state, &path);
}

fn load_from(state: &mut EditorState, path: &std::path::Path) {
    let Ok(src) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(payload) = serde_json::from_str::<PersistedState>(&src) else {
        return;
    };
    // TS hydrate: drop persisted kits whose ids clash with built-ins.
    let builtin_ids: Vec<String> = state
        .ui_kits
        .iter()
        .filter(|k| k.built_in)
        .map(|k| k.id.clone())
        .collect();
    for persisted in payload.imported_kits {
        if persisted.built_in || builtin_ids.contains(&persisted.id) {
            continue;
        }
        if state.ui_kits.iter().any(|k| k.id == persisted.id) {
            continue;
        }
        if let Some(kit) = persisted_to_kit(persisted) {
            state.ui_kits.push(kit);
        }
    }
    if let Some(open) = payload.browser_open {
        state.editor_ui.component_browser_open = open;
    }
}

fn persisted_to_kit(persisted: PersistedKit) -> Option<UIKit> {
    let mut components = Vec::with_capacity(persisted.components.len());
    for meta in persisted.components {
        // Re-find each component's node in the kit document — the
        // persisted metadata keeps category / tags fidelity, the
        // document keeps the template subtree. A missing node skips
        // that component (stale metadata).
        let Some(template) = find_node(&persisted.document, &meta.id) else {
            continue;
        };
        components.push(KitComponent {
            id: meta.id,
            name: meta.name,
            category: ComponentCategory::from_ts_str(&meta.category),
            tags: meta.tags,
            width: meta.width,
            height: meta.height,
            template: template.clone(),
        });
    }
    if components.is_empty() {
        return None;
    }
    Some(UIKit {
        id: persisted.id,
        name: persisted.name,
        version: persisted.version,
        built_in: false,
        components,
        variables: persisted.document.variables.clone(),
    })
}

fn find_node<'a>(
    doc: &'a jian_ops_schema::PenDocument,
    id: &str,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    fn walk<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::pen_node_ext::PenNodeExt;
        if node.base().id == id {
            return Some(node);
        }
        for child in node.children().into_iter().flatten() {
            if let Some(found) = walk(child, id) {
                return Some(found);
            }
        }
        None
    }
    let roots: Vec<&jian_ops_schema::node::PenNode> = match doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages.iter().flat_map(|p| p.children.iter()).collect(),
        _ => doc.children.iter().collect(),
    };
    roots.into_iter().find_map(|root| walk(root, id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::uikit_io::import_kit_from_document;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "op-kit-persistence-{tag}-{}.json",
            std::process::id()
        ))
    }

    fn imported_kit_fixture() -> UIKit {
        // Reuse a built-in template as the reusable node by flagging it.
        let starter = op_editor_core::uikit::builtin_starter_kit();
        let mut template = starter
            .components
            .into_iter()
            .find(|c| c.id == "btn-primary")
            .map(|c| c.template)
            .expect("starter template");
        if let jian_ops_schema::node::PenNode::Frame(f) = &mut template {
            f.base.id = "c1".to_string();
            f.base.name = Some("Primary Button".to_string());
            f.reusable = Some(true);
        }
        let doc = jian_ops_schema::PenDocument {
            version: "1.0.0".to_string(),
            name: Some("My Imported Kit".to_string()),
            themes: None,
            variables: None,
            pages: None,
            children: vec![template],
            format_version: None,
            id: None,
            app: None,
            routes: None,
            state: None,
            lifecycle: None,
            logic_modules: None,
            design_md: None,
            conversion: None,
        };
        import_kit_from_document(&doc, "imp-1".to_string()).expect("kit")
    }

    #[test]
    fn save_load_round_trips_imported_kits_and_browser_open() {
        let path = temp_path("roundtrip");
        let mut state = EditorState::new();
        state.import_kit(imported_kit_fixture());
        state.editor_ui.component_browser_open = true;
        save_to(&state, &path).expect("save");

        let mut fresh = EditorState::new();
        let builtin_count = fresh.ui_kits.len();
        load_from(&mut fresh, &path);
        assert_eq!(fresh.ui_kits.len(), builtin_count + 1);
        let kit = fresh.ui_kits.last().expect("kit");
        assert_eq!(kit.id, "imp-1");
        assert_eq!(kit.name, "My Imported Kit");
        assert!(!kit.built_in);
        assert_eq!(kit.components.len(), 1);
        assert_eq!(kit.components[0].id, "c1");
        assert_eq!(
            kit.components[0].category,
            ComponentCategory::Buttons,
            "category string must round-trip"
        );
        assert!(fresh.editor_ui.component_browser_open);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_filters_builtin_id_clashes_and_duplicates() {
        let path = temp_path("clash");
        let mut state = EditorState::new();
        let mut clashing = imported_kit_fixture();
        clashing.id = "openpencil-starter".to_string();
        state.ui_kits.push(clashing);
        state.import_kit(imported_kit_fixture());
        save_to(&state, &path).expect("save");

        let mut fresh = EditorState::new();
        let builtin_count = fresh.ui_kits.len();
        load_from(&mut fresh, &path);
        // The clashing kit is dropped, the legit one loads once.
        assert_eq!(fresh.ui_kits.len(), builtin_count + 1);
        // Loading again must not duplicate.
        load_from(&mut fresh, &path);
        assert_eq!(fresh.ui_kits.len(), builtin_count + 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_is_a_noop_on_missing_or_malformed_file() {
        let mut state = EditorState::new();
        let count = state.ui_kits.len();
        load_from(&mut state, &temp_path("missing-nonexistent"));
        assert_eq!(state.ui_kits.len(), count);

        let path = temp_path("malformed");
        std::fs::write(&path, "{ not json").expect("write");
        load_from(&mut state, &path);
        assert_eq!(state.ui_kits.len(), count);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persisted_wire_shape_matches_ts_store() {
        let path = temp_path("wire");
        let mut state = EditorState::new();
        state.import_kit(imported_kit_fixture());
        save_to(&state, &path).expect("save");
        let src = std::fs::read_to_string(&path).expect("read");
        let value: serde_json::Value = serde_json::from_str(&src).expect("json");
        let kits = value
            .get("importedKits")
            .and_then(|v| v.as_array())
            .expect("importedKits array");
        let kit = &kits[0];
        for key in ["id", "name", "version", "builtIn", "document", "components"] {
            assert!(kit.get(key).is_some(), "missing TS UIKit key {key}");
        }
        assert_eq!(kit["builtIn"], serde_json::Value::Bool(false));
        assert_eq!(kit["components"][0]["category"], "buttons");
        assert!(value.get("browserOpen").is_some());
        let _ = std::fs::remove_file(&path);
    }
}
