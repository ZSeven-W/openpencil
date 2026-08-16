//! Flow tests for the left rail's slides tab: which documents get the
//! tab row, tab switching, click-to-navigate vs drag-to-reorder, and
//! the scroll seam.

use super::*;
use crate::widgets::slides_panel::DRAG_THRESHOLD_PX;
use crate::widgets::slides_panel_actions::ACTION_BAR_HEIGHT;
use op_editor_core::scene_template_catalog::TemplateScene;

const THREE_BOARDS: &str = r#"{"version":"1.0.0","children":[
    {"type":"frame","id":"slide-1","name":"Cover","x":0,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-2","name":"Agenda","x":2100,"y":0,"width":1920,"height":1080},
    {"type":"frame","id":"slide-3","name":"封面之后的一页","x":4200,"y":0,"width":1920,"height":1080}
]}"#;

const PANEL: Rect = Rect {
    origin: Point2D { x: 0.0, y: 48.0 },
    size: Point2D { x: 240.0, y: 700.0 },
};

fn deck_state(source: &str) -> EditorState {
    let document = jian_ops_schema::load_str(source)
        .expect("fixture parses")
        .value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    state
}

/// The three boards as the layout pass resolves them.
fn scene() -> LayoutScene {
    use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
    let board = |id: &str, x: f32| {
        let mut node = SceneNode::leaf(id, NodeKind::Frame);
        node.bounds = Rect::xywh(x, 0.0, 1920.0, 1080.0);
        node
    };
    LayoutScene {
        pages: vec![ScenePage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![
                board("slide-1", 0.0),
                board("slide-2", 2100.0),
                board("slide-3", 4200.0),
            ],
        }],
        active_page_index: 0,
    }
}

fn laid_out(state: &EditorState) -> (Vec<BoardChip>, SlidesPanelLayout) {
    let chips = slides(state);
    let layout = layout(state, &chips, &scene(), PANEL).expect("the rail has room");
    (chips, layout)
}

fn row_centre(layout: &SlidesPanelLayout, index: usize) -> Point2D {
    let rect = layout.row_rect(index);
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn any_document_with_boards_gets_a_tab_row() {
    let deck = deck_state(THREE_BOARDS);
    assert!(tab_row_visible(&deck));
    assert_eq!(slides_tab_label_key(&deck), "slidesPanel.tabSlides");

    // An untagged design page with frames on it is exactly the case the
    // navigator used to be missing from.
    let mut ordinary = deck_state(THREE_BOARDS);
    ordinary.editor_ui.scenario = None;
    assert!(tab_row_visible(&ordinary));
    assert!(tab_row(&ordinary, PANEL).is_some());
    assert_eq!(slides_tab_label_key(&ordinary), "slidesPanel.tabSlides");

    let mut carousel = deck_state(THREE_BOARDS);
    carousel.editor_ui.scenario = Some(TemplateScene::Carousel);
    assert!(tab_row_visible(&carousel));
}

#[test]
fn the_scenario_names_the_tab_without_gating_it() {
    let mut cards = deck_state(THREE_BOARDS);
    cards.editor_ui.scenario = Some(TemplateScene::Card);
    assert!(tab_row_visible(&cards));
    assert_eq!(slides_tab_label_key(&cards), "slidesPanel.tabCards");
}

#[test]
fn presenting_and_empty_decks_show_no_tab_row() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.enter_preview();
    assert!(!tab_row_visible(&deck), "the rail is gone while presenting");
    deck.editor_ui.exit_preview();
    assert!(tab_row_visible(&deck));

    let empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    assert!(!tab_row_visible(&empty));
}

#[test]
fn the_layers_tree_keeps_the_whole_rail_without_a_tab_row() {
    let empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    assert_eq!(layers_content_rect(&empty, PANEL), PANEL);

    let deck = deck_state(THREE_BOARDS);
    let content = layers_content_rect(&deck, PANEL);
    assert_eq!(
        content.origin.y,
        PANEL.origin.y + crate::widgets::slides_panel::SLIDES_TAB_ROW_HEIGHT
    );
}

#[test]
fn touch_tabs_have_44_point_targets_and_shift_content_together() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.touch = true;
    let tabs = tab_row(&deck, PANEL).expect("deck has touch tabs");
    assert_eq!(tabs.row.size.y, 52.0);
    assert_eq!(tabs.layers.size.y, 44.0);
    assert_eq!(tabs.slides.size.y, 44.0);
    assert_eq!(
        layers_content_rect(&deck, PANEL).origin.y,
        PANEL.origin.y + 52.0
    );
}

#[test]
fn a_stale_slides_tab_cannot_strand_a_document_with_nothing_to_list() {
    let mut empty = deck_state(r#"{"version":"1.0.0","children":[]}"#);
    empty.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    assert!(!slides_tab_active(&empty));
    assert!(layout(&empty, &slides(&empty), &scene(), PANEL).is_none());
}

#[test]
fn the_list_carries_the_documents_board_order_and_names() {
    let deck = deck_state(THREE_BOARDS);
    let chips = slides(&deck);
    assert_eq!(
        chips.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["slide-1", "slide-2", "slide-3"]
    );
    assert_eq!(chips[2].name, "封面之后的一页");
}

#[test]
fn every_board_reports_its_own_aspect() {
    let deck = deck_state(THREE_BOARDS);
    let chips = slides(&deck);
    let aspects = board_aspects(&chips, &scene());
    assert_eq!(aspects.len(), chips.len(), "one aspect per board");
    assert!(aspects.iter().all(|a| (a - 1920.0 / 1080.0).abs() < 0.001));
    // No scene yet (a freshly opened document) falls back to 16:9 for
    // every row rather than collapsing the list.
    let unresolved = board_aspects(&chips, &LayoutScene::default());
    assert_eq!(unresolved.len(), chips.len());
    assert!(unresolved
        .iter()
        .all(|a| (a - DEFAULT_BOARD_ASPECT).abs() < 0.001));
}

#[test]
fn clicking_a_row_activates_that_slide() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let point = row_centre(&layout, 1);
    assert_eq!(
        press(&mut deck, &layout, point),
        SlidesPress::Claimed(Some(SlidesPanelTarget::Slide(1)))
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Activate(1));
    assert_eq!(deck.editor_ui.slides_panel.pressed, None);
}

#[test]
fn a_press_off_the_rail_is_not_the_panels() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    assert_eq!(
        press(&mut deck, &layout, Point2D::new(900.0, 400.0)),
        SlidesPress::Missed
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Idle);
}

#[test]
fn releasing_off_the_pressed_row_cancels() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    press(&mut deck, &layout, row_centre(&layout, 0));
    // The cursor wandered onto a different row without travelling far
    // enough to be a drag: neither slide activates.
    deck.editor_ui.slides_panel.hover = Some(SlidesPanelTarget::Slide(2));
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Cancelled);
}

#[test]
fn dragging_a_row_past_a_neighbour_reorders() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    press(&mut deck, &layout, row_centre(&layout, 0));
    let target = row_centre(&layout, 2);
    assert!(cursor_move(&mut deck, &layout, target), "the drag moved");
    assert_eq!(
        release(&mut deck, &layout),
        SlidesRelease::Reorder { from: 0, to: 1 },
        "dropping on row 2's lower half lands after row 1"
    );
}

#[test]
fn a_drag_dropped_where_it_started_changes_nothing() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let start = row_centre(&layout, 1);
    press(&mut deck, &layout, start);
    cursor_move(
        &mut deck,
        &layout,
        Point2D::new(start.x, start.y + DRAG_THRESHOLD_PX + 1.0),
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Cancelled);
}

#[test]
fn the_reorder_is_the_shared_deck_move_command() {
    // The panel commits through the shared deck reorder, so the two
    // navigators can never write the deck order differently.
    let mut deck = deck_state(THREE_BOARDS);
    let before: Vec<String> = slides(&deck).into_iter().map(|c| c.id).collect();
    assert!(crate::widgets::deck_boards::apply_reorder(
        &mut deck, "slide-1", 2
    ));
    let after: Vec<String> = slides(&deck).into_iter().map(|c| c.id).collect();
    assert_eq!(before, ["slide-1", "slide-2", "slide-3"]);
    assert_eq!(after, ["slide-2", "slide-3", "slide-1"]);
}

#[test]
fn the_tabs_switch_and_dropping_the_slides_tab_drops_its_gesture() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let tabs = tab_row(&deck, PANEL).expect("a deck has tabs");
    let layers_tab = Point2D::new(
        tabs.layers.origin.x + tabs.layers.size.x / 2.0,
        tabs.layers.origin.y + tabs.layers.size.y / 2.0,
    );
    press(&mut deck, &layout, layers_tab);
    assert_eq!(
        release(&mut deck, &layout),
        SlidesRelease::SelectTab(LeftPanelTab::Layers)
    );

    deck.editor_ui.slides_panel.hover = Some(SlidesPanelTarget::Slide(1));
    assert!(select_tab(&mut deck, LeftPanelTab::Layers));
    assert_eq!(deck.editor_ui.slides_panel.tab, LeftPanelTab::Layers);
    assert_eq!(
        deck.editor_ui.slides_panel.hover, None,
        "a hover belonging to a hidden list does not survive"
    );
    assert!(
        !select_tab(&mut deck, LeftPanelTab::Layers),
        "re-selecting the shown tab is not a change"
    );
}

#[test]
fn the_tab_row_takes_clicks_while_the_layers_tab_owns_the_rail() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let tabs = tab_row(&deck, PANEL).expect("a deck has tabs");
    let slides_tab = Point2D::new(
        tabs.slides.origin.x + tabs.slides.size.x / 2.0,
        tabs.slides.origin.y + tabs.slides.size.y / 2.0,
    );
    assert!(tab_cursor_move(&mut deck, &tabs, slides_tab));
    assert_eq!(
        deck.editor_ui.slides_panel.hover,
        Some(SlidesPanelTarget::SlidesTab)
    );
    deck.editor_ui.slides_panel.pressed = Some(SlidesPanelTarget::SlidesTab);
    assert_eq!(
        tab_release(&mut deck),
        SlidesRelease::SelectTab(LeftPanelTab::Slides)
    );
}

#[test]
fn the_footer_button_asks_to_present() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, layout) = laid_out(&deck);
    let point = Point2D::new(
        layout.actions.present.origin.x + layout.actions.present.size.x / 2.0,
        layout.actions.present.origin.y + layout.actions.present.size.y / 2.0,
    );
    assert_eq!(
        press(&mut deck, &layout, point),
        SlidesPress::Claimed(Some(SlidesPanelTarget::Present))
    );
    assert_eq!(release(&mut deck, &layout), SlidesRelease::Present);
}

#[test]
fn the_wheel_scrolls_the_list_only_over_the_rail() {
    let mut deck = deck_state(THREE_BOARDS);
    let tall = SlidesPanelLayout::new(
        PANEL,
        SlidesPanelTabs::new(PANEL, LeftPanelTab::Slides, "Layers", "Slides"),
        &[DEFAULT_BOARD_ASPECT; 20],
        0.0,
        crate::widgets::SlidesActionState::default(),
    )
    .expect("layout");
    let over = Point2D::new(120.0, 300.0);
    assert_eq!(scroll(&mut deck, Some(&tall), over, -120.0), Some(true));
    assert!(deck.editor_ui.slides_panel.scroll.offset > 0.0);
    assert_eq!(
        scroll(&mut deck, Some(&tall), Point2D::new(900.0, 300.0), -120.0),
        None,
        "off the rail the wheel belongs to the canvas"
    );
}

/// The tab row's mode has to reach the product through the LOCALE, not
/// just through a hand-passed label: the flow is the only place either
/// host resolves the pair from, so this is where "Vietnamese at 180 px
/// shows icons" is either true or a lie the unit tests cannot catch.
#[test]
fn the_tab_row_mode_follows_the_documents_own_labels() {
    let mut deck = deck_state(THREE_BOARDS);
    let narrow = Rect {
        origin: PANEL.origin,
        size: Point2D::new(180.0, PANEL.size.y),
    };

    deck.editor_ui.locale = op_editor_core::Locale::EnUs;
    let (layers, slides) = tab_labels(&deck);
    assert_eq!((layers, slides), ("Layers", "Slides"));
    assert!(
        !tab_row(&deck, narrow).expect("tab row").compact,
        "English fits the minimum rail, so it keeps its words"
    );

    deck.editor_ui.locale = op_editor_core::Locale::Vi;
    let (layers, slides) = tab_labels(&deck);
    assert!(
        !layers.is_empty() && !slides.is_empty(),
        "the Vietnamese catalogue answers for both tabs"
    );
    assert!(
        tab_row(&deck, narrow).expect("tab row").compact,
        "Vietnamese does not fit the minimum rail, so it falls back to icons"
    );
    assert!(
        !tab_row(&deck, PANEL).expect("tab row").compact,
        "and gets its words back at the default width"
    );
}

/// The scenario still only picks the WORD, and the word is what the
/// row is measured against — so a scenario rename can flip the mode.
#[test]
fn the_scenario_label_is_the_one_the_row_is_measured_against() {
    let mut cards = deck_state(THREE_BOARDS);
    cards.editor_ui.scenario = Some(TemplateScene::Card);
    let (_, slides) = tab_labels(&cards);
    assert_eq!(
        slides,
        crate::widgets::editor_state_ext::translate(&cards.editor_ui, "slidesPanel.tabCards")
    );
}

// ─── The bottom action bar and its export dropdown ─────────────────────

/// Select boards by id, the way a marquee or a shift-click on the canvas
/// would leave the selection.
fn select_boards(state: &mut EditorState, ids: &[&str]) {
    state.selection.set = ids.iter().map(|id| NodeId::new(id.to_string())).collect();
    state.selection.anchor = state.selection.set.last().cloned().unwrap_or(NodeId::NONE);
}

fn press_release(
    state: &mut EditorState,
    layout: &SlidesPanelLayout,
    point: Point2D,
) -> SlidesRelease {
    assert!(
        matches!(press(state, layout, point), SlidesPress::Claimed(_)),
        "the panel claims a press at {point:?}"
    );
    cursor_move(state, layout, point);
    release(state, layout)
}

fn button_centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// A deck of `count` 16:9 boards, with the scene that resolves them —
/// enough rows to fill the rail top to bottom, which the three-board
/// fixture is deliberately too short for.
fn long_deck(count: usize) -> (String, LayoutScene) {
    use crate::layout_scene::{NodeKind, SceneNode, ScenePage};
    let mut json = String::from(r#"{"version":"1.0.0","children":["#);
    let mut children = Vec::new();
    for index in 0..count {
        let id = format!("slide-{}", index + 1);
        let x = index as f32 * 2100.0;
        if index > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            r#"{{"type":"frame","id":"{id}","name":"Slide {}","x":{x},"y":0,"width":1920,"height":1080}}"#,
            index + 1
        ));
        let mut node = SceneNode::leaf(&id, NodeKind::Frame);
        node.bounds = Rect::xywh(x, 0.0, 1920.0, 1080.0);
        children.push(node);
    }
    json.push_str("]}");
    (
        json,
        LayoutScene {
            pages: vec![ScenePage {
                id: "page-1".into(),
                name: "Page 1".into(),
                children,
            }],
            active_page_index: 0,
        },
    )
}

/// **The action bar belongs to the slides tab alone.** Its rects live
/// only inside `SlidesPanelLayout`, which the Layers tab never builds —
/// so there is nowhere for a stray bar to be painted or hit-tested from
/// while the tree owns the rail.
#[test]
fn the_layers_tab_gets_no_action_bar() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.tab = LeftPanelTab::Layers;
    let chips = slides(&deck);
    assert!(
        layout(&deck, &chips, &scene(), PANEL).is_none(),
        "no slides layout under the Layers tab means no bar and no menu"
    );
    // The tab row is still there — the rail keeps working, it just has
    // no Present / Export on it.
    assert!(tab_row(&deck, PANEL).is_some());
    // And the rail below the tab row is the tree's in full: nothing has
    // been reserved at the bottom for a bar that is not being painted.
    let content = layers_content_rect(&deck, PANEL);
    assert_eq!(
        content.origin.y + content.size.y,
        PANEL.origin.y + PANEL.size.y
    );

    deck.editor_ui.slides_panel.tab = LeftPanelTab::Slides;
    let (_, slides_layout) = laid_out(&deck);
    assert_eq!(slides_layout.actions.bar.size.y, ACTION_BAR_HEIGHT);
}

/// Clicking the export button opens the dropdown; clicking it again
/// closes it, rather than re-toggling it open on the dismiss.
#[test]
fn the_export_button_toggles_its_dropdown() {
    let mut deck = deck_state(THREE_BOARDS);
    let (_, l) = laid_out(&deck);
    let button = button_centre(l.actions.export);

    assert_eq!(
        press_release(&mut deck, &l, button),
        SlidesRelease::ToggleExportMenu
    );
    assert!(deck.editor_ui.slides_panel.export_menu_open);

    // Re-lay out: the menu's rects only exist once it is open.
    let (_, open) = laid_out(&deck);
    assert!(open.actions.menu.is_some());
    assert_eq!(
        press_release(&mut deck, &open, button),
        SlidesRelease::ToggleExportMenu
    );
    assert!(
        !deck.editor_ui.slides_panel.export_menu_open,
        "a second click on the anchor closes rather than reopening"
    );
}

/// A press that is neither a row nor the menu's chrome dismisses it, and
/// is swallowed — the click that closes a menu never also does something
/// underneath it.
#[test]
fn a_press_outside_the_open_menu_dismisses_it_and_is_swallowed() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.export_menu_open = true;
    let (_, l) = laid_out(&deck);
    let menu = l.actions.menu.expect("open");

    // A slide row well above the menu.
    let outside = Point2D::new(120.0, menu.origin.y - 40.0);
    assert!(
        !l.actions.over_menu(outside),
        "the fixture point is genuinely off the menu"
    );
    assert_eq!(press(&mut deck, &l, outside), SlidesPress::Claimed(None));
    assert!(!deck.editor_ui.slides_panel.export_menu_open);
    assert_eq!(
        release(&mut deck, &l),
        SlidesRelease::Idle,
        "the dismiss armed nothing, so its release activates nothing"
    );
}

/// The menu covers the thumbnails it grew into, so a press on its
/// PADDING must stop there rather than reaching the row underneath.
#[test]
fn the_menus_chrome_swallows_presses_meant_for_the_rows_it_covers() {
    // A deck long enough that its rows reach the bottom of the rail —
    // three boards end well above the menu, so the covering the test is
    // about would never happen.
    let (source, scene) = long_deck(8);
    let mut deck = deck_state(&source);
    deck.editor_ui.slides_panel.export_menu_open = true;
    let chips = slides(&deck);
    let l = layout(&deck, &chips, &scene, PANEL).expect("the rail has room");
    let menu = l.actions.menu.expect("open");
    let padding = Point2D::new(menu.origin.x + 4.0, menu.origin.y + 2.0);

    // There really is a slide row under that point — otherwise the test
    // would pass for the wrong reason.
    let plain = deck_state(&source);
    let plain_layout = layout(&plain, &slides(&plain), &scene, PANEL).expect("layout");
    assert_eq!(plain_layout.actions.menu, None, "the control has no menu");
    assert!(
        plain_layout.row_at(padding).is_some(),
        "the fixture point sits over a slide row when no menu covers it"
    );

    assert_eq!(press(&mut deck, &l, padding), SlidesPress::Claimed(None));
    assert!(
        deck.editor_ui.slides_panel.export_menu_open,
        "chrome keeps the menu open"
    );
    assert_eq!(release(&mut deck, &l), SlidesRelease::Idle);
}

/// "Export all slides" queues the very file action the TopBar's PDF row
/// queues, so the two surfaces cannot write two different PDFs.
#[test]
fn exporting_all_slides_queues_the_deck_pdf_file_action() {
    use op_editor_core::editor_ui_state::{ExportFormat, FileAction};

    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.export_menu_open = true;
    let (_, l) = laid_out(&deck);
    let row = button_centre(l.actions.menu_row_rect(0).expect("the all-slides row"));

    assert_eq!(
        press_release(&mut deck, &l, row),
        SlidesRelease::ExportAllSlides
    );
    assert_eq!(deck.editor_ui.export_format, ExportFormat::Pdf);
    assert_eq!(
        deck.editor_ui.pending_file_action,
        Some(FileAction::ExportImageConfirm),
        "the same action `apply_export_quick_row` raises for its PDF row"
    );
    assert!(
        !deck.editor_ui.slides_panel.export_menu_open,
        "picking a row closes the menu"
    );
}

/// The `(N)` on the second row is the number of LISTED slides the
/// selection covers — counted over the very chips the rows are painted
/// from, so it cannot disagree with what the user sees selected.
#[test]
fn the_selected_count_follows_the_selection() {
    let mut deck = deck_state(THREE_BOARDS);
    let chips = slides(&deck);
    assert_eq!(selected_slide_count(&deck, &chips), 0);

    select_boards(&mut deck, &["slide-2"]);
    assert_eq!(selected_slide_count(&deck, &chips), 1);

    select_boards(&mut deck, &["slide-1", "slide-3"]);
    assert_eq!(selected_slide_count(&deck, &chips), 2);

    // A node that is not one of the listed boards does not count, even
    // though it is selected.
    select_boards(&mut deck, &["slide-1", "some-child-node"]);
    assert_eq!(selected_slide_count(&deck, &chips), 1);

    select_boards(&mut deck, &[]);
    assert_eq!(selected_slide_count(&deck, &chips), 0);
}

/// The count reaches the label, with `{{count}}` substituted, at every
/// value including zero.
#[test]
fn the_selected_row_label_carries_the_count() {
    let deck = deck_state(THREE_BOARDS);
    for selected in [0usize, 1, 5] {
        let text = action_labels(&deck, selected);
        assert!(
            text.export_selected.contains(&selected.to_string()),
            "label {:?} does not state the count {selected}",
            text.export_selected
        );
        assert!(
            !text.export_selected.contains("{{count}}"),
            "the placeholder survived into {:?}",
            text.export_selected
        );
    }
    // And the other three are real catalogue strings, not the raw keys.
    let text = action_labels(&deck, 0);
    for label in [text.present, text.export, text.export_all] {
        assert!(!label.starts_with("slidesPanel."), "untranslated: {label}");
        assert!(!label.is_empty());
    }
}

/// With nothing selected the second row cannot be activated at all —
/// the count says `(0)` and the row is not a target.
#[test]
fn nothing_selected_makes_the_selected_row_unclickable() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.export_menu_open = true;
    let (chips, l) = laid_out(&deck);
    assert_eq!(selected_slide_count(&deck, &chips), 0);
    assert!(!l.actions.selected_enabled);

    let row = button_centre(l.actions.menu_row_rect(1).expect("the selected row"));
    assert_eq!(l.hit(row), None, "a disabled row answers nothing");
    // Pressing it is swallowed by the menu's surface and activates
    // nothing — it does not fall through to a slide row either.
    assert_eq!(press(&mut deck, &l, row), SlidesPress::Claimed(None));
    assert_eq!(release(&mut deck, &l), SlidesRelease::Idle);
    assert!(deck.editor_ui.slides_panel.export_menu_open);
}

/// Selecting slides lights the row and picking it queues the SELECTION
/// export — a different file action from its sibling, because the two
/// write different files.
#[test]
fn exporting_selected_slides_queues_the_selection_file_action() {
    use op_editor_core::editor_ui_state::{ExportFormat, FileAction};

    let mut deck = deck_state(THREE_BOARDS);
    select_boards(&mut deck, &["slide-1", "slide-3"]);
    deck.editor_ui.slides_panel.export_menu_open = true;
    let (chips, l) = laid_out(&deck);

    assert_eq!(selected_slide_count(&deck, &chips), 2);
    assert_eq!(l.actions.selected_slides, 2, "the count is live");
    assert!(l.actions.selected_enabled, "and the row is live with it");

    let row = button_centre(l.actions.menu_row_rect(1).expect("the selected row"));
    assert_eq!(
        press_release(&mut deck, &l, row),
        SlidesRelease::ExportSelectedSlides
    );
    assert_eq!(deck.editor_ui.export_format, ExportFormat::Pdf);
    assert_eq!(
        deck.editor_ui.pending_file_action,
        Some(FileAction::ExportDeckPdfSelection),
        "the selection export is its own action, not the whole-deck one"
    );
    assert!(!deck.editor_ui.slides_panel.export_menu_open);
}

/// The boards the host will actually write are the ones the `(N)` counted
/// — one rule, asked twice. A drift here is the failure the whole design
/// is arranged to prevent: a file whose page count contradicts the label
/// the user clicked.
#[test]
fn the_exported_boards_are_exactly_the_ones_the_count_promised() {
    let mut deck = deck_state(THREE_BOARDS);
    for selection in [
        vec![],
        vec!["slide-2"],
        vec!["slide-1", "slide-3"],
        vec!["slide-1", "slide-2", "slide-3"],
        // A selected node that is not a board must not reach the export.
        vec!["slide-2", "some-inner-text"],
    ] {
        select_boards(&mut deck, &selection);
        let chips = slides(&deck);
        let boards = op_editor_core::preview_slideshow::selected_page_boards(&deck);
        assert_eq!(
            boards.len(),
            selected_slide_count(&deck, &chips),
            "selection {selection:?}: exporter takes {boards:?} but the row promised {}",
            selected_slide_count(&deck, &chips)
        );
    }
    // And they come out in page order, not selection order.
    select_boards(&mut deck, &["slide-3", "slide-1"]);
    assert_eq!(
        op_editor_core::preview_slideshow::selected_page_boards(&deck),
        vec!["slide-1".to_string(), "slide-3".to_string()],
        "pages follow the deck's order, never the order the user clicked"
    );
}

/// Leaving the slides tab takes the open menu with it — a menu painted
/// into a rail that is no longer being drawn must not come back open.
#[test]
fn switching_tabs_closes_the_export_menu() {
    let mut deck = deck_state(THREE_BOARDS);
    deck.editor_ui.slides_panel.export_menu_open = true;
    assert!(select_tab(&mut deck, LeftPanelTab::Layers));
    assert!(!deck.editor_ui.slides_panel.export_menu_open);
}
