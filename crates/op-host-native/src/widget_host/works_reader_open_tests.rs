use super::*;

#[test]
fn home_file_picker_preserves_work_and_opens_successful_loads_in_reader_after_a_delay() {
    let mut host = reading_host(HomeFamily::Presentation, 3, 1920, 1080);
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.works_open = true;
    let original = serde_json::to_value(&host.editor_state().doc).unwrap();
    let open = HomeSurface::for_editor(host.editor_state())
        .unwrap()
        .layout(W, H)
        .open_file;
    tap(&mut host, open);
    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(FileAction::Open)
    );
    assert!(host.home_visible());
    assert_eq!(
        serde_json::to_value(&host.editor_state().doc).unwrap(),
        original,
        "the picker may be canceled or fail without replacing the work"
    );
    host.editor_state_mut().editor_ui.pending_file_action = None;
    host.set_now_ms(121_000);
    assert!(host.replace_editor_state(phone_state(boards_document(3, 1080, 1440))));
    assert!(
        host.works_reader_visible(),
        "a slow picker still opens in normal mode"
    );
    assert!(!host.home_visible());
    assert_eq!(host.editor_state().editor_ui.workspace.selected, 0);
}

#[test]
fn an_explicit_professional_choice_clears_canceled_picker_reading_intent() {
    let mut host = reading_host(HomeFamily::Presentation, 2, 1920, 1080);
    host.editor_state_mut().editor_ui.home.visible = true;
    host.editor_state_mut().editor_ui.home.works_open = true;
    let home = HomeSurface::for_editor(host.editor_state()).unwrap();
    let open = home.layout(W, H).open_file;
    let professional = home.layout(W, H).professional;
    tap(&mut host, open);
    host.editor_state_mut().editor_ui.pending_file_action = None;
    tap(&mut host, professional);
    assert!(!host.reader_open_from_picker);
    assert!(host.replace_editor_state(phone_state(boards_document(1, 390, 844))));
    assert!(!host.works_reader_visible());
}
