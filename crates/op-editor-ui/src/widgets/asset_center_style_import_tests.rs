//! The Styles tab's `DESIGN.md` import — the box, both routes into it, and
//! removing what they added.
//!
//! Split from `asset_center_tab_tests` at the file-size cap. The two halves
//! are the two things the Styles tab does: show a grid of guides (there) and
//! get one in or out of it (here).

use super::asset_center_style_cards::style_test_support::exclusive_user_styles;
use super::scene_template_panel::test_rects::MEDIUM as PANEL;
use super::scene_template_panel::SceneTemplatePanel;
use super::SceneTemplateHit;
use crate::widgets::press_flow;
use crate::{Point2D, Rect};
use op_editor_core::{AssetCenterTab, EditorState};
use std::sync::MutexGuard;

/// The Styles tab, with the process-global imported-style registry emptied
/// and held for the duration of the test. See the twin in
/// `asset_center_tab_tests` for why the guard is returned rather than dropped.
fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.open_scene_template_center(0);
    state
}

fn styles_state() -> (MutexGuard<'static, ()>, EditorState) {
    let guard = exclusive_user_styles();
    let mut state = open_state();
    state.editor_ui.scene_template_center.tab = AssetCenterTab::Styles;
    (guard, state)
}

fn press(state: &mut EditorState, point: Point2D) -> Option<bool> {
    crate::widgets::scene_template_press_flow::press_scene_template_center(state, PANEL, point, 0)
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

const SAMPLE_DESIGN_MD: &str =
    "---\nname: Studio Ochre\n---\n\n# Studio Ochre\n\nWarm ochre, no shadows. Accent #C77D3A.\n";

fn press_import_button(state: &mut EditorState) {
    let point = centre(
        SceneTemplatePanel::for_editor(state)
            .expect("open")
            .style_import_button_rect(PANEL)
            .expect("the Styles tab offers an import button"),
    );
    press(state, point);
}

/// The button is a Styles-tab control. Painting it on the Templates tab would
/// offer to import a style into a gallery of documents.
#[test]
fn the_import_button_belongs_to_the_styles_tab_only() {
    let (_guard, state) = styles_state();
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let rect = panel.style_import_button_rect(PANEL).expect("styles");
    // It shares the tab row with the chips and must not sit on top of one.
    for (chip, _) in panel.tab_chip_rects(PANEL) {
        assert!(
            chip.origin.x + chip.size.x <= rect.origin.x,
            "the import button overlaps a tab chip"
        );
    }
    assert_eq!(
        panel.hit_test(PANEL, centre(rect)),
        Some(SceneTemplateHit::ImportStyleGuide)
    );

    let templates = open_state();
    assert!(SceneTemplatePanel::for_editor(&templates)
        .expect("open")
        .style_import_button_rect(PANEL)
        .is_none());
}

/// The import button opens the box on every host, including one with a file
/// dialog. It used to fork — a host with a dialog went straight to it and
/// never showed the box — which left each platform with exactly one of the two
/// ways in and no way to reach the other.
#[test]
fn the_import_button_opens_the_box_whether_or_not_the_host_has_a_file_dialog() {
    for supported in [false, true] {
        let (_guard, mut state) = styles_state();
        state.editor_ui.style_import_file_picker_supported = supported;
        press_import_button(&mut state);

        assert!(
            state.editor_ui.scene_template_center.import.open,
            "picker supported = {supported}"
        );
        assert_eq!(
            state.editor_ui.scene_template_center.focus,
            op_editor_core::SceneTemplateFocus::Import
        );
        // The button itself never asks for a file — the one inside the box does.
        assert!(!state
            .editor_ui
            .scene_template_center
            .take_pending_style_import_file());
    }
}

/// The "choose file" button paints only where a host can open a dialog: a
/// visible-but-inert control is a worse answer to "can I import a file here"
/// than no control at all.
#[test]
fn the_choose_file_button_exists_only_on_a_host_with_a_dialog() {
    let (_guard, mut state) = styles_state();
    press_import_button(&mut state);
    assert!(SceneTemplatePanel::for_editor(&state)
        .expect("open")
        .style_import_pick_rect(PANEL)
        .is_none());

    state.editor_ui.style_import_file_picker_supported = true;
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let pick = panel.style_import_pick_rect(PANEL).expect("choose file");
    // It shares the button row and must not sit on top of either control there.
    assert!(
        pick.origin.x + pick.size.x <= panel.style_import_cancel_rect(PANEL).origin.x,
        "the choose-file button overlaps cancel"
    );
    assert_eq!(
        panel.hit_test(PANEL, centre(pick)),
        Some(SceneTemplateHit::PickStyleImportFile)
    );
}

#[test]
fn pressing_choose_file_asks_the_host_and_leaves_the_box_up() {
    let (_guard, mut state) = styles_state();
    state.editor_ui.style_import_file_picker_supported = true;
    press_import_button(&mut state);
    let pick = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_pick_rect(PANEL)
            .expect("choose file"),
    );
    press(&mut state, pick);

    // The box stays up behind the dialog: a cancelled pick returns the user to
    // what they were doing, and a failed one has somewhere to report itself.
    assert!(state.editor_ui.scene_template_center.import.open);
    assert!(state
        .editor_ui
        .scene_template_center
        .take_pending_style_import_file());
    // Drained once, not forever.
    assert!(!state
        .editor_ui
        .scene_template_center
        .take_pending_style_import_file());
}

/// **One pipeline.** A host reads bytes and hands them over; everything that
/// decides what an imported guide *is* happens in the shared flow the paste
/// box also runs. Both routes are driven here against the same document, and
/// the only difference allowed between them is the fallback name a file
/// contributes when the document names nothing.
#[test]
fn a_picked_file_and_a_paste_of_the_same_document_produce_the_same_guide() {
    let pasted = {
        let (_guard, mut state) = styles_state();
        press_import_button(&mut state);
        state
            .editor_ui
            .scene_template_center
            .import
            .text
            .set_text(SAMPLE_DESIGN_MD);
        let confirm = centre(
            SceneTemplatePanel::for_editor(&state)
                .expect("open")
                .style_import_confirm_rect(PANEL),
        );
        press(&mut state, confirm);
        import_outcome(&mut state)
    };

    let picked = {
        let (_guard, mut state) = styles_state();
        state.editor_ui.style_import_file_picker_supported = true;
        press_import_button(&mut state);
        let pick = centre(
            SceneTemplatePanel::for_editor(&state)
                .expect("open")
                .style_import_pick_rect(PANEL)
                .expect("choose file"),
        );
        press(&mut state, pick);
        assert!(state
            .editor_ui
            .scene_template_center
            .take_pending_style_import_file());
        // What the host does with the bytes it read — the whole of its job.
        press_flow::import_style_guide_text(&mut state, SAMPLE_DESIGN_MD, "studio-ochre");
        import_outcome(&mut state)
    };

    assert_eq!(picked, pasted);
    assert_eq!(picked.0.as_deref(), Some("user:studio-ochre"));
    assert_eq!(picked.1, vec!["user:studio-ochre".to_string()]);
    assert!(!picked.2, "a successful import closes the box");
}

/// A file that could not be read as text reports in the box the user is
/// looking at, and leaves it open so they can still paste.
#[test]
fn an_unreadable_file_reports_without_closing_the_box() {
    let (_guard, mut state) = styles_state();
    state.editor_ui.style_import_file_picker_supported = true;
    press_import_button(&mut state);
    press_flow::fail_style_import_unreadable(&mut state);

    assert!(state.editor_ui.scene_template_center.import.open);
    assert_eq!(
        state.editor_ui.scene_template_center.import.error_key,
        Some("assetCenter.style.importNotText")
    );
    assert!(state.editor_ui.pinned_style_guide.is_none());
    assert!(op_ai_skills::style_guide::user_style_guides().is_empty());
}

/// What an import left behind: the pin, the persist queue, and whether the box
/// is still up. Drains the queue, so call it once per state.
fn import_outcome(state: &mut EditorState) -> (Option<String>, Vec<String>, bool) {
    (
        state.editor_ui.pinned_style_guide.clone(),
        state
            .editor_ui
            .scene_template_center
            .take_pending_style_persist(),
        state.editor_ui.scene_template_center.import.open,
    )
}

#[test]
fn confirming_a_paste_registers_pins_and_queues_the_guide() {
    let (_guard, mut state) = styles_state();
    press_import_button(&mut state);
    state
        .editor_ui
        .scene_template_center
        .import
        .text
        .set_text(SAMPLE_DESIGN_MD);

    let confirm = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_confirm_rect(PANEL),
    );
    press(&mut state, confirm);

    assert!(!state.editor_ui.scene_template_center.import.open);
    // Pinned on arrival: the user went and found this guide, and leaving it
    // unpinned would make the next generation ignore it.
    assert_eq!(
        state.editor_ui.pinned_style_guide.as_deref(),
        Some("user:studio-ochre")
    );
    assert_eq!(
        state
            .editor_ui
            .scene_template_center
            .take_pending_style_persist(),
        vec!["user:studio-ochre".to_string()]
    );

    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let cards = panel.style_cards();
    assert!(cards[0].is_user);
    assert!(cards[0].is_pinned(panel.pinned_style_guide()));
}

#[test]
fn touch_confirming_a_paste_returns_to_browsing_without_reopening_ime() {
    let (_guard, mut state) = styles_state();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
    press_import_button(&mut state);
    assert!(state.editor_ui.scene_template_center.input_active());
    state
        .editor_ui
        .scene_template_center
        .import
        .text
        .set_text(SAMPLE_DESIGN_MD);
    let confirm = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_confirm_rect(PANEL),
    );
    press(&mut state, confirm);

    assert!(!state.editor_ui.scene_template_center.import.open);
    assert!(!state.editor_ui.scene_template_center.input_active());
}

/// A malformed file reports and keeps the box open with the text intact —
/// half-swallowing it would look like the import worked.
#[test]
fn a_malformed_paste_reports_without_closing_the_box() {
    let (_guard, mut state) = styles_state();
    press_import_button(&mut state);
    state
        .editor_ui
        .scene_template_center
        .import
        .text
        .set_text("   ");

    let confirm = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_confirm_rect(PANEL),
    );
    press(&mut state, confirm);

    assert!(state.editor_ui.scene_template_center.import.open);
    assert_eq!(
        state.editor_ui.scene_template_center.import.error_key,
        Some("assetCenter.style.importEmpty")
    );
    assert!(state.editor_ui.pinned_style_guide.is_none());
    assert!(op_ai_skills::style_guide::user_style_guides().is_empty());
}

#[test]
fn cancelling_discards_the_draft_and_hands_the_keyboard_back() {
    let (_guard, mut state) = styles_state();
    press_import_button(&mut state);
    state
        .editor_ui
        .scene_template_center
        .import
        .text
        .set_text(SAMPLE_DESIGN_MD);

    let cancel = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_cancel_rect(PANEL),
    );
    press(&mut state, cancel);

    assert!(!state.editor_ui.scene_template_center.import.open);
    assert!(state
        .editor_ui
        .scene_template_center
        .import
        .text
        .text()
        .is_empty());
    assert_eq!(
        state.editor_ui.scene_template_center.focus,
        op_editor_core::SceneTemplateFocus::Search
    );
    assert!(op_ai_skills::style_guide::user_style_guides().is_empty());
}

#[test]
fn touch_cancelling_an_import_returns_to_browsing_without_ime() {
    let (_guard, mut state) = styles_state();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
    press_import_button(&mut state);
    assert!(state.editor_ui.scene_template_center.input_active());
    let cancel = centre(
        SceneTemplatePanel::for_editor(&state)
            .expect("open")
            .style_import_cancel_rect(PANEL),
    );
    press(&mut state, cancel);

    assert!(!state.editor_ui.scene_template_center.import.open);
    assert!(!state.editor_ui.scene_template_center.input_active());
}

/// Escape takes the paste box back before it takes the gallery: it is the
/// topmost layer, and closing the whole panel would throw away the paste.
#[test]
fn escape_dismisses_the_paste_box_before_the_gallery() {
    let (_guard, mut state) = styles_state();
    press_import_button(&mut state);

    assert!(state.editor_ui.escape_scene_template_center());
    assert!(!state.editor_ui.scene_template_center.import.open);
    assert!(
        state.editor_ui.scene_template_center.open,
        "the gallery survives the first Escape"
    );
    assert!(state.editor_ui.escape_scene_template_center());
    assert!(!state.editor_ui.scene_template_center.open);
}

#[test]
fn the_delete_button_appears_on_hover_and_only_on_imports() {
    let (_guard, mut state) = styles_state();
    op_ai_skills::style_guide::import_design_md(SAMPLE_DESIGN_MD, "x").expect("imports");

    let (import_rect, corpus_rect) = {
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let cards = panel.style_cards();
        let layout = panel.style_layout_for(PANEL, &cards);
        (layout.cards[0].1, layout.cards[1].1)
    };

    // Unhovered, the ✕ is not there and the press pins.
    let delete_point = centre(SceneTemplatePanel::style_delete_rect(import_rect));
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert_eq!(
        panel.hit_test(PANEL, delete_point),
        Some(SceneTemplateHit::ToggleStyleGuide(
            "user:studio-ochre".to_string()
        ))
    );

    // Hovering the card arms it.
    state.editor_ui.scene_template_center.hover = Some(0);
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    assert_eq!(
        panel.hit_test(PANEL, delete_point),
        Some(SceneTemplateHit::DeleteStyleGuide(
            "user:studio-ochre".to_string()
        ))
    );
    assert_eq!(
        panel.hover_at(PANEL, delete_point),
        Some(SceneTemplatePanel::style_delete_hover_token(0))
    );

    // A shipped guide has no delete target at all — pressing its corner pins
    // it, because the corpus is not the user's to remove.
    state.editor_ui.scene_template_center.hover = Some(1);
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let corpus_corner = centre(SceneTemplatePanel::style_delete_rect(corpus_rect));
    assert!(matches!(
        panel.hit_test(PANEL, corpus_corner),
        Some(SceneTemplateHit::ToggleStyleGuide(id)) if !id.starts_with("user:")
    ));
}

#[test]
fn deleting_an_import_unpins_it_and_queues_the_file_removal() {
    let (_guard, mut state) = styles_state();
    op_ai_skills::style_guide::import_design_md(SAMPLE_DESIGN_MD, "x").expect("imports");
    state.editor_ui.pinned_style_guide = Some("user:studio-ochre".to_string());
    state.editor_ui.scene_template_center.hover = Some(0);

    let delete_point = {
        let panel = SceneTemplatePanel::for_editor(&state).expect("open");
        let cards = panel.style_cards();
        centre(SceneTemplatePanel::style_delete_rect(
            panel.style_layout_for(PANEL, &cards).cards[0].1,
        ))
    };
    press(&mut state, delete_point);

    assert!(op_ai_skills::style_guide::user_style_guides().is_empty());
    // A pin left pointing at a deleted guide would have the Asset Center
    // claiming a style is in force that is no longer in the list.
    assert!(state.editor_ui.pinned_style_guide.is_none());
    assert_eq!(
        state
            .editor_ui
            .scene_template_center
            .take_pending_style_delete(),
        vec!["user:studio-ochre".to_string()]
    );
    assert!(state.editor_ui.scene_template_center.hover.is_none());
}

/// Section headings are the whole reason the Styles grid has its own walker:
/// they push the cards below them down, and a scroll limit that ignored them
/// would clip the last row off.
#[test]
fn imports_get_their_own_section_and_lengthen_the_grid() {
    let (_guard, mut state) = styles_state();
    state.editor_ui.scene_template_generate_supported = true;
    let flat_height = SceneTemplatePanel::for_editor(&state)
        .expect("open")
        .max_scroll(PANEL);

    op_ai_skills::style_guide::import_design_md(SAMPLE_DESIGN_MD, "x").expect("imports");
    let panel = SceneTemplatePanel::for_editor(&state).expect("open");
    let layout = panel.style_layout_for(PANEL, &panel.style_cards());
    assert_eq!(layout.headers.len(), 2);
    assert!(layout.headers[0].is_user);
    assert!(panel.max_scroll(PANEL) > flat_height);
}
