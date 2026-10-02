use super::seed_name;
use op_editor_core::EditorState;

#[test]
fn save_name_keeps_explicit_filename_over_work_title() {
    let mut state = EditorState::starter();
    state.editor_ui.file_name_display = Some("我的咖啡设计.OP".into());
    state.editor_ui.workspace.active = true;
    state.editor_ui.workspace.brief = "做一个咖啡点单 App".into();
    assert_eq!(seed_name(&state), "我的咖啡设计");
}

#[test]
fn first_save_names_the_active_work_from_its_brief_or_template() {
    let mut state = EditorState::starter();
    state.editor_ui.workspace.active = true;
    state.editor_ui.workspace.brief = "做一个咖啡点单 App，包含菜单和购物车".into();
    state.editor_ui.workspace.draft_template = Some("coffee-order-app");
    assert_eq!(seed_name(&state), "咖啡点单 App");
    state.editor_ui.workspace.brief.clear();
    let template =
        op_editor_core::scene_template_catalog::scene_template_by_id("coffee-order-app").unwrap();
    assert_eq!(
        seed_name(&state),
        template.title_for_locale(state.editor_ui.effective_locale())
    );
}

#[test]
fn an_unnamed_existing_work_can_use_its_chat_title() {
    let source = include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");
    let document = jian_ops_schema::load_str(source).unwrap().value;
    let mut state = EditorState::from_document(document);
    assert_eq!(
        seed_name(&state),
        "未命名",
        "the New Chat sentinel is not a filename"
    );
    state.chat.title = "周末/咖啡海报".into();
    assert_eq!(seed_name(&state), "周末 咖啡海报");
}

#[test]
fn blank_work_and_default_chat_titles_keep_the_localized_fallback() {
    let mut state = EditorState::starter();
    assert_eq!(seed_name(&state), "未命名");
    state.editor_ui.home.set_draft("还没有开始的咖啡设计");
    state.chat.title = "关于设计的一般问题".into();
    assert_eq!(
        seed_name(&state),
        "未命名",
        "a chat alone is not a saved work title"
    );
    state.editor_ui.locale = op_editor_core::Locale::EnUs;
    assert_eq!(seed_name(&state), "Untitled");
}
