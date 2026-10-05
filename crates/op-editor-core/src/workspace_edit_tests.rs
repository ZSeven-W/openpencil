use super::*;

#[test]
fn unpunctuated_preservation_tail_does_not_erase_a_phone_page_edit() {
    let mut state = normal_work();
    state.editor_ui.workspace.selected = 2;
    state.editor_ui.workspace.stage_page_edit("profile-id", 2);
    assert_eq!(resolve_workspace_edit_scope(&state,"change only the action title on this page to checked tutorial keep everything else unchanged"),WorkspaceEditScope::Target(PageEditTarget {board_id:"profile-id".into(),index:2}));
    state.editor_ui.workspace.clear_staged_page_edit();
    assert_eq!(
        resolve_workspace_edit_scope(&state, "change Home title keep Orders unchanged"),
        WorkspaceEditScope::Target(PageEditTarget {
            board_id: "home-id".into(),
            index: 0
        })
    );
    for text in [
        "do not change Home keep Orders unchanged",
        "change Home title keep Orders unchanged and change Profile title",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::NeedsTarget,
            "{text}"
        );
    }
}

fn normal_work() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    for (id, name) in [
        ("home-id", "Coffee Home"),
        ("orders-id", "Orders"),
        ("profile-id", "Profile"),
    ] {
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "id": id, "type": "frame", "name": name, "width": 390, "height": 844,
                "children": [{"id": format!("{id}-title"), "type": "text", "content": name}]
            }))
            .unwrap(),
        );
    }
    state
        .editor_ui
        .workspace
        .open_for_reading(crate::HomeFamily::AppUi, 1);
    state
}

#[test]
fn homepage_followup_resolves_without_canvas_selection() {
    let state = normal_work();
    assert!(state.selection.is_empty());
    assert_eq!(
        resolve_workspace_edit_scope(
            &state,
            "把首页标题改成“周末，来杯好咖啡”，保留其他页面和布局。"
        ),
        WorkspaceEditScope::Target(PageEditTarget {
            board_id: "home-id".into(),
            index: 0
        })
    );
}

#[test]
fn replacement_copy_does_not_change_intent_or_add_targets() {
    let state = normal_work();
    for text in [
        "把首页标题改成“如何做咖啡”",
        "Change the Home title to \"Create a new page\"",
        "把首页标题改成“Orders”",
        "把“首页”标题改成“如何做咖啡”",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::Target(PageEditTarget {
                board_id: "home-id".into(),
                index: 0
            }),
            "{text}"
        );
    }
}

#[test]
fn numbered_board_names_accept_the_display_name_without_its_ordinal() {
    let mut state = normal_work();
    state.active_children_mut()[1].base_mut().name = Some("02 菜单".into());
    assert_eq!(
        resolve_workspace_edit_scope(&state, "把菜单页标题改成今日精选"),
        WorkspaceEditScope::Target(PageEditTarget {
            board_id: "orders-id".into(),
            index: 1
        })
    );
}

#[test]
fn page_names_numbers_and_current_reader_page_resolve_real_ids() {
    let mut state = normal_work();
    for text in [
        "把第二页标题改为我的订单",
        "Change the Orders title",
        "Change page 2 title",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::Target(PageEditTarget {
                board_id: "orders-id".into(),
                index: 1
            }),
            "{text}"
        );
    }
    state.editor_ui.workspace.selected = 2;
    assert_eq!(
        resolve_workspace_edit_scope(&state, "改这一页的标题"),
        WorkspaceEditScope::Target(PageEditTarget {
            board_id: "profile-id".into(),
            index: 2
        })
    );
}

#[test]
fn questions_new_pages_and_professional_mode_keep_their_routes() {
    let mut state = normal_work();
    for text in [
        "首页是什么字体",
        "怎么把首页标题改成蓝色",
        "重新画一个首页",
        "新增一页，再修改首页标题",
        "Create a new page with a changed title",
        "继续完成 explore/profile界面",
        "你好",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::NotApplicable,
            "{text}"
        );
    }
    state.editor_ui.workspace.visible = false;
    assert_eq!(
        resolve_workspace_edit_scope(&state, "把首页标题改成咖啡"),
        WorkspaceEditScope::NotApplicable
    );
}

#[test]
fn missing_and_ambiguous_targets_never_choose_an_arbitrary_board() {
    let mut state = normal_work();
    for text in [
        "改一下标题",
        "修改不存在的页面",
        "Change Home and Orders titles",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::NeedsTarget,
            "{text}"
        );
    }
    state.active_children_mut()[1].base_mut().name = Some("Alternate Home".into());
    assert_eq!(
        resolve_workspace_edit_scope(&state, "把首页标题改成咖啡"),
        WorkspaceEditScope::NeedsTarget
    );
    state.editor_ui.workspace.stage_page_edit("gone", 0);
    assert_eq!(
        resolve_workspace_edit_scope(&state, "改标题"),
        WorkspaceEditScope::NeedsTarget
    );
}

#[test]
fn protection_clauses_are_excluded_and_all_affirmative_targets_are_checked() {
    let state = normal_work();
    for text in [
        "首页不要改，把第二页标题改成待处理",
        "Do not change Home; change page 2 title to Pending",
        "不要重新生成，把第二页标题改成待处理",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::Target(PageEditTarget {
                board_id: "orders-id".into(),
                index: 1
            }),
            "{text}"
        );
    }
    for text in [
        "把首页标题改成咖啡，把第二页标题改成待处理",
        "Change Home title; change Orders title",
        "首页不要改",
    ] {
        assert_eq!(
            resolve_workspace_edit_scope(&state, text),
            WorkspaceEditScope::NeedsTarget,
            "{text}"
        );
    }
}

#[test]
fn staged_reader_target_is_explicit_and_fenced_against_sibling_changes() {
    let mut state = normal_work();
    state.editor_ui.workspace.stage_page_edit("home-id", 0);
    let WorkspaceEditScope::Target(target) = resolve_workspace_edit_scope(&state, "标题更醒目一点")
    else {
        panic!("reader binding");
    };
    let before = state.active_children().to_vec();
    state.active_children_mut()[0].children_mut().unwrap()[0]
        .base_mut()
        .name = Some("Updated".into());
    state.active_children_mut()[1].base_mut().name = Some("Unwanted change".into());
    assert!(crate::workspace_page_edit::restore_other_boards(
        &mut state,
        &before,
        &target.board_id
    ));
    assert_ne!(state.active_children()[0], before[0]);
    assert_eq!(state.active_children()[1..], before[1..]);
}

#[test]
fn review_only_requests_do_not_enter_modification_or_creation_routes() {
    let state = normal_work();
    for prompt in [
        "请检查这一页的排版，先给出具体建议，不要修改设计。",
        "Review this page. Suggest improvements without editing the design.",
    ] {
        assert!(is_workspace_question(&state, prompt));
        assert_eq!(
            resolve_workspace_edit_scope(&state, prompt),
            WorkspaceEditScope::NotApplicable
        );
    }
    assert!(!is_workspace_question(
        &state,
        "只修改这一页的标题，不要修改其他页面。"
    ));
}

#[test]
fn english_sentences_separate_protection_but_decimal_values_remain_intact() {
    let state = normal_work();
    for prompt in [
        "Only update the main title on this page. Keep the other content, layout, and pages unchanged.",
        "Update this page to make it simpler: remove repeated content and secondary decoration. Keep the other pages unchanged.",
        "Update the colors of this page to a warm, light palette. Keep its layout and the other pages unchanged.",
        "Update this page’s title to a font size of 14.5. Keep other pages unchanged.",
    ] {
        assert_eq!(resolve_workspace_edit_scope(&state,prompt),WorkspaceEditScope::Target(PageEditTarget {board_id:"home-id".into(),index:0}),"{prompt}");
    }
}
