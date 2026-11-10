//! System tab: the auto-update row and its click target.
//!
//! Split out of `agent_settings_panel_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;
use crate::widgets::agent_settings_metrics::CONTENT_TAIL_PAD;
use crate::widgets::agent_settings_rows::{row_height, tab_intro_height, RowLines, FOOTNOTE_H};

/// Row order and shape on the System tab: Appearance (one line),
/// Auto-update (two), Experimental (two), Pencil cursor (one),
/// Cursor swarm (two).
const SYSTEM_ROWS: [RowLines; 5] = [
    RowLines::One,
    RowLines::Two,
    RowLines::Two,
    RowLines::One,
    RowLines::Two,
];

/// Centre-y of System row `index`, below the tab's compact heading.
fn system_row_mid_y(rect: Rect, index: usize) -> f32 {
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    let top: f32 = content.origin.y
        + tab_intro_height(true)
        + SYSTEM_ROWS[..index]
            .iter()
            .copied()
            .map(row_height)
            .sum::<f32>();
    top + row_height(SYSTEM_ROWS[index]) / 2.0
}

#[test]
fn system_auto_update_switch_has_click_target() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    // Row order is Appearance, Auto-update, Experimental, Pencil cursor,
    // Cursor swarm.
    let point = crate::Point2D::new(
        content.origin.x + content.size.x - 18.0,
        system_row_mid_y(rect, 1),
    );

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::ToggleAutoUpdate
    );
}

#[test]
fn system_agent_swarm_switch_toggles_the_flag() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        !state.editor_ui.agent_swarm_enabled,
        "the cursor swarm ships off"
    );
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    let point = crate::Point2D::new(
        content.origin.x + content.size.x - 18.0,
        system_row_mid_y(rect, 4),
    );
    let hit = panel.hit_test(rect, point);
    assert_eq!(hit, AgentSettingsHit::ToggleAgentSwarm);

    for expected in [true, false] {
        crate::widgets::agent_settings_press_flow::apply_agent_settings_hit(
            &mut state,
            hit,
            op_editor_core::host_settings_commit::SettingsCommitScope::Operator,
            0,
        );
        assert_eq!(state.editor_ui.agent_swarm_enabled, expected);
    }
}

#[test]
fn system_tab_reserves_space_for_update_status() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);

    assert_eq!(
        panel.content_total_height(),
        tab_intro_height(true)
            + SYSTEM_ROWS.iter().copied().map(row_height).sum::<f32>()
            + FOOTNOTE_H
            + CONTENT_TAIL_PAD,
        "System tab = heading + Appearance/Auto-update/Experimental/Pencil-cursor/\
         Cursor-swarm rows + version footnote + bottom pad"
    );
}

#[test]
fn system_tab_paints_each_update_probe_result() {
    let cases = [
        (op_editor_core::UpdateStatus::Idle, "Not checked yet"),
        (op_editor_core::UpdateStatus::Checking, "Checking…"),
        (op_editor_core::UpdateStatus::UpToDate, "Up to date"),
        (
            op_editor_core::UpdateStatus::Available {
                version: "9.8.7".to_string(),
            },
            "Update available v9.8.7",
        ),
        (op_editor_core::UpdateStatus::Error, "Check failed"),
    ];

    for (status, expected) in cases {
        let mut state = EditorState::default();
        state.editor_ui.locale = op_editor_core::Locale::EnUs;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
        state.editor_ui.update_status = status;
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .text_effective_points
                .iter()
                .any(|(text, _)| text == expected),
            "System tab should paint update status {expected:?}"
        );
    }
}
