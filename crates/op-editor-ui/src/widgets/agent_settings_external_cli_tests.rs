//! Settings-modal coverage for the host capability
//! `EditorUiState::external_cli_available`.
//!
//! Mobile shells (iOS / Android / HarmonyOS) cannot spawn subprocess
//! CLIs, so the modal must hide every surface that only exists to add,
//! connect, or configure one: the Agents tab's ACP section and its
//! external-provider card list (with the section headers that introduce
//! them), and the MCP tab's terminal-integration toggle grid. What stays
//! is the built-in API-key provider section — the ONLY agent path on
//! mobile — and the MCP server card itself.
//!
//! Every assertion here is a lockstep check: a hidden block must leave no
//! live hit rect behind, and the content below it must shift up by
//! exactly the hidden height (proved by pressing the shifted control).

use crate::widgets::agent_settings_panel::{
    content_viewport, mcp_copy_config_button, AgentSettingsHit, AgentSettingsPanel,
};
use crate::widgets::agent_settings_rows::{FOOTNOTE_H, ROW_HEIGHT, SECTION_GAP, SECTION_HEADER_H};
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::{PaintCx, Widget};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{AgentSettingsTab, McpCli};
use op_editor_core::{AgentProvider, EditorState};

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;
/// Probe grid over the scrollable body. Every control on these tabs is at
/// least 24 pt tall and 64 pt wide, so an 8 pt lattice cannot step over
/// one.
const PROBE_STEP: f32 = 8.0;
/// Scroll stride for the sweep. Smaller than the shortest row so no band
/// of the document can hide between two sampled offsets.
const SCROLL_STEP: f32 = 24.0;

fn state_for(tab: AgentSettingsTab, external_cli_available: bool) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.locale = op_i18n::Locale::EnUs;
    state.editor_ui.agent_settings.tab = tab;
    state.editor_ui.external_cli_available = external_cli_available;
    state
}

fn panel_rect(state: &EditorState) -> Rect {
    AgentSettingsPanel::for_editor(state).rect(VIEWPORT_W, VIEWPORT_H)
}

/// Every distinct control hit reachable anywhere in the scrollable body,
/// across the whole scroll range. This is the "no live hit rect" oracle:
/// a control that is not in this set cannot be pressed by any gesture the
/// modal accepts.
fn reachable_hits(state: &mut EditorState) -> Vec<AgentSettingsHit> {
    let rect = panel_rect(state);
    let max_scroll = AgentSettingsPanel::for_editor(state).max_scroll(rect);
    let restore = state.editor_ui.agent_settings.scroll_y.offset;
    let mut hits: Vec<AgentSettingsHit> = Vec::new();
    let mut scroll = 0.0_f32;
    loop {
        state.editor_ui.agent_settings.scroll_y.offset = scroll;
        {
            let panel = AgentSettingsPanel::for_editor(state);
            let content = panel.resolved_content_viewport(rect);
            let mut y = content.origin.y;
            while y <= content.origin.y + content.size.y {
                let mut x = content.origin.x;
                while x <= content.origin.x + content.size.x {
                    let hit = panel.hit_test(rect, Point2D::new(x, y));
                    if !matches!(hit, AgentSettingsHit::Inside | AgentSettingsHit::Outside)
                        && !hits.contains(&hit)
                    {
                        hits.push(hit);
                    }
                    x += PROBE_STEP;
                }
                y += PROBE_STEP;
            }
        }
        if scroll >= max_scroll {
            break;
        }
        scroll = (scroll + SCROLL_STEP).min(max_scroll);
    }
    state.editor_ui.agent_settings.scroll_y.offset = restore;
    hits
}

/// Every provider card the modal offers a hover target for, across the
/// scroll range — the hover ladder's own view of the card list.
fn hovered_card_indices(state: &mut EditorState) -> Vec<usize> {
    let rect = panel_rect(state);
    let max_scroll = AgentSettingsPanel::for_editor(state).max_scroll(rect);
    let restore = state.editor_ui.agent_settings.scroll_y.offset;
    let mut cards: Vec<usize> = Vec::new();
    let mut scroll = 0.0_f32;
    loop {
        state.editor_ui.agent_settings.scroll_y.offset = scroll;
        {
            let panel = AgentSettingsPanel::for_editor(state);
            let content = panel.resolved_content_viewport(rect);
            let mut y = content.origin.y;
            while y <= content.origin.y + content.size.y {
                let mut x = content.origin.x;
                while x <= content.origin.x + content.size.x {
                    if let Some(index) = panel.card_at(rect, Point2D::new(x, y)) {
                        if !cards.contains(&index) {
                            cards.push(index);
                        }
                    }
                    x += PROBE_STEP;
                }
                y += PROBE_STEP;
            }
        }
        if scroll >= max_scroll {
            break;
        }
        scroll = (scroll + SCROLL_STEP).min(max_scroll);
    }
    state.editor_ui.agent_settings.scroll_y.offset = restore;
    cards
}

/// Every string the modal paints for `state`, across the scroll range.
fn painted_texts(state: &mut EditorState) -> Vec<String> {
    let rect = panel_rect(state);
    let max_scroll = AgentSettingsPanel::for_editor(state).max_scroll(rect);
    let restore = state.editor_ui.agent_settings.scroll_y.offset;
    let mut texts: Vec<String> = Vec::new();
    let mut scroll = 0.0_f32;
    loop {
        state.editor_ui.agent_settings.scroll_y.offset = scroll;
        {
            let panel = AgentSettingsPanel::for_editor(state);
            let mut backend = CaptureBackend::default();
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            panel.paint(&mut cx, rect);
            texts.extend(backend.texts.into_iter().map(|(text, _)| text));
        }
        if scroll >= max_scroll {
            break;
        }
        scroll = (scroll + SCROLL_STEP).min(max_scroll);
    }
    state.editor_ui.agent_settings.scroll_y.offset = restore;
    texts
}

fn connect_hits(hits: &[AgentSettingsHit]) -> Vec<AgentProvider> {
    hits.iter()
        .filter_map(|hit| match hit {
            AgentSettingsHit::Connect(provider) => Some(*provider),
            _ => None,
        })
        .collect()
}

fn cli_toggle_hits(hits: &[AgentSettingsHit]) -> Vec<McpCli> {
    hits.iter()
        .filter_map(|hit| match hit {
            AgentSettingsHit::ToggleMcpCli(cli) => Some(*cli),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// Agents tab
// ---------------------------------------------------------------------

#[test]
fn desktop_agents_tab_still_offers_every_external_provider_card() {
    let mut state = state_for(AgentSettingsTab::Agents, true);
    let hits = reachable_hits(&mut state);

    let mut connects = connect_hits(&hits);
    connects.sort_by_key(|provider| provider.index());
    let mut expected = AgentProvider::ALL.to_vec();
    expected.sort_by_key(|provider| provider.index());
    assert_eq!(
        connects, expected,
        "the desktop card list must stay byte-identical: every provider keeps a Connect target"
    );
    assert!(
        hits.contains(&AgentSettingsHit::AddAcpAgent),
        "the ACP section's add action belongs to the external-CLI half"
    );
    assert!(
        hits.contains(&AgentSettingsHit::AddProvider),
        "the built-in API-key section's add action must be reachable on desktop"
    );
}

#[test]
fn hiding_external_clis_removes_every_agents_tab_cli_target() {
    let mut state = state_for(AgentSettingsTab::Agents, false);
    let hits = reachable_hits(&mut state);

    assert!(
        connect_hits(&hits).is_empty(),
        "a hidden provider card must not leave a live Connect rect: {hits:?}"
    );
    for hit in &hits {
        assert!(
            !matches!(
                hit,
                AgentSettingsHit::AddAcpAgent
                    | AgentSettingsHit::AddAcpPreset(_)
                    | AgentSettingsHit::ToggleAcpConnected(_)
                    | AgentSettingsHit::EditAcpAgent(_)
                    | AgentSettingsHit::RemoveAcpAgent(_)
                    | AgentSettingsHit::FocusAcpAgent { .. }
                    | AgentSettingsHit::FocusAcpAgentDraft(_)
                    | AgentSettingsHit::SaveAcpAgentDraft
                    | AgentSettingsHit::CancelAcpAgentDraft
            ),
            "{hit:?} is an external-CLI target and must be gone with the section"
        );
    }
}

/// Anything the built-in API-key section owns. Its geometry sits ABOVE
/// the external-CLI half, so hiding that half must leave this set
/// untouched — same controls, same rects.
fn builtin_hits(hits: &[AgentSettingsHit]) -> Vec<String> {
    let mut out: Vec<String> = hits
        .iter()
        .filter(|hit| {
            matches!(
                hit,
                AgentSettingsHit::AddProvider
                    | AgentSettingsHit::FocusBuiltinAgent { .. }
                    | AgentSettingsHit::FocusBuiltinAgentDraft(_)
                    | AgentSettingsHit::ToggleBuiltinAgentKind(_)
                    | AgentSettingsHit::ToggleBuiltinAgentDraftKind
                    | AgentSettingsHit::ToggleBuiltinAgentPresetMenu(_)
                    | AgentSettingsHit::SelectBuiltinAgentPreset { .. }
                    | AgentSettingsHit::ToggleBuiltinModelMenu(_)
                    | AgentSettingsHit::SelectBuiltinModel { .. }
                    | AgentSettingsHit::SaveBuiltinAgentDraft
                    | AgentSettingsHit::CancelBuiltinAgentDraft
                    | AgentSettingsHit::ToggleBuiltinAgentEnabled(_)
                    | AgentSettingsHit::EditBuiltinAgent(_)
                    | AgentSettingsHit::RemoveBuiltinAgent(_)
            )
        })
        .map(|hit| format!("{hit:?}"))
        .collect();
    out.sort();
    out
}

#[test]
fn hiding_external_clis_leaves_the_builtin_agent_section_untouched() {
    for seed in [false, true] {
        let mut with_clis = state_for(AgentSettingsTab::Agents, true);
        let mut without_clis = state_for(AgentSettingsTab::Agents, false);
        if seed {
            for state in [&mut with_clis, &mut without_clis] {
                state
                    .editor_ui
                    .agent_settings
                    .add_builtin_agent_with_defaults("Provider", "sk-test", "model");
            }
        }

        let desktop = builtin_hits(&reachable_hits(&mut with_clis));
        let mobile = builtin_hits(&reachable_hits(&mut without_clis));

        assert!(
            !mobile.is_empty(),
            "built-in API-key providers are the only agent path on mobile — \
             their controls must survive"
        );
        assert_eq!(
            desktop, mobile,
            "the built-in section must be identical with and without external CLIs \
             (seeded agent: {seed})"
        );
    }
}

#[test]
fn hiding_external_clis_removes_every_provider_card_hover_target() {
    let mut with_clis = state_for(AgentSettingsTab::Agents, true);
    assert_eq!(
        hovered_card_indices(&mut with_clis).len(),
        AgentProvider::ALL.len(),
        "desktop hover still covers every card"
    );

    let mut without_clis = state_for(AgentSettingsTab::Agents, false);
    assert!(
        hovered_card_indices(&mut without_clis).is_empty(),
        "no card is painted, so `card_at` must never report one"
    );
}

#[test]
fn hiding_external_clis_shortens_the_agents_scroll_range_and_stops_painting_names() {
    let with_clis = AgentSettingsPanel::for_editor(&state_for(AgentSettingsTab::Agents, true))
        .content_total_height();
    let state_without = state_for(AgentSettingsTab::Agents, false);
    let without_clis = AgentSettingsPanel::for_editor(&state_without).content_total_height();
    assert!(
        without_clis < with_clis,
        "the hidden block must give its height back to the scroll range: \
         {without_clis} is not shorter than {with_clis}"
    );

    let mut state = state_for(AgentSettingsTab::Agents, false);
    let texts = painted_texts(&mut state);
    for provider in AgentProvider::ALL {
        assert!(
            !texts.iter().any(|text| text.contains(provider.name())),
            "{:?} is still painted on the Agents tab",
            provider.name()
        );
    }

    let mut state = state_for(AgentSettingsTab::Agents, true);
    let texts = painted_texts(&mut state);
    for provider in AgentProvider::ALL {
        assert!(
            texts.iter().any(|text| text.contains(provider.name())),
            "desktop must keep painting {:?}",
            provider.name()
        );
    }
}

// ---------------------------------------------------------------------
// MCP tab
// ---------------------------------------------------------------------

fn mcp_state(external_cli_available: bool) -> EditorState {
    let mut state = state_for(AgentSettingsTab::Mcp, external_cli_available);
    // A listening server is what makes the custom-configuration section
    // exist, which is the block that has to shift up by the hidden
    // integrations height.
    state.editor_ui.agent_settings.mcp_server.running = true;
    state
}

#[test]
fn desktop_mcp_tab_still_offers_every_cli_toggle() {
    let mut state = mcp_state(true);
    let hits = reachable_hits(&mut state);

    let mut toggles = cli_toggle_hits(&hits);
    toggles.sort_by_key(|cli| cli.index());
    let mut expected = McpCli::ALL.to_vec();
    expected.sort_by_key(|cli| cli.index());
    assert_eq!(
        toggles, expected,
        "the desktop toggle grid must stay byte-identical"
    );
}

#[test]
fn hiding_external_clis_removes_every_mcp_cli_toggle_but_keeps_the_server_card() {
    let mut state = mcp_state(false);
    let hits = reachable_hits(&mut state);

    assert!(
        cli_toggle_hits(&hits).is_empty(),
        "a hidden toggle row must not leave a live rect: {hits:?}"
    );
    assert!(
        hits.contains(&AgentSettingsHit::ToggleMcpServer),
        "the MCP server card's Start/Stop stays: {hits:?}"
    );
    assert!(
        hits.contains(&AgentSettingsHit::CopyMcpClientConfig),
        "the custom-configuration copy action stays: {hits:?}"
    );

    // The port field is only editable while the server is stopped.
    let mut stopped = state_for(AgentSettingsTab::Mcp, false);
    stopped.editor_ui.agent_settings.mcp_server.running = false;
    let hits = reachable_hits(&mut stopped);
    assert!(
        hits.contains(&AgentSettingsHit::FocusMcpPort),
        "the MCP port input stays editable: {hits:?}"
    );

    let mut state = mcp_state(false);
    let texts = painted_texts(&mut state);
    for cli in McpCli::ALL {
        assert!(
            !texts.iter().any(|text| text.contains(cli.label())),
            "{:?} is still painted on the MCP tab",
            cli.label()
        );
    }
}

#[test]
fn hidden_mcp_toggles_shift_the_custom_config_section_up_by_exactly_their_height() {
    let with_clis = mcp_state(true);
    let without_clis = mcp_state(false);
    let rect = panel_rect(&with_clis);
    assert_eq!(rect, panel_rect(&without_clis), "the shell must not move");

    let copy_with = mcp_copy_config_button(rect, true);
    let copy_without = mcp_copy_config_button(rect, false);
    assert_eq!(copy_with.origin.x, copy_without.origin.x);
    assert_eq!(copy_with.size, copy_without.size);

    // Section header + one row per displayed CLI + the footnote, plus the
    // gap that separated the block from what follows it.
    let hidden_block =
        SECTION_GAP + SECTION_HEADER_H + McpCli::DISPLAY.len() as f32 * ROW_HEIGHT + FOOTNOTE_H;
    assert!(
        (copy_with.origin.y - copy_without.origin.y - hidden_block).abs() < 0.01,
        "content below the hidden toggle grid must shift up by exactly {hidden_block}, \
         not {}",
        copy_with.origin.y - copy_without.origin.y
    );

    // The content height gives back the same amount.
    let height_with = AgentSettingsPanel::for_editor(&with_clis).content_total_height();
    let height_without = AgentSettingsPanel::for_editor(&without_clis).content_total_height();
    assert!(
        (height_with - height_without - hidden_block).abs() < 0.01,
        "the scroll range must shrink by the same {hidden_block}, not {}",
        height_with - height_without
    );
}

#[test]
fn pressing_where_a_hidden_mcp_toggle_used_to_be_does_nothing() {
    let with_clis = mcp_state(true);
    let rect = panel_rect(&with_clis);
    let content = content_viewport(rect);
    // Row centres of the desktop toggle grid, in unscrolled document
    // space — the exact points that used to flip a CLI.
    let rows: Vec<Rect> = (0..McpCli::DISPLAY.len())
        .map(|index| crate::widgets::agent_settings_mcp::cli_row_rect(content, index))
        .collect();

    let mut state = mcp_state(false);
    for row in rows {
        let scroll = (row.origin.y - content.origin.y).max(0.0);
        state.editor_ui.agent_settings.scroll_y.offset = scroll;
        let panel = AgentSettingsPanel::for_editor(&state);
        let effective = panel.effective_scroll(rect);
        let point = Point2D::new(
            row.origin.x + row.size.x / 2.0,
            row.origin.y + row.size.y / 2.0 - effective,
        );
        if !panel.resolved_content_viewport(rect).contains(point) {
            continue;
        }
        assert!(
            !matches!(
                panel.hit_test(rect, point),
                AgentSettingsHit::ToggleMcpCli(_)
            ),
            "a press at the old toggle row still flips a CLI"
        );
    }
}
