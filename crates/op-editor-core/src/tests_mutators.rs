//! Mutator tests — selection / tree-ops / grouping / history.
//! Ported from `openpencil-shell-core::document::tests_mutators`,
//! retargeted onto `EditorState` + the canonical `PenNode` tree.

#![cfg(test)]

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, group, rect, sample, state_with};
use crate::walkers::{find_node, ReorderDirection};
use jian_ops_schema::style::PenFill;
use jian_ops_schema::variable::{VariableKind, VariableScalar};

// --- Selection -------------------------------------------------------

#[test]
fn set_single_selection_replaces_set_and_anchor() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n10"));
    assert_eq!(s.selection.anchor, NodeId::new("n10"));
    assert_eq!(s.selection.set, vec![NodeId::new("n10")]);
}

// --- Preview (Play) mode — document invariance -----------------------

#[test]
fn enter_exit_preview_leaves_document_byte_identical() {
    // Phase D5: entering and exiting Preview must NOT mutate the saved
    // document. The runtime is built host-side from the serialized doc;
    // the editor state only flips the flag. Assert the canonical
    // serialization is identical across an enter → exit cycle.
    let mut s = state_with(vec![frame(
        "root",
        "Root",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![rect("a", "A", 10.0, 10.0, 50.0, 50.0)],
    )]);
    let before = serde_json::to_string(&s.doc).expect("serialize before");

    s.editor_ui.enter_preview();
    assert!(s.editor_ui.preview_mode);
    s.editor_ui.exit_preview();
    assert!(!s.editor_ui.preview_mode);

    let after = serde_json::to_string(&s.doc).expect("serialize after");
    assert_eq!(before, after, "preview enter→exit must not touch doc");
}

#[test]
fn set_single_selection_none_clears() {
    let mut s = sample();
    s.set_single_selection(NodeId::NONE);
    assert!(s.selection.is_empty());
}

#[test]
fn toggle_selection_adds_then_removes() {
    let mut s = sample();
    s.clear_selection();
    s.toggle_selection(NodeId::new("n10"));
    s.toggle_selection(NodeId::new("n11"));
    assert_eq!(s.selection_count(), 2);
    assert_eq!(s.selection.anchor, NodeId::new("n11"));
    s.toggle_selection(NodeId::new("n11"));
    assert_eq!(s.selection_count(), 1);
    assert_eq!(s.selection.anchor, NodeId::new("n10"));
}

#[test]
fn select_all_top_level_picks_every_root() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.select_all_top_level());
    assert_eq!(s.selection_count(), 2);
    assert_eq!(s.selection.anchor, NodeId::new("n2"));
}

#[test]
fn select_all_top_level_empty_page_is_noop() {
    let mut s = state_with(vec![]);
    assert!(!s.select_all_top_level());
}

#[test]
fn set_selected_image_fill_mode_updates_primary_image_fill() {
    let mut node = rect("n60", "Photo fill", 0.0, 0.0, 100.0, 80.0);
    crate::fills::set_primary_fill_type(&mut node, crate::FillType::Image);
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n60"));

    assert!(s.set_selected_image_fill_mode(crate::ImageFillMode::Crop));

    let node = find_node(s.active_children(), &NodeId::new("n60")).unwrap();
    match crate::fills::node_fills(node).unwrap().first().unwrap() {
        PenFill::Image(body) => {
            assert_eq!(body.mode, Some(jian_ops_schema::style::ImageFillMode::Crop));
        }
        other => panic!("expected image fill, got {other:?}"),
    }
}

#[test]
fn image_fill_summary_exposes_selected_image_url_for_preview() {
    let mut node = rect("n62", "Photo fill", 0.0, 0.0, 100.0, 80.0);
    crate::fills::set_primary_fill_type(&mut node, crate::FillType::Image);
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n62"));

    let url = "data:image/png;base64,iVBORw0KGgo=";
    assert!(s.set_selected_fill_image_url(url));

    let node = find_node(s.active_children(), &NodeId::new("n62")).unwrap();
    let summary = crate::fills::first_image_fill_summary(node).unwrap();
    assert!(summary.has_image);
    assert_eq!(summary.image_url.as_deref(), Some(url));
}

#[test]
fn set_selected_image_adjustment_clamps_and_resets() {
    let mut node = rect("n61", "Photo fill", 0.0, 0.0, 100.0, 80.0);
    crate::fills::set_primary_fill_type(&mut node, crate::FillType::Image);
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n61"));

    assert!(s.set_selected_image_adjustment(crate::ImageAdjustmentField::Exposure, 125.0));
    assert!(s.set_selected_image_adjustment(crate::ImageAdjustmentField::Contrast, -125.0));

    let node = find_node(s.active_children(), &NodeId::new("n61")).unwrap();
    match crate::fills::node_fills(node).unwrap().first().unwrap() {
        PenFill::Image(body) => {
            assert_eq!(body.exposure, Some(100.0));
            assert_eq!(body.contrast, Some(-100.0));
        }
        other => panic!("expected image fill, got {other:?}"),
    }

    assert!(s.reset_selected_image_adjustments());
    let node = find_node(s.active_children(), &NodeId::new("n61")).unwrap();
    match crate::fills::node_fills(node).unwrap().first().unwrap() {
        PenFill::Image(body) => {
            assert_eq!(body.exposure, Some(0.0));
            assert_eq!(body.contrast, Some(0.0));
            assert_eq!(body.saturation, Some(0.0));
            assert_eq!(body.temperature, Some(0.0));
            assert_eq!(body.tint, Some(0.0));
            assert_eq!(body.highlights, Some(0.0));
            assert_eq!(body.shadows, Some(0.0));
        }
        other => panic!("expected image fill, got {other:?}"),
    }
}

#[test]
fn replace_selected_icon_updates_icon_font_name_without_closing_over_color() {
    let mut s = sample();
    let id = s
        .insert_icon_font_node_at("search", "lucide", 50.0, 50.0)
        .expect("insert icon");

    assert!(s.replace_selected_icon("home", "lucide", None));

    let node = find_node(s.active_children(), &id).unwrap();
    let jian_ops_schema::node::PenNode::IconFont(icon) = node else {
        panic!("expected icon_font node");
    };
    assert_eq!(icon.icon_font_name, "home");
    assert_eq!(icon.icon_font_family.as_deref(), Some("lucide"));
    assert!(icon.fill.is_some(), "replacement preserves display color");
}

// --- Delete ----------------------------------------------------------

#[test]
fn delete_selected_removes_top_level_node_and_clears_selection() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n10"));
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
    assert!(s.delete_selected());
    assert_eq!(s.selection.anchor, NodeId::NONE);
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_none());
}

#[test]
fn delete_selected_removes_nested_node() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n13"));
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n13")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
}

#[test]
fn delete_selected_returns_false_when_unselected() {
    let mut s = sample();
    s.clear_selection();
    assert!(!s.delete_selected());
}

#[test]
fn delete_selected_removes_every_node_in_the_set() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 20.0, 0.0, 10.0, 10.0),
        rect("n3", "C", 40.0, 0.0, 10.0, 10.0),
    ]);
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    assert_eq!(s.selection_count(), 2);
    assert!(s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n1")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n2")).is_none());
    assert!(find_node(s.active_children(), &NodeId::new("n3")).is_some());
    assert_eq!(s.active_children().len(), 1);
    assert!(s.selection.is_empty());
}

#[test]
fn delete_selected_protects_ancestor_of_locked_descendant() {
    let mut child = rect("n61", "child", 0.0, 0.0, 10.0, 10.0);
    child.base_mut().locked = Some(true);
    let parent = frame("n60", "parent", 0.0, 0.0, 50.0, 50.0, vec![child]);
    let mut s = state_with(vec![parent]);
    s.set_single_selection(NodeId::new("n60"));
    // Subtree contains a locked node → delete refused.
    assert!(!s.delete_selected());
    assert!(find_node(s.active_children(), &NodeId::new("n60")).is_some());
}

// --- Duplicate -------------------------------------------------------

#[test]
fn duplicate_selected_clones_subtree_with_fresh_ids_and_selects_it() {
    let mut s = sample();
    s.set_single_selection(NodeId::new("n10"));
    let mut next_id = 1_000u64;
    let clone_id = s
        .duplicate_selected(&mut next_id, 10.0)
        .expect("duplicate should return new id");
    assert!(clone_id.is_real());
    assert_eq!(s.selection.anchor, clone_id);
    // Original survives.
    assert!(find_node(s.active_children(), &NodeId::new("n10")).is_some());
    // Clone present with fresh id.
    let original = find_node(s.active_children(), &NodeId::new("n10")).unwrap();
    let clone = find_node(s.active_children(), &clone_id).unwrap();
    // Clone offset by 10 px on both axes.
    assert!((clone.base().x.unwrap() - original.base().x.unwrap() - 10.0).abs() < 1e-3);
    assert!((clone.base().y.unwrap() - original.base().y.unwrap() - 10.0).abs() < 1e-3);
    // Descendant count preserved.
    assert_eq!(
        clone.children().map(|c| c.len()),
        original.children().map(|c| c.len())
    );
    assert!(s.validate().is_ok());
}

#[test]
fn duplicate_selected_returns_none_when_unselected() {
    let mut s = sample();
    s.clear_selection();
    let mut next_id = 1u64;
    assert!(s.duplicate_selected(&mut next_id, 10.0).is_none());
}

// --- Reorder ---------------------------------------------------------

fn three_rects() -> crate::state::EditorState {
    state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
        rect("n3", "C", 0.0, 0.0, 10.0, 10.0),
    ])
}

fn root_ids(s: &crate::state::EditorState) -> Vec<String> {
    s.active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect()
}

#[test]
fn reorder_selected_up_moves_toward_front_index() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n2"));
    assert!(s.reorder_selected(ReorderDirection::Up));
    assert_eq!(root_ids(&s), vec!["n2", "n1", "n3"]);
}

#[test]
fn reorder_selected_down_moves_toward_back_index() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n2"));
    assert!(s.reorder_selected(ReorderDirection::Down));
    assert_eq!(root_ids(&s), vec!["n1", "n3", "n2"]);
}

#[test]
fn reorder_selected_at_edges_is_noop() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.reorder_selected(ReorderDirection::Up));
    s.set_single_selection(NodeId::new("n3"));
    assert!(!s.reorder_selected(ReorderDirection::Down));
}

#[test]
fn reorder_before_moves_source_to_anchor_position() {
    let mut s = three_rects();
    assert!(s.reorder_before(NodeId::new("n3"), NodeId::new("n1")));
    assert_eq!(root_ids(&s), vec!["n3", "n1", "n2"]);
}

#[test]
fn reorder_after_moves_source_after_anchor() {
    let mut s = three_rects();
    assert!(s.reorder_after(NodeId::new("n1"), NodeId::new("n2")));
    assert_eq!(root_ids(&s), vec!["n2", "n1", "n3"]);
}

#[test]
fn reorder_into_reparents_under_container() {
    let mut s = state_with(vec![
        frame("n1", "Frame", 0.0, 0.0, 100.0, 100.0, vec![]),
        rect("n2", "Loose", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.reorder_into(NodeId::new("n2"), NodeId::new("n1")));
    assert_eq!(root_ids(&s), vec!["n1"]);
    let parent = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(parent.children().unwrap().len(), 1);
}

#[test]
fn reorder_into_rejects_cycle() {
    let mut s = state_with(vec![frame(
        "n1",
        "Frame",
        0.0,
        0.0,
        100.0,
        100.0,
        vec![rect("n2", "Child", 0.0, 0.0, 10.0, 10.0)],
    )]);
    // Can't move the parent under its own child.
    assert!(!s.reorder_into(NodeId::new("n1"), NodeId::new("n2")));
}

// --- Grouping --------------------------------------------------------

#[test]
fn group_selected_wraps_siblings_in_a_group() {
    let mut s = three_rects();
    s.clear_selection();
    s.toggle_selection(NodeId::new("n1"));
    s.toggle_selection(NodeId::new("n2"));
    let mut next_id = 1u64;
    let group_id = s.group_selected(&mut next_id).expect("group");
    // Group replaces n1 + n2 at position 0; n3 stays.
    assert_eq!(root_ids(&s), vec![group_id.as_str(), "n3"]);
    let g = find_node(s.active_children(), &group_id).unwrap();
    assert!(g.is_group());
    assert_eq!(g.children().unwrap().len(), 2);
    assert_eq!(s.selection.anchor, group_id);
}

#[test]
fn group_selected_empty_is_none() {
    let mut s = three_rects();
    s.clear_selection();
    let mut next_id = 1u64;
    assert!(s.group_selected(&mut next_id).is_none());
}

#[test]
fn ungroup_selected_splices_children_inline() {
    let mut s = state_with(vec![
        group(
            "n9",
            "G",
            vec![
                rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
                rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
            ],
        ),
        rect("n3", "C", 0.0, 0.0, 10.0, 10.0),
    ]);
    s.set_single_selection(NodeId::new("n9"));
    assert!(s.ungroup_selected());
    assert_eq!(root_ids(&s), vec!["n1", "n2", "n3"]);
    assert_eq!(s.selection.anchor, NodeId::new("n2"));
}

#[test]
fn ungroup_selected_rejects_non_group() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.ungroup_selected());
}

// --- History ---------------------------------------------------------

#[test]
fn undo_redo_round_trips_a_translate() {
    let mut s = state_with(vec![rect("n1", "A", 10.0, 10.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    s.commit_history();
    s.translate_selected(20.0, 5.0);
    let moved = find_node(s.active_children(), &NodeId::new("n1"))
        .unwrap()
        .base()
        .x
        .unwrap();
    assert_eq!(moved, 30.0);
    assert!(s.undo());
    assert_eq!(
        find_node(s.active_children(), &NodeId::new("n1"))
            .unwrap()
            .base()
            .x
            .unwrap(),
        10.0
    );
    assert!(s.redo());
    assert_eq!(
        find_node(s.active_children(), &NodeId::new("n1"))
            .unwrap()
            .base()
            .x
            .unwrap(),
        30.0
    );
}

#[test]
fn undo_on_empty_history_is_false() {
    let mut s = sample();
    assert!(!s.undo());
    assert!(!s.redo());
}

#[test]
fn history_caps_at_100_entries() {
    let mut s = sample();
    for _ in 0..150 {
        s.commit_history();
    }
    assert_eq!(s.history.past.len(), 100);
}

#[test]
fn commit_history_clears_redo_stack() {
    let mut s = sample();
    s.commit_history();
    assert!(s.undo());
    assert!(s.history.can_redo());
    s.commit_history();
    assert!(!s.history.can_redo());
}

// --- Flag toggles ----------------------------------------------------

#[test]
fn toggle_node_hidden_flips_visible() {
    let mut s = three_rects();
    assert!(s.toggle_node_hidden(&NodeId::new("n1")));
    let n = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(n.base().visible, Some(false));
    assert!(s.toggle_node_hidden(&NodeId::new("n1")));
    let n = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert_eq!(n.base().visible, Some(true));
}

#[test]
fn toggle_node_locked_flips_locked() {
    let mut s = three_rects();
    assert!(s.toggle_node_locked(&NodeId::new("n2")));
    let n = find_node(s.active_children(), &NodeId::new("n2")).unwrap();
    assert_eq!(n.base().locked, Some(true));
}

// --- Fill type (Gap 1) ----------------------------------------------

#[test]
fn set_selected_fill_type_writes_first_fill_variant() {
    let mut s = three_rects();
    s.set_single_selection(NodeId::new("n1"));
    // Default rect has no fills → reports Solid.
    assert_eq!(
        crate::first_fill_type(find_node(s.active_children(), &NodeId::new("n1")).unwrap()),
        crate::FillType::Solid
    );
    assert!(s.set_selected_fill_type(crate::FillType::LinearGradient));
    assert_eq!(
        crate::first_fill_type(find_node(s.active_children(), &NodeId::new("n1")).unwrap()),
        crate::FillType::LinearGradient
    );
    // Flipping again to Image lands too.
    assert!(s.set_selected_fill_type(crate::FillType::Image));
    assert_eq!(
        crate::first_fill_type(find_node(s.active_children(), &NodeId::new("n1")).unwrap()),
        crate::FillType::Image
    );
}

#[test]
fn set_selected_fill_type_no_selection_is_noop() {
    let mut s = three_rects();
    s.clear_selection();
    assert!(!s.set_selected_fill_type(crate::FillType::RadialGradient));
}

#[test]
fn set_selected_fill_type_rejects_locked_node() {
    let mut s = three_rects();
    s.toggle_node_locked(&NodeId::new("n1"));
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.set_selected_fill_type(crate::FillType::LinearGradient));
}

#[test]
fn bind_selected_color_variable_writes_fill_and_stroke_refs() {
    let mut s = three_rects();
    assert!(s.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    s.set_single_selection(NodeId::new("n1"));

    assert!(s.bind_selected_color_variable(crate::ColorTarget::Fill, "color-1"));
    assert_eq!(
        crate::fills::first_solid_fill_hex(
            find_node(s.active_children(), &NodeId::new("n1")).unwrap()
        ),
        Some("$color-1")
    );
    assert_eq!(
        s.ui.variables
            .fill_refs
            .get(&NodeId::new("n1"))
            .map(String::as_str),
        Some("color-1")
    );

    assert!(s.bind_selected_color_variable(crate::ColorTarget::Stroke, "color-1"));
    assert_eq!(
        crate::fills::first_solid_stroke_hex(
            find_node(s.active_children(), &NodeId::new("n1")).unwrap()
        ),
        Some("$color-1")
    );
    assert_eq!(
        s.ui.variables
            .stroke_refs
            .get(&NodeId::new("n1"))
            .map(String::as_str),
        Some("color-1")
    );
}

// --- Chat model (Gap 2) ---------------------------------------------

#[test]
fn select_chat_model_picks_model_and_syncs_agent() {
    let mut s = sample();
    s.chat.available_models = vec![
        crate::ModelEntry::new(crate::AgentProvider::ClaudeCode, "claude", "Claude"),
        crate::ModelEntry::new(crate::AgentProvider::GeminiCli, "gemini", "Gemini"),
    ];
    s.editor_ui.chat_model_picker.open = true;
    s.select_chat_model(1);
    assert_eq!(s.chat.selected_model, 1);
    // GeminiCli is index 4 in AgentProvider::ALL.
    assert_eq!(s.editor_ui.chat_selected_agent, 4);
    // Picker closes on selection.
    assert!(!s.editor_ui.chat_model_picker.open);
}

#[test]
fn select_chat_model_bad_index_still_closes_picker() {
    let mut s = sample();
    s.chat.available_models = vec![crate::ModelEntry::new(
        crate::AgentProvider::ClaudeCode,
        "c",
        "C",
    )];
    s.editor_ui.chat_model_picker.open = true;
    s.select_chat_model(9);
    // Out-of-range index ignored — selected_model unchanged.
    assert_eq!(s.chat.selected_model, 0);
    assert!(!s.editor_ui.chat_model_picker.open);
}

#[test]
fn rebuild_chat_models_syncs_agent_to_selected_model_provider() {
    let mut s = sample();
    s.chat.discovered_models = vec![crate::ModelEntry::new(
        crate::AgentProvider::CodexCli,
        "gpt-5.5",
        "GPT-5.5",
    )];
    s.editor_ui.agent_settings.connected = [false, true, false, false, false];
    s.editor_ui.chat_selected_agent = 0;

    s.rebuild_chat_models();

    assert_eq!(s.chat.selected_model, 0);
    assert_eq!(s.editor_ui.chat_selected_agent, 1);
}

#[test]
fn rebuild_chat_models_keeps_connected_provider_selectable_without_discovery() {
    let mut s = sample();
    s.chat.discovered_models.clear();
    s.editor_ui.agent_settings.connected = [false, true, false, false, false];

    s.rebuild_chat_models();

    assert!(s
        .chat
        .available_models
        .iter()
        .any(|m| m.provider == crate::AgentProvider::CodexCli));
}

#[test]
fn rebuild_chat_models_includes_ready_builtin_agents() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_builtin_agent_with_defaults(
        "Built-in Claude",
        "sk-test",
        "claude-sonnet-4-5",
    );

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("ready built-in agent should appear in model picker");
    assert_eq!(entry.display_name, "claude-sonnet-4-5");
    assert!(entry.value.starts_with("builtin:"));
}

#[test]
fn rebuild_chat_models_retains_builtin_agent_display_name_as_group_label() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_builtin_agent_with_defaults(
        "MiniMax",
        "sk-test",
        "MiniMax-M2.7",
    );

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("ready built-in agent should appear in model picker");
    assert_eq!(entry.display_name, "MiniMax-M2.7");
    assert_eq!(
        entry.builtin_provider_display_name.as_deref(),
        Some("MiniMax")
    );
}

#[test]
fn rebuild_chat_models_includes_connected_acp_agents() {
    let mut s = sample();
    let id = s.editor_ui.agent_settings.add_acp_agent_config(
        "Local ACP",
        crate::AcpConnectionType::Local,
        "op-agent",
        Vec::new(),
        std::collections::BTreeMap::new(),
        None,
        true,
    );
    s.editor_ui.agent_settings.acp_agents[0].connected = true;

    s.rebuild_chat_models();

    let entry = s
        .chat
        .available_models
        .iter()
        .find(|m| m.value == format!("acp:{id}"))
        .expect("connected ACP agent should appear in model picker");
    assert_eq!(entry.display_name, "Local ACP");
}

#[test]
fn select_chat_model_keeps_agent_sync_unchanged_for_acp_models() {
    let mut s = sample();
    s.chat.available_models = vec![
        crate::ModelEntry::new(crate::AgentProvider::ClaudeCode, "claude", "Claude"),
        crate::ModelEntry::new(crate::AgentProvider::CodexCli, "acp:acp-1", "Local ACP"),
    ];
    s.editor_ui.chat_selected_agent = 0;
    s.editor_ui.chat_model_picker.open = true;

    s.select_chat_model(1);

    assert_eq!(s.chat.selected_model, 1);
    assert_eq!(s.editor_ui.chat_selected_agent, 0);
    assert!(!s.editor_ui.chat_model_picker.open);
}

// --- Layer collapse (Gap 3) -----------------------------------------

#[test]
fn toggle_node_collapsed_inserts_then_removes() {
    let mut s = three_rects();
    let id = NodeId::new("n1");
    assert!(!s.is_node_collapsed(&id));
    // First toggle collapses (returns true = now collapsed).
    assert!(s.toggle_node_collapsed(&id));
    assert!(s.is_node_collapsed(&id));
    // Second toggle expands (returns false = now expanded).
    assert!(!s.toggle_node_collapsed(&id));
    assert!(!s.is_node_collapsed(&id));
}

#[test]
fn toggle_node_collapsed_none_id_is_noop() {
    let mut s = three_rects();
    assert!(!s.toggle_node_collapsed(&NodeId::NONE));
    assert!(s.editor_ui.collapsed_layers.is_empty());
}

// --- Panel visibility predicates (Gap 4) ----------------------------

#[test]
fn property_panel_visible_tracks_selection() {
    let mut s = three_rects();
    s.clear_selection();
    assert!(!s.property_panel_visible());
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.property_panel_visible());
    // A selection of an id that does not resolve is not visible.
    s.set_single_selection(NodeId::new("nope"));
    assert!(!s.property_panel_visible());
}

#[test]
fn right_rail_visible_true_on_selection_only() {
    let mut s = three_rects();
    s.clear_selection();
    // No selection → hidden.
    assert!(!s.right_rail_visible());
    // The VariablesPanel is a floating canvas overlay, not a right-rail panel.
    s.editor_ui.variables_panel_open = true;
    assert!(!s.right_rail_visible());
    s.editor_ui.variables_panel_open = false;
    // Selection makes it visible.
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.right_rail_visible());
}

#[test]
fn right_rail_stays_visible_on_code_tab_without_selection() {
    let mut s = three_rects();
    s.clear_selection();
    // Design tab + no selection → hidden (baseline).
    assert!(!s.right_rail_visible());
    // The Code tab is selection-independent (TS falls back to the active
    // page's children), so the rail must stay open with no selection.
    s.editor_ui.property_tab = crate::PropertyTab::Code;
    assert!(s.property_panel_visible());
    assert!(s.right_rail_visible());
    // Back to Design without a selection → hidden again.
    s.editor_ui.property_tab = crate::PropertyTab::Design;
    assert!(!s.right_rail_visible());
}

#[test]
fn validate_catches_duplicate_ids() {
    let s = state_with(vec![
        rect("dup", "A", 0.0, 0.0, 10.0, 10.0),
        rect("dup", "B", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.validate().is_err());
}
