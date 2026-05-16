//! Type-bridge: `op_editor_core::EditorState` → the chrome / chat /
//! components / scalar fields of a shell-core paint `Document`.
//!
//! `pen_document_to_document` already converts the *node tree* +
//! geometry from a `PenDocument`, but it leaves `Document.ui` /
//! `.chat` / `.components` at `Default` and the scalar
//! selection / tool / viewport / active-page fields untouched. This
//! module fills exactly those gaps so a host's per-frame
//! `paint_document()` can produce a faithful paint snapshot of the
//! live `EditorState`.
//!
//! Lives in `op-pen-loader`, NOT `op-editor-core`: it materializes
//! `openpencil-shell-core` types, and `op-editor-core` must stay free
//! of any shell-core dependency to keep its `wasm32-unknown-unknown`
//! invariant. `op-pen-loader` already depends on both crates.
//!
//! The UI-struct merge lives in [`crate::bridge_ui`]; the ~16 enum
//! translators in [`crate::bridge_enums`].

use op_editor_core as ec;
use openpencil_shell_core as sc;

use crate::bridge_ui::editor_ui_to_ui_state;

/// Convert `op_editor_core::ChatState` → shell-core's `ChatState`.
/// Field-by-field, including the message transcript, the streaming
/// `pending_send` flag and the model catalog.
pub fn editor_chat_to_chat_state(chat: &ec::ChatState) -> sc::document::ChatState {
    sc::document::ChatState {
        messages: chat
            .messages
            .iter()
            .map(|m| sc::document::ChatMessage {
                role: chat_role(m.role),
                content: m.content.clone(),
            })
            .collect(),
        input: chat.input.clone(),
        focused: chat.focused,
        anchor: chat_anchor(chat.anchor),
        collapsed: chat.collapsed,
        caret_anchor_ms: chat.caret_anchor_ms,
        pending_send: chat.pending_send.clone(),
        available_models: chat
            .available_models
            .iter()
            .map(|m| sc::chat_models::ModelEntry {
                provider: crate::bridge_enums::agent_provider(m.provider),
                value: m.value.clone(),
                display_name: m.display_name.clone(),
            })
            .collect(),
        selected_model: chat.selected_model,
    }
}

/// `op_editor_core::ChatRole` → shell-core's.
fn chat_role(r: ec::ChatRole) -> sc::document::ChatRole {
    match r {
        ec::ChatRole::User => sc::document::ChatRole::User,
        ec::ChatRole::Assistant => sc::document::ChatRole::Assistant,
    }
}

/// `op_editor_core::ChatAnchor` → shell-core's.
fn chat_anchor(a: ec::ChatAnchor) -> sc::document::ChatAnchor {
    match a {
        ec::ChatAnchor::TopLeft => sc::document::ChatAnchor::TopLeft,
        ec::ChatAnchor::TopRight => sc::document::ChatAnchor::TopRight,
        ec::ChatAnchor::BottomLeft => sc::document::ChatAnchor::BottomLeft,
        ec::ChatAnchor::BottomRight => sc::document::ChatAnchor::BottomRight,
    }
}

/// Convert `op_editor_core::ComponentLibrary` → shell-core's.
///
/// The crucial difference between the two: `op-editor-core`'s
/// `Component.root` is a canonical `jian_ops_schema::node::PenNode`,
/// while shell-core's is the private `Document` `Node`. There is no
/// direct `PenNode → Node` converter, but [`pen_document_to_document`]
/// converts a whole `PenDocument`, so each component root is wrapped
/// in a one-child throwaway `PenDocument`, converted, and its single
/// resulting node lifted out. A component whose root somehow fails to
/// convert (no node produced) is skipped rather than panicking.
pub fn editor_components_to_component_library(
    lib: &ec::ComponentLibrary,
) -> sc::document::ComponentLibrary {
    let mut out = sc::document::ComponentLibrary::default();
    for c in &lib.components {
        let Some(root) = pen_node_to_document_node(&c.root) else {
            continue;
        };
        out.insert(sc::document::Component {
            id: crate::bridge_ui::node_id(&c.id),
            name: c.name.clone(),
            root,
        });
    }
    out
}

/// Convert a single canonical `PenNode` into a shell-core `Document`
/// `Node` by routing it through the full document converter. Returns
/// `None` when the conversion produced no top-level node.
fn pen_node_to_document_node(
    node: &jian_ops_schema::node::PenNode,
) -> Option<sc::document::Node> {
    let wrapper = jian_ops_schema::PenDocument {
        version: "0.8.0".to_string(),
        name: None,
        themes: None,
        variables: None,
        pages: None,
        children: vec![node.clone()],
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
    };
    let doc = crate::pen_document_to_document(&wrapper);
    doc.pages
        .into_iter()
        .next()
        .and_then(|p| p.children.into_iter().next())
}

/// Apply the non-tree state of an `EditorState` onto a `Document` that
/// [`pen_document_to_document`] already filled with the node tree +
/// geometry.
///
/// This sets everything `pen_document_to_document` left at `Default`:
///
///   - `doc.ui` — merged from `EditorState.editor_ui` + `.ui`.
///   - `doc.chat` — from `EditorState.chat`.
///   - `doc.components` — from `EditorState.components`.
///   - `doc.var_table` — rebuilt via [`crate::editor_state_var_table`].
///   - the scalar `selected` / `selected_set` / `tool` / `viewport` /
///     `active_page_index` fields.
///
/// After `pen_document_to_document(&state.doc)` followed by
/// `apply_editor_state_ui(&mut doc, &state)`, the `Document` is a
/// faithful paint snapshot of the `EditorState`. Named
/// `apply_editor_state_ui` (not `..._chrome`) per the crate's
/// established `editor_ui` / `ui` terminology.
pub fn apply_editor_state_ui(doc: &mut sc::document::Document, state: &ec::EditorState) {
    // Chrome / panel / menu + transient edit UI.
    doc.ui = editor_ui_to_ui_state(&state.editor_ui, &state.ui);
    // AI chat sub-state.
    doc.chat = editor_chat_to_chat_state(&state.chat);
    // Component library.
    doc.components = editor_components_to_component_library(&state.components);
    // Variable table — persisted defs + transient theme/ref caches.
    doc.var_table = crate::editor_state_var_table(state);
    // Scalar selection / tool / viewport / active-page fields.
    doc.tool = crate::bridge_enums::tool(state.tool);
    doc.active_page_index = state.ui.active_page_index;
    doc.viewport = sc::document::Viewport {
        pan_x: state.viewport.pan_x,
        pan_y: state.viewport.pan_y,
        zoom: state.viewport.zoom,
    };
    doc.selected = crate::bridge_ui::node_id(&state.selection.anchor);
    doc.selected_set = state
        .selection
        .set
        .iter()
        .map(crate::bridge_ui::node_id)
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_bridge_maps_a_populated_chat() {
        let mut chat = ec::ChatState::default();
        chat.input = "draft me a card".into();
        chat.focused = true;
        chat.anchor = ec::ChatAnchor::TopRight;
        chat.collapsed = true;
        chat.caret_anchor_ms = 321;
        chat.pending_send = Some("draft me a card".into());
        chat.messages.push(ec::ChatMessage {
            role: ec::ChatRole::User,
            content: "hi".into(),
        });
        chat.messages.push(ec::ChatMessage {
            role: ec::ChatRole::Assistant,
            content: "hello".into(),
        });
        chat.available_models.push(ec::ModelEntry::new(
            ec::AgentProvider::CodexCli,
            "gpt-5.5",
            "GPT-5.5",
        ));
        chat.selected_model = 0;

        let out = editor_chat_to_chat_state(&chat);
        assert_eq!(out.input, "draft me a card");
        assert!(out.focused);
        assert_eq!(out.anchor, sc::document::ChatAnchor::TopRight);
        assert!(out.collapsed);
        assert_eq!(out.caret_anchor_ms, 321);
        assert_eq!(out.pending_send.as_deref(), Some("draft me a card"));
        assert_eq!(out.messages.len(), 2);
        assert_eq!(out.messages[0].role, sc::document::ChatRole::User);
        assert_eq!(out.messages[0].content, "hi");
        assert_eq!(out.messages[1].role, sc::document::ChatRole::Assistant);
        assert_eq!(out.messages[1].content, "hello");
        assert_eq!(out.available_models.len(), 1);
        assert_eq!(out.available_models[0].value, "gpt-5.5");
        assert_eq!(
            out.available_models[0].provider,
            sc::document::AgentProvider::CodexCli
        );
        assert_eq!(out.selected_model, 0);
    }

    #[test]
    fn components_bridge_maps_a_populated_library() {
        // Build a canonical Frame node through the schema directly.
        let frame = make_frame("comp-1", "Button");
        let mut lib = ec::ComponentLibrary::default();
        lib.insert(ec::Component {
            id: ec::NodeId::new("comp-1"),
            name: "Button".into(),
            root: frame,
        });

        let out = editor_components_to_component_library(&lib);
        assert_eq!(out.components.len(), 1);
        let c = out
            .find_by_id(sc::document::NodeId::new("comp-1"))
            .expect("component present");
        assert_eq!(c.name, "Button");
        assert_eq!(c.root.kind, sc::document::NodeKind::Frame);
    }

    #[test]
    fn apply_editor_state_ui_yields_a_faithful_snapshot() {
        let mut state = ec::EditorState::new();
        // Non-trivial chrome.
        state.editor_ui.sidebar_open = false;
        state.editor_ui.theme_mode = ec::ThemeMode::Light;
        state.editor_ui.shape_tool = ec::Tool::Polygon;
        // Non-trivial transient UI.
        state.ui.active_page_index = 0;
        state.ui.property_focus = Some(ec::PropertyFocus::Rotation);
        // Tool / viewport / selection.
        state.tool = ec::Tool::Pen;
        state.viewport.pan_x = 50.0;
        state.viewport.pan_y = -25.0;
        state.viewport.zoom = 2.0;
        state.selection.set = vec![ec::NodeId::new("n3"), ec::NodeId::new("n7")];
        state.selection.anchor = ec::NodeId::new("n7");
        // Chat + components.
        state.chat.input = "hello".into();
        state.components.insert(ec::Component {
            id: ec::NodeId::new("comp-x"),
            name: "Card".into(),
            root: make_frame("comp-x", "Card"),
        });

        // Start from a node-tree-only Document, then apply the bridge.
        let mut doc = crate::pen_document_to_document(&state.doc);
        apply_editor_state_ui(&mut doc, &state);

        // UI reflects the editor state.
        assert!(!doc.ui.sidebar_open);
        assert_eq!(doc.ui.theme_mode, sc::document::ThemeMode::Light);
        assert_eq!(doc.ui.shape_tool, sc::document::Tool::Polygon);
        assert_eq!(
            doc.ui.property_focus,
            Some(sc::document::PropertyFocus::Rotation)
        );
        // Chat reflects the editor state.
        assert_eq!(doc.chat.input, "hello");
        // Components reflect the editor state.
        assert_eq!(doc.components.components.len(), 1);
        assert!(doc
            .components
            .find_by_id(sc::document::NodeId::new("comp-x"))
            .is_some());
        // Scalar fields.
        assert_eq!(doc.tool, sc::document::Tool::Pen);
        assert_eq!(doc.viewport.pan_x, 50.0);
        assert_eq!(doc.viewport.pan_y, -25.0);
        assert_eq!(doc.viewport.zoom, 2.0);
        assert_eq!(doc.active_page_index, 0);
        assert_eq!(doc.selected, sc::document::NodeId::new("n7"));
        assert_eq!(
            doc.selected_set,
            vec![
                sc::document::NodeId::new("n3"),
                sc::document::NodeId::new("n7")
            ]
        );
    }

    #[test]
    fn apply_editor_state_ui_var_table_reflects_variables() {
        use jian_ops_schema::variable::{VariableKind, VariableScalar};
        let mut state = ec::EditorState::new();
        state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#112233".into()),
        );
        let mut doc = crate::pen_document_to_document(&state.doc);
        apply_editor_state_ui(&mut doc, &state);
        assert_eq!(doc.var_table.variables.len(), 1);
        assert_eq!(doc.var_table.variables[0].name, "brand");
    }

    /// A minimal canonical Frame `PenNode` fixture, parsed from a
    /// `.op` JSON snippet so it stays robust to schema growth (the
    /// concrete node structs do not derive `Default`).
    fn make_frame(id: &str, name: &str) -> jian_ops_schema::node::PenNode {
        let src = format!(
            r#"{{"version":"0.8.0","children":[
                {{"type":"frame","id":"{id}","name":"{name}",
                 "x":0,"y":0,"width":100,"height":100,"children":[]}}
            ]}}"#
        );
        let loaded = jian_ops_schema::load_str(&src).expect("fixture parses");
        loaded
            .value
            .children
            .into_iter()
            .next()
            .expect("one node")
    }
}
