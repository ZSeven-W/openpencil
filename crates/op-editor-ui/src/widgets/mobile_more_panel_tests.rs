use super::*;
use crate::widgets::test_family_gap_backend::FamilyGapBackend;
use op_editor_core::size_class::EditorSizeClass;

fn touch_state(size_class: EditorSizeClass) -> EditorState {
    let mut state = EditorState::starter();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class;
    state
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected {expected}, got {actual}"
    );
}

fn assert_grid(
    state: &EditorState,
    viewport_w: f32,
    viewport_h: f32,
    expected_columns: usize,
    expected_bottom_padding: f32,
) {
    let panel = more_panel_rect(state, viewport_w, viewport_h);
    let entries = MobileMoreEntry::visible(state);
    assert_eq!(column_count(state, panel), expected_columns);
    assert_approx(
        panel.size.y,
        panel_height(entries.len(), expected_columns, expected_bottom_padding),
    );

    let rows = row_count(entries.len(), expected_columns);
    for (index, entry) in entries.iter().copied().enumerate() {
        let tile = more_entry_rect(state, panel, index);
        assert!(tile.size.x >= 44.0, "tile {index} is too narrow: {tile:?}");
        assert!(tile.size.y >= 44.0, "tile {index} is too short: {tile:?}");
        assert!(tile.origin.x >= panel.origin.x);
        assert!(tile.origin.y >= panel.origin.y);
        assert!(tile.origin.x + tile.size.x <= panel.origin.x + panel.size.x + 0.01);
        assert!(tile.origin.y + tile.size.y <= panel.origin.y + panel.size.y + 0.01);

        let center = Point2D::new(
            tile.origin.x + tile.size.x / 2.0,
            tile.origin.y + tile.size.y / 2.0,
        );
        assert_eq!(more_hit_test(state, panel, center), Some(entry));
    }

    let last_row_start = (rows - 1) * expected_columns;
    let first = more_entry_rect(state, panel, last_row_start);
    let last = more_entry_rect(state, panel, entries.len() - 1);
    let left_gap = first.origin.x - panel.origin.x - PANEL_PADDING;
    let right_gap = panel.origin.x + panel.size.x - PANEL_PADDING - last.origin.x - last.size.x;
    assert_approx(left_gap, right_gap);
}

fn rect_for_entry(state: &EditorState, panel: Rect, want: MobileMoreEntry) -> Rect {
    let index = MobileMoreEntry::visible(state)
        .iter()
        .position(|entry| *entry == want)
        .expect("entry is visible");
    more_entry_rect(state, panel, index)
}

#[test]
fn compact_portrait_uses_a_short_hierarchical_touch_layout() {
    let state = touch_state(EditorSizeClass::Compact);
    let panel = more_panel_rect(&state, 320.0, 568.0);
    assert_eq!(panel.origin.x, 0.0);
    assert_eq!(panel.size.x, 320.0);
    assert_eq!(panel.size.y, PHONE_PORTRAIT_PANEL_HEIGHT);
    assert_eq!(panel.origin.y + panel.size.y, 568.0);
    assert!(
        panel.size.y
            < panel_height(
                MobileMoreEntry::visible(&state).len(),
                PORTRAIT_COLUMN_COUNT,
                PHONE_BOTTOM_PADDING,
            ),
        "the hierarchical sheet must expose more canvas than the old five-row grid"
    );

    let entries = MobileMoreEntry::visible(&state);
    for (index, entry) in entries.iter().copied().enumerate() {
        let target = more_entry_rect(&state, panel, index);
        assert!(target.size.x >= 44.0, "{entry:?} is too narrow: {target:?}");
        assert!(target.size.y >= 44.0, "{entry:?} is too short: {target:?}");
        assert!(panel.contains(Point2D::new(
            target.origin.x + target.size.x / 2.0,
            target.origin.y + target.size.y / 2.0,
        )));
        assert_eq!(
            more_hit_test(
                &state,
                panel,
                Point2D::new(
                    target.origin.x + target.size.x / 2.0,
                    target.origin.y + target.size.y / 2.0,
                ),
            ),
            Some(entry)
        );
    }

    let files = [
        MobileMoreEntry::NewFile,
        MobileMoreEntry::OpenFile,
        MobileMoreEntry::SaveFile,
        MobileMoreEntry::Export,
    ]
    .map(|entry| rect_for_entry(&state, panel, entry));
    for pair in files.windows(2) {
        assert_eq!(pair[0].origin.y, pair[1].origin.y);
        assert_approx(pair[0].origin.x + pair[0].size.x, pair[1].origin.x);
    }

    let ai = rect_for_entry(&state, panel, MobileMoreEntry::Ai);
    let templates = rect_for_entry(&state, panel, MobileMoreEntry::Templates);
    let assets = rect_for_entry(&state, panel, MobileMoreEntry::Assets);
    assert!(ai.size.x > templates.size.x * 2.0);
    assert_eq!(templates.origin.y, assets.origin.y);
    assert_eq!(templates.size, assets.size);

    for (left, right) in [
        (MobileMoreEntry::Collaboration, MobileMoreEntry::SignIn),
        (MobileMoreEntry::SaveAsFile, MobileMoreEntry::Variables),
        (MobileMoreEntry::Language, MobileMoreEntry::Settings),
    ] {
        let left = rect_for_entry(&state, panel, left);
        let right = rect_for_entry(&state, panel, right);
        assert_eq!(left.origin.y, right.origin.y);
        assert_eq!(left.size, right.size);
        assert!(left.origin.x + left.size.x < right.origin.x);
    }
    let settings = rect_for_entry(&state, panel, MobileMoreEntry::Settings);
    assert_approx(
        settings.origin.y + settings.size.y,
        panel.origin.y + panel.size.y - 16.0,
    );
}

#[test]
fn short_narrow_portrait_uses_four_columns_without_tiny_targets() {
    let state = touch_state(EditorSizeClass::Compact);
    for height in [350.0, 400.0] {
        let panel = more_panel_rect(&state, 320.0, height);
        assert!(!uses_phone_portrait_layout(&state, 320.0, height));
        assert_eq!(column_count(&state, panel), PORTRAIT_FALLBACK_COLUMN_COUNT);
        for index in 0..MobileMoreEntry::visible(&state).len() {
            let target = more_entry_rect(&state, panel, index);
            assert!(target.size.x >= 44.0, "target {index} is too narrow");
            assert!(target.size.y >= 44.0, "target {index} is too short");
            assert!(
                target.origin.y + target.size.y <= panel.origin.y + panel.size.y + 0.01,
                "target {index} overflows height {height}: {target:?}"
            );
        }
    }
}

#[test]
fn compact_landscape_uses_a_seven_by_two_sheet() {
    let state = touch_state(EditorSizeClass::Compact);
    assert_grid(&state, 568.0, 320.0, 7, PHONE_BOTTOM_PADDING);
}

#[test]
fn medium_and_expanded_keep_a_bounded_three_column_popover() {
    for (class, width, height) in [
        (EditorSizeClass::Medium, 600.0, 900.0),
        (EditorSizeClass::Expanded, 960.0, 600.0),
    ] {
        let state = touch_state(class);
        assert_grid(&state, width, height, 3, TABLET_BOTTOM_PADDING);
        let panel = more_panel_rect(&state, width, height);
        assert_eq!(panel.size.x, TABLET_PANEL_WIDTH);
        assert!(panel.origin.x + panel.size.x / 2.0 > width / 2.0);
        assert!(panel.origin.y >= host_canvas_geometry::TABLET_APP_BAR_HEIGHT);
    }
}

#[test]
fn code_entry_is_visible_only_on_touch_tablets() {
    let compact = touch_state(EditorSizeClass::Compact);
    assert!(!MobileMoreEntry::visible(&compact).contains(&MobileMoreEntry::Code));

    for class in [EditorSizeClass::Medium, EditorSizeClass::Expanded] {
        let state = touch_state(class);
        let entries = MobileMoreEntry::visible(&state);
        assert_eq!(entries.len(), 14);
        assert!(entries.contains(&MobileMoreEntry::Code));
    }

    // The widget is mobile-only; constructing its model for a desktop state
    // must not inject a destination that desktop chrome never paints.
    let desktop = EditorState::starter();
    assert!(!MobileMoreEntry::visible(&desktop).contains(&MobileMoreEntry::Code));
}

#[test]
fn restored_entries_reuse_localized_labels_and_desktop_icons() {
    let mut state = EditorState::starter();
    assert_eq!(MobileMoreEntry::ALL.len(), 15);
    assert_eq!(MobileMoreEntry::visible(&state).len(), 13);
    assert_eq!(MobileMoreEntry::ALL[0], MobileMoreEntry::NewFile);
    assert_eq!(MobileMoreEntry::ALL[1], MobileMoreEntry::OpenFile);
    assert_eq!(MobileMoreEntry::ALL[2], MobileMoreEntry::SaveFile);
    assert_eq!(MobileMoreEntry::ALL[3], MobileMoreEntry::SaveAsFile);
    assert_eq!(MobileMoreEntry::NewFile.icon(), Icon::FilePlus);
    assert_eq!(MobileMoreEntry::OpenFile.icon(), Icon::FolderOpen);
    assert_eq!(MobileMoreEntry::SaveFile.icon(), Icon::Save);
    assert_eq!(MobileMoreEntry::SaveAsFile.icon(), Icon::Copy);
    assert_eq!(MobileMoreEntry::Templates.icon(), Icon::LayoutDashboard);
    assert_eq!(MobileMoreEntry::Assets.icon(), Icon::Palette);
    assert_eq!(MobileMoreEntry::Code.icon(), Icon::Braces);

    for locale in op_i18n::Locale::ALL {
        state.editor_ui.locale = locale;
        assert_ne!(
            MobileMoreEntry::NewFile.label(&state.editor_ui),
            "fileMenu.newFile"
        );
        let label = MobileMoreEntry::OpenFile.label(&state.editor_ui);
        assert!(!label.ends_with('.'));
        assert!(!label.ends_with('…'));
        assert_ne!(label, "fileMenu.openFile");
        assert_ne!(
            MobileMoreEntry::SaveFile.label(&state.editor_ui),
            "fileMenu.save"
        );
        let save_as = MobileMoreEntry::SaveAsFile.label(&state.editor_ui);
        assert!(!save_as.ends_with('.'));
        assert!(!save_as.ends_with('…'));
        assert_ne!(save_as, "fileMenu.saveAs");
        assert_ne!(
            MobileMoreEntry::Templates.label(&state.editor_ui),
            "sceneTemplate.title"
        );
        assert_ne!(
            MobileMoreEntry::Assets.label(&state.editor_ui),
            "assetCenter.title"
        );
        assert_ne!(
            MobileMoreEntry::Code.label(&state.editor_ui),
            "rightPanel.code"
        );
    }
}

/// The mobile editor has no Run/Preview mode; the sheet must not offer
/// one (the tile was removed together with its press arm).
#[test]
fn run_preview_is_not_offered_on_touch() {
    let state = touch_state(EditorSizeClass::Compact);
    for entry in MobileMoreEntry::visible(&state) {
        assert_ne!(entry.icon(), Icon::Play, "{entry:?} looks like Run");
    }
}

#[test]
fn account_state_swaps_one_tile_without_moving_collaboration_or_changing_count() {
    let mut state = touch_state(EditorSizeClass::Compact);
    let anonymous = MobileMoreEntry::visible(&state);
    assert_eq!(anonymous.len(), 13);
    assert!(anonymous.contains(&MobileMoreEntry::SignIn));
    assert!(!anonymous.contains(&MobileMoreEntry::Account));
    assert_eq!(anonymous[7], MobileMoreEntry::Collaboration);
    assert_eq!(MobileMoreEntry::SignIn.icon(), Icon::User);
    assert_eq!(MobileMoreEntry::Collaboration.icon(), Icon::Users);
    assert_ne!(
        MobileMoreEntry::SignIn.label(&state.editor_ui),
        "settings.account.signIn"
    );
    assert_ne!(
        MobileMoreEntry::Collaboration.label(&state.editor_ui),
        "collab.topbar.collaborate"
    );

    state.editor_ui.account = op_editor_core::AccountState::SignedIn {
        display_name: "Fini".into(),
        username: "fini".into(),
    };
    let signed_in = MobileMoreEntry::visible(&state);
    assert_eq!(signed_in.len(), anonymous.len());
    assert!(!signed_in.contains(&MobileMoreEntry::SignIn));
    assert!(signed_in.contains(&MobileMoreEntry::Account));
    assert_eq!(signed_in[7], MobileMoreEntry::Collaboration);
    assert_eq!(
        signed_in
            .iter()
            .position(|entry| *entry == MobileMoreEntry::Account),
        anonymous
            .iter()
            .position(|entry| *entry == MobileMoreEntry::SignIn)
    );
}

#[test]
fn every_locale_fits_labels_using_the_painted_font_family() {
    let mut state = touch_state(EditorSizeClass::Medium);
    let panel = more_panel_rect(&state, 600.0, 900.0);

    for locale in op_i18n::Locale::ALL {
        state.editor_ui.locale = locale;
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_more_panel(&mut cx, &state, &Theme::dark(), panel);

        let visible_count = MobileMoreEntry::visible(&state).len();
        assert_eq!(backend.runs.len(), visible_count + 1);
        for index in 0..visible_count {
            let tile = more_entry_rect(&state, panel, index);
            let run = &backend.runs[index + 1];
            assert!(
                run.origin.x >= tile.origin.x + LABEL_SIDE_PADDING - 0.01,
                "{} label {index} starts outside its inset: {run:?}",
                locale.code()
            );
            assert!(
                run.right_edge() <= tile.origin.x + tile.size.x - LABEL_SIDE_PADDING + 0.01,
                "{} label {index} overflows its tile: {run:?}",
                locale.code()
            );
        }
    }
}

#[test]
fn compact_portrait_every_locale_fits_and_keeps_the_intended_weight_hierarchy() {
    let mut state = touch_state(EditorSizeClass::Compact);
    let panel = more_panel_rect(&state, 390.0, 782.0);

    for locale in op_i18n::Locale::ALL {
        state.editor_ui.locale = locale;
        let mut backend = FamilyGapBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_more_panel(&mut cx, &state, &Theme::dark(), panel);

        assert_eq!(
            backend.runs.len(),
            MobileMoreEntry::visible(&state).len() + 1,
            "{} must paint one title plus every visible action",
            locale.code()
        );
        assert!(
            backend.overflowing(panel).is_empty(),
            "{} has a compact portrait label outside its target: {:?}",
            locale.code(),
            backend.overflowing(panel)
        );
        assert_eq!(backend.runs[0].font_weight, 600);
        let ai_label = MobileMoreEntry::Ai.label(&state.editor_ui);
        let ai_run = backend
            .runs
            .iter()
            .find(|run| run.text == ai_label)
            .expect("AI label is painted without truncation");
        assert_eq!(ai_run.font_weight, 600);
    }
}
